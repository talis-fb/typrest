pub(crate) mod config;
pub(crate) mod errors;
pub(crate) mod handlers;
pub(crate) mod router;
pub(crate) mod state;

use std::net::SocketAddr;

use tracing::info;

use crate::http::config::ServerConfig;
use crate::http::errors::ServeError;

use self::router::router;
use self::state::AppState;

pub(crate) async fn serve(config: ServerConfig, state: AppState) -> Result<(), ServeError> {
    let app_router = router(state, &config);

    let address = SocketAddr::from((config.host, config.port));
    let listener = tokio::net::TcpListener::bind(address).await?;

    info!(
        event = "server.listening",
        %address,
        "typrest server listening on {address}"
    );
    axum::serve(listener, app_router).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::sync::Arc;

    use crate::render_executor::{BlockingRenderExecutor, RenderLimits};
    use async_trait::async_trait;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use bytes::Bytes;
    use serde_json::{Value, json};
    use tempfile::tempdir;
    use tower::ServiceExt;
    use typrest_core::loader::memory::MemoryLoader;
    use typrest_core::{
        CompilationDiagnostic, CompilationError, CompilationReport, DiagnosticLocation,
        DiagnosticPosition, DiagnosticSeverity, Document, PdfCompiler, PreparedTemplate,
        PreparedTemplateBundle, RelatedDiagnostic, RenderService, Schema, TemplateBundle,
        TemplateId, TemplateLoader, TemplateRegistry, TemplateRegistryError, TemplateSource,
    };

    use crate::args::LogFormat;
    use crate::http::config::ServerConfig;
    use crate::http::router::router;
    use crate::http::state::AppState;
    use crate::loader::filesystem::FilesystemLoader;
    use crate::package::universal_package::UniversePackageRepository;

    #[derive(Clone)]
    struct MockCompiler {
        result: Result<Document, CompilationError>,
    }

    #[async_trait]
    impl PdfCompiler for MockCompiler {
        async fn prepare(&self, _: &TemplateSource) -> Result<PreparedTemplate, CompilationError> {
            Ok(PreparedTemplate::for_test())
        }

        fn compile_prepared(
            &self,
            _: &PreparedTemplate,
            _: &Value,
        ) -> Result<Document, CompilationError> {
            self.result.clone()
        }
    }

    fn executor() -> Arc<BlockingRenderExecutor> {
        Arc::new(BlockingRenderExecutor::new(RenderLimits::default()))
    }

    fn app(loader: Arc<MemoryLoader>, result: Result<Document, CompilationError>) -> axum::Router {
        app_with_config(loader, result, &ServerConfig::new(0))
    }

    fn app_with_config(
        loader: Arc<MemoryLoader>,
        result: Result<Document, CompilationError>,
        config: &ServerConfig,
    ) -> axum::Router {
        let compiler = Arc::new(MockCompiler { result });
        let registry = registry_from_loader(loader.as_ref());
        let render_service = RenderService::new(registry.clone(), compiler, executor());
        router(
            AppState::new(registry, render_service, LogFormat::Pretty),
            config,
        )
    }

    fn app_with_web_ui(
        loader: Arc<MemoryLoader>,
        result: Result<Document, CompilationError>,
    ) -> axum::Router {
        let compiler = Arc::new(MockCompiler { result });
        let registry = registry_from_loader(loader.as_ref());
        let render_service = RenderService::new(registry.clone(), compiler, executor());
        router(
            AppState::new(registry, render_service, LogFormat::Pretty),
            &ServerConfig::new(0).with_web_ui(true),
        )
    }

    fn registry_from_loader(loader: &MemoryLoader) -> TemplateRegistry {
        let registry = TemplateRegistry::default();
        let names = futures_executor::block_on(loader.list()).unwrap();

        for name in names {
            let bundle = futures_executor::block_on(loader.load(&name)).unwrap();
            let entry = match bundle.schema {
                Some(schema) => PreparedTemplateBundle::new(schema, PreparedTemplate::for_test()),
                None => PreparedTemplateBundle::without_schema(PreparedTemplate::for_test()),
            };
            registry.insert_for_test(name, entry);
        }

        registry
    }

    fn schema(body: Value) -> Schema {
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
                .map(|(name, bundle)| (TemplateId::new(name).unwrap(), bundle))
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
                    .uri("/_healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response_body(response).await, json!({ "status": "ok" }));
    }

    #[tokio::test]
    async fn templates_returns_template_ids() {
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
                    .uri("/_api/templates")
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
                    { "id": "invoice", "hasSchema": true },
                    { "id": "welcome", "hasSchema": true }
                ]
            })
        );
    }

    #[tokio::test]
    async fn web_ui_routes_are_not_mounted_when_disabled() {
        let app = app(
            memory_loader(vec![("invoice", template(schema(json!({}))))]),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
        );

        for uri in ["/", "/_webui", "/_webui/", "/_webui/assets/missing.js"] {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{uri}");
        }
    }

    #[tokio::test]
    async fn web_ui_serves_embedded_index_and_assets_when_enabled() {
        let app = app_with_web_ui(
            memory_loader(Vec::new()),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
        );

        let redirect = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/_webui")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(redirect.status(), StatusCode::PERMANENT_REDIRECT);
        assert_eq!(redirect.headers().get("location").unwrap(), "/_webui/");

        let root_redirect = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(root_redirect.status(), StatusCode::PERMANENT_REDIRECT);
        assert_eq!(root_redirect.headers().get("location").unwrap(), "/_webui/");

        let index = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/_webui/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(index.status(), StatusCode::OK);
        assert_eq!(
            index.headers().get("content-type").unwrap(),
            "text/html; charset=utf-8"
        );
        assert_eq!(index.headers().get("cache-control").unwrap(), "no-cache");
        let index_bytes = axum::body::to_bytes(index.into_body(), usize::MAX)
            .await
            .unwrap();
        let index_html = String::from_utf8(index_bytes.to_vec()).unwrap();
        let script_path = index_html
            .split("src=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();

        let asset = app
            .oneshot(
                Request::builder()
                    .uri(script_path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(asset.status(), StatusCode::OK);
        assert!(
            asset
                .headers()
                .get("content-type")
                .unwrap()
                .to_str()
                .unwrap()
                .contains("javascript")
        );
        assert_eq!(
            asset.headers().get("cache-control").unwrap(),
            "public, max-age=31536000, immutable"
        );
    }

    #[tokio::test]
    async fn web_ui_metadata_comes_from_prepared_registry() {
        let schema = schema(json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "required": true }
            }
        }));
        let source = TemplateSource::new(BTreeMap::from([(
            "main.typ".to_owned(),
            Bytes::from_static(b"Hello"),
        )]));
        let app = app_with_web_ui(
            memory_loader(vec![
                ("invoice", template(schema)),
                ("welcome", TemplateBundle::without_schema(source)),
            ]),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
        );

        let list = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/_api/templates")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(list.status(), StatusCode::OK);
        assert_eq!(
            response_body(list).await,
            json!({
                "templates": [
                    { "id": "invoice", "hasSchema": true },
                    { "id": "welcome", "hasSchema": false }
                ]
            })
        );

        let invoice = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/_api/templates/invoice")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let invoice_body = response_body(invoice).await;
        assert_eq!(invoice_body["id"], "invoice");
        assert_eq!(invoice_body["schema"]["schemaVersion"], "v1");
        assert_eq!(
            invoice_body["schema"]["request"]["body"]["properties"]["name"]["type"],
            "string"
        );
        assert!(
            invoice_body["schema"]["request"]["body"]["properties"]["name"]
                .get("default")
                .is_none()
        );

        let welcome = app
            .oneshot(
                Request::builder()
                    .uri("/_api/templates/welcome")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response_body(welcome).await,
            json!({ "id": "welcome", "schema": null })
        );
    }

    #[tokio::test]
    async fn web_ui_metadata_rejects_invalid_and_missing_ids() {
        let app = app_with_web_ui(
            memory_loader(Vec::new()),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
        );

        let missing = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/_api/templates/missing")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        assert_eq!(response_body(missing).await["error"], "template_not_found");

        let invalid = app
            .oneshot(
                Request::builder()
                    .uri("/_api/templates/%24")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn metadata_resolves_nested_template_ids() {
        let app = app(
            memory_loader(vec![("financial/invoice", template(schema(json!({}))))]),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
        );

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/_api/templates/financial/invoice")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response_body(response).await["id"], "financial/invoice");
    }

    #[tokio::test]
    async fn render_returns_pdf() {
        let first_app = app(
            memory_loader(vec![("financial/invoice", template(schema(json!({}))))]),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
        );

        let response = first_app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/financial/invoice")
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
    async fn get_render_without_body_returns_pdf_for_schema_less_template() {
        let source = TemplateSource::new(BTreeMap::from([(
            "main.typ".to_owned(),
            Bytes::from_static(b"Hello"),
        )]));
        let app = app(
            memory_loader(vec![("welcome", TemplateBundle::without_schema(source))]),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/welcome")
                    .body(Body::empty())
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
    async fn get_render_without_body_matches_post_without_body_for_schema_template() {
        let post_app = app(
            memory_loader(vec![("invoice", template(schema(json!({}))))]),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
        );
        let post = post_app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/invoice")
                    .header("content-type", "application/json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let post_status = post.status();
        let post_body = axum::body::to_bytes(post.into_body(), usize::MAX)
            .await
            .unwrap();

        let get_app = app(
            memory_loader(vec![("invoice", template(schema(json!({}))))]),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
        );
        let get = get_app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/invoice")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let get_status = get.status();
        let get_body = axum::body::to_bytes(get.into_body(), usize::MAX)
            .await
            .unwrap();

        assert_eq!(get_status, post_status);
        assert_eq!(get_body, post_body);
    }

    #[tokio::test]
    async fn render_without_schema_accepts_any_json_payload_and_empty_body() {
        let source = TemplateSource::new(BTreeMap::from([(
            "main.typ".to_owned(),
            Bytes::from_static(b"Hello"),
        )]));
        let second_app = app(
            memory_loader(vec![("welcome", TemplateBundle::without_schema(source))]),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
        );

        let response = second_app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/welcome")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"["not an object"]"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let source = TemplateSource::new(BTreeMap::from([(
            "main.typ".to_owned(),
            Bytes::from_static(b"Hello"),
        )]));
        let empty_app = app(
            memory_loader(vec![("welcome", TemplateBundle::without_schema(source))]),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
        );
        let response = empty_app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/welcome")
                    .header("content-type", "application/json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
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
                    .uri("/missing")
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
    async fn render_has_no_legacy_prefix_alias_and_rejects_reserved_ids() {
        let app = app(
            memory_loader(vec![("invoice", template(schema(json!({}))))]),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
        );

        let legacy = app
            .clone()
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
        assert_eq!(legacy.status(), StatusCode::NOT_FOUND);

        let trailing_slash = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/invoice/")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(trailing_slash.status(), StatusCode::BAD_REQUEST);

        let reserved = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/_private/invoice")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(reserved.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response_body(reserved).await["error"],
            "invalid_template_id"
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
                    .uri("/invoice")
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
            Err(CompilationError::failed("syntax error", Some(3))),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/invoice")
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
                "line": 3,
                "diagnostics": [
                    {
                        "severity": "error",
                        "message": "syntax error",
                        "location": {
                            "path": "main.typ",
                            "range": {
                                "start": { "line": 3, "column": 1 },
                                "end": { "line": 3, "column": 1 }
                            }
                        }
                    }
                ]
            })
        );
    }

    #[tokio::test]
    async fn render_compilation_failure_returns_all_safe_diagnostic_details() {
        let mut first = CompilationDiagnostic::new(DiagnosticSeverity::Error, "unknown variable");
        first.location = Some(DiagnosticLocation::new(
            "_shared/brand.typ",
            DiagnosticPosition::new(4, 3),
            DiagnosticPosition::new(4, 8),
        ));
        first
            .hints
            .push(RelatedDiagnostic::new("check the spelling", None));
        first.trace.push(RelatedDiagnostic::new(
            "while calling `brand`",
            Some(DiagnosticLocation::new(
                "main.typ",
                DiagnosticPosition::new(9, 1),
                DiagnosticPosition::new(9, 6),
            )),
        ));
        let second =
            CompilationDiagnostic::new(DiagnosticSeverity::Error, "second compilation error");
        let app = app(
            memory_loader(vec![("invoice", template(schema(json!({}))))]),
            Err(CompilationError::Failed(CompilationReport::new(vec![
                first, second,
            ]))),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/invoice")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = response_body(response).await;
        assert_eq!(body["diagnostics"].as_array().unwrap().len(), 2);
        assert_eq!(
            body["diagnostics"][0]["location"]["path"],
            "_shared/brand.typ"
        );
        assert_eq!(
            body["diagnostics"][0]["hints"][0]["message"],
            "check the spelling"
        );
        assert_eq!(
            body["diagnostics"][0]["trace"][0]["location"]["path"],
            "main.typ"
        );
        assert!(body["diagnostics"][0].get("source").is_none());
        assert!(!body.to_string().contains("/var/"));
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
                    .uri("/invoice")
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
    async fn post_render_requires_json_content_type() {
        let app = app(
            memory_loader(vec![("invoice", template(schema(json!({}))))]),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/invoice")
                    .body(Body::empty())
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

    fn render_with_body(template: &str, content_type: Option<&str>, body: &str) -> Request<Body> {
        let mut builder = Request::builder().method(Method::POST).uri(template);
        if let Some(content_type) = content_type {
            builder = builder.header("content-type", content_type);
        }
        builder.body(Body::from(body.to_owned())).unwrap()
    }

    #[tokio::test]
    async fn oversized_body_returns_413_json_envelope_without_parsing() {
        let app = app_with_config(
            memory_loader(vec![("invoice", template(schema(json!({}))))]),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
            &ServerConfig::new(0).with_max_body_bytes(16),
        );

        for body in [
            r#"{"name":"Ada Lovelace"}"#,
            "{ not json at all, far beyond the limit",
        ] {
            let response = app
                .clone()
                .oneshot(render_with_body("/invoice", Some("application/json"), body))
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE, "{body}");
            assert_eq!(
                response_body(response).await["error"],
                json!("payload_too_large")
            );
        }
    }

    #[tokio::test]
    async fn body_exactly_at_limit_is_accepted() {
        let app = app_with_config(
            memory_loader(vec![("invoice", template(schema(json!({}))))]),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
            &ServerConfig::new(0).with_max_body_bytes(2),
        );

        let response = app
            .oneshot(render_with_body("/invoice", Some("application/json"), "{}"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn invalid_template_id_wins_over_payload_too_large() {
        let app = app_with_config(
            memory_loader(vec![("invoice", template(schema(json!({}))))]),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
            &ServerConfig::new(0).with_max_body_bytes(4),
        );

        let response = app
            .oneshot(render_with_body(
                "/_private/invoice",
                Some("application/json"),
                r#"{"name":"Ada"}"#,
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response_body(response).await["error"],
            json!("invalid_template_id")
        );
    }

    #[tokio::test]
    async fn content_type_error_wins_over_payload_too_large() {
        let app = app_with_config(
            memory_loader(vec![("invoice", template(schema(json!({}))))]),
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock"))),
            &ServerConfig::new(0).with_max_body_bytes(4),
        );

        let response = app
            .oneshot(render_with_body(
                "/invoice",
                Some("text/plain"),
                r#"{"name":"Ada"}"#,
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response_body(response).await["error"],
            json!("invalid_json")
        );
    }

    #[tokio::test]
    async fn startup_registry_allows_templates_without_schema() {
        let missing_schema = tempdir().unwrap();
        write(&missing_schema.path().join("invoice/main.typ"), "No schema");
        let compiler: Arc<dyn PdfCompiler> = Arc::new(typrest_core::TypstCompiler::new());
        let loader = FilesystemLoader::new(missing_schema.path());

        let build = TemplateRegistry::build(&loader, &compiler, executor().as_ref())
            .await
            .unwrap();
        assert_eq!(
            build.value().ids(),
            vec![TemplateId::new("invoice").unwrap()]
        );
        assert!(
            build
                .value()
                .get(&TemplateId::new("invoice").unwrap())
                .unwrap()
                .schema()
                .is_none()
        );
    }

    #[tokio::test]
    async fn startup_registry_reports_template_compilation_failures() {
        let broken_typst = tempdir().unwrap();
        write(
            &broken_typst.path().join("invoice/schema.yml"),
            "schemaVersion: v1\nrequest:\n  body: {}\n",
        );
        write(
            &broken_typst.path().join("invoice/main.typ"),
            "#let broken =",
        );
        let loader = FilesystemLoader::new(broken_typst.path());
        let compiler: Arc<dyn PdfCompiler> = Arc::new(typrest_core::TypstCompiler::new());

        let compile_error =
            match TemplateRegistry::build(&loader, &compiler, executor().as_ref()).await {
                Ok(_) => panic!("broken Typst should fail startup"),
                Err(error) => error,
            };
        assert!(matches!(
            compile_error.value().failures(),
            [TemplateRegistryError::Warmup {
                template,
                source: CompilationError::Failed(_)
            }] if template.as_str() == "invoice"
        ));
        let TemplateRegistryError::Warmup { source, .. } = &compile_error.value().failures()[0]
        else {
            unreachable!()
        };
        assert!(source.primary_line().is_some());
    }

    #[tokio::test]
    async fn render_real_template_over_http() {
        let root = tempdir().unwrap();
        write(
            &root.path().join("invoice/schema.yml"),
            "schemaVersion: v1\nrequest:\n  body:\n    type: object\n    properties:\n      name:\n        type: string\n        required: true\n",
        );
        write(
            &root.path().join("invoice/main.typ"),
            r#"
#let data = json(bytes(sys.inputs.data))
Hello #data.name
"#,
        );

        let loader = Arc::new(FilesystemLoader::new(root.path()));
        let compiler: Arc<dyn PdfCompiler> = Arc::new(typrest_core::TypstCompiler::new());
        let executor = executor();
        let registry = TemplateRegistry::build(loader.as_ref(), &compiler, executor.as_ref())
            .await
            .unwrap()
            .into_value();
        let render_service = RenderService::new(registry.clone(), compiler, executor);
        let app = router(
            AppState::new(registry, render_service, LogFormat::Pretty),
            &ServerConfig::new(0),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let stream = tokio::net::TcpStream::connect(address).await.unwrap();
        stream.writable().await.unwrap();
        stream
            .try_write(
                b"POST /invoice HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: 14\r\nConnection: close\r\n\r\n{\"name\":\"Ada\"}",
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

    #[tokio::test]
    async fn prepares_and_renders_template_with_cached_preview_package() {
        let root = tempdir().unwrap();
        let cache = root.path().join(".typrest/packages");
        let package = cache.join("preview/fixture/1.0.0");
        write(
            &package.join("typst.toml"),
            "[package]\nname = \"fixture\"\nversion = \"1.0.0\"\nentrypoint = \"lib.typ\"\n",
        );
        write(&package.join("lib.typ"), "#let message = [From package]");
        let templates = root.path().join("templates");
        write(
            &templates.join("invoice/main.typ"),
            "#import \"@preview/fixture:1.0.0\": message\n#message",
        );

        let loader = FilesystemLoader::new(&templates);
        let packages = Arc::new(UniversePackageRepository::new(&cache, "test"));
        let compiler: Arc<dyn PdfCompiler> =
            Arc::new(typrest_core::TypstCompiler::new().with_package_repository(packages));
        let executor = executor();
        let registry = TemplateRegistry::build(&loader, &compiler, executor.as_ref())
            .await
            .unwrap()
            .into_value();
        let render_service = RenderService::new(registry, compiler, executor);

        let document = render_service
            .render(typrest_core::RenderRequest::new(
                TemplateId::new("invoice").unwrap(),
                json!({}),
            ))
            .await
            .unwrap();

        assert!(document.bytes().starts_with(b"%PDF"));
    }

    #[tokio::test]
    async fn invoice_example_project_renders_over_http() {
        let templates_dir = example_templates_dir("invoice");
        let loader = FilesystemLoader::new(&templates_dir);
        let names = loader.list().await.unwrap();

        assert!(names.iter().any(|name| name.as_str() == "invoice"));
        loader
            .load(&TemplateId::new("invoice").unwrap())
            .await
            .unwrap();

        let loader = Arc::new(loader);
        let compiler: Arc<dyn PdfCompiler> = Arc::new(typrest_core::TypstCompiler::new());
        let executor = executor();
        let registry = TemplateRegistry::build(loader.as_ref(), &compiler, executor.as_ref())
            .await
            .unwrap()
            .into_value();
        let render_service = RenderService::new(registry.clone(), compiler, executor);
        let app = router(
            AppState::new(registry, render_service, LogFormat::Pretty),
            &ServerConfig::new(0),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let payload = fs::read(templates_dir.join("invoice/payload.json")).unwrap();
        let response = post_json(address, "/invoice", &payload).await;

        server.abort();

        let response = String::from_utf8_lossy(&response);
        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("content-type: application/pdf"));
        assert!(response.contains("%PDF"));
    }

    /// Blocks inside `compile_prepared` until released, so tests can hold a render slot.
    struct GatedCompiler {
        started: std::sync::mpsc::Sender<()>,
        release: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
    }

    #[async_trait]
    impl PdfCompiler for GatedCompiler {
        async fn prepare(&self, _: &TemplateSource) -> Result<PreparedTemplate, CompilationError> {
            Ok(PreparedTemplate::for_test())
        }

        fn compile_prepared(
            &self,
            _: &PreparedTemplate,
            _: &Value,
        ) -> Result<Document, CompilationError> {
            self.started.send(()).unwrap();
            let _ = self.release.lock().unwrap().recv();
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock")))
        }
    }

    struct PanicOnceCompiler {
        panicked: std::sync::atomic::AtomicBool,
    }

    #[async_trait]
    impl PdfCompiler for PanicOnceCompiler {
        async fn prepare(&self, _: &TemplateSource) -> Result<PreparedTemplate, CompilationError> {
            Ok(PreparedTemplate::for_test())
        }

        fn compile_prepared(
            &self,
            _: &PreparedTemplate,
            _: &Value,
        ) -> Result<Document, CompilationError> {
            if !self
                .panicked
                .swap(true, std::sync::atomic::Ordering::SeqCst)
            {
                panic!("secret template source /srv/templates/invoice/main.typ");
            }
            Ok(Document::new(Bytes::from_static(b"%PDF-1.4 mock")))
        }
    }

    fn app_with(compiler: Arc<dyn PdfCompiler>, limits: RenderLimits) -> axum::Router {
        let loader = memory_loader(vec![("invoice", template(schema(json!({}))))]);
        let registry = registry_from_loader(loader.as_ref());
        let render_service = RenderService::new(
            registry.clone(),
            compiler,
            Arc::new(BlockingRenderExecutor::new(limits)),
        );
        router(
            AppState::new(registry, render_service, LogFormat::Pretty),
            &ServerConfig::new(0),
        )
    }

    fn render_request() -> Request<Body> {
        Request::builder()
            .method(Method::POST)
            .uri("/invoice")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap()
    }

    fn health_request() -> Request<Body> {
        Request::builder()
            .uri("/_healthz")
            .body(Body::empty())
            .unwrap()
    }

    async fn wait_for_start(
        started: std::sync::mpsc::Receiver<()>,
    ) -> std::sync::mpsc::Receiver<()> {
        tokio::task::spawn_blocking(move || {
            started.recv().unwrap();
            started
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn healthz_responds_while_a_render_is_running() {
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let app = app_with(
            Arc::new(GatedCompiler {
                started: started_tx,
                release: std::sync::Mutex::new(release_rx),
            }),
            RenderLimits::new(1, 0),
        );
        let render = tokio::spawn(app.clone().oneshot(render_request()));
        let _started = wait_for_start(started_rx).await;

        let health = app.clone().oneshot(health_request()).await.unwrap();

        assert_eq!(health.status(), StatusCode::OK);
        release_tx.send(()).unwrap();
        assert_eq!(render.await.unwrap().unwrap().status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn saturated_server_returns_503_with_retry_after_and_keeps_health_ok() {
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let app = app_with(
            Arc::new(GatedCompiler {
                started: started_tx,
                release: std::sync::Mutex::new(release_rx),
            }),
            RenderLimits::new(1, 0),
        );
        let render = tokio::spawn(app.clone().oneshot(render_request()));
        let _started = wait_for_start(started_rx).await;

        let rejected = app.clone().oneshot(render_request()).await.unwrap();
        let health = app.clone().oneshot(health_request()).await.unwrap();

        assert_eq!(rejected.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(rejected.headers()["retry-after"], "1");
        assert_eq!(
            response_body(rejected).await["error"],
            json!("render_capacity_exceeded")
        );
        assert_eq!(health.status(), StatusCode::OK);
        release_tx.send(()).unwrap();
        assert_eq!(render.await.unwrap().unwrap().status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn invalid_requests_do_not_wait_for_capacity_when_saturated() {
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let app = app_with(
            Arc::new(GatedCompiler {
                started: started_tx,
                release: std::sync::Mutex::new(release_rx),
            }),
            RenderLimits::new(1, 0),
        );
        let render = tokio::spawn(app.clone().oneshot(render_request()));
        let _started = wait_for_start(started_rx).await;

        let missing = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/missing")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        let malformed = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/invoice")
                    .header("content-type", "application/json")
                    .body(Body::from("{"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
        release_tx.send(()).unwrap();
        render.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn panicking_compilation_returns_500_and_the_server_keeps_serving() {
        let app = app_with(
            Arc::new(PanicOnceCompiler {
                panicked: std::sync::atomic::AtomicBool::new(false),
            }),
            RenderLimits::new(1, 0),
        );

        let failed = app.clone().oneshot(render_request()).await.unwrap();

        assert_eq!(failed.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = response_body(failed).await;
        assert_eq!(body["error"], json!("render_execution_failed"));
        assert!(body.get("diagnostics").is_none());
        assert!(!body.to_string().contains("secret template source"));
        let recovered = app.clone().oneshot(render_request()).await.unwrap();
        assert_eq!(recovered.status(), StatusCode::OK);
    }

    fn write(path: &std::path::Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn example_templates_dir(project: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples")
            .join(project)
            .join("templates")
            .canonicalize()
            .unwrap()
    }

    async fn post_json(address: std::net::SocketAddr, path: &str, body: &[u8]) -> Vec<u8> {
        let stream = tokio::net::TcpStream::connect(address).await.unwrap();
        stream.writable().await.unwrap();

        let request = format!(
            "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.try_write(request.as_bytes()).unwrap();
        stream.try_write(body).unwrap();

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

        response
    }
}
