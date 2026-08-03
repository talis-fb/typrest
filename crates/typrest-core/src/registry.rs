use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use thiserror::Error;

use crate::{
    CompilationError, LoaderError, PdfCompiler, TemplateId, TemplateLoader, ValidationError,
    WithWarnings, validate_with_defaults,
};

use crate::compiler::PreparedTemplateBundle;

#[derive(Clone, Default)]
pub struct TemplateRegistry {
    entries: Arc<RwLock<BTreeMap<TemplateId, Arc<PreparedTemplateBundle>>>>,
}

impl TemplateRegistry {
    pub async fn build(
        loader: &dyn TemplateLoader,
        compiler: &dyn PdfCompiler,
    ) -> Result<WithWarnings<TemplateRegistry>, WithWarnings<TemplateRegistryBuildFailure>> {
        let names = match loader.list().await {
            Ok(names) => names,
            Err(source) => {
                return Err(WithWarnings::new(
                    TemplateRegistryBuildFailure {
                        template_count: 0,
                        failures: vec![TemplateRegistryError::List { source }],
                    },
                    Vec::new(),
                ));
            }
        };
        let template_count = names.len();
        let registry = TemplateRegistry::default();
        let mut failures = Vec::new();
        let mut warnings = Vec::new();

        for name in names {
            match prepare_template_bundle(loader, compiler, &name).await {
                Ok(bundle) => {
                    let (bundle_prepared_template, bundle_warnings) = bundle.into_parts();
                    registry.replace(name, bundle_prepared_template);
                    warnings.extend(bundle_warnings);
                }
                Err(error) => failures.push(error),
            }
        }

        if failures.is_empty() {
            Ok(WithWarnings::new(registry, warnings))
        } else {
            Err(WithWarnings::new(
                TemplateRegistryBuildFailure {
                    template_count,
                    failures,
                },
                warnings,
            ))
        }
    }

    pub async fn reload(
        &self,
        loader: &dyn TemplateLoader,
        compiler: &dyn PdfCompiler,
        template_id: &TemplateId,
    ) -> Result<WithWarnings<()>, TemplateRegistryError> {
        let bundle = prepare_template_bundle(loader, compiler, template_id).await?;

        let (prepared_template_bundle, warnings) = bundle.into_parts();

        self.replace(template_id.clone(), prepared_template_bundle);
        Ok(WithWarnings::new((), warnings))
    }

    pub fn get(&self, template_id: &TemplateId) -> Option<Arc<PreparedTemplateBundle>> {
        self.entries
            .read()
            .expect("template registry lock poisoned")
            .get(template_id)
            .cloned()
    }

    pub fn ids(&self) -> Vec<TemplateId> {
        self.entries
            .read()
            .expect("template registry lock poisoned")
            .keys()
            .cloned()
            .collect()
    }

    pub fn replace_all(&self, candidate: &TemplateRegistry) {
        let entries = candidate
            .entries
            .read()
            .expect("candidate template registry lock poisoned")
            .clone();
        *self
            .entries
            .write()
            .expect("template registry lock poisoned") = entries;
    }

    fn replace(&self, template_id: TemplateId, entry: PreparedTemplateBundle) {
        self.entries
            .write()
            .expect("template registry lock poisoned")
            .insert(template_id, Arc::new(entry));
    }

    #[cfg(any(test, feature = "test-util"))]
    #[doc(hidden)]
    pub fn insert_for_test(&self, template_id: TemplateId, entry: PreparedTemplateBundle) {
        self.replace(template_id, entry);
    }
}

async fn prepare_template_bundle(
    loader: &dyn TemplateLoader,
    compiler: &dyn PdfCompiler,
    template_id: &TemplateId,
) -> Result<WithWarnings<PreparedTemplateBundle>, TemplateRegistryError> {
    let bundle = loader
        .load(template_id)
        .await
        .map_err(|source| TemplateRegistryError::Load {
            template: template_id.clone(),
            source,
        })?;

    let prepared = compiler.prepare(&bundle.source).await.map_err(|source| {
        TemplateRegistryError::Prepare {
            template: template_id.clone(),
            source,
        }
    })?;
    let sample = match bundle.schema.as_ref() {
        Some(schema) => {
            let sample = schema.sample_payload();
            validate_with_defaults(schema, &sample).map_err(|violations| {
                TemplateRegistryError::SamplePayload {
                    template: template_id.clone(),
                    violations,
                }
            })?
        }
        None => serde_json::json!({}),
    };

    let document = compiler
        .compile_prepared(&prepared, &sample)
        .await
        .map_err(|source| TemplateRegistryError::Warmup {
            template: template_id.clone(),
            source,
        })?;

    let warnings = document
        .warnings()
        .iter()
        .cloned()
        .map(|mut diagnostic| {
            diagnostic.template = Some(template_id.clone());
            diagnostic
        })
        .collect();

    let prepared_bundle = match bundle.schema {
        Some(schema) => PreparedTemplateBundle::new(schema, prepared),
        None => PreparedTemplateBundle::without_schema(prepared),
    };

    Ok(WithWarnings::new(prepared_bundle, warnings))
}

