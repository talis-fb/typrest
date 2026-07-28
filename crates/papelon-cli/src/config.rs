use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

pub const CONFIG_FILE: &str = "papelon.toml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectConfig {
    pub templates_root: PathBuf,
    pub web_ui: bool,
}

#[derive(Debug, Deserialize)]
struct RawProjectConfig {
    schema_version: String,
    templates_root: String,
    #[serde(default)]
    web_ui: bool,
}

pub fn load_project_config(project_root: &Path) -> std::io::Result<ProjectConfig> {
    let config_path = project_root.join(CONFIG_FILE);
    let content = fs::read_to_string(&config_path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{} not found in {}", CONFIG_FILE, project_root.display()),
            )
        } else {
            error
        }
    })?;

    let config = parse_project_config(&content)?;
    let templates_root = resolve_templates_root(project_root, &config.templates_root)?;

    if !templates_root.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "templates_root '{}' does not exist",
                templates_root.display()
            ),
        ));
    }

    if !templates_root.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "templates_root '{}' is not a directory",
                templates_root.display()
            ),
        ));
    }

    Ok(ProjectConfig {
        templates_root,
        web_ui: config.web_ui,
    })
}

pub fn find_project_root(start_dir: &Path) -> std::io::Result<PathBuf> {
    for dir in start_dir.ancestors() {
        if dir.join(CONFIG_FILE).is_file() {
            return Ok(dir.to_path_buf());
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!(
            "{} not found in {} or any parent directory",
            CONFIG_FILE,
            start_dir.display()
        ),
    ))
}

fn parse_project_config(content: &str) -> std::io::Result<RawProjectConfig> {
    let config = toml::from_str::<RawProjectConfig>(content).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid {}: {}", CONFIG_FILE, error.message()),
        )
    })?;

    if config.schema_version != "v1" {
        return invalid_config(format!(
            "invalid {}: unsupported schema_version '{}'",
            CONFIG_FILE, config.schema_version
        ));
    }

    Ok(config)
}

fn resolve_templates_root(project_root: &Path, templates_root: &str) -> std::io::Result<PathBuf> {
    let templates_root = Path::new(templates_root);

    if templates_root.is_absolute() {
        return invalid_config("invalid papelon.toml: templates_root must be relative".to_owned());
    }

    let mut normalized = PathBuf::new();

    for component in templates_root.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                if !normalized.pop() {
                    return invalid_config(
                        "invalid papelon.toml: templates_root must stay inside the project root"
                            .to_owned(),
                    );
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return invalid_config(
                    "invalid papelon.toml: templates_root must be relative".to_owned(),
                );
            }
        }
    }

    Ok(project_root.join(normalized))
}

fn invalid_config<T>(message: String) -> std::io::Result<T> {
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        message,
    ))
}

#[cfg(test)]
mod tests {
    use super::{find_project_root, load_project_config, parse_project_config};

    #[test]
    fn load_project_config_resolves_templates_root() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("papelon.toml"),
            "schema_version = \"v1\"\ntemplates_root = \"./templates\"\n",
        )
        .unwrap();
        std::fs::create_dir(root.path().join("templates")).unwrap();

        let config = load_project_config(root.path()).unwrap();

        assert_eq!(config.templates_root, root.path().join("templates"));
        assert!(!config.web_ui);
    }

    #[test]
    fn load_project_config_rejects_missing_config() {
        let root = tempfile::tempdir().unwrap();

        let error = load_project_config(root.path()).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn load_project_config_rejects_missing_templates_root() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("papelon.toml"),
            "schema_version = \"v1\"\ntemplates_root = \"templates\"\n",
        )
        .unwrap();

        let error = load_project_config(root.path()).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn load_project_config_rejects_non_directory_templates_root() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("papelon.toml"),
            "schema_version = \"v1\"\ntemplates_root = \"templates\"\n",
        )
        .unwrap();
        std::fs::write(root.path().join("templates"), "").unwrap();

        let error = load_project_config(root.path()).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn parse_project_config_rejects_unsupported_schema_version() {
        let error =
            parse_project_config("schema_version = \"v2\"\ntemplates_root = \"templates\"\n")
                .unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn parse_project_config_rejects_missing_templates_root() {
        let error = parse_project_config("schema_version = \"v1\"\n").unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn parse_project_config_rejects_invalid_toml() {
        let error =
            parse_project_config("schema_version = \"v1\"\ntemplates_root = \n").unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn parse_project_config_ignores_unknown_top_level_fields() {
        let config = parse_project_config(
            "schema_version = \"v1\"\ntemplates_root = \"templates\"\nunknown = true\n",
        )
        .unwrap();

        assert_eq!(config.templates_root, "templates");
    }

    #[test]
    fn parse_project_config_defaults_web_ui_to_false() {
        let config =
            parse_project_config("schema_version = \"v1\"\ntemplates_root = \"templates\"\n")
                .unwrap();

        assert!(!config.web_ui);
    }

    #[test]
    fn parse_project_config_accepts_web_ui_boolean() {
        let config = parse_project_config(
            "schema_version = \"v1\"\ntemplates_root = \"templates\"\nweb_ui = true\n",
        )
        .unwrap();

        assert!(config.web_ui);
    }

    #[test]
    fn parse_project_config_rejects_non_boolean_web_ui() {
        let error = parse_project_config(
            "schema_version = \"v1\"\ntemplates_root = \"templates\"\nweb_ui = \"yes\"\n",
        )
        .unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn load_project_config_rejects_templates_root_outside_project() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("papelon.toml"),
            "schema_version = \"v1\"\ntemplates_root = \"../templates\"\n",
        )
        .unwrap();

        let error = load_project_config(root.path()).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn find_project_root_returns_nearest_ancestor_with_config() {
        let root = tempfile::tempdir().unwrap();
        let nested = root.path().join("workspace/project/subfolder");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(
            root.path().join("papelon.toml"),
            "schema_version = \"v1\"\ntemplates_root = \"templates\"\n",
        )
        .unwrap();

        let found = find_project_root(&nested).unwrap();

        assert_eq!(found, root.path());
    }

    #[test]
    fn find_project_root_prefers_nearest_config() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("outer/project");
        let nested = project.join("child/grandchild");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(
            root.path().join("papelon.toml"),
            "schema_version = \"v1\"\ntemplates_root = \"templates\"\n",
        )
        .unwrap();
        std::fs::write(
            project.join("papelon.toml"),
            "schema_version = \"v1\"\ntemplates_root = \"templates\"\n",
        )
        .unwrap();

        let found = find_project_root(&nested).unwrap();

        assert_eq!(found, project);
    }

    #[test]
    fn find_project_root_rejects_missing_config_in_ancestors() {
        let root = tempfile::tempdir().unwrap();
        let nested = root.path().join("workspace/project/subfolder");
        std::fs::create_dir_all(&nested).unwrap();

        let error = find_project_root(&nested).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    }
}
