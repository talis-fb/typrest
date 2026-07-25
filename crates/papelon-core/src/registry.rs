use std::collections::BTreeMap;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use thiserror::Error;

use crate::{
    CompilationError, LoaderError, PdfCompiler, PreparedTemplate, Schema, TemplateBundle,
    TemplateLoader, TemplateName, ValidationError, validate_with_defaults,
};

pub struct PreparedTemplateEntry {
    schema: Schema,
    prepared: PreparedTemplate,
}

impl PreparedTemplateEntry {
    pub fn new(schema: Schema, prepared: PreparedTemplate) -> Self {
        Self { schema, prepared }
    }

    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    pub fn prepared(&self) -> &PreparedTemplate {
        &self.prepared
    }
}

#[derive(Clone, Default)]
pub struct TemplateRegistry {
    entries: Arc<RwLock<BTreeMap<TemplateName, Arc<PreparedTemplateEntry>>>>,
}

impl TemplateRegistry {
    pub async fn build(
        loader: &dyn TemplateLoader,
        compiler: &dyn PdfCompiler,
    ) -> Result<Self, TemplateRegistryError> {
        let names = loader
            .list()
            .await
            .map_err(|source| TemplateRegistryError::List { source })?;
        let registry = Self::default();

        for name in names {
            registry.reload(loader, compiler, &name).await?;
        }

        Ok(registry)
    }

    pub async fn reload(
        &self,
        loader: &dyn TemplateLoader,
        compiler: &dyn PdfCompiler,
        name: &TemplateName,
    ) -> Result<(), TemplateRegistryError> {
        let bundle = loader
            .load(name)
            .await
            .map_err(|source| TemplateRegistryError::Load {
                template: name.clone(),
                source,
            })?;
        let entry = prepare_entry(name, bundle, compiler).await?;

        self.replace(name.clone(), entry);
        Ok(())
    }

    pub fn get(&self, name: &TemplateName) -> Option<Arc<PreparedTemplateEntry>> {
        read_entries(&self.entries).get(name).cloned()
    }

    pub fn names(&self) -> Vec<TemplateName> {
        read_entries(&self.entries).keys().cloned().collect()
    }

    fn replace(&self, name: TemplateName, entry: PreparedTemplateEntry) {
        write_entries(&self.entries).insert(name, Arc::new(entry));
    }

    #[cfg(any(test, feature = "test-util"))]
    #[doc(hidden)]
    pub fn insert_for_test(&self, name: TemplateName, entry: PreparedTemplateEntry) {
        self.replace(name, entry);
    }
}

async fn prepare_entry(
    name: &TemplateName,
    bundle: TemplateBundle,
    compiler: &dyn PdfCompiler,
) -> Result<PreparedTemplateEntry, TemplateRegistryError> {
    let prepared = compiler.prepare(&bundle.source).await.map_err(|source| {
        TemplateRegistryError::Prepare {
            template: name.clone(),
            source,
        }
    })?;
    let sample = bundle.schema.sample_payload();
    let sample = validate_with_defaults(&bundle.schema, &sample).map_err(|violations| {
        TemplateRegistryError::SamplePayload {
            template: name.clone(),
            violations,
        }
    })?;

    compiler
        .compile_prepared(&prepared, &sample)
        .await
        .map_err(|source| TemplateRegistryError::Warmup {
            template: name.clone(),
            source,
        })?;

    Ok(PreparedTemplateEntry::new(bundle.schema, prepared))
}

fn read_entries(
    entries: &RwLock<BTreeMap<TemplateName, Arc<PreparedTemplateEntry>>>,
) -> RwLockReadGuard<'_, BTreeMap<TemplateName, Arc<PreparedTemplateEntry>>> {
    match entries.read() {
        Ok(entries) => entries,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn write_entries(
    entries: &RwLock<BTreeMap<TemplateName, Arc<PreparedTemplateEntry>>>,
) -> RwLockWriteGuard<'_, BTreeMap<TemplateName, Arc<PreparedTemplateEntry>>> {
    match entries.write() {
        Ok(entries) => entries,
        Err(poisoned) => poisoned.into_inner(),
    }
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
        template: TemplateName,
        #[source]
        source: LoaderError,
    },

    #[error("template '{template}' failed to prepare: {source}")]
    Prepare {
        template: TemplateName,
        #[source]
        source: CompilationError,
    },

    #[error("template '{template}' could not generate a valid startup payload: {violations:?}")]
    SamplePayload {
        template: TemplateName,
        violations: Vec<ValidationError>,
    },

    #[error("template '{template}' failed warmup compilation: {source}")]
    Warmup {
        template: TemplateName,
        #[source]
        source: CompilationError,
    },
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
        CompilationError, Document, LoaderError, PdfCompiler, PreparedTemplate, RenderRequest,
        RenderService, Schema, TemplateBundle, TemplateLoader, TemplateName, TemplateSource,
    };

    use super::TemplateRegistry;

    struct CountingLoader {
        bundle: TemplateBundle,
        loads: AtomicUsize,
    }

    #[async_trait]
    impl TemplateLoader for CountingLoader {
        async fn load(&self, _: &TemplateName) -> Result<TemplateBundle, LoaderError> {
            self.loads.fetch_add(1, Ordering::SeqCst);
            Ok(self.bundle.clone())
        }

        async fn list(&self) -> Result<Vec<TemplateName>, LoaderError> {
            Ok(vec![TemplateName::new("invoice").unwrap()])
        }
    }

    struct CountingCompiler {
        prepares: AtomicUsize,
        compiles: AtomicUsize,
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
        let schema = Schema::parse(
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
        let registry =
            futures_executor::block_on(TemplateRegistry::build(loader.as_ref(), compiler.as_ref()))
                .unwrap();

        assert_eq!(loader.loads.load(Ordering::SeqCst), 1);
        assert_eq!(compiler.prepares.load(Ordering::SeqCst), 1);
        assert_eq!(compiler.compiles.load(Ordering::SeqCst), 1);
        assert_eq!(
            registry
                .names()
                .iter()
                .map(TemplateName::as_str)
                .collect::<Vec<_>>(),
            vec!["invoice"]
        );

        let service = RenderService::new(registry, compiler.clone());
        let document = futures_executor::block_on(service.render(RenderRequest::new(
            TemplateName::new("invoice").unwrap(),
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
        invalid_bundle.schema = Schema::parse(
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
        .unwrap();
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
            error,
            super::TemplateRegistryError::SamplePayload { template, .. }
                if template.as_str() == "invoice"
        ));
        assert_eq!(compiler.compiles.load(Ordering::SeqCst), 0);
    }
}
