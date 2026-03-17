use crate::{Error, Result};

/// Output format for the Spring alternative-format environment endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentFormat {
    /// YAML output using the `.yml` suffix.
    Yml,
    /// YAML output using the `.yaml` suffix.
    Yaml,
    /// Java properties output using the `.properties` suffix.
    Properties,
}

impl EnvironmentFormat {
    pub(crate) fn suffix(self) -> &'static str {
        match self {
            Self::Yml => ".yml",
            Self::Yaml => ".yaml",
            Self::Properties => ".properties",
        }
    }
}

/// A request for the Spring `Environment` endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentRequest {
    application: String,
    profiles: Vec<String>,
    label: Option<String>,
    resolve_placeholders: bool,
}

impl EnvironmentRequest {
    /// Creates a new environment request.
    pub fn new<A, I, P>(application: A, profiles: I) -> Result<Self>
    where
        A: Into<String>,
        I: IntoIterator<Item = P>,
        P: Into<String>,
    {
        let application = sanitize_required(application.into(), Error::EmptyApplication)?;
        let profiles = sanitize_profiles(profiles)?;

        Ok(Self {
            application,
            profiles,
            label: None,
            resolve_placeholders: false,
        })
    }

    /// Overrides the git label, branch, tag, or commit for this request.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        let label = label.into().trim().to_string();
        self.label = if label.is_empty() { None } else { Some(label) };
        self
    }

    /// Controls the `resolvePlaceholders` query parameter used by YAML and properties endpoints.
    pub fn resolve_placeholders(mut self, enabled: bool) -> Self {
        self.resolve_placeholders = enabled;
        self
    }

    /// Returns the application name.
    pub fn application(&self) -> &str {
        &self.application
    }

    /// Returns the active profiles in request order.
    pub fn profiles(&self) -> &[String] {
        &self.profiles
    }

    /// Returns the explicit label, when set.
    pub fn label_ref(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Returns whether placeholder resolution should be requested for alternative formats.
    pub fn resolve_placeholders_enabled(&self) -> bool {
        self.resolve_placeholders
    }

    pub(crate) fn joined_profiles(&self) -> String {
        self.profiles.join(",")
    }
}

/// A request for the plain-text Spring resource endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRequest {
    application: String,
    profiles: Vec<String>,
    label: Option<String>,
    path: String,
}

impl ResourceRequest {
    /// Creates a new resource request.
    pub fn new<A, I, P, R>(application: A, profiles: I, path: R) -> Result<Self>
    where
        A: Into<String>,
        I: IntoIterator<Item = P>,
        P: Into<String>,
        R: Into<String>,
    {
        let application = sanitize_required(application.into(), Error::EmptyApplication)?;
        let profiles = sanitize_profiles(profiles)?;
        let path = sanitize_required(
            normalize_resource_path(path.into()),
            Error::EmptyResourcePath,
        )?;

        Ok(Self {
            application,
            profiles,
            label: None,
            path,
        })
    }

    /// Overrides the git label, branch, tag, or commit for this request.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        let label = label.into().trim().to_string();
        self.label = if label.is_empty() { None } else { Some(label) };
        self
    }

    /// Returns the application name.
    pub fn application(&self) -> &str {
        &self.application
    }

    /// Returns the active profiles in request order.
    pub fn profiles(&self) -> &[String] {
        &self.profiles
    }

    /// Returns the explicit label, when set.
    pub fn label_ref(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Returns the resource path.
    pub fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn joined_profiles(&self) -> String {
        self.profiles.join(",")
    }

    pub(crate) fn path_segments(&self) -> Vec<String> {
        self.path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    }
}

fn sanitize_profiles<I, P>(profiles: I) -> Result<Vec<String>>
where
    I: IntoIterator<Item = P>,
    P: Into<String>,
{
    let sanitized: Vec<String> = profiles
        .into_iter()
        .map(Into::into)
        .map(|profile| profile.trim().to_string())
        .filter(|profile| !profile.is_empty())
        .collect();

    if sanitized.is_empty() {
        Err(Error::EmptyProfiles)
    } else {
        Ok(sanitized)
    }
}

fn sanitize_required(value: String, error: Error) -> Result<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        Err(error)
    } else {
        Ok(value)
    }
}

fn normalize_resource_path(path: String) -> String {
    path.replace('\\', "/").trim_matches('/').to_string()
}
