use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use bytes::Bytes;
use thiserror::Error;
use typrest_core::{
    LoaderError, Schema, TemplateBundle, TemplateId, TemplateLoader, TemplateSource,
};

#[derive(Clone, Debug)]
pub struct FilesystemLoader {
    root: PathBuf,
}

impl FilesystemLoader {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn load_sync(&self, name: &TemplateId) -> Result<TemplateBundle, LoaderError> {
        let template_dir = name
            .segments()
            .fold(self.root.clone(), |path, segment| path.join(segment));

        if !template_dir.is_dir() {
            return Err(LoaderError::TemplateNotFound(name.clone()));
        }

        let main_path = template_dir.join("main.typ");
        if !main_path.is_file() {
            return Err(FilesystemLoaderError::MissingMain {
                template: name.clone(),
            }
            .into());
        }

        let unallowed_paths = [template_dir.join("schema.yaml")];
        if unallowed_paths.iter().any(|path| path.is_file()) {
            return Err(FilesystemLoaderError::UnsupportedSchemaFilename {
                template: name.clone(),
            }
            .into());
        }

        let yaml_schema_path = template_dir.join("schema.yml");
        let json_schema_path = template_dir.join("schema.json");

        let schema = match (yaml_schema_path.is_file(), json_schema_path.is_file()) {
            (true, true) => {
                return Err(FilesystemLoaderError::ConflictingSchemaFiles {
                    template: name.clone(),
                }
                .into());
            }
            (true, false) => Some(read_schema(&yaml_schema_path, Schema::parse_yaml)?),
            (false, true) => Some(read_schema(&json_schema_path, Schema::parse_json)?),
            (false, false) => None,
        };

        let mut files = BTreeMap::new();
        read_tree(&template_dir, &template_dir, "", &mut files)?;

        let shared_dir = self.root.join("_shared");
        if shared_dir.is_dir() {
            read_tree(&shared_dir, &shared_dir, "_shared", &mut files)?;
        }

        let source = TemplateSource::new(files);
        Ok(match schema {
            Some(schema) => TemplateBundle::new(source, schema),
            None => TemplateBundle::without_schema(source),
        })
    }

    pub(crate) fn list_sync(&self) -> Result<Vec<TemplateId>, LoaderError> {
        if self.root.join("main.typ").is_file() {
            return Err(FilesystemLoaderError::RootMain.into());
        }

        let mut names = Vec::new();
        discover_templates(&self.root, &self.root, &[], &mut names)?;
        names.sort();
        Ok(names)
    }
}

#[async_trait]
impl TemplateLoader for FilesystemLoader {
    async fn load(&self, name: &TemplateId) -> Result<TemplateBundle, LoaderError> {
        self.load_sync(name)
    }

    async fn list(&self) -> Result<Vec<TemplateId>, LoaderError> {
        self.list_sync()
    }
}

#[derive(Debug, Error)]
enum FilesystemLoaderError {
    #[error("templates/main.typ cannot define an unnamed template; move it into a named folder")]
    RootMain,

    #[error("template '{template}' is missing main.typ")]
    MissingMain { template: TemplateId },

    #[error(
        "template '{template}' contains both schema.yml and schema.json; keep only one schema definition"
    )]
    ConflictingSchemaFiles { template: TemplateId },

    #[error("template '{template}' contains unsupported schema.yaml; rename it to schema.yml")]
    UnsupportedSchemaFilename { template: TemplateId },

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

fn discover_templates(
    root: &Path,
    current_dir: &Path,
    parent_segments: &[String],
    names: &mut Vec<TemplateId>,
) -> Result<(), LoaderError> {
    let entries = fs::read_dir(current_dir).map_err(|error| FilesystemLoaderError::ReadDir {
        path: current_dir.to_owned(),
        error,
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| FilesystemLoaderError::ReadDir {
            path: current_dir.to_owned(),
            error,
        })?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| FilesystemLoaderError::ReadDir {
                path: path.clone(),
                error,
            })?;
        if !file_type.is_dir() {
            continue;
        }

        let segment = entry
            .file_name()
            .to_str()
            .map(str::to_owned)
            .ok_or_else(|| FilesystemLoaderError::NonUtf8Path { path: path.clone() })?;
        if segment.starts_with('_') {
            continue;
        }

        let mut segments = parent_segments.to_vec();
        segments.push(segment);
        let id = TemplateId::new(segments.join("/"))?;
        if path.join("main.typ").is_file() {
            names.push(id);
            continue;
        }

        debug_assert!(path.starts_with(root));
        discover_templates(root, &path, &segments, names)?;
    }

    Ok(())
}

