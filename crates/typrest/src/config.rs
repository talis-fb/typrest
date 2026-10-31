use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

use tracing::error;

use crate::render_executor::{
    MAX_CONCURRENT_RENDERS_LIMIT, MAX_QUEUED_RENDERS_LIMIT, RenderLimits,
    default_max_concurrent_renders, default_max_queued_renders,
};

pub const CONFIG_FILE: &str = "typrest.toml";
pub const PROJECT_DATA_DIR: &str = ".typrest";
pub const DEFAULT_SHUTDOWN_TIMEOUT_SECONDS: usize = 25;
pub const MAX_SHUTDOWN_TIMEOUT_SECONDS: usize = 3600;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectConfig {
    pub project_root: PathBuf,
    pub templates_root: PathBuf,
    pub package_cache_dir: PathBuf,
    pub web_ui: bool,
    pub render_limits: RenderLimits,
    pub shutdown_timeout: Duration,
}

#[derive(Debug, Deserialize)]
struct RawProjectConfig {
    schema_version: String,
    templates_root: String,
    #[serde(default)]
    web_ui: bool,
    #[serde(default)]
    server: RawServerConfig,
}

#[derive(Debug, Default, Deserialize)]
struct RawServerConfig {
    max_concurrent_renders: Option<i64>,
    max_queued_renders: Option<i64>,
    shutdown_timeout_seconds: Option<i64>,
}

