use serde_json::Value;

use crate::TemplateId;

#[derive(Clone, Debug, PartialEq)]
pub struct RenderRequest {
    pub template: TemplateId,
    pub payload: Value,
}

impl RenderRequest {
    pub fn new(template: TemplateId, payload: Value) -> Self {
        Self { template, payload }
    }
}
