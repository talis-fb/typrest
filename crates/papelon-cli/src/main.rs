use std::fs;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use papelon_server::server::ServerConfig;

#[derive(Debug, Parser)]
#[command(name = "papelon")]
#[command(about = "HTTP server for server-side PDF generation with Typst")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Start the HTTP server in filesystem mode.
    Serve {
        /// Templates directory for filesystem mode.
        templates: PathBuf,

        /// Port to listen on.
        #[arg(default_value_t = 8080)]
        port: u16,

        /// Watch the templates directory and log template reloads.
        #[arg(long)]
        watch: bool,
    },

    /// Create a new template directory with boilerplate files.
    Init {
        /// Template directory to create.
        name: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    match args.command {
        Command::Serve {
            templates,
            port,
            watch,
        } => papelon_server::serve(ServerConfig::new(templates, port).with_watch(watch)).await,
        Command::Init { name } => {
            init_template(&name)?;
            Ok(())
        }
    }
}

fn init_template(name: &Path) -> std::io::Result<()> {
    if name.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("template directory '{}' already exists", name.display()),
        ));
    }

    fs::create_dir_all(name)?;
    fs::write(name.join("main.typ"), MAIN_TYP)?;
    fs::write(name.join("schema.json"), SCHEMA_JSON)?;

    Ok(())
}

const MAIN_TYP: &str = r#"#let data = json(bytes(sys.inputs.data))

= #data.title

#data.message
"#;

const SCHEMA_JSON: &str = r#"{
  "schemaVersion": "v1",
  "request": {
    "body": {
      "type": "object",
      "properties": {
        "title": {
          "type": "string",
          "required": true
        },
        "message": {
          "type": "string",
          "required": true
        }
      }
    }
  }
}
"#;

#[cfg(test)]
mod tests {
    use super::{Args, Command, init_template};
    use clap::Parser;

    #[test]
    fn parses_serve_with_default_port() {
        let args = Args::parse_from(["papelon", "serve", "./templates"]);

        let Command::Serve {
            templates,
            port,
            watch,
        } = args.command
        else {
            panic!("expected serve command");
        };

        assert_eq!(templates, std::path::PathBuf::from("./templates"));
        assert_eq!(port, 8080);
        assert!(!watch);
    }

    #[test]
    fn parses_serve_with_port_and_watch() {
        let args = Args::parse_from(["papelon", "serve", "./templates", "9090", "--watch"]);

        let Command::Serve {
            templates,
            port,
            watch,
        } = args.command
        else {
            panic!("expected serve command");
        };

        assert_eq!(templates, std::path::PathBuf::from("./templates"));
        assert_eq!(port, 9090);
        assert!(watch);
    }

    #[test]
    fn rejects_old_templates_flag_form() {
        let result = Args::try_parse_from(["papelon", "--templates", "./templates"]);

        assert!(result.is_err());
    }

    #[test]
    fn parses_init_command() {
        let args = Args::parse_from(["papelon", "init", "invoice"]);

        let Command::Init { name } = args.command else {
            panic!("expected init command");
        };

        assert_eq!(name, std::path::PathBuf::from("invoice"));
    }

    #[test]
    fn init_creates_boilerplate_files() {
        let root = tempfile::tempdir().unwrap();
        let template_dir = root.path().join("invoice");

        init_template(&template_dir).unwrap();

        let main = std::fs::read_to_string(template_dir.join("main.typ")).unwrap();
        let schema = std::fs::read_to_string(template_dir.join("schema.json")).unwrap();

        assert!(main.contains("#let data = json(bytes(sys.inputs.data))"));
        assert!(schema.contains(r#""schemaVersion": "v1""#));
    }

    #[test]
    fn init_refuses_to_overwrite_existing_directory() {
        let root = tempfile::tempdir().unwrap();
        let template_dir = root.path().join("invoice");
        std::fs::create_dir(&template_dir).unwrap();

        let error = init_template(&template_dir).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    }
}
