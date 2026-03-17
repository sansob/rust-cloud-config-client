use rust_cloud_config_client::{BootstrapConfig, Error};
use temp_env::with_vars;

#[test]
fn reads_bootstrap_settings_from_environment() {
    with_vars(
        [
            (
                BootstrapConfig::SERVER_URL_ENV,
                Some("http://localhost:8888"),
            ),
            (BootstrapConfig::APPLICATION_ENV, Some("inventory-service")),
            (BootstrapConfig::PROFILES_ENV, Some("dev,aws")),
            (BootstrapConfig::LABEL_ENV, Some("main")),
            (BootstrapConfig::TIMEOUT_SECONDS_ENV, Some("15")),
        ],
        || {
            let bootstrap = BootstrapConfig::from_env().expect("bootstrap config should parse");
            assert_eq!(bootstrap.server_url(), "http://localhost:8888");
            assert_eq!(bootstrap.application(), "inventory-service");
            assert_eq!(
                bootstrap.profiles(),
                &["dev".to_string(), "aws".to_string()]
            );
            assert_eq!(bootstrap.label_ref(), Some("main"));
        },
    );
}

#[test]
fn defaults_profile_to_default_when_missing() {
    with_vars(
        [
            (
                BootstrapConfig::SERVER_URL_ENV,
                Some("http://localhost:8888"),
            ),
            (BootstrapConfig::APPLICATION_ENV, Some("inventory-service")),
            (BootstrapConfig::PROFILES_ENV, None),
        ],
        || {
            let bootstrap = BootstrapConfig::from_env().expect("bootstrap config should parse");
            assert_eq!(bootstrap.profiles(), &["default".to_string()]);
        },
    );
}

#[test]
fn rejects_conflicting_auth_environment_variables() {
    with_vars(
        [
            (
                BootstrapConfig::SERVER_URL_ENV,
                Some("http://localhost:8888"),
            ),
            (BootstrapConfig::APPLICATION_ENV, Some("inventory-service")),
            (BootstrapConfig::PROFILES_ENV, Some("dev")),
            (BootstrapConfig::USERNAME_ENV, Some("user")),
            (BootstrapConfig::PASSWORD_ENV, Some("pass")),
            (BootstrapConfig::BEARER_TOKEN_ENV, Some("token")),
        ],
        || {
            let error = BootstrapConfig::from_env().expect_err("bootstrap config should fail");
            assert!(matches!(error, Error::InvalidBootstrapConfiguration(_)));
        },
    );
}
