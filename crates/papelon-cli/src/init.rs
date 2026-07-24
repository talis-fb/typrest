use std::fs;
use std::path::Path;

use crate::config::CONFIG_FILE;

const DEFAULT_TEMPLATES_ROOT: &str = "templates";
const HELLO_WORLD_TEMPLATE: &str = "hello-world";

pub fn init_project(project_root: &Path) -> std::io::Result<()> {
    let config_path = project_root.join(CONFIG_FILE);
    if config_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("{} already exists", config_path.display()),
        ));
    }

    let templates_root = project_root.join(DEFAULT_TEMPLATES_ROOT);
    if templates_root.exists() && !templates_root.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "templates root '{}' already exists and is not a directory",
                templates_root.display()
            ),
        ));
    }

    let hello_world = templates_root.join(HELLO_WORLD_TEMPLATE);
    if hello_world.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "template directory '{}' already exists",
                hello_world.display()
            ),
        ));
    }

    fs::create_dir_all(&hello_world)?;
    fs::write(config_path, PAPELON_TOML)?;
    fs::write(hello_world.join("main.typ"), MAIN_TYP)?;
    fs::write(hello_world.join("schema.json"), SCHEMA_JSON)?;

    Ok(())
}

const PAPELON_TOML: &str = r#"schema_version = "v1"
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
    use super::init_project;

    #[test]
    fn init_creates_project_files() {
        let root = tempfile::tempdir().unwrap();

        init_project(root.path()).unwrap();

        let config = std::fs::read_to_string(root.path().join("papelon.toml")).unwrap();
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
    fn init_refuses_existing_config_file() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("papelon.toml"), "").unwrap();

        let error = init_project(root.path()).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    }
}
