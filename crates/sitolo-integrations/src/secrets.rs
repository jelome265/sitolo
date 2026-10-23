//! Production secret retrieval through AWS Secrets Manager.

use std::time::Duration;

use async_trait::async_trait;
use aws_config::{BehaviorVersion, ecs::EcsCredentialsProvider};
use aws_sdk_secretsmanager::{Client, error::ProvideErrorMetadata};
use sitolo_security::{SecretClass, SecretError, SecretProvider, SecretRef, SecretValue};
use thiserror::Error;

const MAX_SECRET_BYTES: usize = 65_536;
const MAX_SECRET_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Initialization failure for the ECS task-role-backed secret provider.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum SecretsManagerInitError {
    /// Region, reference class, or timeout does not meet the provider contract.
    #[error("invalid managed secret provider configuration")]
    InvalidConfiguration,
}

/// AWS Secrets Manager adapter constrained to one configured secret reference.
///
/// Production callers should create a distinct instance for each task role and
/// configure IAM to grant `secretsmanager:GetSecretValue` only for that exact
/// secret. The SDK is configured with the ECS task credential provider only;
/// static environment credentials, developer profiles, and IMDS are not used.
pub struct AwsSecretsManagerProvider {
    client: Client,
    allowed_reference: SecretRef,
    request_timeout: Duration,
}

impl AwsSecretsManagerProvider {
    /// Builds the provider using explicit region and ECS task-role credentials.
    ///
    /// # Errors
    /// Returns an error for invalid region, non-database reference, or timeout.
    /// AWS credential and service failures are returned later by [`SecretProvider::get`].
    ///
    /// # Panics
    /// This constructor does not panic for caller-provided input.
    pub async fn from_ecs_task_role(
        region: impl Into<String>,
        allowed_reference: SecretRef,
        request_timeout: Duration,
    ) -> Result<Self, SecretsManagerInitError> {
        let region = region.into();
        validate_provider_configuration(&region, &allowed_reference, request_timeout)?;

        let credentials = EcsCredentialsProvider::builder()
            .connect_timeout(Duration::from_secs(2))
            .read_timeout(Duration::from_secs(5))
            .build();
        let shared_config = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(region))
            .credentials_provider(credentials)
            .load()
            .await;
        Ok(Self {
            client: Client::new(&shared_config),
            allowed_reference,
            request_timeout,
        })
    }
}

#[async_trait]
impl SecretProvider for AwsSecretsManagerProvider {
    async fn get(&self, reference: &SecretRef) -> Result<SecretValue, SecretError> {
        if reference != &self.allowed_reference {
            return Err(SecretError::ProviderAuthorizationDenied {
                class: reference.class(),
            });
        }

        let response = tokio::time::timeout(
            self.request_timeout,
            self.client
                .get_secret_value()
                .secret_id(self.allowed_reference.path())
                .send(),
        )
        .await
        .map_err(|_| SecretError::Unavailable {
            class: reference.class(),
        })?
        .map_err(|error| map_service_error(reference, &error))?;

        resolve_secret_value(&self.allowed_reference, reference, response.secret_string())
    }
}

fn map_service_error(
    reference: &SecretRef,
    error: &aws_sdk_secretsmanager::error::SdkError<
        aws_sdk_secretsmanager::operation::get_secret_value::GetSecretValueError,
    >,
) -> SecretError {
    if let Some(service_error) = error.as_service_error() {
        match service_error.code() {
            Some("AccessDeniedException" | "AccessDenied") => {
                return SecretError::ProviderAuthorizationDenied {
                    class: reference.class(),
                };
            }
            Some("ResourceNotFoundException") => {
                return SecretError::Missing {
                    class: reference.class(),
                    reference: reference.reference_fingerprint(),
                };
            }
            _ => {}
        }
    }
    SecretError::Unavailable {
        class: reference.class(),
    }
}

fn resolve_secret_value(
    allowed_reference: &SecretRef,
    requested_reference: &SecretRef,
    secret_string: Option<&str>,
) -> Result<SecretValue, SecretError> {
    if allowed_reference != requested_reference {
        return Err(SecretError::ProviderAuthorizationDenied {
            class: requested_reference.class(),
        });
    }
    let value = secret_string.filter(|value| !value.is_empty() && value.len() <= MAX_SECRET_BYTES);
    value.map(SecretValue::new).ok_or(SecretError::Invalid {
        class: requested_reference.class(),
    })
}

fn validate_provider_configuration(
    region: &str,
    allowed_reference: &SecretRef,
    request_timeout: Duration,
) -> Result<(), SecretsManagerInitError> {
    if region.is_empty()
        || region.len() > 32
        || !region
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        || allowed_reference.class() != SecretClass::Database
        || request_timeout.is_zero()
        || request_timeout > MAX_SECRET_REQUEST_TIMEOUT
    {
        return Err(SecretsManagerInitError::InvalidConfiguration);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use sitolo_security::{SecretClass, SecretRef};

    use super::{SecretsManagerInitError, resolve_secret_value, validate_provider_configuration};
    use std::time::Duration;

    fn runtime_database_ref() -> Result<SecretRef, Box<dyn std::error::Error>> {
        Ok(SecretRef::new(
            SecretClass::Database,
            "prod/sitolo/api/runtime-db-password",
        )?)
    }

    #[test]
    fn accepts_only_the_exact_configured_database_secret_reference()
    -> Result<(), Box<dyn std::error::Error>> {
        let allowed = runtime_database_ref()?;
        let value = resolve_secret_value(&allowed, &allowed, Some("test-secret"))?;
        assert_eq!(value.into_string(), "test-secret");

        let another_database_ref = SecretRef::new(
            SecretClass::Database,
            "prod/sitolo/migrations/admin-db-credential",
        )?;
        assert!(resolve_secret_value(&allowed, &another_database_ref, Some("wrong")).is_err());
        Ok(())
    }

    #[test]
    fn missing_empty_and_oversized_secret_values_fail_closed()
    -> Result<(), Box<dyn std::error::Error>> {
        let allowed = runtime_database_ref()?;
        assert!(resolve_secret_value(&allowed, &allowed, None).is_err());
        assert!(resolve_secret_value(&allowed, &allowed, Some("")).is_err());
        assert!(resolve_secret_value(&allowed, &allowed, Some(&"x".repeat(65_537))).is_err());
        Ok(())
    }

    #[test]
    fn provider_requires_a_bounded_region_database_reference_and_timeout()
    -> Result<(), Box<dyn std::error::Error>> {
        let allowed = runtime_database_ref()?;
        assert!(
            validate_provider_configuration("af-south-1", &allowed, Duration::from_secs(5)).is_ok()
        );
        assert!(validate_provider_configuration("", &allowed, Duration::from_secs(5)).is_err());
        assert!(validate_provider_configuration("af-south-1", &allowed, Duration::ZERO).is_err());
        assert!(
            validate_provider_configuration("af-south-1", &allowed, Duration::from_secs(11))
                .is_err()
        );
        let identity_ref = SecretRef::new(SecretClass::Identity, "prod/sitolo/oidc/client")?;
        assert_eq!(
            validate_provider_configuration("af-south-1", &identity_ref, Duration::from_secs(5)),
            Err(SecretsManagerInitError::InvalidConfiguration)
        );
        Ok(())
    }
}
