use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "papelon")]
#[command(about = "HTTP server for server-side PDF generation with Typst")]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Start the HTTP server in filesystem mode.
    Serve {
        /// Port to listen on.
        #[arg(long)]
        #[arg(default_value_t = 8080)]
        port: u16,

        /// Watch the templates directory and log template reloads.
        #[arg(long)]
        watch: bool,
    },

    /// Create a new template directory with boilerplate files.
    Init {
        /// Target directory to initialize.
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
    },
}

#[cfg(test)]
mod tests {
    use super::{Args, Command};
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn parses_serve_with_default_port() {
        let args = Args::parse_from(["papelon", "serve"]);

        let Command::Serve { port, watch } = args.command else {
            panic!("expected serve command");
        };

        assert_eq!(port, 8080);
        assert!(!watch);
    }

    #[test]
    fn parses_serve_with_port_and_watch() {
        let args = Args::parse_from(["papelon", "serve", "--port", "9090", "--watch"]);

        let Command::Serve { port, watch } = args.command else {
            panic!("expected serve command");
        };

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
        let args = Args::parse_from(["papelon", "init"]);

        let Command::Init { path } = args.command else {
            panic!("expected init command");
        };

        assert!(path.is_none());
    }

    #[test]
    fn parses_init_command_with_path() {
        let args = Args::parse_from(["papelon", "init", "invoice"]);

        let Command::Init { path } = args.command else {
            panic!("expected init command");
        };

        assert_eq!(path, Some(PathBuf::from("invoice")));
    }

    #[test]
    fn rejects_old_init_name_form() {
        let result = Args::try_parse_from(["papelon", "init", "--path", "invoice"]);

        assert!(result.is_err());
    }
}
