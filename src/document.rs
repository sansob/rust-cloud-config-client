use std::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{
    Error, Result, ScalarCoercion,
    binding::{coerce_json_value, deserialize_json_value, nested_value_from_string_map},
    properties,
};

/// Supported structured document kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentFormat {
    /// JSON content.
    Json,
    /// YAML or YML content.
    Yaml,
    /// TOML content.
    Toml,
    /// Java properties content.
    Properties,
    /// UTF-8 text with an unknown structure.
    Text,
    /// Opaque binary content.
    Binary,
}

impl DocumentFormat {
    /// Returns a human-readable format name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Json => "JSON",
            Self::Yaml => "YAML",
            Self::Toml => "TOML",
            Self::Properties => "Java properties",
            Self::Text => "text",
            Self::Binary => "binary",
        }
    }

    pub(crate) fn from_path(path: &str) -> Option<Self> {
        let (_, extension) = path.rsplit_once('.')?;
        match extension.to_ascii_lowercase().as_str() {
            "json" => Some(Self::Json),
            "yaml" | "yml" => Some(Self::Yaml),
            "toml" => Some(Self::Toml),
            "properties" | "props" => Some(Self::Properties),
            _ => None,
        }
    }

    pub(crate) fn from_content_type(content_type: &str) -> Option<Self> {
        let content_type = content_type.to_ascii_lowercase();
        if content_type.contains("json") {
            Some(Self::Json)
        } else if content_type.contains("yaml") || content_type.contains("yml") {
            Some(Self::Yaml)
        } else if content_type.contains("toml") {
            Some(Self::Toml)
        } else if content_type.contains("properties") {
            Some(Self::Properties)
        } else if content_type.contains("octet-stream") {
            Some(Self::Binary)
        } else if content_type.starts_with("text/") {
            Some(Self::Text)
        } else {
            None
        }
    }
}

/// Parsed configuration content returned by the library.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigDocument {
    /// Parsed JSON payload.
    Json(Value),
    /// Parsed YAML payload converted into a JSON-like value.
    Yaml(Value),
    /// Parsed TOML payload converted into a JSON-like value.
    Toml(Value),
    /// Parsed Java properties payload.
    Properties(PropertiesDocument),
    /// UTF-8 text with no built-in structure.
    Text(String),
    /// Raw binary content.
    Binary(Vec<u8>),
}

impl ConfigDocument {
    /// Returns the document format.
    pub fn format(&self) -> DocumentFormat {
        match self {
            Self::Json(_) => DocumentFormat::Json,
            Self::Yaml(_) => DocumentFormat::Yaml,
            Self::Toml(_) => DocumentFormat::Toml,
            Self::Properties(_) => DocumentFormat::Properties,
            Self::Text(_) => DocumentFormat::Text,
            Self::Binary(_) => DocumentFormat::Binary,
        }
    }

    /// Converts the document into a JSON-like nested value without scalar coercion.
    pub fn to_value(&self) -> Result<Value> {
        self.to_value_with_coercion(ScalarCoercion::None)
    }

    /// Converts the document into a JSON-like nested value.
    pub fn to_value_with_coercion(&self, coercion: ScalarCoercion) -> Result<Value> {
        match self {
            Self::Json(value) | Self::Yaml(value) | Self::Toml(value) => {
                Ok(coerce_json_value(value.clone(), coercion))
            }
            Self::Properties(document) => Ok(document.to_value_with_coercion(coercion)),
            Self::Text(_) => Err(Error::UnsupportedBindingFormat { format: "text" }),
            Self::Binary(_) => Err(Error::UnsupportedBindingFormat { format: "binary" }),
        }
    }

    /// Deserializes the document into a Rust type using smart scalar coercion.
    pub fn deserialize<T>(&self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.deserialize_with_coercion(ScalarCoercion::Smart)
    }

    /// Deserializes the document into a Rust type using the requested coercion mode.
    pub fn deserialize_with_coercion<T>(&self, coercion: ScalarCoercion) -> Result<T>
    where
        T: DeserializeOwned,
    {
        deserialize_json_value(
            self.to_value_with_coercion(coercion)?,
            format!("{} document", self.format().as_str()),
        )
    }

    /// Deserializes the document without scalar coercion.
    pub fn deserialize_strict<T>(&self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.deserialize_with_coercion(ScalarCoercion::None)
    }

