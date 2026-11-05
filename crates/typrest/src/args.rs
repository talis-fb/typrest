use std::net::Ipv4Addr;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "typrest", version)]
#[command(about = "HTTP server for server-side PDF generation with Typst")]
pub struct Args {
    /// Output format for diagnostics and operational logs.
    #[arg(long, global = true, value_enum, default_value_t = LogFormat::Pretty)]
    pub log_format: LogFormat,

    /// Control ANSI colors in pretty output.
    #[arg(long, global = true, value_enum, default_value_t = ColorChoice::Auto)]
    pub color: ColorChoice,

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

        /// Hostname to bind to.
        #[arg(long)]
        #[arg(default_value_t = Ipv4Addr::LOCALHOST)]
        hostname: Ipv4Addr,

        /// Watch the templates directory and log template reloads.
        #[arg(long)]
        watch: bool,
    },

    /// Compile and validate every template without starting the server.
    Check,

    /// Create a new template directory with boilerplate files.
    Init {
        /// Target directory to initialize.
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum LogFormat {
    Pretty,
    Json,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

#[cfg(test)]
mod tests {
    use super::{Args, Command};
    use clap::Parser;
    use std::net::Ipv4Addr;
    use std::path::PathBuf;

    #[test]
    fn parses_serve_with_default_port() {
        let args = Args::parse_from(["typrest", "serve"]);

        let Command::Serve {
            port,
            hostname,
            watch,
        } = args.command
        else {
            panic!("expected serve command");
        };

        assert_eq!(port, 8080);
        assert_eq!(hostname, Ipv4Addr::LOCALHOST);
        assert!(!watch);
    }

    #[test]
    fn parses_serve_with_port_and_watch() {
        let args = Args::parse_from(["typrest", "serve", "--port", "9090", "--watch"]);

        let Command::Serve {
            port,
            hostname,
            watch,
        } = args.command
        else {
            panic!("expected serve command");
        };

        assert_eq!(port, 9090);
        assert_eq!(hostname, Ipv4Addr::LOCALHOST);
        assert!(watch);
    }

    #[test]
    fn parses_serve_with_hostname() {
        let args = Args::parse_from(["typrest", "serve", "--hostname", "127.0.0.1"]);

        let Command::Serve { hostname, .. } = args.command else {
            panic!("expected serve command");
        };

        assert_eq!(hostname, Ipv4Addr::new(127, 0, 0, 1));
    }

    #[test]
    fn parses_serve_with_all_interfaces_hostname() {
        let args = Args::parse_from(["typrest", "serve", "--hostname", "0.0.0.0"]);

        let Command::Serve { hostname, .. } = args.command else {
            panic!("expected serve command");
        };

        assert_eq!(hostname, Ipv4Addr::UNSPECIFIED);
    }

    #[test]
    fn version_flag_prints_package_version() {
        let error = Args::try_parse_from(["typrest", "--version"]).unwrap_err();

        assert_eq!(error.kind(), clap::error::ErrorKind::DisplayVersion);
        assert_eq!(
            error.to_string().trim(),
            format!("typrest {}", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn rejects_invalid_serve_hostname() {
        let result = Args::try_parse_from(["typrest", "serve", "--hostname", "localhost"]);

        assert!(result.is_err());
    }

    #[test]
    fn rejects_old_templates_flag_form() {
        let result = Args::try_parse_from(["typrest", "--templates", "./templates"]);

        assert!(result.is_err());
    }

    #[test]
    fn parses_init_command() {
        let args = Args::parse_from(["typrest", "init"]);

        let Command::Init { path } = args.command else {
            panic!("expected init command");
        };

        assert!(path.is_none());
    }

    #[test]
    fn parses_check_command_and_global_output_options() {
        let args = Args::parse_from([
            "typrest",
            "check",
            "--log-format",
            "json",
            "--color",
            "never",
        ]);

        assert!(matches!(args.command, Command::Check));
        assert_eq!(args.log_format, super::LogFormat::Json);
        assert_eq!(args.color, super::ColorChoice::Never);
    }

    #[test]
    fn parses_init_command_with_path() {
        let args = Args::parse_from(["typrest", "init", "invoice"]);

        let Command::Init { path } = args.command else {
            panic!("expected init command");
        };

        assert_eq!(path, Some(PathBuf::from("invoice")));
    }

    #[test]
    fn rejects_old_init_name_form() {
        let result = Args::try_parse_from(["typrest", "init", "--path", "invoice"]);

        assert!(result.is_err());
    }
}