pub fn load_config() -> Option<ProjectConfig> {
    let current_dir = std::env::current_dir().unwrap_or_default();
    let result = find_project_root(&current_dir).and_then(|root| load_project_config(&root));

    match result {
        Ok(config) => Some(config),
        Err(error) => {
            error!(
                event = "config.load.failed",
                error = %error,
                "failed to load project configuration: {error}"
            );
            None
        }
    }
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
    let render_limits = resolve_render_limits(&config.server)?;
    let shutdown_timeout = resolve_shutdown_timeout(&config.server)?;
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
        project_root: project_root.to_path_buf(),
        templates_root,
        package_cache_dir: project_root.join(PROJECT_DATA_DIR).join("packages"),
        web_ui: config.web_ui,
        render_limits,
        shutdown_timeout,
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

fn resolve_render_limits(server: &RawServerConfig) -> std::io::Result<RenderLimits> {
    let max_concurrent_renders = match server.max_concurrent_renders {
        Some(value) => bounded_setting(
            "max_concurrent_renders",
            value,
            1,
            MAX_CONCURRENT_RENDERS_LIMIT,
        )?,
        None => default_max_concurrent_renders(),
    };
    let max_queued_renders = match server.max_queued_renders {
        Some(value) => bounded_setting("max_queued_renders", value, 0, MAX_QUEUED_RENDERS_LIMIT)?,
        None => default_max_queued_renders(max_concurrent_renders),
    };

    Ok(RenderLimits::new(
        max_concurrent_renders,
        max_queued_renders,
    ))
}

fn resolve_shutdown_timeout(server: &RawServerConfig) -> std::io::Result<Duration> {
    let seconds = match server.shutdown_timeout_seconds {
        Some(value) => bounded_setting(
            "shutdown_timeout_seconds",
            value,
            1,
            MAX_SHUTDOWN_TIMEOUT_SECONDS,
        )?,
        None => DEFAULT_SHUTDOWN_TIMEOUT_SECONDS,
    };

    Ok(Duration::from_secs(seconds as u64))
}

fn bounded_setting(name: &str, value: i64, min: usize, max: usize) -> std::io::Result<usize> {
    usize::try_from(value)
        .ok()
        .filter(|value| (min..=max).contains(value))
        .map_or_else(
            || {
                invalid_config(format!(
                    "invalid {CONFIG_FILE}: server.{name} must be an integer from {min} to {max}"
                ))
            },
            Ok,
        )
}

fn resolve_templates_root(project_root: &Path, templates_root: &str) -> std::io::Result<PathBuf> {
    let templates_root = Path::new(templates_root);

    if templates_root.is_absolute() {
        return invalid_config("invalid typrest.toml: templates_root must be relative".to_owned());
    }

    let mut normalized = PathBuf::new();

    for component in templates_root.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                if !normalized.pop() {
                    return invalid_config(
                        "invalid typrest.toml: templates_root must stay inside the project root"
                            .to_owned(),
                    );
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return invalid_config(
                    "invalid typrest.toml: templates_root must be relative".to_owned(),
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
    use std::time::Duration;

    use super::{PROJECT_DATA_DIR, find_project_root, load_project_config, parse_project_config};
    use crate::render_executor::{RenderLimits, default_max_concurrent_renders};

    #[test]
    fn load_project_config_resolves_templates_root() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("typrest.toml"),
            "schema_version = \"v1\"\ntemplates_root = \"./templates\"\n",
        )
        .unwrap();
        std::fs::create_dir(root.path().join("templates")).unwrap();

        let config = load_project_config(root.path()).unwrap();

        assert_eq!(config.project_root, root.path());
        assert_eq!(config.templates_root, root.path().join("templates"));
        assert_eq!(
            config.package_cache_dir,
            root.path().join(PROJECT_DATA_DIR).join("packages")
        );
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
            root.path().join("typrest.toml"),
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
            root.path().join("typrest.toml"),
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
            root.path().join("typrest.toml"),
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
            root.path().join("typrest.toml"),
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
            root.path().join("typrest.toml"),
            "schema_version = \"v1\"\ntemplates_root = \"templates\"\n",
        )
        .unwrap();
        std::fs::write(
            project.join("typrest.toml"),
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

    fn load_with_server_table(server: &str) -> std::io::Result<super::ProjectConfig> {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("typrest.toml"),
            format!("schema_version = \"v1\"\ntemplates_root = \"templates\"\n{server}"),
        )
        .unwrap();
        std::fs::create_dir(root.path().join("templates")).unwrap();

        load_project_config(root.path())
    }

    #[test]
    fn render_limits_use_defaults_when_server_table_is_absent() {
        let config = load_with_server_table("").unwrap();

        assert_eq!(config.render_limits, RenderLimits::default());
    }

    #[test]
    fn render_limits_use_defaults_for_omitted_fields() {
        let config = load_with_server_table("[server]\nmax_concurrent_renders = 3\n").unwrap();

        assert_eq!(config.render_limits, RenderLimits::new(3, 6));
        let config = load_with_server_table("[server]\nmax_queued_renders = 5\n").unwrap();
        assert_eq!(
            config.render_limits,
            RenderLimits::new(default_max_concurrent_renders(), 5)
        );
    }

    #[test]
    fn render_limits_honor_explicit_values() {
        let config = load_with_server_table(
            "[server]\nmax_concurrent_renders = 4\nmax_queued_renders = 10\n",
        )
        .unwrap();

        assert_eq!(config.render_limits, RenderLimits::new(4, 10));
    }

    #[test]
    fn render_limits_accept_bounds_and_zero_queue() {
        let config = load_with_server_table(
            "[server]\nmax_concurrent_renders = 256\nmax_queued_renders = 0\n",
        )
        .unwrap();
        assert_eq!(config.render_limits, RenderLimits::new(256, 0));

        let config = load_with_server_table(
            "[server]\nmax_concurrent_renders = 1\nmax_queued_renders = 1024\n",
        )
        .unwrap();
        assert_eq!(config.render_limits, RenderLimits::new(1, 1024));
    }

    #[test]
    fn render_limits_reject_out_of_range_values() {
        for server in [
            "[server]\nmax_concurrent_renders = 0\n",
            "[server]\nmax_concurrent_renders = -1\n",
            "[server]\nmax_concurrent_renders = 257\n",
            "[server]\nmax_queued_renders = -1\n",
            "[server]\nmax_queued_renders = 1025\n",
        ] {
            let error = load_with_server_table(server).unwrap_err();

            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput, "{server}");
        }
    }

    #[test]
    fn render_limits_reject_wrong_types() {
        for server in [
            "[server]\nmax_concurrent_renders = \"4\"\n",
            "[server]\nmax_queued_renders = 1.5\n",
            "server = 4\n",
        ] {
            let error = load_with_server_table(server).unwrap_err();

            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput, "{server}");
        }
    }

    #[test]
    fn render_limits_ignore_unknown_server_keys() {
        let config = load_with_server_table("[server]\nunknown = true\n").unwrap();

        assert_eq!(config.render_limits, RenderLimits::default());
    }

    #[test]
    fn shutdown_timeout_defaults_to_25_seconds() {
        let config = load_with_server_table("").unwrap();
        assert_eq!(config.shutdown_timeout, Duration::from_secs(25));

        let config = load_with_server_table("[server]\nmax_queued_renders = 1\n").unwrap();
        assert_eq!(config.shutdown_timeout, Duration::from_secs(25));
    }

    #[test]
    fn shutdown_timeout_honors_explicit_values_and_bounds() {
        for (value, seconds) in [(1, 1), (30, 30), (3600, 3600)] {
            let config =
                load_with_server_table(&format!("[server]\nshutdown_timeout_seconds = {value}\n"))
                    .unwrap();

            assert_eq!(config.shutdown_timeout, Duration::from_secs(seconds));
        }
    }

    #[test]
    fn shutdown_timeout_rejects_invalid_values() {
        for server in [
            "[server]\nshutdown_timeout_seconds = 0\n",
            "[server]\nshutdown_timeout_seconds = -1\n",
            "[server]\nshutdown_timeout_seconds = 3601\n",
            "[server]\nshutdown_timeout_seconds = 1.5\n",
            "[server]\nshutdown_timeout_seconds = \"25\"\n",
        ] {
            let error = load_with_server_table(server).unwrap_err();

            assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput, "{server}");
        }
    }
}
