use std::fmt;

use thiserror::Error;

/// Identifier of a template available to Typrest.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TemplateName(String);

impl TemplateName {
    /// Creates a template name after validating that it is safe for routes and future tags.
    pub fn new(value: impl Into<String>) -> Result<Self, TemplateNameError> {
        let value = value.into();
        validate(&value)?;

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl AsRef<str> for TemplateName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for TemplateName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<String> for TemplateName {
    type Error = TemplateNameError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for TemplateName {
    type Error = TemplateNameError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum TemplateNameError {
    #[error("template name cannot be empty")]
    Empty,

    #[error(
        "template name contains invalid character '{character}' at position {position}; only ASCII alphanumeric characters, '-' and '_' are allowed"
    )]
    InvalidCharacter { character: char, position: usize },
}

fn validate(value: &str) -> Result<(), TemplateNameError> {
    if value.is_empty() {
        return Err(TemplateNameError::Empty);
    }

    let characters = value.chars();

    for (position, character) in characters.enumerate() {
        if !character.is_ascii_alphanumeric() && character != '-' && character != '_' {
            return Err(TemplateNameError::InvalidCharacter {
                character,
                position: position + 1,
            });
        }
    }

    Ok(())
}
#[cfg(test)]
mod tests {
    use super::{TemplateName, TemplateNameError};

    #[test]
    fn accepts_valid_template_names() {
        for value in [
            "invoice",
            "invoice-v2",
            "invoice_v2",
            "Invoice2",
            "_shared",
            "-invoice",
        ] {
            let name = TemplateName::new(value).expect("template name should be valid");

            assert_eq!(name.as_str(), value);
        }
    }

    #[test]
    fn rejects_empty_template_name() {
        assert_eq!(TemplateName::new(""), Err(TemplateNameError::Empty));
    }

    #[test]
    fn rejects_path_segments_and_unsupported_characters() {
        assert_eq!(
            TemplateName::new(".invoice"),
            Err(TemplateNameError::InvalidCharacter {
                character: '.',
                position: 1,
            })
        );

        assert_eq!(
            TemplateName::new("invoice/v2"),
            Err(TemplateNameError::InvalidCharacter {
                character: '/',
                position: 8,
            })
        );
        assert_eq!(
            TemplateName::new("invoice v2"),
            Err(TemplateNameError::InvalidCharacter {
                character: ' ',
                position: 8,
            })
        );
        assert_eq!(
            TemplateName::new("invoiceλ"),
            Err(TemplateNameError::InvalidCharacter {
                character: 'λ',
                position: 8,
            })
        );
    }
}