    pub(crate) fn from_text(origin: &str, format: DocumentFormat, text: String) -> Result<Self> {
        match format {
            DocumentFormat::Json => serde_json::from_str::<Value>(&text)
                .map(Self::Json)
                .map_err(|source| Error::Json {
                    url: origin.to_string(),
                    source,
                }),
            DocumentFormat::Yaml => serde_yaml::from_str::<Value>(&text)
                .map(Self::Yaml)
                .map_err(|source| Error::Yaml {
                    url: origin.to_string(),
                    source,
                }),
            DocumentFormat::Toml => {
                let value = toml::from_str::<toml::Value>(&text).map_err(|source| Error::Toml {
                    url: origin.to_string(),
                    source,
                })?;

                Ok(Self::Toml(
                    serde_json::to_value(value).expect("serializing TOML value should succeed"),
                ))
            }
            DocumentFormat::Properties => {
                Ok(Self::Properties(PropertiesDocument::parse(origin, &text)?))
            }
            DocumentFormat::Text => Ok(Self::Text(text)),
            DocumentFormat::Binary => Ok(Self::Binary(text.into_bytes())),
        }
    }
}

/// A parsed Java properties document.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertiesDocument {
    entries: BTreeMap<String, String>,
}

impl PropertiesDocument {
    /// Parses a Java properties document from text.
    pub fn parse(origin: &str, text: &str) -> Result<Self> {
        Ok(Self {
            entries: properties::parse(text, origin)?,
        })
    }

    /// Returns the flattened key-value entries.
    pub fn entries(&self) -> &BTreeMap<String, String> {
        &self.entries
    }

    /// Consumes the document and returns the flattened key-value entries.
    pub fn into_entries(self) -> BTreeMap<String, String> {
        self.entries
    }

    /// Converts the properties document into a nested JSON-like value without scalar coercion.
    pub fn to_value(&self) -> Value {
        self.to_value_with_coercion(ScalarCoercion::None)
    }

    /// Converts the properties document into a nested JSON-like value.
    pub fn to_value_with_coercion(&self, coercion: ScalarCoercion) -> Value {
        nested_value_from_string_map(
            self.entries
                .iter()
                .map(|(key, value)| (key.clone(), value.clone())),
            coercion,
        )
    }

    /// Deserializes the properties document into a Rust type using smart scalar coercion.
    pub fn deserialize<T>(&self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.deserialize_with_coercion(ScalarCoercion::Smart)
    }

    /// Deserializes the properties document into a Rust type using the requested coercion mode.
    pub fn deserialize_with_coercion<T>(&self, coercion: ScalarCoercion) -> Result<T>
    where
        T: DeserializeOwned,
    {
        deserialize_json_value(self.to_value_with_coercion(coercion), "properties document")
    }

    /// Deserializes the properties document without scalar coercion.
    pub fn deserialize_strict<T>(&self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.deserialize_with_coercion(ScalarCoercion::None)
    }
}

/// Raw resource content fetched from the plain-text Spring Config endpoint.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigResource {
    path: String,
    url: String,
    content_type: Option<String>,
    bytes: Vec<u8>,
}

impl ConfigResource {
    pub(crate) fn new(
        path: String,
        url: String,
        content_type: Option<String>,
        bytes: Vec<u8>,
    ) -> Self {
        Self {
            path,
            url,
            content_type,
            bytes,
        }
    }

    /// Returns the logical path requested from Spring Config Server.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the final resource URL used to fetch this payload.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the response content type, when present.
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    /// Returns the raw bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes the resource and returns the raw bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Decodes the resource as UTF-8 text.
    pub fn text(&self) -> Result<String> {
        String::from_utf8(self.bytes.clone()).map_err(|source| Error::Utf8 {
            url: self.url.clone(),
            source,
        })
    }

    /// Guesses the document format from the resource path, content type, and payload.
    pub fn format(&self) -> DocumentFormat {
        detect_format(&self.path, self.content_type(), &self.bytes)
    }

    /// Parses the resource into a [`ConfigDocument`].
    pub fn parse(&self) -> Result<ConfigDocument> {
        let format = self.format();
        match format {
            DocumentFormat::Binary => Ok(ConfigDocument::Binary(self.bytes.clone())),
            other => ConfigDocument::from_text(&self.url, other, self.text()?),
        }
    }

    /// Parses and deserializes the resource into a Rust type.
    pub fn deserialize<T>(&self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.parse()?.deserialize()
    }
}

fn detect_format(path: &str, content_type: Option<&str>, bytes: &[u8]) -> DocumentFormat {
    if let Some(format) = DocumentFormat::from_path(path) {
        return format;
    }

    if let Some(content_type) = content_type {
        if let Some(format) = DocumentFormat::from_content_type(content_type) {
            if format != DocumentFormat::Binary || String::from_utf8(bytes.to_vec()).is_err() {
                return format;
            }
        }
    }

    if String::from_utf8(bytes.to_vec()).is_ok() {
        DocumentFormat::Text
    } else {
        DocumentFormat::Binary
    }
}
