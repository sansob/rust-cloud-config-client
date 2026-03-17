use std::mem;

use serde::de::DeserializeOwned;
use serde_json::{Map, Number, Value};

use crate::{Error, Result};

/// Controls how string scalars are converted before typed deserialization.
///
/// `Smart` is useful when configuration originates from Java properties or other
/// string-only sources and should still bind into numeric or boolean Rust fields.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ScalarCoercion {
    /// Preserve all string values as strings.
    None,
    /// Convert obvious scalar strings such as `"true"` or `"8080"` into native values.
    #[default]
    Smart,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PathSegment {
    Field(String),
    Index(usize),
}

pub(crate) fn deserialize_json_value<T>(value: Value, origin: impl Into<String>) -> Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_value(value).map_err(|source| Error::Bind {
        origin: origin.into(),
        source,
    })
}

pub(crate) fn nested_value_from_flat_map<I>(flat: I, coercion: ScalarCoercion) -> Value
where
    I: IntoIterator<Item = (String, Value)>,
{
    let mut root = Value::Object(Map::new());

    for (key, value) in flat {
        let segments = parse_property_path(&key);
        if segments.is_empty() {
            continue;
        }
        insert_path(&mut root, &segments, coerce_json_value(value, coercion));
    }

    root
}

pub(crate) fn nested_value_from_string_map<I>(flat: I, coercion: ScalarCoercion) -> Value
where
    I: IntoIterator<Item = (String, String)>,
{
    nested_value_from_flat_map(
        flat.into_iter()
            .map(|(key, value)| (key, Value::String(value))),
        coercion,
    )
}

pub(crate) fn coerce_json_value(value: Value, coercion: ScalarCoercion) -> Value {
    match value {
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(|item| coerce_json_value(item, coercion))
                .collect(),
        ),
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, coerce_json_value(value, coercion)))
                .collect(),
        ),
        Value::String(value) if coercion == ScalarCoercion::Smart => coerce_string(value),
        other => other,
    }
}

fn parse_property_path(path: &str) -> Vec<PathSegment> {
    let mut segments = Vec::new();
    let mut buffer = String::new();
    let chars: Vec<char> = path.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        match chars[index] {
            '.' => {
                if !buffer.is_empty() {
                    segments.push(PathSegment::Field(mem::take(&mut buffer)));
                }
                index += 1;
            }
            '[' => {
                if !buffer.is_empty() {
                    segments.push(PathSegment::Field(mem::take(&mut buffer)));
                }

                index += 1;
                let start = index;
                while index < chars.len() && chars[index] != ']' {
                    index += 1;
                }

                let token: String = chars[start..index.min(chars.len())].iter().collect();
                let token = token.trim().trim_matches('"').trim_matches('\'');
                if let Ok(position) = token.parse::<usize>() {
                    segments.push(PathSegment::Index(position));
                } else if !token.is_empty() {
                    segments.push(PathSegment::Field(token.to_string()));
                }

                if index < chars.len() && chars[index] == ']' {
                    index += 1;
                }
            }
            character => {
                buffer.push(character);
                index += 1;
            }
        }
    }

    if !buffer.is_empty() {
        segments.push(PathSegment::Field(buffer));
    }

    segments
}

fn insert_path(target: &mut Value, segments: &[PathSegment], value: Value) {
    if segments.is_empty() {
        *target = value;
        return;
    }

    match &segments[0] {
        PathSegment::Field(name) => {
            if !target.is_object() {
                *target = Value::Object(Map::new());
            }

            let object = target
                .as_object_mut()
                .expect("object should exist after initialization");

            if segments.len() == 1 {
                object.insert(name.clone(), value);
                return;
            }

            let entry = object
                .entry(name.clone())
                .or_insert_with(|| empty_container_for(&segments[1]));

            if !matches_container(entry, &segments[1]) {
                *entry = empty_container_for(&segments[1]);
            }

            insert_path(entry, &segments[1..], value);
        }
        PathSegment::Index(position) => {
            if !target.is_array() {
                *target = Value::Array(Vec::new());
            }

            let array = target
                .as_array_mut()
                .expect("array should exist after initialization");

            while array.len() <= *position {
                array.push(Value::Null);
            }

            if segments.len() == 1 {
                array[*position] = value;
                return;
            }

            if array[*position].is_null() || !matches_container(&array[*position], &segments[1]) {
                array[*position] = empty_container_for(&segments[1]);
            }

            insert_path(&mut array[*position], &segments[1..], value);
        }
    }
}

fn empty_container_for(next: &PathSegment) -> Value {
    match next {
        PathSegment::Field(_) => Value::Object(Map::new()),
        PathSegment::Index(_) => Value::Array(Vec::new()),
    }
}

fn matches_container(value: &Value, next: &PathSegment) -> bool {
    match next {
        PathSegment::Field(_) => value.is_object(),
        PathSegment::Index(_) => value.is_array(),
    }
}

fn coerce_string(value: String) -> Value {
    if value.eq_ignore_ascii_case("true") {
        return Value::Bool(true);
    }

    if value.eq_ignore_ascii_case("false") {
        return Value::Bool(false);
    }

    if let Some(number) = parse_number(&value) {
        return Value::Number(number);
    }

    Value::String(value)
}

fn parse_number(value: &str) -> Option<Number> {
    if looks_like_integer(value) {
        if let Ok(parsed) = value.parse::<i64>() {
            return Some(Number::from(parsed));
        }
        if let Ok(parsed) = value.parse::<u64>() {
            return Some(Number::from(parsed));
        }
    }

    if looks_like_float(value) {
        if let Ok(parsed) = value.parse::<f64>() {
            return Number::from_f64(parsed);
        }
    }

    None
}

fn looks_like_integer(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return false;
    }

    let digits = if bytes[0] == b'-' { &bytes[1..] } else { bytes };
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
        return false;
    }

    !(digits.len() > 1 && digits[0] == b'0')
}

fn looks_like_float(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return false;
    }

    let has_decimal = bytes.contains(&b'.');
    let has_exponent = bytes.contains(&b'e') || bytes.contains(&b'E');

    if !has_decimal && !has_exponent {
        return false;
    }

    value.parse::<f64>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::{ScalarCoercion, nested_value_from_flat_map};
    use serde_json::json;

    #[test]
    fn builds_nested_objects_and_arrays() {
        let value = nested_value_from_flat_map(
            [
                ("server.port".to_string(), json!("8080")),
                ("replicas[0].id".to_string(), json!("blue")),
                ("replicas[0].weight".to_string(), json!("100")),
                ("replicas[1].id".to_string(), json!("green")),
            ],
            ScalarCoercion::Smart,
        );

        assert_eq!(
            value,
            json!({
                "server": { "port": 8080 },
                "replicas": [
                    { "id": "blue", "weight": 100 },
                    { "id": "green" }
                ]
            })
        );
    }
}