fn read_schema(
    path: &Path,
    parse: fn(&[u8]) -> Result<Schema, LoaderError>,
) -> Result<Schema, LoaderError> {
    let bytes = fs::read(path).map_err(|error| FilesystemLoaderError::Read {
        path: path.to_owned(),
        error,
    })?;
    parse(&bytes)
}

impl From<FilesystemLoaderError> for LoaderError {
    fn from(error: FilesystemLoaderError) -> Self {
        Self::IoError(error.to_string())
    }
}

fn read_tree(
    base_dir: &Path,
    current_dir: &Path,
    prefix: &str,
    files: &mut BTreeMap<String, Bytes>,
) -> Result<(), LoaderError> {
    let entries = fs::read_dir(current_dir).map_err(|error| FilesystemLoaderError::ReadDir {
        path: current_dir.to_owned(),
        error,
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| FilesystemLoaderError::ReadDir {
            path: current_dir.to_owned(),
            error,
        })?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| FilesystemLoaderError::ReadDir {
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
            .map_err(|_| FilesystemLoaderError::NonUtf8Path { path: path.clone() })?;
        let render_path = render_path(prefix, relative_path)?;
        let bytes = fs::read(&path).map_err(|error| FilesystemLoaderError::Read {
            path: path.clone(),
            error,
        })?;

        files.insert(render_path, Bytes::from(bytes));
    }

    Ok(())
}

fn render_path(prefix: &str, relative_path: &Path) -> Result<String, LoaderError> {
    let mut segments = Vec::new();
    if !prefix.is_empty() {
        segments.push(prefix.to_owned());
    }

    for component in relative_path.components() {
        let value =
            component
                .as_os_str()
                .to_str()
                .ok_or_else(|| FilesystemLoaderError::NonUtf8Path {
                    path: relative_path.to_owned(),
                })?;
        segments.push(value.to_owned());
    }

    Ok(segments.join("/"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use bytes::Bytes;
    use futures_executor::block_on;
    use tempfile::tempdir;

    use typrest_core::{TemplateId, TemplateLoader};

    use super::FilesystemLoader;

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
            &root.path().join("invoice/schema.yml"),
            "schemaVersion: v1\nrequest:\n  body:\n    type: object\n",
        );
        write(&root.path().join("invoice/assets/item.typ"), "item");
        write(&root.path().join("_shared/brand.typ"), "brand");

        let loader = FilesystemLoader::new(root.path());
        let bundle = block_on(loader.load(&TemplateId::new("invoice").unwrap())).unwrap();

        assert!(bundle.source.files().contains_key("main.typ"));
        assert!(bundle.source.files().contains_key("schema.yml"));
        assert!(bundle.source.files().contains_key("assets/item.typ"));
        assert!(bundle.source.files().contains_key("_shared/brand.typ"));
        assert_eq!(bundle.schema.as_ref().unwrap().schema_version, "v1");
    }

    #[test]
    fn resolves_template_without_schema() {
        let root = tempdir().unwrap();
        write(&root.path().join("welcome/main.typ"), "Hello");

        let loader = FilesystemLoader::new(root.path());
        let bundle = block_on(loader.load(&TemplateId::new("welcome").unwrap())).unwrap();

        assert!(bundle.schema.is_none());
        assert!(bundle.source.files().contains_key("main.typ"));
    }

    #[test]
    fn resolves_legacy_json_schema() {
        let root = tempdir().unwrap();
        write(&root.path().join("invoice/main.typ"), "Hello");
        write(
            &root.path().join("invoice/schema.json"),
            r#"{"schemaVersion":"v1","request":{"body":{"type":"object"}}}"#,
        );

        let loader = FilesystemLoader::new(root.path());
        let bundle = block_on(loader.load(&TemplateId::new("invoice").unwrap())).unwrap();

        assert_eq!(bundle.schema.as_ref().unwrap().schema_version, "v1");
        assert!(bundle.source.files().contains_key("schema.json"));
    }

    #[test]
    fn rejects_conflicting_schema_files() {
        let root = tempdir().unwrap();
        write(&root.path().join("invoice/main.typ"), "Hello");
        write(
            &root.path().join("invoice/schema.yml"),
            "schemaVersion: v1\nrequest:\n  body: {}\n",
        );
        write(
            &root.path().join("invoice/schema.json"),
            r#"{"schemaVersion":"v1","request":{"body":{}}}"#,
        );

        let loader = FilesystemLoader::new(root.path());
        let error = block_on(loader.load(&TemplateId::new("invoice").unwrap())).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("both schema.yml and schema.json")
        );
    }

    #[test]
    fn rejects_unsupported_yaml_suffix() {
        let root = tempdir().unwrap();
        write(&root.path().join("invoice/main.typ"), "Hello");
        write(
            &root.path().join("invoice/schema.yaml"),
            "schemaVersion: v1\nrequest:\n  body: {}\n",
        );

        let loader = FilesystemLoader::new(root.path());
        let error = block_on(loader.load(&TemplateId::new("invoice").unwrap())).unwrap_err();

        assert!(error.to_string().contains("rename it to schema.yml"));
    }

    #[test]
    fn lists_template_directories_without_shared() {
        let root = tempdir().unwrap();
        write(
            &root.path().join("invoice/schema.yml"),
            "schemaVersion: v1\nrequest:\n  body: {}\n",
        );
        write(&root.path().join("invoice/main.typ"), "");
        write(&root.path().join("welcome/main.typ"), "");
        write(
            &root.path().join("welcome/schema.yml"),
            "schemaVersion: v1\nrequest:\n  body: {}\n",
        );
        write(&root.path().join("_shared/brand.typ"), "");

        let loader = FilesystemLoader::new(root.path());
        let names = block_on(loader.list()).unwrap();

        assert_eq!(
            names.iter().map(|name| name.as_str()).collect::<Vec<_>>(),
            vec!["invoice", "welcome"]
        );
    }

    #[test]
    fn discovers_nested_templates_in_route_order() {
        let root = tempdir().unwrap();
        for id in [
            "financial/invoice",
            "financial/report",
            "support/leads/welcome",
            "support/leads/feedback",
            "birthday-cart",
        ] {
            write(&root.path().join(id).join("main.typ"), id);
        }

        let names = block_on(FilesystemLoader::new(root.path()).list()).unwrap();

        assert_eq!(
            names.iter().map(TemplateId::as_str).collect::<Vec<_>>(),
            vec![
                "birthday-cart",
                "financial/invoice",
                "financial/report",
                "support/leads/feedback",
                "support/leads/welcome",
            ]
        );
    }

    #[test]
    fn shallow_template_hides_descendant_entrypoints_but_bundles_their_files() {
        let root = tempdir().unwrap();
        write(&root.path().join("financial/main.typ"), "Financial");
        write(
            &root.path().join("financial/invoice/main.typ"),
            "Hidden invoice",
        );
        write(
            &root.path().join("financial/_components/header.typ"),
            "Header",
        );

        let loader = FilesystemLoader::new(root.path());
        let names = block_on(loader.list()).unwrap();
        let bundle = block_on(loader.load(&TemplateId::new("financial").unwrap())).unwrap();

        assert_eq!(
            names.iter().map(TemplateId::as_str).collect::<Vec<_>>(),
            vec!["financial"]
        );
        assert!(bundle.source.files().contains_key("invoice/main.typ"));
        assert!(bundle.source.files().contains_key("_components/header.typ"));
    }

    #[test]
    fn skips_private_directories_during_discovery() {
        let root = tempdir().unwrap();
        write(&root.path().join("_draft/main.typ"), "Draft");
        write(
            &root.path().join("support/_draft/welcome/main.typ"),
            "Draft welcome",
        );
        write(
            &root.path().join("support/leads/welcome/main.typ"),
            "Welcome",
        );

        let names = block_on(FilesystemLoader::new(root.path()).list()).unwrap();

        assert_eq!(
            names.iter().map(TemplateId::as_str).collect::<Vec<_>>(),
            vec!["support/leads/welcome"]
        );
    }

    #[test]
    fn rejects_root_main_and_invalid_visible_directories() {
        let root = tempdir().unwrap();
        write(&root.path().join("main.typ"), "Unnamed");
        let loader = FilesystemLoader::new(root.path());
        assert!(
            block_on(loader.list())
                .unwrap_err()
                .to_string()
                .contains("unnamed")
        );

        fs::remove_file(root.path().join("main.typ")).unwrap();
        write(
            &root.path().join("financial reports/invoice/main.typ"),
            "Invalid",
        );
        assert!(
            block_on(loader.list())
                .unwrap_err()
                .to_string()
                .contains("invalid template ID")
        );
    }

    #[test]
    fn global_shared_files_replace_same_path_local_files() {
        let root = tempdir().unwrap();
        write(&root.path().join("invoice/main.typ"), "Invoice");
        write(&root.path().join("invoice/_shared/brand.typ"), "local");
        write(&root.path().join("_shared/brand.typ"), "global");

        let bundle =
            block_on(FilesystemLoader::new(root.path()).load(&TemplateId::new("invoice").unwrap()))
                .unwrap();

        assert_eq!(
            bundle.source.files().get("_shared/brand.typ").unwrap(),
            &Bytes::from_static(b"global")
        );
    }
}
