use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use bytes::Bytes;
use papelon_core::{
    ResolvedTemplate, ResolverError, Schema, TemplateName, TemplateResolver, TemplateSource,
};
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct FilesystemResolver {
    root: PathBuf,
}

impl FilesystemResolver {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn resolve_sync(&self, name: &TemplateName) -> Result<ResolvedTemplate, ResolverError> {
        let template_dir = self.root.join(name.as_str());

        if !template_dir.is_dir() {
            return Err(ResolverError::TemplateNotFound(name.clone()));
        }

        let main_path = template_dir.join("main.typ");
        if !main_path.is_file() {
            return Err(FilesystemResolverError::MissingMain {
                template: name.clone(),
            }
            .into());
        }

        let schema_path = template_dir.join("schema.json");
        let schema_bytes =
            fs::read(&schema_path).map_err(|error| FilesystemResolverError::Read {
                path: schema_path.clone(),
                error,
            })?;
        let schema = Schema::parse(&schema_bytes)?;

        let mut files = BTreeMap::new();
        read_tree(&template_dir, &template_dir, "", &mut files)?;

        let shared_dir = self.root.join("_shared");
        if shared_dir.is_dir() {
            read_tree(&shared_dir, &shared_dir, "_shared", &mut files)?;
        }

        Ok(ResolvedTemplate::new(TemplateSource::new(files), schema))
    }

    fn list_sync(&self) -> Result<Vec<TemplateName>, ResolverError> {
        let entries =
            fs::read_dir(&self.root).map_err(|error| FilesystemResolverError::ReadDir {
                path: self.root.clone(),
                error,
            })?;
        let mut names = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|error| FilesystemResolverError::ReadDir {
                path: self.root.clone(),
                error,
            })?;

            if !entry
                .file_type()
                .map_err(|error| FilesystemResolverError::ReadDir {
                    path: entry.path(),
                    error,
                })?
                .is_dir()
            {
                continue;
            }

            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };

            if name == "_shared" {
                continue;
            }

            names.push(TemplateName::new(name)?);
        }

        names.sort();
        Ok(names)
    }
}

#[async_trait]
impl TemplateResolver for FilesystemResolver {
    async fn resolve(&self, name: &TemplateName) -> Result<ResolvedTemplate, ResolverError> {
        self.resolve_sync(name)
    }

    async fn list(&self) -> Result<Vec<TemplateName>, ResolverError> {
        self.list_sync()
    }
}

#[derive(Debug, Error)]
enum FilesystemResolverError {
    #[error("template '{template}' is missing main.typ")]
    MissingMain { template: TemplateName },

    #[error("failed to read '{path}': {error}")]
    Read {
        path: PathBuf,
        #[source]
        error: std::io::Error,
    },

    #[error("failed to read directory '{path}': {error}")]
    ReadDir {
        path: PathBuf,
        #[source]
        error: std::io::Error,
    },

    #[error("template path '{path}' is not valid UTF-8")]
    NonUtf8Path { path: PathBuf },
}

impl From<FilesystemResolverError> for ResolverError {
    fn from(error: FilesystemResolverError) -> Self {
        Self::Source(error.to_string())
    }
}

fn read_tree(
    base_dir: &Path,
    current_dir: &Path,
    prefix: &str,
    files: &mut BTreeMap<String, Bytes>,
) -> Result<(), ResolverError> {
    let entries = fs::read_dir(current_dir).map_err(|error| FilesystemResolverError::ReadDir {
        path: current_dir.to_owned(),
        error,
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| FilesystemResolverError::ReadDir {
            path: current_dir.to_owned(),
            error,
        })?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| FilesystemResolverError::ReadDir {
                path: path.clone(),
                error,
            })?;

        if file_type.is_dir() {
            read_tree(base_dir, &path, prefix, files)?;
            continue;
        }

        if !file_type.is_file() {
            continue;
        }

        let relative_path = path
            .strip_prefix(base_dir)
            .map_err(|_| FilesystemResolverError::NonUtf8Path { path: path.clone() })?;
        let render_path = render_path(prefix, relative_path)?;
        let bytes = fs::read(&path).map_err(|error| FilesystemResolverError::Read {
            path: path.clone(),
            error,
        })?;

        files.insert(render_path, Bytes::from(bytes));
    }

    Ok(())
}

fn render_path(prefix: &str, relative_path: &Path) -> Result<String, ResolverError> {
    let mut segments = Vec::new();
    if !prefix.is_empty() {
        segments.push(prefix.to_owned());
    }

    for component in relative_path.components() {
        let value =
            component
                .as_os_str()
                .to_str()
                .ok_or_else(|| FilesystemResolverError::NonUtf8Path {
                    path: relative_path.to_owned(),
                })?;
        segments.push(value.to_owned());
    }

    Ok(segments.join("/"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use futures_executor::block_on;
    use tempfile::tempdir;

    use papelon_core::{TemplateName, TemplateResolver};

    use super::FilesystemResolver;

    fn write(path: &std::path::Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn resolves_template_files_schema_and_shared_files() {
        let root = tempdir().unwrap();
        write(
            &root.path().join("invoice/main.typ"),
            "#import \"assets/item.typ\"",
        );
        write(
            &root.path().join("invoice/schema.json"),
            r#"{"schemaVersion":"v1","request":{"body":{"type":"object"}}}"#,
        );
        write(&root.path().join("invoice/assets/item.typ"), "item");
        write(&root.path().join("_shared/brand.typ"), "brand");

        let resolver = FilesystemResolver::new(root.path());
        let resolved = block_on(resolver.resolve(&TemplateName::new("invoice").unwrap())).unwrap();

        assert!(resolved.source.files().contains_key("main.typ"));
        assert!(resolved.source.files().contains_key("schema.json"));
        assert!(resolved.source.files().contains_key("assets/item.typ"));
        assert!(resolved.source.files().contains_key("_shared/brand.typ"));
        assert_eq!(resolved.schema.schema_version, "v1");
    }

    #[test]
    fn lists_template_directories_without_shared() {
        let root = tempdir().unwrap();
        write(
            &root.path().join("invoice/schema.json"),
            r#"{"schemaVersion":"v1","request":{"body":{}}}"#,
        );
        write(&root.path().join("invoice/main.typ"), "");
        write(&root.path().join("welcome/main.typ"), "");
        write(
            &root.path().join("welcome/schema.json"),
            r#"{"schemaVersion":"v1","request":{"body":{}}}"#,
        );
        write(&root.path().join("_shared/brand.typ"), "");

        let resolver = FilesystemResolver::new(root.path());
        let names = block_on(resolver.list()).unwrap();

        assert_eq!(
            names.iter().map(|name| name.as_str()).collect::<Vec<_>>(),
            vec!["invoice", "welcome"]
        );
    }
}
