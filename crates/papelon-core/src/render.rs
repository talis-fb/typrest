use std::sync::Arc;

use crate::{
    LoaderError, PdfCompiler, RenderError, RenderRequest, TemplateLoader, validate_with_defaults,
};

#[derive(Clone)]
pub struct RenderService {
    loader: Arc<dyn TemplateLoader>,
    compiler: Arc<dyn PdfCompiler>,
}

impl RenderService {
    pub fn new(loader: Arc<dyn TemplateLoader>, compiler: Arc<dyn PdfCompiler>) -> Self {
        Self { loader, compiler }
    }

    pub async fn render(&self, request: RenderRequest) -> Result<crate::Document, RenderError> {
        let bundle = self
            .loader
            .load(&request.template)
            .await
            .map_err(map_loader_error)?;
        let payload = validate_with_defaults(&bundle.schema, &request.payload)
            .map_err(RenderError::ValidationError)?;

        self.compiler
            .compile(&bundle.source, &payload)
            .await
            .map_err(RenderError::CompilationError)
    }
}

fn map_loader_error(error: LoaderError) -> RenderError {
    match error {
        LoaderError::TemplateNotFound(name) => RenderError::TemplateNotFound(name),
        error => RenderError::CompilationError(crate::CompilationError::Failed {
            message: error.to_string(),
            line: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use async_trait::async_trait;
    use bytes::Bytes;
    use serde_json::json;

    use crate::loader::memory::MemoryLoader;
    use crate::{
        CompilationError, Document, PdfCompiler, RenderError, RenderRequest, Schema,
        TemplateBundle, TemplateName, TemplateSource,
    };

    use super::RenderService;

    struct MockCompiler {
        called: AtomicBool,
        result: Result<Document, CompilationError>,
    }

    impl MockCompiler {
        fn new(result: Result<Document, CompilationError>) -> Self {
            Self {
                called: AtomicBool::new(false),
                result,
            }
        }

        fn was_called(&self) -> bool {
            self.called.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl PdfCompiler for MockCompiler {
        async fn compile(
            &self,
            _: &TemplateSource,
            _: &serde_json::Value,
        ) -> Result<Document, CompilationError> {
            self.called.store(true, Ordering::SeqCst);
            self.result.clone()
        }
    }

    fn schema(body: serde_json::Value) -> Schema {
        Schema::parse(
            json!({
                "schemaVersion": "v1",
                "request": { "body": body }
            })
            .to_string()
            .as_bytes(),
        )
        .unwrap()
    }

    fn template(schema: Schema) -> TemplateBundle {
        TemplateBundle::new(
            TemplateSource::new(BTreeMap::from([(
                "main.typ".to_owned(),
                Bytes::from_static(b"Hello"),
            )])),
            schema,
        )
    }

    fn loader(bundle: TemplateBundle) -> Arc<MemoryLoader> {
        Arc::new(MemoryLoader::new(BTreeMap::from([(
            TemplateName::new("invoice").unwrap(),
            bundle,
        )])))
    }

    #[test]
    fn renders_valid_payload() {
        let compiler = Arc::new(MockCompiler::new(Ok(Document::new(Bytes::from_static(
            b"%PDF-1.4 mock",
        )))));
        let service = RenderService::new(loader(template(schema(json!({})))), compiler);

        let document = futures_executor::block_on(service.render(RenderRequest::new(
            TemplateName::new("invoice").unwrap(),
            json!({}),
        )))
        .unwrap();

        assert!(document.bytes().starts_with(b"%PDF"));
    }

    #[test]
    fn returns_validation_error_before_calling_compiler() {
        let compiler = Arc::new(MockCompiler::new(Ok(Document::new(Bytes::from_static(
            b"%PDF-1.4 mock",
        )))));
        let service = RenderService::new(
            loader(template(schema(json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "required": true }
                }
            })))),
            compiler.clone(),
        );

        let error = futures_executor::block_on(service.render(RenderRequest::new(
            TemplateName::new("invoice").unwrap(),
            json!({}),
        )))
        .unwrap_err();

        assert!(matches!(error, RenderError::ValidationError(_)));
        assert!(!compiler.was_called());
    }

    #[test]
    fn returns_template_not_found() {
        let compiler = Arc::new(MockCompiler::new(Ok(Document::new(Bytes::from_static(
            b"%PDF-1.4 mock",
        )))));
        let service = RenderService::new(Arc::new(MemoryLoader::default()), compiler);

        let error = futures_executor::block_on(service.render(RenderRequest::new(
            TemplateName::new("missing").unwrap(),
            json!({}),
        )))
        .unwrap_err();

        assert!(matches!(error, RenderError::TemplateNotFound(_)));
    }

    #[test]
    fn returns_compilation_error() {
        let compiler = Arc::new(MockCompiler::new(Err(CompilationError::Failed {
            message: "boom".to_owned(),
            line: Some(1),
        })));
        let service = RenderService::new(loader(template(schema(json!({})))), compiler);

        let error = futures_executor::block_on(service.render(RenderRequest::new(
            TemplateName::new("invoice").unwrap(),
            json!({}),
        )))
        .unwrap_err();

        assert!(matches!(
            error,
            RenderError::CompilationError(CompilationError::Failed { .. })
        ));
    }
}
