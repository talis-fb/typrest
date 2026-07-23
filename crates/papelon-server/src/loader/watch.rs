use std::path::Path;

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use papelon_core::TemplateName;
use tracing::{error, info, warn};

use crate::loader::filesystem::FilesystemLoader;

pub fn start_template_watcher(loader: FilesystemLoader) -> notify::Result<RecommendedWatcher> {
    let root = loader.root().to_owned();
    let watch_root = root.clone();
    let mut watcher = RecommendedWatcher::new(
        move |result| handle_event(result, &root, &loader),
        Config::default(),
    )?;

    watcher.watch(&watch_root, RecursiveMode::Recursive)?;
    info!(templates_dir = %watch_root.display(), "template watch mode enabled");

    Ok(watcher)
}

fn handle_event(result: notify::Result<Event>, root: &Path, loader: &FilesystemLoader) {
    let event = match result {
        Ok(event) => event,
        Err(error) => {
            error!(error = %error, "template watch event failed");
            return;
        }
    };

    if !is_relevant_event(&event) {
        return;
    }

    let mut saw_shared = false;
    let mut templates = Vec::new();

    for path in event.paths {
        match changed_template(root, &path) {
            Some(ChangedTemplate::Shared) => saw_shared = true,
            Some(ChangedTemplate::Template(name)) if !templates.contains(&name) => {
                templates.push(name);
            }
            Some(ChangedTemplate::Template(_)) | None => {}
        }
    }

    if saw_shared {
        reload_shared(loader);
    }

    for template in templates {
        reload_template(loader, &template);
    }
}

fn is_relevant_event(event: &Event) -> bool {
    matches!(
        event.kind,
        notify::EventKind::Create(_) | notify::EventKind::Modify(_) | notify::EventKind::Remove(_)
    )
}

fn reload_shared(loader: &FilesystemLoader) {
    match loader.list_sync() {
        Ok(templates) => {
            for template in templates {
                reload_template(loader, &template);
            }
            info!("shared template files changed");
        }
        Err(error) => warn!(error = %error, "failed to list templates after shared file change"),
    }
}

fn reload_template(loader: &FilesystemLoader, template: &TemplateName) {
    match loader.load_sync(template) {
        Ok(_) => info!(template = %template, "template reloaded"),
        Err(error) => warn!(template = %template, error = %error, "template reload failed"),
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ChangedTemplate {
    Template(TemplateName),
    Shared,
}

pub(crate) fn changed_template(root: &Path, path: &Path) -> Option<ChangedTemplate> {
    let relative_path = path.strip_prefix(root).ok()?;
    let first_segment = relative_path.components().next()?.as_os_str().to_str()?;

    if first_segment == "_shared" {
        return Some(ChangedTemplate::Shared);
    }

    TemplateName::new(first_segment.to_owned())
        .ok()
        .map(ChangedTemplate::Template)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use papelon_core::TemplateName;

    use super::{ChangedTemplate, changed_template};

    #[test]
    fn resolves_template_path() {
        assert_eq!(
            changed_template(
                Path::new("/templates"),
                Path::new("/templates/invoice/main.typ")
            ),
            Some(ChangedTemplate::Template(
                TemplateName::new("invoice").unwrap()
            ))
        );
    }

    #[test]
    fn resolves_shared_path() {
        assert_eq!(
            changed_template(
                Path::new("/templates"),
                Path::new("/templates/_shared/brand.typ")
            ),
            Some(ChangedTemplate::Shared)
        );
    }

    #[test]
    fn ignores_paths_outside_root() {
        assert_eq!(
            changed_template(Path::new("/templates"), Path::new("/elsewhere/main.typ")),
            None
        );
    }

    #[test]
    fn ignores_invalid_template_names() {
        assert_eq!(
            changed_template(
                Path::new("/templates"),
                Path::new("/templates/not.valid/main.typ")
            ),
            None
        );
    }
}
