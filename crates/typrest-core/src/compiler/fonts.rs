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
            if is_font_file(path) {
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

fn is_font_file(path: &str) -> bool {
    path.ends_with(".ttf") || path.ends_with(".otf")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bytes::Bytes;

    use crate::TemplateSource;

    use super::{FontRegistry, is_font_file};

    #[test]
    fn loads_default_typst_fonts() {
        let registry = FontRegistry::new();

        assert!(!registry.default_fonts.is_empty());
    }

    #[test]
    fn recognizes_lowercase_fonts_anywhere_in_template_source() {
        for path in [
            "root.ttf",
            "fonts/brand.otf",
            "nested/design/fonts/heading.ttf",
            "_shared/fonts/company.otf",
        ] {
            assert!(is_font_file(path), "expected '{path}' to be a font");
        }

        for path in [
            "font.TTF",
            "font.OTF",
            "fonts/font.woff2",
            "font.ttf.backup",
        ] {
            assert!(!is_font_file(path), "expected '{path}' not to be a font");
        }
    }

    #[test]
    fn template_fonts_do_not_mutate_other_template_font_books() {
        let registry = FontRegistry::new();
        let font_data = typst_assets::fonts().next().unwrap();
        let source_with_font = TemplateSource::new(BTreeMap::from([(
            "nested/custom.ttf".to_owned(),
            Bytes::copy_from_slice(font_data),
        )]));

        let with_font = registry.for_source(&source_with_font);
        let without_font = registry.for_source(&TemplateSource::default());

        assert!(with_font.fonts.len() > registry.default_fonts.len());
        assert_eq!(without_font.fonts.len(), registry.default_fonts.len());
    }
}