#[derive(Debug, Error)]
pub enum TemplateRegistryError {
    #[error("failed to list templates during startup: {source}")]
    List {
        #[source]
        source: LoaderError,
    },

    #[error("template '{template}' failed to load: {source}")]
    Load {
        template: TemplateId,
        #[source]
        source: LoaderError,
    },

    #[error("template '{template}' failed to prepare: {source}")]
    Prepare {
        template: TemplateId,
        #[source]
        source: CompilationError,
    },

    #[error("template '{template}' could not generate a valid startup payload: {violations:?}")]
    SamplePayload {
        template: TemplateId,
        violations: Vec<ValidationError>,
    },

    #[error("template '{template}' failed warmup compilation: {source}")]
    Warmup {
        template: TemplateId,
        #[source]
        source: CompilationError,
    },
}

#[derive(Debug, Error)]
#[error("{} of {} templates failed to compile", .failures.len(), .template_count)]
pub struct TemplateRegistryBuildFailure {
    template_count: usize,
    failures: Vec<TemplateRegistryError>,
}

impl TemplateRegistryBuildFailure {
    pub fn template_count(&self) -> usize {
        self.template_count
    }

    pub fn failures(&self) -> &[TemplateRegistryError] {
        &self.failures
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;
    use bytes::Bytes;
    use serde_json::json;

    use crate::{
        CompilationDiagnostic, CompilationError, DiagnosticSeverity, Document, LoaderError,
        PdfCompiler, PreparedTemplate, RenderRequest, RenderService, Schema, TemplateBundle,
        TemplateId, TemplateLoader, TemplateSource,
    };

    use super::TemplateRegistry;

    struct CountingLoader {
        bundle: TemplateBundle,
        loads: AtomicUsize,
    }

    #[async_trait]
    impl TemplateLoader for CountingLoader {
        async fn load(&self, _: &TemplateId) -> Result<TemplateBundle, LoaderError> {
            self.loads.fetch_add(1, Ordering::SeqCst);
            Ok(self.bundle.clone())
        }

        async fn list(&self) -> Result<Vec<TemplateId>, LoaderError> {
            Ok(vec![TemplateId::new("invoice").unwrap()])
        }
    }

    struct CountingCompiler {
        prepares: AtomicUsize,
        compiles: AtomicUsize,
    }

    struct FailingLoader;

    #[async_trait]
    impl TemplateLoader for FailingLoader {
        async fn load(&self, name: &TemplateId) -> Result<TemplateBundle, LoaderError> {
            Err(LoaderError::IoError(format!("{name} is broken")))
        }

        async fn list(&self) -> Result<Vec<TemplateId>, LoaderError> {
            Ok(vec![
                TemplateId::new("first").unwrap(),
                TemplateId::new("second").unwrap(),
            ])
        }
    }

    struct WarningCompiler;

    #[async_trait]
    impl PdfCompiler for WarningCompiler {
        async fn prepare(&self, _: &TemplateSource) -> Result<PreparedTemplate, CompilationError> {
            Ok(PreparedTemplate::for_test())
        }

        async fn compile_prepared(
            &self,
            _: &PreparedTemplate,
            _: &serde_json::Value,
        ) -> Result<Document, CompilationError> {
            Ok(Document::with_warnings(
                Bytes::from_static(b"%PDF-1.4 mock"),
                vec![CompilationDiagnostic::new(
                    DiagnosticSeverity::Warning,
                    "careful",
                )],
            ))
        }
    }

    #[async_trait]
    impl PdfCompiler for CountingCompiler {
        async fn prepare(&self, _: &TemplateSource) -> Result<PreparedTemplate, CompilationError> {
            self.prepares.fetch_add(1, Ordering::SeqCst);
            Ok(PreparedTemplate::for_test())
        }

        async fn compile_prepared(
            &self,
            _: &PreparedTemplate,
            _: &serde_json::Value,
        ) -> Result<Document, CompilationError> {
            self.compiles.fetch_add(1, Ordering::SeqCst);
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock")))
        }
    }

    fn bundle() -> TemplateBundle {
        let schema = Schema::parse_json(
            json!({
                "schemaVersion": "v1",
                "request": {
                    "body": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "required": true }
                        }
                    }
                }
            })
            .to_string()
            .as_bytes(),
        )
        .unwrap();
        let source = TemplateSource::new(BTreeMap::from([(
            "main.typ".to_owned(),
            Bytes::from_static(b"Hello"),
        )]));
        TemplateBundle::new(source, schema)
    }

    #[test]
    fn build_warms_entries_and_render_reuses_prepared_state() {
        let loader = Arc::new(CountingLoader {
            bundle: bundle(),
            loads: AtomicUsize::new(0),
        });
        let compiler = Arc::new(CountingCompiler {
            prepares: AtomicUsize::new(0),
            compiles: AtomicUsize::new(0),
        });
        let build =
            futures_executor::block_on(TemplateRegistry::build(loader.as_ref(), compiler.as_ref()))
                .unwrap();
        let registry = build.into_value();

        assert_eq!(loader.loads.load(Ordering::SeqCst), 1);
        assert_eq!(compiler.prepares.load(Ordering::SeqCst), 1);
        assert_eq!(compiler.compiles.load(Ordering::SeqCst), 1);
        assert_eq!(
            registry
                .ids()
                .iter()
                .map(TemplateId::as_str)
                .collect::<Vec<_>>(),
            vec!["invoice"]
        );

        let service = RenderService::new(registry, compiler.clone());
        let document = futures_executor::block_on(service.render(RenderRequest::new(
            TemplateId::new("invoice").unwrap(),
            json!({ "name": "Ada" }),
        )))
        .unwrap();

        assert!(document.bytes().starts_with(b"%PDF"));
        assert_eq!(loader.loads.load(Ordering::SeqCst), 1);
        assert_eq!(compiler.prepares.load(Ordering::SeqCst), 1);
        assert_eq!(compiler.compiles.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn invalid_generated_payload_fails_registry_build() {
        let mut invalid_bundle = bundle();
        invalid_bundle.schema = Some(
            Schema::parse_json(
                json!({
                    "schemaVersion": "v1",
                    "request": {
                        "body": {
                            "type": "string",
                            "default": "invalid",
                            "enum": ["valid"]
                        }
                    }
                })
                .to_string()
                .as_bytes(),
            )
            .unwrap(),
        );
        let loader = CountingLoader {
            bundle: invalid_bundle,
            loads: AtomicUsize::new(0),
        };
        let compiler = CountingCompiler {
            prepares: AtomicUsize::new(0),
            compiles: AtomicUsize::new(0),
        };

        let error = match futures_executor::block_on(TemplateRegistry::build(&loader, &compiler)) {
            Ok(_) => panic!("invalid generated sample should fail registry build"),
            Err(error) => error,
        };

        assert!(matches!(
            error.value().failures(),
            [super::TemplateRegistryError::SamplePayload { template, .. }]
                if template.as_str() == "invoice"
        ));
        assert_eq!(compiler.compiles.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn build_aggregates_failures_in_template_order() {
        let compiler = CountingCompiler {
            prepares: AtomicUsize::new(0),
            compiles: AtomicUsize::new(0),
        };

        let error =
            match futures_executor::block_on(TemplateRegistry::build(&FailingLoader, &compiler)) {
                Ok(_) => panic!("both templates should fail"),
                Err(error) => error,
            };

        assert_eq!(error.value().template_count(), 2);
        assert_eq!(error.value().failures().len(), 2);
        assert!(matches!(
            &error.value().failures()[0],
            super::TemplateRegistryError::Load { template, .. }
                if template.as_str() == "first"
        ));
        assert!(matches!(
            &error.value().failures()[1],
            super::TemplateRegistryError::Load { template, .. }
                if template.as_str() == "second"
        ));
    }

    #[test]
    fn build_returns_successful_compilation_warnings() {
        let loader = CountingLoader {
            bundle: bundle(),
            loads: AtomicUsize::new(0),
        };

        let build =
            futures_executor::block_on(TemplateRegistry::build(&loader, &WarningCompiler)).unwrap();

        assert_eq!(build.warnings().len(), 1);
        assert_eq!(
            build.warnings()[0].template,
            Some(TemplateId::new("invoice").unwrap())
        );
        assert_eq!(build.warnings()[0].message, "careful");
    }
}
