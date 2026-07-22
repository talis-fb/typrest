use std::collections::BTreeMap;

use async_trait::async_trait;

use crate::domain::{ResolvedTemplate, TemplateName};

use super::{ResolverError, TemplateResolver};

#[derive(Clone, Debug, Default)]
pub struct MemoryResolver {
    templates: BTreeMap<TemplateName, ResolvedTemplate>,
}

impl MemoryResolver {
    pub fn new(templates: BTreeMap<TemplateName, ResolvedTemplate>) -> Self {
        Self { templates }
    }

    pub fn insert(&mut self, name: TemplateName, template: ResolvedTemplate) {
        self.templates.insert(name, template);
    }
}

#[async_trait]
impl TemplateResolver for MemoryResolver {
    async fn resolve(&self, name: &TemplateName) -> Result<ResolvedTemplate, ResolverError> {
        self.templates
            .get(name)
            .cloned()
            .ok_or_else(|| ResolverError::TemplateNotFound(name.clone()))
    }

    async fn list(&self) -> Result<Vec<TemplateName>, ResolverError> {
        Ok(self.templates.keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bytes::Bytes;
    use serde_json::json;

    use crate::TemplateName;
    use crate::domain::{ResolvedTemplate, TemplateSource};
    use crate::resolver::{MemoryResolver, ResolverError, TemplateResolver};
    use crate::schema::Schema;

    fn template() -> ResolvedTemplate {
        let schema = Schema::parse(
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

        ResolvedTemplate::new(source, schema)
    }

    #[test]
    fn resolves_templates_from_memory() {
        let name = TemplateName::new("invoice").unwrap();
        let resolver = MemoryResolver::new(BTreeMap::from([(name.clone(), template())]));

        let resolved = futures_executor::block_on(resolver.resolve(&name)).unwrap();

        assert!(resolved.source.files().contains_key("main.typ"));
    }

    #[test]
    fn returns_not_found_for_missing_template() {
        let name = TemplateName::new("missing").unwrap();
        let resolver = MemoryResolver::default();

        let error = futures_executor::block_on(resolver.resolve(&name)).unwrap_err();

        assert!(matches!(error, ResolverError::TemplateNotFound(_)));
    }
}
