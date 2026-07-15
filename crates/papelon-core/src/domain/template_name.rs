use std::fmt;

use thiserror::Error;

/// Identifier of a template available to Papelon.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TemplateName(String);

impl TemplateName {
    /// Creates a template name after validating that it is safe for use in
    /// routes and filesystem-backed resolvers.
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

    #[error("template name must start with an ASCII alphanumeric character")]
    InvalidStart,

    #[error(
        "template name contains invalid character '{character}' at position {position}; only ASCII alphanumeric characters, '-' and '_' are allowed"
    )]
    InvalidCharacter { character: char, position: usize },
}

fn validate(value: &str) -> Result<(), TemplateNameError> {
    let mut characters = value.chars();
    let first = characters.next().ok_or(TemplateNameError::Empty)?;

    if !first.is_ascii_alphanumeric() {
        return Err(TemplateNameError::InvalidStart);
    }

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
        for value in ["invoice", "invoice-v2", "invoice_v2", "Invoice2"] {
            let name = TemplateName::new(value).expect("template name should be valid");

            assert_eq!(name.as_str(), value);
        }
    }

    #[test]
    fn rejects_empty_template_name() {
        assert_eq!(TemplateName::new(""), Err(TemplateNameError::Empty));
    }

    #[test]
    fn rejects_invalid_first_character() {
        for value in ["_shared", "-invoice", ".invoice"] {
            assert_eq!(
                TemplateName::new(value),
                Err(TemplateNameError::InvalidStart)
            );
        }
    }

    #[test]
    fn rejects_path_segments_and_unsupported_characters() {
        assert_eq!(
            TemplateName::new("invoice/v2"),
            Err(TemplateNameError::InvalidCharacter {
                character: '/',
                position: 7,
            })
        );
        assert_eq!(
            TemplateName::new("invoice v2"),
            Err(TemplateNameError::InvalidCharacter {
                character: ' ',
                position: 7,
            })
        );
        assert_eq!(
            TemplateName::new("fatura-á"),
            Err(TemplateNameError::InvalidCharacter {
                character: 'á',
                position: 7,
            })
        );
    }

    #[test]
    fn supports_standard_string_conversions_and_display() {
        let from_str = TemplateName::try_from("invoice").expect("valid template name");
        let from_string =
            TemplateName::try_from(String::from("invoice")).expect("valid template name");

        assert_eq!(from_str, from_string);
        assert_eq!(from_str.to_string(), "invoice");
        assert_eq!(from_string.into_inner(), "invoice");
    }
}
