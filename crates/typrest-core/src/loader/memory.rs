use std::collections::BTreeMap;

use async_trait::async_trait;

use crate::{TemplateBundle, TemplateId};

use super::{LoaderError, TemplateLoader};

#[derive(Clone, Debug, Default)]
pub struct MemoryLoader {
    templates: BTreeMap<TemplateId, TemplateBundle>,
}

impl MemoryLoader {
    pub fn new(templates: BTreeMap<TemplateId, TemplateBundle>) -> Self {
        Self { templates }
    }

    pub fn insert(&mut self, template_id: TemplateId, template: TemplateBundle) {
        self.templates.insert(template_id, template);
    }
}

#[async_trait]
impl TemplateLoader for MemoryLoader {
    async fn load(&self, template_id: &TemplateId) -> Result<TemplateBundle, LoaderError> {
        self.templates
            .get(template_id)
            .cloned()
            .ok_or_else(|| LoaderError::TemplateNotFound(template_id.clone()))
    }

    async fn list(&self) -> Result<Vec<TemplateId>, LoaderError> {
        Ok(self.templates.keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bytes::Bytes;
    use serde_json::json;

    use crate::loader::{LoaderError, TemplateLoader};
    use crate::schema::Schema;
    use crate::{TemplateBundle, TemplateId, TemplateSource};

    use super::MemoryLoader;

    fn template() -> TemplateBundle {
        let schema = Schema::parse_json(
            json!({
                "schemaVersion": "v1",
                "request": {
                    "body": {}
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
    fn loads_templates_from_memory() {
        let name = TemplateId::new("invoice").unwrap();
        let loader = MemoryLoader::new(BTreeMap::from([(name.clone(), template())]));

        let bundle = futures_executor::block_on(loader.load(&name)).unwrap();

        assert!(bundle.source.files().contains_key("main.typ"));
    }

    #[test]
    fn returns_not_found_for_missing_template() {
        let name = TemplateId::new("missing").unwrap();
        let loader = MemoryLoader::default();

        let error = futures_executor::block_on(loader.load(&name)).unwrap_err();

        assert!(matches!(error, LoaderError::TemplateNotFound(_)));
    }
}
