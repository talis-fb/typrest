use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use papelon_core::{Schema, TemplateName};
use serde::Serialize;

use crate::errors::ApiError;
use crate::server::AppState;

pub async fn list_templates(
    State(state): State<AppState>,
) -> Result<Json<WebUiTemplatesResponse>, ApiError> {
    let templates = state
        .registry
        .names()
        .into_iter()
        .map(|name| {
            let has_schema = state
                .registry
                .get(&name)
                .is_some_and(|entry| entry.schema().is_some());
            WebUiTemplateSummary {
                name: name.into_inner(),
                has_schema,
            }
        })
        .collect();

    Ok(Json(WebUiTemplatesResponse { templates }))
}

pub async fn template_metadata(
    State(state): State<AppState>,
    Path(template): Path<String>,
) -> Result<Json<WebUiTemplateMetadata>, ApiError> {
    let name = TemplateName::new(template).map_err(|error| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_template_name",
            error.to_string(),
        )
    })?;
    let entry = state.registry.get(&name).ok_or_else(|| {
        ApiError::new(
            StatusCode::NOT_FOUND,
            "template_not_found",
            format!("Template '{name}' was not found"),
        )
    })?;

    Ok(Json(WebUiTemplateMetadata {
        name: name.into_inner(),
        schema: entry.schema().cloned(),
    }))
}

#[derive(Serialize)]
pub struct WebUiTemplatesResponse {
    pub templates: Vec<WebUiTemplateSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebUiTemplateSummary {
    pub name: String,
    pub has_schema: bool,
}

#[derive(Serialize)]
pub struct WebUiTemplateMetadata {
    pub name: String,
    pub schema: Option<Schema>,
}
