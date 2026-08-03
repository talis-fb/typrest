use typst::foundations::Bytes;
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;

use crate::TemplateSource;

#[derive(Clone)]
pub(super) struct FontRegistry {
    default_book: FontBook,
    default_fonts: Vec<Font>,
}

impl FontRegistry {
    pub(super) fn new() -> Self {
        let mut book = FontBook::new();
        let mut fonts = Vec::new();

        for data in typst_assets::fonts() {
            load_font(Bytes::new(data), &mut book, &mut fonts);
        }

        Self {
            default_book: book,
            default_fonts: fonts,
        }
    }

    pub(super) fn for_source(&self, source: &TemplateSource) -> LoadedFonts {
        let mut book = self.default_book.clone();
        let mut fonts = self.default_fonts.clone();

        for (path, bytes) in source.files() {
            if is_template_font(path) {
                load_font(Bytes::new(bytes.to_vec()), &mut book, &mut fonts);
            }
        }

        LoadedFonts {
            book: LazyHash::new(book),
            fonts,
        }
    }
}

pub(super) struct LoadedFonts {
    pub(super) book: LazyHash<FontBook>,
    pub(super) fonts: Vec<Font>,
}

fn load_font(bytes: Bytes, book: &mut FontBook, fonts: &mut Vec<Font>) {
    for font in Font::iter(bytes) {
        book.push(font.info().clone());
        fonts.push(font);
    }
}

fn is_template_font(path: &str) -> bool {
    path.starts_with("assets/") && (path.ends_with(".ttf") || path.ends_with(".otf"))
}

#[cfg(test)]
mod tests {
    use super::FontRegistry;

    #[test]
    fn loads_default_typst_fonts() {
        let registry = FontRegistry::new();

        assert!(!registry.default_fonts.is_empty());
    }
}
