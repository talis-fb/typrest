use serde_json::Value;

use crate::TemplateName;

#[derive(Clone, Debug, PartialEq)]
pub struct RenderRequest {
    pub template: TemplateName,
    pub payload: Value,
}

impl RenderRequest {
    pub fn new(template: TemplateName, payload: Value) -> Self {
        Self { template, payload }
    }
}
