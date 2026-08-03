use std::sync::Arc;

use crate::{PdfCompiler, RenderError, RenderRequest, TemplateRegistry, validate_with_defaults};

#[derive(Clone)]
pub struct RenderService {
    registry: TemplateRegistry,
    compiler: Arc<dyn PdfCompiler>,
}

impl RenderService {
    pub fn new(registry: TemplateRegistry, compiler: Arc<dyn PdfCompiler>) -> Self {
        Self { registry, compiler }
    }

    pub async fn render(&self, request: RenderRequest) -> Result<crate::Document, RenderError> {
        let prepared_template_bundle = self
            .registry
            .get(&request.template)
            .ok_or_else(|| RenderError::TemplateNotFound(request.template.clone()))?;

        let payload = match prepared_template_bundle.schema() {
            Some(schema) => validate_with_defaults(schema, &request.payload)
                .map_err(RenderError::ValidationError)?,
            None => request.payload,
        };

        self.compiler
            .compile_prepared(prepared_template_bundle.prepared(), &payload)
            .await
            .map_err(RenderError::CompilationError)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use async_trait::async_trait;
    use bytes::Bytes;
    use serde_json::json;

    use crate::compiler::PreparedTemplateBundle;
    use crate::{
        CompilationError, Document, PdfCompiler, PreparedTemplate, RenderError, RenderRequest,
        Schema, TemplateId, TemplateRegistry, TemplateSource,
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
        async fn prepare(&self, _: &TemplateSource) -> Result<PreparedTemplate, CompilationError> {
            Ok(PreparedTemplate::for_test())
        }

        async fn compile_prepared(
            &self,
            _: &PreparedTemplate,
            _: &serde_json::Value,
        ) -> Result<Document, CompilationError> {
            self.called.store(true, Ordering::SeqCst);
            self.result.clone()
        }
    }

    fn schema(body: serde_json::Value) -> Schema {
        Schema::parse_json(
            json!({
                "schemaVersion": "v1",
                "request": { "body": body }
            })
            .to_string()
            .as_bytes(),
        )
        .unwrap()
    }

    fn registry(schema: Schema) -> TemplateRegistry {
        let registry = TemplateRegistry::default();
        registry.insert_for_test(
            TemplateId::new("invoice").unwrap(),
            PreparedTemplateBundle::new(schema, PreparedTemplate::for_test()),
        );
        registry
    }

    #[test]
    fn renders_valid_payload() {
        let compiler = Arc::new(MockCompiler::new(Ok(Document::new(Bytes::from_static(
            b"%PDF-1.4 mock",
        )))));
        let service = RenderService::new(registry(schema(json!({}))), compiler);

        let document = futures_executor::block_on(service.render(RenderRequest::new(
            TemplateId::new("invoice").unwrap(),
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
            registry(schema(json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "required": true }
                }
            }))),
            compiler.clone(),
        );

        let error = futures_executor::block_on(service.render(RenderRequest::new(
            TemplateId::new("invoice").unwrap(),
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
        let service = RenderService::new(TemplateRegistry::default(), compiler);

        let error = futures_executor::block_on(service.render(RenderRequest::new(
            TemplateId::new("missing").unwrap(),
            json!({}),
        )))
        .unwrap_err();

        assert!(matches!(error, RenderError::TemplateNotFound(_)));
    }

    #[test]
    fn returns_compilation_error() {
        let compiler = Arc::new(MockCompiler::new(Err(CompilationError::failed(
            "boom",
            Some(1),
        ))));
        let service = RenderService::new(registry(schema(json!({}))), compiler);

        let error = futures_executor::block_on(service.render(RenderRequest::new(
            TemplateId::new("invoice").unwrap(),
            json!({}),
        )))
        .unwrap_err();

        assert!(matches!(
            error,
            RenderError::CompilationError(CompilationError::Failed(_))
        ));
    }
}
