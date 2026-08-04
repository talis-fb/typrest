use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use serde::Serialize;
use typrest_core::{Schema, TemplateId};

use crate::http::errors::ApiError;
use crate::http::state::AppState;

pub async fn list_templates(
    State(state): State<AppState>,
) -> Result<Json<WebUiTemplatesResponse>, ApiError> {
    let templates = state
        .registry
        .ids()
        .into_iter()
        .map(|id| {
            let has_schema = state
                .registry
                .get(&id)
                .is_some_and(|entry| entry.schema().is_some());
            WebUiTemplateSummary {
                id: id.into_inner(),
                has_schema,
            }
        })
        .collect();

    Ok(Json(WebUiTemplatesResponse { templates }))
}

pub async fn template_metadata(
    State(state): State<AppState>,
    Path(template_id): Path<String>,
) -> Result<Json<WebUiTemplateMetadata>, ApiError> {
    let id = TemplateId::new(template_id).map_err(|error| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "invalid_template_id",
            error.to_string(),
        )
    })?;
    let entry = state.registry.get(&id).ok_or_else(|| {
        ApiError::new(
            StatusCode::NOT_FOUND,
            "template_not_found",
            format!("Template '{id}' was not found"),
        )
    })?;

    Ok(Json(WebUiTemplateMetadata {
        id: id.into_inner(),
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
    pub id: String,
    pub has_schema: bool,
}

#[derive(Serialize)]
pub struct WebUiTemplateMetadata {
    pub id: String,
    pub schema: Option<Schema>,
}
