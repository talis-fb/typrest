use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer};
use serde_json::Value;
use thiserror::Error;

use crate::ValidationError;
use crate::loader::LoaderError;

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Schema {
    pub schema_version: String,
    pub request: SchemaRequest,
}

impl Schema {
    pub fn parse(bytes: &[u8]) -> Result<Self, LoaderError> {
        let value: Value = serde_json::from_slice(bytes).map_err(SchemaError::from)?;

        match value.get("schemaVersion") {
            None => return Err(SchemaVersionError::Missing.into()),
            Some(Value::String(version)) if version == "v1" => {}
            Some(Value::String(version)) => {
                return Err(SchemaVersionError::Unsupported(version.clone()).into());
            }
            Some(_) => {
                return Err(SchemaError::InvalidStructure {
                    path: "$.schemaVersion".to_owned(),
                    message: "schemaVersion must be a string".to_owned(),
                }
                .into());
            }
        }

        let schema: Self = serde_json::from_value(value).map_err(SchemaError::from)?;

        schema.request.body.ensure_consistent("$")?;

        Ok(schema)
    }

    pub fn body(&self) -> &SchemaField {
        &self.request.body
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct SchemaRequest {
    pub body: SchemaField,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct SchemaField {
    #[serde(rename = "type")]
    pub field_type: Option<SchemaType>,
    #[serde(default)]
    pub required: bool,
    pub properties: Option<BTreeMap<String, SchemaField>>,
    pub items: Option<Box<SchemaField>>,
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<Value>>,
    #[serde(default, deserialize_with = "deserialize_optional_value")]
    pub default: Option<Value>,
}

impl SchemaField {
    fn ensure_consistent(&self, path: &str) -> Result<(), SchemaError> {
        if self.properties.is_some() && self.field_type != Some(SchemaType::Object) {
            return Err(SchemaError::InvalidStructure {
                path: path.to_owned(),
                message: "properties is only allowed when type is object".to_owned(),
            });
        }

        if self.items.is_some() && self.field_type != Some(SchemaType::Array) {
            return Err(SchemaError::InvalidStructure {
                path: path.to_owned(),
                message: "items is only allowed when type is array".to_owned(),
            });
        }

        if let Some(default) = &self.default
            && let Some(field_type) = self.field_type
            && !field_type.matches(default)
        {
            return Err(SchemaError::InvalidStructure {
                path: path.to_owned(),
                message: format!("default value must have type {field_type}"),
            });
        }

        if let Some(enum_values) = &self.enum_values
            && let Some(first) = enum_values.first()
        {
            let first_type = SchemaType::from_json(first);

            for value in enum_values {
                if SchemaType::from_json(value) != first_type {
                    return Err(SchemaError::InvalidStructure {
                        path: path.to_owned(),
                        message: "enum values must all have the same type".to_owned(),
                    });
                }

                if let Some(field_type) = self.field_type
                    && !field_type.matches(value)
                {
                    return Err(SchemaError::InvalidStructure {
                        path: path.to_owned(),
                        message: format!("enum value must have type {field_type}"),
                    });
                }
            }
        }

        if let Some(properties) = &self.properties {
            for (name, property) in properties {
                property.ensure_consistent(&join_path(path, name))?;
            }
        }

        if let Some(items) = &self.items {
            items.ensure_consistent(&format!("{path}[]"))?;
        }

        Ok(())
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    String,
    Number,
    Boolean,
    Object,
    Array,
    Null,
}

impl SchemaType {
    fn matches(self, value: &Value) -> bool {
        match self {
            Self::String => value.is_string(),
            Self::Number => value.is_number(),
            Self::Boolean => value.is_boolean(),
            Self::Object => value.is_object(),
            Self::Array => value.is_array(),
            Self::Null => value.is_null(),
        }
    }

    fn from_json(value: &Value) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Bool(_) => Self::Boolean,
            Value::Number(_) => Self::Number,
            Value::String(_) => Self::String,
            Value::Array(_) => Self::Array,
            Value::Object(_) => Self::Object,
        }
    }
}

impl std::fmt::Display for SchemaType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::String => "string",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Object => "object",
            Self::Array => "array",
            Self::Null => "null",
        })
    }
}

#[derive(Debug, Error)]
pub enum SchemaVersionError {
    #[error("schemaVersion is missing")]
    Missing,

    #[error("unsupported schema version '{0}'")]
    Unsupported(String),
}

#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("invalid schema JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid schema at {path}: {message}")]
    InvalidStructure { path: String, message: String },
}

pub fn validate(schema: &Schema, payload: &Value) -> Result<(), Vec<ValidationError>> {
    validate_with_defaults(schema, payload).map(|_| ())
}

pub fn validate_with_defaults(
    schema: &Schema,
    payload: &Value,
) -> Result<Value, Vec<ValidationError>> {
    let mut violations = Vec::new();
    let value = validate_value(schema.body(), Some(payload), "$", &mut violations);

    if violations.is_empty() {
        Ok(value.unwrap_or(Value::Null))
    } else {
        Err(violations)
    }
}

