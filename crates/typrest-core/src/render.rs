use std::sync::Arc;

use crate::{
    Admission, ExecutionError, PdfCompiler, RenderError, RenderExecutor, RenderRequest,
    TemplateRegistry, validate_with_defaults,
};

#[derive(Clone)]
pub struct RenderService {
    registry: TemplateRegistry,
    compiler: Arc<dyn PdfCompiler>,
    executor: Arc<dyn RenderExecutor>,
}

impl RenderService {
    pub fn new(
        registry: TemplateRegistry,
        compiler: Arc<dyn PdfCompiler>,
        executor: Arc<dyn RenderExecutor>,
    ) -> Self {
        Self {
            registry,
            compiler,
            executor,
        }
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

        let compiler = Arc::clone(&self.compiler);
        let job_bundle = Arc::clone(&prepared_template_bundle);
        let outcome = self
            .executor
            .execute(
                Admission::Request,
                Box::new(move || compiler.compile_prepared(job_bundle.prepared(), &payload)),
            )
            .await;

        match outcome {
            Ok(result) => result.map_err(RenderError::CompilationError),
            Err(ExecutionError::Saturated) => Err(RenderError::Overloaded),
            Err(ExecutionError::Aborted { message }) => {
                Err(RenderError::ExecutionFailed { message })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use async_trait::async_trait;
    use bytes::Bytes;
    use serde_json::json;

    use crate::compiler::PreparedTemplateBundle;
    use crate::{
        Admission, CompilationError, CompileJob, Document, ExecutionError, InlineExecutor,
        PdfCompiler, PreparedTemplate, RenderError, RenderExecutor, RenderRequest, Schema,
        TemplateId, TemplateRegistry, TemplateSource,
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

        fn compile_prepared(
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
        let service = RenderService::new(
            registry(schema(json!({}))),
            compiler,
            Arc::new(InlineExecutor),
        );

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
            Arc::new(InlineExecutor),
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
        let service = RenderService::new(
            TemplateRegistry::default(),
            compiler,
            Arc::new(InlineExecutor),
        );

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
        let service = RenderService::new(
            registry(schema(json!({}))),
            compiler,
            Arc::new(InlineExecutor),
        );

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

    struct CountingExecutor {
        calls: AtomicUsize,
    }

    #[async_trait]
    impl RenderExecutor for CountingExecutor {
        async fn execute(
            &self,
            _: Admission,
            job: CompileJob,
        ) -> Result<Result<Document, CompilationError>, ExecutionError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(job())
        }
    }

    #[test]
    fn rejected_requests_never_reach_the_executor() {
        let compiler = Arc::new(MockCompiler::new(Ok(Document::new(Bytes::from_static(
            b"%PDF-1.4 mock",
        )))));
        let executor = Arc::new(CountingExecutor {
            calls: AtomicUsize::new(0),
        });
        let service = RenderService::new(
            registry(schema(json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "required": true }
                }
            }))),
            compiler,
            executor.clone(),
        );

        let invalid = futures_executor::block_on(service.render(RenderRequest::new(
            TemplateId::new("invoice").unwrap(),
            json!({}),
        )))
        .unwrap_err();
        let missing = futures_executor::block_on(service.render(RenderRequest::new(
            TemplateId::new("missing").unwrap(),
            json!({}),
        )))
        .unwrap_err();

        assert!(matches!(invalid, RenderError::ValidationError(_)));
        assert!(matches!(missing, RenderError::TemplateNotFound(_)));
        assert_eq!(executor.calls.load(Ordering::SeqCst), 0);
    }

    struct FailingExecutor(fn() -> ExecutionError);

    #[async_trait]
    impl RenderExecutor for FailingExecutor {
        async fn execute(
            &self,
            _: Admission,
            _: CompileJob,
        ) -> Result<Result<Document, CompilationError>, ExecutionError> {
            Err((self.0)())
        }
    }

    #[test]
    fn maps_execution_errors_to_render_errors() {
        let request = || RenderRequest::new(TemplateId::new("invoice").unwrap(), json!({}));
        let service = |executor: FailingExecutor| {
            RenderService::new(
                registry(schema(json!({}))),
                Arc::new(MockCompiler::new(Ok(Document::new(Bytes::new())))),
                Arc::new(executor),
            )
        };

        let saturated = futures_executor::block_on(
            service(FailingExecutor(|| ExecutionError::Saturated)).render(request()),
        )
        .unwrap_err();
        let aborted = futures_executor::block_on(
            service(FailingExecutor(|| ExecutionError::Aborted {
                message: "panic".to_owned(),
            }))
            .render(request()),
        )
        .unwrap_err();

        assert_eq!(saturated, RenderError::Overloaded);
        assert_eq!(
            aborted,
            RenderError::ExecutionFailed {
                message: "panic".to_owned()
            }
        );
    }

    struct CapturingExecutor {
        job: Mutex<Option<CompileJob>>,
    }

    #[async_trait]
    impl RenderExecutor for CapturingExecutor {
        async fn execute(
            &self,
            admission: Admission,
            job: CompileJob,
        ) -> Result<Result<Document, CompilationError>, ExecutionError> {
            assert_eq!(admission, Admission::Request);
            *self.job.lock().unwrap() = Some(job);
            Err(ExecutionError::Aborted {
                message: "captured".to_owned(),
            })
        }
    }

    struct AddressCompiler {
        seen: AtomicUsize,
    }

    #[async_trait]
    impl PdfCompiler for AddressCompiler {
        async fn prepare(&self, _: &TemplateSource) -> Result<PreparedTemplate, CompilationError> {
            Ok(PreparedTemplate::for_test())
        }

        fn compile_prepared(
            &self,
            template: &PreparedTemplate,
            _: &serde_json::Value,
        ) -> Result<Document, CompilationError> {
            self.seen.store(
                template as *const PreparedTemplate as usize,
                Ordering::SeqCst,
            );
            Ok(Document::new(Bytes::new()))
        }
    }

    #[test]
    fn captured_job_keeps_the_prepared_bundle_it_looked_up() {
        let id = TemplateId::new("invoice").unwrap();
        let registry = registry(schema(json!({})));
        let old = registry.get(&id).unwrap();
        let old_address = old.prepared() as *const PreparedTemplate as usize;
        let compiler = Arc::new(AddressCompiler {
            seen: AtomicUsize::new(0),
        });
        let executor = Arc::new(CapturingExecutor {
            job: Mutex::new(None),
        });
        let service = RenderService::new(registry.clone(), compiler.clone(), executor.clone());

        let _ =
            futures_executor::block_on(service.render(RenderRequest::new(id.clone(), json!({}))));
        drop(old);
        registry.replace_all(&self::registry(schema(json!({}))));
        let new_address = registry.get(&id).unwrap().prepared() as *const PreparedTemplate as usize;
        let job = executor.job.lock().unwrap().take().unwrap();
        job().unwrap();

        assert_ne!(new_address, old_address);
        assert_eq!(compiler.seen.load(Ordering::SeqCst), old_address);
    }
}
