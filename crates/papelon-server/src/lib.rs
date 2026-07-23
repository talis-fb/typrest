pub mod errors;
pub mod handlers;
pub mod loader;
pub mod router;
pub mod server;

use std::net::SocketAddr;
use std::sync::Arc;

use papelon_core::{RenderService, TypstCompiler};
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::loader::filesystem::FilesystemLoader;
use crate::router::router;
use crate::server::{AppState, ServerConfig};

pub async fn serve(config: ServerConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_tracing();

    let loader = Arc::new(FilesystemLoader::new(config.templates_dir));
    let compiler = Arc::new(TypstCompiler::new());
    let render_service = RenderService::new(loader.clone(), compiler);
    let app = router(AppState::new(loader, render_service));
    let address = SocketAddr::from((config.host, config.port));
    let listener = tokio::net::TcpListener::bind(address).await?;

    info!(%address, "papelon server listening");
    axum::serve(listener, app).await?;

    Ok(())
}

fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("papelon_server=info"));

    tracing_subscriber::fmt().with_env_filter(filter).init();
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use bytes::Bytes;
    use papelon_core::loader::memory::MemoryLoader;
    use papelon_core::{
        CompilationError, Document, PdfCompiler, RenderService, Schema, TemplateBundle,
        TemplateName, TemplateSource,
    };
    use serde_json::{Value, json};
    use tempfile::tempdir;
    use tower::ServiceExt;

    use crate::loader::filesystem::FilesystemLoader;
    use crate::router::router;
    use crate::server::AppState;

    #[derive(Clone)]
    struct MockCompiler {
        result: Result<Document, CompilationError>,
    }

    #[async_trait]
    impl PdfCompiler for MockCompiler {
        async fn compile(
            &self,
            _: &TemplateSource,
            _: &Value,
        ) -> Result<Document, CompilationError> {
            self.result.clone()
        }
    }

    fn app(loader: Arc<MemoryLoader>, result: Result<Document, CompilationError>) -> axum::Router {
        let compiler = Arc::new(MockCompiler { result });
        let render_service = RenderService::new(loader.clone(), compiler);
        router(AppState::new(loader, render_service))
    }

    fn schema(body: Value) -> Schema {
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

    fn memory_loader(templates: Vec<(&str, TemplateBundle)>) -> Arc<MemoryLoader> {
        Arc::new(MemoryLoader::new(
            templates
                .into_iter()
                .map(|(name, bundle)| (TemplateName::new(name).unwrap(), bundle))
                .collect(),
        ))
    }

    async fn response_body(response: axum::response::Response) -> Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn healthz_returns_ok() {
        let app = app(
            memory_loader(Vec::new()),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response_body(response).await, json!({ "status": "ok" }));
    }

    #[tokio::test]
    async fn templates_returns_template_names() {
        let app = app(
            memory_loader(vec![
                ("welcome", template(schema(json!({})))),
                ("invoice", template(schema(json!({})))),
            ]),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/templates")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response_body(response).await,
            json!({
                "templates": [
                    { "name": "invoice" },
                    { "name": "welcome" }
                ]
            })
        );
    }

    #[tokio::test]
    async fn render_returns_pdf() {
        let app = app(
            memory_loader(vec![("invoice", template(schema(json!({}))))]),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/render/invoice")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/pdf"
        );
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[tokio::test]
    async fn render_missing_template_returns_404() {
        let app = app(
            memory_loader(Vec::new()),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/render/missing")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response_body(response).await["error"],
            json!("template_not_found")
        );
    }

    #[tokio::test]
    async fn render_invalid_payload_returns_422() {
        let app = app(
            memory_loader(vec![(
                "invoice",
                template(schema(json!({
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "required": true }
                    }
                }))),
            )]),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/render/invoice")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            response_body(response).await,
            json!({
                "error": "schema_validation_failed",
                "message": "Payload validation failed",
                "violations": [
                    { "field": "$.name", "issue": "required field missing" }
                ]
            })
        );
    }

    #[tokio::test]
    async fn render_compilation_failure_returns_500() {
        let app = app(
            memory_loader(vec![("invoice", template(schema(json!({}))))]),
            Err(CompilationError::Failed {
                message: "syntax error".to_owned(),
                line: Some(3),
            }),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/render/invoice")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            response_body(response).await,
            json!({
                "error": "compilation_failed",
                "message": "syntax error",
                "line": 3
            })
        );
    }

    #[tokio::test]
    async fn render_invalid_json_returns_400() {
        let app = app(
            memory_loader(vec![("invoice", template(schema(json!({}))))]),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/render/invoice")
                    .header("content-type", "application/json")
                    .body(Body::from("{"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response_body(response).await["error"],
            json!("invalid_json")
        );
    }

    #[tokio::test]
    async fn render_real_template_over_http() {
        let root = tempdir().unwrap();
        write(
            &root.path().join("invoice/schema.json"),
            r#"{"schemaVersion":"v1","request":{"body":{"type":"object","properties":{"name":{"type":"string","required":true}}}}}"#,
        );
        write(
            &root.path().join("invoice/main.typ"),
            r#"
#let data = json(bytes(sys.inputs.data))
Hello #data.name
"#,
        );

        let loader = Arc::new(FilesystemLoader::new(root.path()));
        let compiler = Arc::new(papelon_core::TypstCompiler::new());
        let render_service = RenderService::new(loader.clone(), compiler);
        let app = router(AppState::new(loader, render_service));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let stream = tokio::net::TcpStream::connect(address).await.unwrap();
        stream.writable().await.unwrap();
        stream
            .try_write(
                b"POST /render/invoice HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: 14\r\nConnection: close\r\n\r\n{\"name\":\"Ada\"}",
            )
            .unwrap();

        let mut response = Vec::new();
        loop {
            stream.readable().await.unwrap();
            let mut chunk = [0; 4096];
            match stream.try_read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => response.extend_from_slice(&chunk[..n]),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(error) => panic!("failed to read response: {error}"),
            }
        }

        server.abort();

        let response = String::from_utf8_lossy(&response);
        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("content-type: application/pdf"));
        assert!(response.contains("%PDF"));
    }

    fn write(path: &std::path::Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
}
