use crate::config::CONFIG_FILE;
use std::fs;
use std::path::{Path, PathBuf};
use typrest_server::output::{
    ColorChoice as ServerColorChoice, DiagnosticContext, DiagnosticReporter, DiagnosticStage,
    OutputFormat,
};

pub(crate) async fn run_init(
    path: Option<PathBuf>,
    output_format: OutputFormat,
    color: ServerColorChoice,
) -> bool {
    let current_dir = std::env::current_dir().unwrap_or_default();
    let reporter = DiagnosticReporter::new(output_format, color, &current_dir);
    match init_project(&current_dir, path.as_deref()) {
        Ok(()) => true,
        Err(error) => {
            reporter.report_simple_error(
                DiagnosticContext::Startup,
                DiagnosticStage::Load,
                None,
                &error.to_string(),
            );
            false
        }
    }
}

const DEFAULT_TEMPLATES_ROOT: &str = "templates";
const HELLO_WORLD_TEMPLATE: &str = "hello-world";

pub fn init_project(project_root: &Path, target_root: Option<&Path>) -> std::io::Result<()> {
    let project_root = resolve_target_root(project_root, target_root)?;

    if project_root.exists() && !project_root.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "{} already exists and is not a directory",
                project_root.display()
            ),
        ));
    }

    let config_path = project_root.join(CONFIG_FILE);
    if config_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("{} already exists", config_path.display()),
        ));
    }

    let templates_root = project_root.join(DEFAULT_TEMPLATES_ROOT);
    if templates_root.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "templates root '{}' already exists",
                templates_root.display()
            ),
        ));
    }

    fs::create_dir_all(&templates_root)?;

    let hello_world = templates_root.join(HELLO_WORLD_TEMPLATE);
    fs::create_dir_all(&hello_world)?;
    fs::write(config_path, TYPREST_TOML)?;
    fs::write(hello_world.join("main.typ"), MAIN_TYP)?;
    fs::write(hello_world.join("schema.json"), SCHEMA_JSON)?;

    Ok(())
}

fn resolve_target_root(
    project_root: &Path,
    target_root: Option<&Path>,
) -> std::io::Result<PathBuf> {
    match target_root {
        Some(target_root) if target_root.is_absolute() => Ok(target_root.to_path_buf()),
        Some(target_root) => Ok(project_root.join(target_root)),
        None => Ok(project_root.to_path_buf()),
    }
}

const TYPREST_TOML: &str = r#"schema_version = "v1"
templates_root = "templates"
"#;

const MAIN_TYP: &str = r#"#let data = json(bytes(sys.inputs.data))

= Hello #data.name
"#;

const SCHEMA_JSON: &str = r#"{
  "schemaVersion": "v1",
  "request": {
    "body": {
      "type": "object",
      "properties": {
        "name": {
          "type": "string",
          "default": "World"
        }
      }
    }
  }
}
"#;

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::init_project;

    #[test]
    fn init_creates_project_files() {
        let root = tempfile::tempdir().unwrap();

        init_project(root.path(), None).unwrap();

        let config = std::fs::read_to_string(root.path().join("typrest.toml")).unwrap();
        let template_dir = root.path().join("templates").join("hello-world");
        let main = std::fs::read_to_string(template_dir.join("main.typ")).unwrap();
        let schema = std::fs::read_to_string(template_dir.join("schema.json")).unwrap();

        assert_eq!(
            config,
            "schema_version = \"v1\"\ntemplates_root = \"templates\"\n"
        );
        assert!(main.contains("#let data = json(bytes(sys.inputs.data))"));
        assert!(main.contains("= Hello #data.name"));
        assert!(schema.contains(r#""name""#));
        assert!(schema.contains(r#""default": "World""#));
    }

    #[test]
    fn init_creates_project_files_in_target_directory() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("invoice");

        init_project(root.path(), Some(Path::new("invoice"))).unwrap();

        let config = std::fs::read_to_string(target.join("typrest.toml")).unwrap();
        let template_dir = target.join("templates").join("hello-world");
        let main = std::fs::read_to_string(template_dir.join("main.typ")).unwrap();
        let schema = std::fs::read_to_string(template_dir.join("schema.json")).unwrap();

        assert_eq!(
            config,
            "schema_version = \"v1\"\ntemplates_root = \"templates\"\n"
        );
        assert!(main.contains("#let data = json(bytes(sys.inputs.data))"));
        assert!(main.contains("= Hello #data.name"));
        assert!(schema.contains(r#""name""#));
        assert!(schema.contains(r#""default": "World""#));
    }

    #[test]
    fn init_creates_project_files_in_absolute_target_directory() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("absolute-project");

        init_project(root.path(), Some(target.as_path())).unwrap();

        let config = std::fs::read_to_string(target.join("typrest.toml")).unwrap();
        let template_dir = target.join("templates").join("hello-world");
        let main = std::fs::read_to_string(template_dir.join("main.typ")).unwrap();
        let schema = std::fs::read_to_string(template_dir.join("schema.json")).unwrap();

        assert_eq!(
            config,
            "schema_version = \"v1\"\ntemplates_root = \"templates\"\n"
        );
        assert!(main.contains("#let data = json(bytes(sys.inputs.data))"));
        assert!(main.contains("= Hello #data.name"));
        assert!(schema.contains(r#""name""#));
        assert!(schema.contains(r#""default": "World""#));
    }

    #[test]
    fn init_reuses_existing_empty_target_directory() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("invoice");
        std::fs::create_dir(&target).unwrap();

        init_project(root.path(), Some(Path::new("invoice"))).unwrap();

        assert!(target.join("typrest.toml").exists());
        assert!(target.join("templates").join("hello-world").exists());
    }

    #[test]
    fn init_rejects_existing_non_directory_target() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("invoice");
        std::fs::write(&target, "").unwrap();

        let error = init_project(root.path(), Some(Path::new("invoice"))).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    }

    #[test]
    fn init_rejects_existing_config_file() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("typrest.toml"), "").unwrap();

        let error = init_project(root.path(), None).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    }

    #[test]
    fn init_rejects_existing_config_in_target_directory() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("invoice");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("typrest.toml"), "").unwrap();

        let error = init_project(root.path(), Some(Path::new("invoice"))).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    }

    #[test]
    fn init_rejects_existing_templates_directory() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("templates")).unwrap();

        let error = init_project(root.path(), None).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    }

    #[test]
    fn init_rejects_existing_templates_directory_in_target_directory() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("invoice");
        std::fs::create_dir(&target).unwrap();
        std::fs::create_dir(target.join("templates")).unwrap();

        let error = init_project(root.path(), Some(Path::new("invoice"))).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    }
}
