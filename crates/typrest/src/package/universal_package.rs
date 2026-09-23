use std::collections::BTreeMap;
use std::path::{Component, Path};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use bytes::Bytes;
use tracing::{debug, error, info};
use typrest_core::{PackageCoordinate, PackageRepository, PackageResolutionError, PackageSource};
use typst_kit::downloader::{Downloader, SystemDownloader};
use typst_kit::packages::{FsPackages, SystemPackages, UniversePackages};
use typst_syntax::package::PackageSpec;

#[derive(Clone)]
pub struct UniversePackageRepository {
    packages: Arc<SystemPackages>,
    loaded: Arc<Mutex<BTreeMap<PackageCoordinate, Arc<PackageSource>>>>,
    initial_context: &'static str,
    watch_context: Arc<AtomicBool>,
}

impl UniversePackageRepository {
    pub fn new(cache_dir: impl Into<std::path::PathBuf>, context: &'static str) -> Self {
        let downloader = SystemDownloader::new(format!("typrest/{}", env!("CARGO_PKG_VERSION")));
        Self::with_downloader(cache_dir, downloader, context)
    }

    fn with_downloader(
        cache_dir: impl Into<std::path::PathBuf>,
        downloader: impl Downloader,
        context: &'static str,
    ) -> Self {
        let cache = FsPackages::new(cache_dir);
        Self {
            packages: Arc::new(SystemPackages::from_parts(
                None,
                Some(cache),
                UniversePackages::new(downloader),
            )),
            loaded: Arc::new(Mutex::new(BTreeMap::new())),
            initial_context: context,
            watch_context: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn use_watch_context(&self) {
        self.watch_context.store(true, Ordering::Relaxed);
    }

    fn context(&self) -> &'static str {
        if self.watch_context.load(Ordering::Relaxed) {
            "watch"
        } else {
            self.initial_context
        }
    }

    fn loaded(
        &self,
        package: &PackageCoordinate,
    ) -> Result<Option<Arc<PackageSource>>, PackageResolutionError> {
        self.loaded
            .lock()
            .map(|loaded| loaded.get(package).cloned())
            .map_err(|error| cache_error(package, error))
    }

    fn remember(
        &self,
        package: PackageCoordinate,
        source: Arc<PackageSource>,
    ) -> Result<Arc<PackageSource>, PackageResolutionError> {
        let mut loaded = self
            .loaded
            .lock()
            .map_err(|error| cache_error(&package, error))?;
        Ok(loaded.entry(package).or_insert(source).clone())
    }
}

#[async_trait]
impl PackageRepository for UniversePackageRepository {
    async fn resolve(
        &self,
        package: &PackageCoordinate,
    ) -> Result<Arc<PackageSource>, PackageResolutionError> {
        if package.namespace() != UniversePackages::NAMESPACE {
            return Err(PackageResolutionError::UnsupportedNamespace {
                namespace: package.namespace().to_owned(),
            });
        }
        if let Some(source) = self.loaded(package)? {
            return Ok(source);
        }

        let coordinate = package.clone();
        let spec = PackageSpec::from_str(&coordinate.to_string()).map_err(|error| {
            PackageResolutionError::Malformed {
                package: coordinate.clone(),
                message: error.to_string(),
            }
        })?;
        let installing = self
            .packages
            .cache()
            .is_none_or(|cache| cache.obtain(&spec).is_none());
        if installing {
            info!(
                event = "package.install.started",
                context = self.context(),
                package = %package,
                status = "started",
                "installing package `{package}`"
            );
        } else {
            debug!(
                event = "package.cache.hit",
                context = self.context(),
                package = %package,
                status = "ok",
                "using cached package `{package}`"
            );
        }
        let packages = self.packages.clone();
        let source = tokio::task::spawn_blocking(move || {
            let root =
                packages
                    .obtain(&spec)
                    .map_err(|error| PackageResolutionError::Download {
                        package: coordinate.clone(),
                        message: error.to_string(),
                    })?;
            read_package(&coordinate, root.path())
        })
        .await
        .map_err(|error| PackageResolutionError::Cache {
            package: package.clone(),
            message: format!("package resolver task failed: {error}"),
        })
        .and_then(|result| result);

        let resolved = source.and_then(|source| self.remember(package.clone(), Arc::new(source)));
        if installing {
            if resolved.is_ok() {
                info!(
                    event = "package.install.completed",
                    context = self.context(),
                    package = %package,
                    status = "ok",
                    "installed package `{package}`"
                );
            } else {
                error!(
                    event = "package.install.failed",
                    context = self.context(),
                    package = %package,
                    status = "error",
                    "failed to install package `{package}`"
                );
            }
        }
        resolved
    }
}

fn read_package(
    package: &PackageCoordinate,
    root: &Path,
) -> Result<PackageSource, PackageResolutionError> {
    let mut files = BTreeMap::new();
    read_directory(package, root, root, &mut files)?;
    Ok(PackageSource::new(files))
}

fn read_directory(
    package: &PackageCoordinate,
    root: &Path,
    directory: &Path,
    files: &mut BTreeMap<String, Bytes>,
) -> Result<(), PackageResolutionError> {
    let entries = std::fs::read_dir(directory).map_err(|error| PackageResolutionError::Cache {
        package: package.clone(),
        message: error.to_string(),
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| PackageResolutionError::Cache {
            package: package.clone(),
            message: error.to_string(),
        })?;
        let file_type = entry
            .file_type()
            .map_err(|error| PackageResolutionError::Cache {
                package: package.clone(),
                message: error.to_string(),
            })?;
        let path = entry.path();
        if file_type.is_dir() {
            read_directory(package, root, &path, files)?;
        } else if file_type.is_file() {
            let relative =
                path.strip_prefix(root)
                    .map_err(|error| PackageResolutionError::Malformed {
                        package: package.clone(),
                        message: error.to_string(),
                    })?;
            let virtual_path = virtual_path(package, relative)?;
            let bytes = std::fs::read(&path).map_err(|error| PackageResolutionError::Cache {
                package: package.clone(),
                message: error.to_string(),
            })?;
            files.insert(virtual_path, Bytes::from(bytes));
        }
    }

    Ok(())
}

fn virtual_path(
    package: &PackageCoordinate,
    path: &Path,
) -> Result<String, PackageResolutionError> {
    let mut parts = Vec::new();
    for component in path.components() {
        let Component::Normal(part) = component else {
            return Err(PackageResolutionError::Malformed {
                package: package.clone(),
                message: format!("invalid package path '{}'", path.display()),
            });
        };
        let part = part
            .to_str()
            .ok_or_else(|| PackageResolutionError::Malformed {
                package: package.clone(),
                message: format!("package path '{}' is not valid UTF-8", path.display()),
            })?;
        parts.push(part);
    }
    Ok(parts.join("/"))
}

fn cache_error(
    package: &PackageCoordinate,
    error: impl std::fmt::Display,
) -> PackageResolutionError {
    PackageResolutionError::Cache {
        package: package.clone(),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::fs;
    use std::io::{self, Read};
    use typrest_core::{PackageCoordinate, PackageRepository, PackageResolutionError};
    use typst_kit::downloader::Downloader;

    use super::UniversePackageRepository;

    #[derive(Debug)]
    struct FailingDownloader;

    impl Downloader for FailingDownloader {
        fn stream(&self, _: &dyn Any, _: &str) -> io::Result<(Option<usize>, Box<dyn Read>)> {
            Err(io::Error::other("offline test"))
        }
    }

    #[tokio::test]
    async fn loads_preview_package_from_project_cache() {
        let root = tempfile::tempdir().unwrap();
        let package_dir = root.path().join("preview").join("fixture").join("1.0.0");
        fs::create_dir_all(package_dir.join("src")).unwrap();
        fs::write(
            package_dir.join("typst.toml"),
            "[package]\nname = \"fixture\"\nversion = \"1.0.0\"\nentrypoint = \"src/lib.typ\"\n",
        )
        .unwrap();
        fs::write(package_dir.join("src/lib.typ"), "#let value = 42").unwrap();
        let repository = UniversePackageRepository::new(root.path(), "test");
        let coordinate = PackageCoordinate::new("preview", "fixture", "1.0.0");

        let source = repository.resolve(&coordinate).await.unwrap();

        assert!(source.files().contains_key("typst.toml"));
        assert!(source.files().contains_key("src/lib.typ"));
    }

    #[tokio::test]
    async fn reuses_loaded_packages() {
        let root = tempfile::tempdir().unwrap();
        let package_dir = root.path().join("preview").join("fixture").join("1.0.0");
        fs::create_dir_all(&package_dir).unwrap();
        fs::write(
            package_dir.join("typst.toml"),
            "[package]\nname = \"fixture\"\nversion = \"1.0.0\"\nentrypoint = \"lib.typ\"\n",
        )
        .unwrap();
        fs::write(package_dir.join("lib.typ"), "#let value = 42").unwrap();
        let repository = UniversePackageRepository::new(root.path(), "test");
        let coordinate = PackageCoordinate::new("preview", "fixture", "1.0.0");

        repository.resolve(&coordinate).await.unwrap();
        repository.resolve(&coordinate).await.unwrap();

        assert!(repository.loaded(&coordinate).unwrap().is_some());
    }

    #[tokio::test]
    async fn reports_a_download_error_for_a_missing_package() {
        let root = tempfile::tempdir().unwrap();
        let repository =
            UniversePackageRepository::with_downloader(root.path(), FailingDownloader, "test");
        let coordinate = PackageCoordinate::new("preview", "missing", "1.0.0");

        let error = repository.resolve(&coordinate).await.unwrap_err();

        assert!(matches!(error, PackageResolutionError::Download { .. }));
    }

    #[tokio::test]
    async fn rejects_non_preview_namespace_without_accessing_the_cache() {
        let root = tempfile::tempdir().unwrap();
        let repository = UniversePackageRepository::new(root.path(), "test");
        let coordinate = PackageCoordinate::new("local", "fixture", "1.0.0");

        let error = repository.resolve(&coordinate).await.unwrap_err();

        assert!(matches!(
            error,
            PackageResolutionError::UnsupportedNamespace { .. }
        ));
    }
}
