#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod binding;
mod client;
mod document;
mod environment;
mod error;
mod properties;
mod request;
/// Bootstrap helpers for loading Spring Cloud Config settings from environment variables.
pub mod bootstrap;

pub use binding::ScalarCoercion;
pub use crate::bootstrap::BootstrapConfig;
pub use client::{SpringConfigClient, SpringConfigClientBuilder};
pub use document::{ConfigDocument, ConfigResource, DocumentFormat, PropertiesDocument};
pub use environment::{Environment, PropertySource};
pub use error::{Error, Result};
pub use request::{EnvironmentFormat, EnvironmentRequest, ResourceRequest};
