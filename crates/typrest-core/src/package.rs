use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use thiserror::Error;

#[async_trait]
pub trait PackageRepository: Send + Sync {
    async fn resolve(
        &self,
        package: &PackageCoordinate,
    ) -> Result<Arc<PackageSource>, PackageResolutionError>;
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PackageCoordinate {
    namespace: String,
    name: String,
    version: String,
}

impl PackageCoordinate {
    pub fn new(
        namespace: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
            version: version.into(),
        }
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}

impl std::fmt::Display for PackageCoordinate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "@{}/{}:{}",
            self.namespace, self.name, self.version
        )
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackageSource {
    files: BTreeMap<String, Bytes>,
}

impl PackageSource {
    pub fn new(files: BTreeMap<String, Bytes>) -> Self {
        Self { files }
    }

    pub fn files(&self) -> &BTreeMap<String, Bytes> {
        &self.files
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum PackageResolutionError {
    #[error(
        "package namespace '{namespace}' is unsupported; only '@preview' packages are supported"
    )]
    UnsupportedNamespace { namespace: String },

    #[error("package '{package}' was not found")]
    NotFound { package: PackageCoordinate },

    #[error("failed to download package '{package}': {message}")]
    Download {
        package: PackageCoordinate,
        message: String,
    },

    #[error("failed to read package '{package}' from the cache: {message}")]
    Cache {
        package: PackageCoordinate,
        message: String,
    },

    #[error("package '{package}' is malformed: {message}")]
    Malformed {
        package: PackageCoordinate,
        message: String,
    },
}
