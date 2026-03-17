use std::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    Result, ScalarCoercion,
    binding::{coerce_json_value, deserialize_json_value, nested_value_from_flat_map},
};

/// Spring Cloud Config `Environment` payload returned by `/{application}/{profile}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Environment {
    /// Application name resolved by the Config Server.
    pub name: String,
    /// Active profiles returned by the Config Server.
    #[serde(default)]
    pub profiles: Vec<String>,
    /// Git label, branch, tag, or commit used by the Config Server.
    #[serde(default)]
    pub label: Option<String>,
    /// Backend version metadata, when available.
    #[serde(default)]
    pub version: Option<String>,
    /// Backend state metadata, when available.
    #[serde(default)]
    pub state: Option<String>,
    /// Ordered property sources. Earlier entries have higher precedence.
    #[serde(rename = "propertySources", default)]
    pub property_sources: Vec<PropertySource>,
}

impl Environment {
    /// Returns the effective flat property map after applying Spring property-source precedence.
    ///
    /// Spring Cloud Config returns higher-precedence property sources earlier in the list.
    /// This method applies them accordingly and returns the final flat key-value map.
    pub fn effective_properties(&self) -> BTreeMap<String, Value> {
        let mut merged = BTreeMap::new();

        for source in self.property_sources.iter().rev() {
            for (key, value) in &source.source {
                merged.insert(key.clone(), value.clone());
            }
        }

        merged
    }

    /// Converts the effective flat property map into a nested JSON value without scalar coercion.
    pub fn to_value(&self) -> Value {
        self.to_value_with_coercion(ScalarCoercion::None)
    }

    /// Converts the effective flat property map into a nested JSON value.
    pub fn to_value_with_coercion(&self, coercion: ScalarCoercion) -> Value {
        nested_value_from_flat_map(self.effective_properties(), coercion)
    }

    /// Deserializes the effective configuration into a Rust type using smart scalar coercion.
    pub fn deserialize<T>(&self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.deserialize_with_coercion(ScalarCoercion::Smart)
    }

    /// Deserializes the effective configuration into a Rust type using the requested coercion mode.
    pub fn deserialize_with_coercion<T>(&self, coercion: ScalarCoercion) -> Result<T>
    where
        T: DeserializeOwned,
    {
        deserialize_json_value(
            coerce_json_value(self.to_value_with_coercion(ScalarCoercion::None), coercion),
            format!("environment `{}`", self.name),
        )
    }

    /// Deserializes the effective configuration into a Rust type without scalar coercion.
    pub fn deserialize_strict<T>(&self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.deserialize_with_coercion(ScalarCoercion::None)
    }
}

/// A single Spring property source within an [`Environment`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertySource {
    /// Property source name as reported by Spring Cloud Config.
    pub name: String,
    /// Flat key-value properties from the source.
    #[serde(default)]
    pub source: BTreeMap<String, Value>,
}
