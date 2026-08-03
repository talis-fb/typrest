use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::str::FromStr;
use std::sync::Arc;

use typst::foundations::Bytes;
use typst::syntax::ast::{Expr, ModuleImport};
use typst::syntax::package::PackageSpec;
use typst::syntax::{SyntaxNode, VirtualRoot};

use crate::{
    CompilationError, PackageCoordinate, PackageRepository, PackageSource, TemplateSource,
};

pub(super) type PreparedPackages = BTreeMap<PackageCoordinate, Arc<PreparedPackageSource>>;

pub(super) struct PreparedPackageSource {
    files: BTreeMap<String, Bytes>,
}

impl PreparedPackageSource {
    pub(super) fn file(&self, path: &str) -> Option<&Bytes> {
        self.files.get(path)
    }
}

pub(super) async fn resolve(
    source: &TemplateSource,
    repository: Option<&dyn PackageRepository>,
) -> Result<PreparedPackages, CompilationError> {
    let mut pending = discover_template_packages(source)?;
    if pending.is_empty() {
        return Ok(BTreeMap::new());
    }

    let repository = repository.ok_or_else(|| {
        CompilationError::failed(
            "external package imports are unavailable without a package repository",
            None,
        )
    })?;
    let mut resolved = BTreeMap::new();
    let mut seen = BTreeSet::new();

    while let Some(package) = pending.pop_front() {
        if !seen.insert(package.clone()) {
            continue;
        }
        if package.namespace() != "preview" {
            return Err(CompilationError::failed(
                format!(
                    "package namespace '{}' is unsupported; only '@preview' packages are supported",
                    package.namespace()
                ),
                None,
            ));
        }

        let source = repository
            .resolve(&package)
            .await
            .map_err(|error| CompilationError::failed(error.to_string(), None))?;
        pending.extend(discover_package_packages(&source)?);
        resolved.insert(package, prepare_source(&source));
    }

    Ok(resolved)
}

pub(super) fn coordinate(root: &VirtualRoot) -> Option<PackageCoordinate> {
    let VirtualRoot::Package(spec) = root else {
        return None;
    };
    Some(PackageCoordinate::new(
        spec.namespace.as_str(),
        spec.name.as_str(),
        spec.version.to_string(),
    ))
}

fn discover_template_packages(
    source: &TemplateSource,
) -> Result<VecDeque<PackageCoordinate>, CompilationError> {
    discover(
        source
            .files()
            .iter()
            .map(|(path, bytes)| (path.as_str(), bytes.as_ref())),
    )
}

fn discover_package_packages(
    source: &PackageSource,
) -> Result<VecDeque<PackageCoordinate>, CompilationError> {
    discover(
        source
            .files()
            .iter()
            .map(|(path, bytes)| (path.as_str(), bytes.as_ref())),
    )
}

fn discover<'a>(
    files: impl Iterator<Item = (&'a str, &'a [u8])>,
) -> Result<VecDeque<PackageCoordinate>, CompilationError> {
    let mut packages = BTreeSet::new();
    for (path, bytes) in files.filter(|(path, _)| path.ends_with(".typ")) {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| CompilationError::failed(format!("'{path}' must be valid UTF-8"), None))?;
        collect_imports(&typst::syntax::parse(text), &mut packages)?;
    }
    Ok(packages.into_iter().collect())
}

fn collect_imports(
    node: &SyntaxNode,
    packages: &mut BTreeSet<PackageCoordinate>,
) -> Result<(), CompilationError> {
    if let Some(import) = node.cast::<ModuleImport>()
        && let Expr::Str(value) = import.source()
        && value.get().starts_with('@')
    {
        let raw = value.get();
        let spec = PackageSpec::from_str(raw.as_str()).map_err(|error| {
            CompilationError::failed(format!("invalid package import '{raw}': {error}"), None)
        })?;
        packages.insert(PackageCoordinate::new(
            spec.namespace.as_str(),
            spec.name.as_str(),
            spec.version.to_string(),
        ));
    }

    for child in node.children() {
        collect_imports(child, packages)?;
    }
    Ok(())
}

fn prepare_source(source: &PackageSource) -> Arc<PreparedPackageSource> {
    Arc::new(PreparedPackageSource {
        files: source
            .files()
            .iter()
            .map(|(path, bytes)| (path.clone(), Bytes::new(bytes.to_vec())))
            .collect(),
    })
}