fn validate_value(
    schema: &SchemaField,
    value: Option<&Value>,
    path: &str,
    violations: &mut Vec<ValidationError>,
) -> Option<Value> {
    let mut current = match value {
        Some(value) => Some(value.clone()),
        None if schema.required => {
            violations.push(ValidationError::new(path, "required field missing"));
            return None;
        }
        None => schema.default.clone(),
    };

    let value = current.as_mut()?;

    if let Some(field_type) = schema.field_type
        && !field_type.matches(value)
    {
        violations.push(ValidationError::new(
            path,
            format!("expected type {field_type}, got {}", json_type_name(value)),
        ));
        return current;
    }

    if let Some(enum_values) = &schema.enum_values
        && !enum_values.iter().any(|enum_value| enum_value == value)
    {
        violations.push(ValidationError::new(
            path,
            format!("invalid enum value {}", value),
        ));
        return current;
    }

    if schema.field_type == Some(SchemaType::Object)
        && let (Some(properties), Some(object)) = (&schema.properties, value.as_object_mut())
    {
        for (name, property_schema) in properties {
            let validated = validate_value(
                property_schema,
                object.get(name),
                &join_path(path, name),
                violations,
            );

            if !object.contains_key(name)
                && let Some(defaulted) = validated
            {
                object.insert(name.clone(), defaulted);
            }
        }
    }

    if schema.field_type == Some(SchemaType::Array)
        && let (Some(items_schema), Some(items)) = (&schema.items, value.as_array_mut())
    {
        for (index, item) in items.iter_mut().enumerate() {
            if let Some(validated) = validate_value(
                items_schema,
                Some(item),
                &format!("{path}[{index}]"),
                violations,
            ) {
                *item = validated;
            }
        }
    }

    current
}

fn join_path(parent: &str, name: &str) -> String {
    if parent == "$" {
        format!("$.{name}")
    } else {
        format!("{parent}.{name}")
    }
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn deserialize_optional_value<'de, D>(deserializer: D) -> Result<Option<Value>, D::Error>
where
    D: Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::loader::LoaderError;

    use super::{Schema, SchemaError, SchemaVersionError, validate, validate_with_defaults};

    fn parse(schema: serde_json::Value) -> Schema {
        Schema::parse(schema.to_string().as_bytes()).expect("schema should parse")
    }

    #[test]
    fn validates_required_object_properties() {
        let schema = parse(json!({
            "schemaVersion": "v1",
            "request": {
                "body": {
                    "type": "object",
                    "properties": {
                        "amount": { "type": "number", "required": true },
                        "currency": { "type": "string", "enum": ["BRL", "USD"] }
                    }
                }
            }
        }));

        let violations = validate(&schema, &json!({ "currency": "EUR" })).unwrap_err();

        assert_eq!(violations.len(), 2);
        assert_eq!(violations[0].field, "$.amount");
        assert_eq!(violations[0].issue, "required field missing");
        assert_eq!(violations[1].field, "$.currency");
        assert_eq!(violations[1].issue, "invalid enum value \"EUR\"");
    }

    #[test]
    fn applies_defaults_only_when_value_is_missing() {
        let schema = parse(json!({
            "schemaVersion": "v1",
            "request": {
                "body": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "default": "anonymous" },
                        "nickname": { "type": "null", "default": null }
                    }
                }
            }
        }));

        let validated = validate_with_defaults(&schema, &json!({ "nickname": null })).unwrap();

        assert_eq!(validated, json!({ "name": "anonymous", "nickname": null }));
    }

    #[test]
    fn validates_array_items() {
        let schema = parse(json!({
            "schemaVersion": "v1",
            "request": {
                "body": {
                    "type": "array",
                    "items": { "type": "boolean" }
                }
            }
        }));

        let violations = validate(&schema, &json!([true, "no"])).unwrap_err();

        assert_eq!(violations[0].field, "$[1]");
        assert_eq!(violations[0].issue, "expected type boolean, got string");
    }

    #[test]
    fn rejects_inconsistent_schema_during_parse() {
        let error = Schema::parse(
            json!({
                "schemaVersion": "v1",
                "request": {
                    "body": {
                        "type": "string",
                        "properties": {}
                    }
                }
            })
            .to_string()
            .as_bytes(),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            LoaderError::SchemaError(SchemaError::InvalidStructure { .. })
        ));
    }

    #[test]
    fn rejects_mixed_enum_values_during_parse() {
        let error = Schema::parse(
            json!({
                "schemaVersion": "v1",
                "request": {
                    "body": { "enum": ["BRL", 10] }
                }
            })
            .to_string()
            .as_bytes(),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            LoaderError::SchemaError(SchemaError::InvalidStructure { .. })
        ));
    }

    #[test]
    fn rejects_missing_schema_version_during_parse() {
        let error = Schema::parse(
            json!({
                "request": {
                    "body": {}
                }
            })
            .to_string()
            .as_bytes(),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            LoaderError::SchemaVersionError(SchemaVersionError::Missing)
        ));
    }

    #[test]
    fn rejects_unsupported_schema_version_during_parse() {
        let error = Schema::parse(
            json!({
                "schemaVersion": "v2",
                "request": {
                    "body": {}
                }
            })
            .to_string()
            .as_bytes(),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            LoaderError::SchemaVersionError(SchemaVersionError::Unsupported(version))
                if version == "v2"
        ));
    }
}
