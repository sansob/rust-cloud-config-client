use std::string::FromUtf8Error;

use reqwest::StatusCode;
use thiserror::Error;

/// Result type used by this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Error type returned by the Spring Cloud Config client.
#[derive(Debug, Error)]
pub enum Error {
    /// The base URL could not be parsed.
    #[error("base URL is invalid: {0}")]
    InvalidBaseUrl(String),

    /// The base URL contains a query string or fragment, which is not supported.
    #[error("base URL must not contain a query string or fragment: {0}")]
    InvalidBaseUrlShape(String),

    /// The application name is empty.
    #[error("application name cannot be empty")]
    EmptyApplication,

    /// No profiles were supplied.
    #[error("at least one profile must be provided")]
    EmptyProfiles,

    /// The resource path is empty.
    #[error("resource path cannot be empty")]
    EmptyResourcePath,

    /// A required environment variable is missing.
    #[error("environment variable `{name}` is required")]
    MissingEnvironmentVariable {
        /// The environment variable name.
        name: &'static str,
    },

    /// An environment variable has an invalid value.
    #[error("environment variable `{name}` is invalid: {reason} (value: {value})")]
    InvalidEnvironmentVariable {
        /// The environment variable name.
        name: &'static str,
        /// A short reason.
        reason: &'static str,
        /// The provided value.
        value: String,
    },

    /// The bootstrap configuration is internally inconsistent.
    #[error("bootstrap configuration is invalid: {0}")]
    InvalidBootstrapConfiguration(String),

    /// A custom HTTP header name is invalid.
    #[error("header name is invalid: {0}")]
    InvalidHeaderName(String),

    /// A custom HTTP header value is invalid.
    #[error("header value is invalid for `{name}`: {value}")]
    InvalidHeaderValue {
        /// The header name.
        name: String,
        /// The header value.
        value: String,
    },

    /// The HTTP request failed before a valid response was received.
    #[error("request to {url} failed: {source}")]
    Transport {
        /// The target URL.
        url: String,
        /// The transport error.
        #[source]
        source: reqwest::Error,
    },

    /// The Config Server returned a non-success HTTP status.
    #[error("config server returned {status} for {url}: {body}")]
    HttpStatus {
        /// The HTTP status code.
        status: StatusCode,
        /// The target URL.
        url: String,
        /// The response body, when available.
        body: String,
    },

    /// The response body could not be parsed as JSON.
    #[error("response from {url} is not valid JSON: {source}")]
    Json {
        /// The target URL.
        url: String,
        /// The parse error.
        #[source]
        source: serde_json::Error,
    },

    /// The response body could not be parsed as YAML.
    #[error("response from {url} is not valid YAML: {source}")]
    Yaml {
        /// The target URL.
        url: String,
        /// The parse error.
        #[source]
        source: serde_yaml::Error,
    },

    /// The response body could not be parsed as TOML.
    #[error("response from {url} is not valid TOML: {source}")]
    Toml {
        /// The target URL.
        url: String,
        /// The parse error.
        #[source]
        source: toml::de::Error,
    },

    /// The response body could not be parsed as Java properties.
    #[error("response from {origin} is not valid Java properties: {reason}")]
    Properties {
        /// The origin being parsed.
        origin: String,
        /// A human-readable reason.
        reason: String,
    },

    /// The response body was expected to be UTF-8 text but was not valid UTF-8.
    #[error("response from {url} is not valid UTF-8: {source}")]
    Utf8 {
        /// The target URL.
        url: String,
        /// The UTF-8 decode error.
        #[source]
        source: FromUtf8Error,
    },

    /// Typed deserialization is not supported for the requested document kind.
    #[error("typed deserialization is not supported for {format}")]
    UnsupportedBindingFormat {
        /// The format name.
        format: &'static str,
    },

    /// The configuration payload could not be bound into the requested Rust type.
    #[error("failed to bind configuration from {origin}: {source}")]
    Bind {
        /// A short description of the binding source.
        origin: String,
        /// The Serde error.
        #[source]
        source: serde_json::Error,
    },
}
