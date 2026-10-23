//! Inventory validators for HTTP routes and outbound HTTP clients.
//!
//! This crate is the executable form of the FW-INV and FW-RQW rules in the
//! `rust-enterprise-frameworks` skill. It contains pure validation logic with no
//! framework dependency, so it is compiled and tested on its own and does not depend
//! on axum's or reqwest's minimum supported Rust version.
//!
//! Enforcement layers that surround this logic (see the skill, FW-1):
//!
//! 1. `clippy.toml` bans direct route and client construction outside the modules that
//!    call these validators.
//! 2. The tests below fail when a route lacks a permission or a body limit, when a client
//!    policy is unsafe, or when the reviewed route inventory changes without a snapshot
//!    update.
//!
//! Framework glue (turning a [`RouteSpec`] into an `axum::Router` route, and a
//! [`ClientPolicy`] into a `reqwest::Client`) lives in the service crate and calls these
//! validators first.

use std::time::Duration;

/// Largest request body any route may declare, in bytes (SEC-IN-03).
pub const MAX_BODY_BYTES_CEILING: usize = 16 * 1024 * 1024;

/// Longest timeout any route may declare (RES-TO-03).
pub const MAX_ROUTE_TIMEOUT: Duration = Duration::from_secs(60);

/// Longest total timeout any client policy may declare (RES-TO-01).
pub const MAX_CLIENT_TOTAL_TIMEOUT: Duration = Duration::from_secs(120);

/// Largest idle connection pool any client policy may declare per host.
pub const MAX_IDLE_PER_HOST: usize = 1024;

/// Access requirement for a route.
///
/// There is deliberately no implicit "public" variant. A route that needs no
/// authentication declares [`Access::Public`] with a reason, and that declaration is
/// reviewed as part of the inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Access {
    /// The route is reachable without authentication. The reason is recorded in the
    /// inventory and must not be empty.
    Public {
        /// Why the route is public. Must be non-blank.
        reason: &'static str,
    },
    /// The route requires the named permission, checked at the domain operation
    /// (SEC-AUTHZ-02).
    Permission(&'static str),
}

/// HTTP method of a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Method {
    /// `GET`.
    Get,
    /// `POST`.
    Post,
    /// `PUT`.
    Put,
    /// `PATCH`.
    Patch,
    /// `DELETE`.
    Delete,
}

/// One route declaration. Constructing this value is the only sanctioned way to declare
/// a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteSpec {
    /// HTTP method.
    pub method: Method,
    /// Path template, for example `/v1/payments`.
    pub path: &'static str,
    /// Access requirement.
    pub access: Access,
    /// Maximum accepted request body, in bytes.
    pub body_limit_bytes: usize,
    /// Per-request deadline.
    pub timeout: Duration,
}

/// Reasons a single route declaration is rejected.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum InventoryError {
    /// The route requires a permission but the permission name is blank.
    MissingPermission {
        /// Path of the rejected route.
        path: &'static str,
    },
    /// The route is public but its reason is blank.
    EmptyPublicReason {
        /// Path of the rejected route.
        path: &'static str,
    },
    /// The body limit is zero, which would reject every request.
    BodyLimitZero {
        /// Path of the rejected route.
        path: &'static str,
    },
    /// The body limit exceeds [`MAX_BODY_BYTES_CEILING`].
    BodyLimitTooLarge {
        /// Path of the rejected route.
        path: &'static str,
        /// The declared limit in bytes.
        limit: usize,
    },
    /// The timeout is zero.
    TimeoutZero {
        /// Path of the rejected route.
        path: &'static str,
    },
    /// The timeout exceeds [`MAX_ROUTE_TIMEOUT`].
    TimeoutTooLarge {
        /// Path of the rejected route.
        path: &'static str,
    },
}

/// Validates one route declaration.
///
/// Called at registration. A failure is a startup error.
///
/// # Errors
///
/// Returns [`InventoryError`] describing the first rule the declaration breaks.
pub fn validate_route(spec: &RouteSpec) -> Result<(), InventoryError> {
    match spec.access {
        Access::Permission(permission) if permission.trim().is_empty() => {
            return Err(InventoryError::MissingPermission { path: spec.path });
        }
        Access::Public { reason } if reason.trim().is_empty() => {
            return Err(InventoryError::EmptyPublicReason { path: spec.path });
        }
        Access::Permission(_) | Access::Public { .. } => {}
    }
    if spec.body_limit_bytes == 0 {
        return Err(InventoryError::BodyLimitZero { path: spec.path });
    }
    if spec.body_limit_bytes > MAX_BODY_BYTES_CEILING {
        return Err(InventoryError::BodyLimitTooLarge {
            path: spec.path,
            limit: spec.body_limit_bytes,
        });
    }
    if spec.timeout.is_zero() {
        return Err(InventoryError::TimeoutZero { path: spec.path });
    }
    if spec.timeout > MAX_ROUTE_TIMEOUT {
        return Err(InventoryError::TimeoutTooLarge { path: spec.path });
    }
    Ok(())
}

/// Reasons a registration fails.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RegistryError {
    /// The route declaration itself is invalid.
    Invalid(InventoryError),
    /// A route with the same method and path is already registered.
    Duplicate {
        /// Path of the duplicate route.
        path: &'static str,
    },
}

impl std::fmt::Display for InventoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPermission { path } => write!(f, "route {path} has a blank permission"),
            Self::EmptyPublicReason { path } => write!(f, "public route {path} has no reason"),
            Self::BodyLimitZero { path } => write!(f, "route {path} has a zero body limit"),
            Self::BodyLimitTooLarge { path, limit } => {
                write!(f, "route {path} body limit {limit} exceeds the ceiling")
            }
            Self::TimeoutZero { path } => write!(f, "route {path} has a zero timeout"),
            Self::TimeoutTooLarge { path } => write!(f, "route {path} timeout exceeds the ceiling"),
        }
    }
}

impl std::error::Error for InventoryError {}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(inner) => write!(f, "invalid route: {inner}"),
            Self::Duplicate { path } => write!(f, "route {path} is already registered"),
        }
    }
}

impl std::error::Error for RegistryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Invalid(inner) => Some(inner),
            Self::Duplicate { .. } => None,
        }
    }
}

impl std::fmt::Display for ClientPolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::ConnectTimeoutZero => "connect timeout is zero",
            Self::TotalTimeoutZero => "total timeout is zero",
            Self::ConnectExceedsTotal => "connect timeout exceeds total timeout",
            Self::TotalTimeoutTooLarge => "total timeout exceeds the ceiling",
            Self::IdlePoolOutOfBounds => "idle pool size is out of bounds",
            Self::PlaintextAllowed => "plaintext HTTP is not permitted",
        };
        f.write_str(text)
    }
}

impl std::error::Error for ClientPolicyError {}

/// The single list of routes for a service, built once at startup.
#[derive(Debug, Default)]
pub struct Registry {
    routes: Vec<RouteSpec>,
}

impl Registry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Validates and registers one route.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::Invalid`] if the declaration fails validation, or
    /// [`RegistryError::Duplicate`] if the method and path are already registered.
    pub fn register(&mut self, spec: RouteSpec) -> Result<(), RegistryError> {
        validate_route(&spec).map_err(RegistryError::Invalid)?;
        let duplicate = self
            .routes
            .iter()
            .any(|existing| existing.method == spec.method && existing.path == spec.path);
        if duplicate {
            return Err(RegistryError::Duplicate { path: spec.path });
        }
        self.routes.push(spec);
        Ok(())
    }

    /// Returns the registered routes in registration order.
    #[must_use]
    pub fn routes(&self) -> &[RouteSpec] {
        &self.routes
    }

    /// Returns a stable, reviewable inventory: one line per route, sorted.
    ///
    /// CI compares this text with a committed snapshot, so every new route appears as a
    /// reviewable diff.
    #[must_use]
    pub fn snapshot(&self) -> String {
        let mut lines: Vec<String> = self.routes.iter().map(render_route).collect();
        lines.sort();
        lines.join("\n")
    }
}

fn render_route(route: &RouteSpec) -> String {
    let access = match route.access {
        Access::Public { reason } => format!("public({reason})"),
        Access::Permission(permission) => format!("perm({permission})"),
    };
    format!(
        "{:?} {} access={} body<={} timeout={}ms",
        route.method,
        route.path,
        access,
        route.body_limit_bytes,
        route.timeout.as_millis()
    )
}

/// Outbound HTTP client policy (SEC-TLS-04, RES-TO-01, RES-BULK-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientPolicy {
    /// Maximum time to establish a connection.
    pub connect_timeout: Duration,
    /// Maximum total time for one request, including reading the body.
    pub total_timeout: Duration,
    /// Maximum idle pooled connections per host.
    pub max_idle_per_host: usize,
    /// Whether plaintext `http` URLs are refused.
    pub https_only: bool,
}

/// Reasons a client policy is rejected.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClientPolicyError {
    /// The connect timeout is zero.
    ConnectTimeoutZero,
    /// The total timeout is zero.
    TotalTimeoutZero,
    /// The connect timeout is longer than the total timeout.
    ConnectExceedsTotal,
    /// The total timeout exceeds [`MAX_CLIENT_TOTAL_TIMEOUT`].
    TotalTimeoutTooLarge,
    /// The idle pool is zero or larger than [`MAX_IDLE_PER_HOST`].
    IdlePoolOutOfBounds,
    /// Plaintext HTTP is allowed.
    PlaintextAllowed,
}

/// Validates a client policy before a client is built from it.
///
/// # Errors
///
/// Returns [`ClientPolicyError`] describing the first rule the policy breaks.
pub fn validate_client_policy(policy: &ClientPolicy) -> Result<(), ClientPolicyError> {
    if policy.connect_timeout.is_zero() {
        return Err(ClientPolicyError::ConnectTimeoutZero);
    }
    if policy.total_timeout.is_zero() {
        return Err(ClientPolicyError::TotalTimeoutZero);
    }
    if policy.connect_timeout > policy.total_timeout {
        return Err(ClientPolicyError::ConnectExceedsTotal);
    }
    if policy.total_timeout > MAX_CLIENT_TOTAL_TIMEOUT {
        return Err(ClientPolicyError::TotalTimeoutTooLarge);
    }
    if policy.max_idle_per_host == 0 || policy.max_idle_per_host > MAX_IDLE_PER_HOST {
        return Err(ClientPolicyError::IdlePoolOutOfBounds);
    }
    if !policy.https_only {
        return Err(ClientPolicyError::PlaintextAllowed);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests return `Result` instead of using `expect` or `panic`, so the workspace lint
    //! policy applies to test code without a test-only exception (BP-TS-06).

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn payments_route(path: &'static str) -> RouteSpec {
        RouteSpec {
            method: Method::Post,
            path,
            access: Access::Permission("payments:capture"),
            body_limit_bytes: 64 * 1024,
            timeout: Duration::from_secs(10),
        }
    }

    fn policy() -> ClientPolicy {
        ClientPolicy {
            connect_timeout: Duration::from_secs(3),
            total_timeout: Duration::from_secs(10),
            max_idle_per_host: 32,
            https_only: true,
        }
    }

    #[test]
    fn valid_route_is_accepted() -> TestResult {
        validate_route(&payments_route("/v1/payments"))?;
        Ok(())
    }

    #[test]
    fn route_without_permission_is_rejected() {
        let mut route = payments_route("/v1/x");
        route.access = Access::Permission("   ");
        assert_eq!(
            validate_route(&route),
            Err(InventoryError::MissingPermission { path: "/v1/x" })
        );
    }

    #[test]
    fn public_route_needs_a_reason() {
        let mut route = payments_route("/healthz");
        route.access = Access::Public { reason: "" };
        assert_eq!(
            validate_route(&route),
            Err(InventoryError::EmptyPublicReason { path: "/healthz" })
        );
    }

    #[test]
    fn body_limit_zero_is_rejected() {
        let mut route = payments_route("/a");
        route.body_limit_bytes = 0;
        assert_eq!(
            validate_route(&route),
            Err(InventoryError::BodyLimitZero { path: "/a" })
        );
    }

    #[test]
    fn body_limit_over_ceiling_is_rejected() {
        let mut route = payments_route("/b");
        route.body_limit_bytes = MAX_BODY_BYTES_CEILING + 1;
        assert_eq!(
            validate_route(&route),
            Err(InventoryError::BodyLimitTooLarge {
                path: "/b",
                limit: MAX_BODY_BYTES_CEILING + 1,
            })
        );
    }

    #[test]
    fn timeout_bounds_are_enforced() {
        let mut zero = payments_route("/c");
        zero.timeout = Duration::ZERO;
        assert_eq!(
            validate_route(&zero),
            Err(InventoryError::TimeoutZero { path: "/c" })
        );

        let mut long = payments_route("/d");
        long.timeout = MAX_ROUTE_TIMEOUT + Duration::from_secs(1);
        assert_eq!(
            validate_route(&long),
            Err(InventoryError::TimeoutTooLarge { path: "/d" })
        );
    }

    #[test]
    fn duplicate_route_is_rejected() -> TestResult {
        let mut registry = Registry::new();
        registry.register(payments_route("/v1/payments"))?;
        assert_eq!(
            registry.register(payments_route("/v1/payments")),
            Err(RegistryError::Duplicate {
                path: "/v1/payments"
            })
        );
        Ok(())
    }

    /// The reviewed inventory. Adding a route changes this text, and the change must be
    /// made in the same pull request so that the new route is reviewed.
    #[test]
    fn route_inventory_matches_reviewed_snapshot() -> TestResult {
        let mut registry = Registry::new();
        registry.register(RouteSpec {
            method: Method::Get,
            path: "/healthz",
            access: Access::Public {
                reason: "liveness probe, no internal detail",
            },
            body_limit_bytes: 1024,
            timeout: Duration::from_secs(2),
        })?;
        registry.register(payments_route("/v1/payments"))?;

        let expected = "Get /healthz access=public(liveness probe, no internal detail) body<=1024 timeout=2000ms\n\
                        Post /v1/payments access=perm(payments:capture) body<=65536 timeout=10000ms";
        assert_eq!(registry.snapshot(), expected);
        Ok(())
    }

    #[test]
    fn valid_client_policy_passes() -> TestResult {
        validate_client_policy(&policy())?;
        Ok(())
    }

    #[test]
    fn client_policy_rejects_zero_total_timeout() {
        let mut candidate = policy();
        candidate.total_timeout = Duration::ZERO;
        assert_eq!(
            validate_client_policy(&candidate),
            Err(ClientPolicyError::TotalTimeoutZero)
        );
    }

    #[test]
    fn client_policy_rejects_inverted_timeouts() {
        let mut candidate = policy();
        candidate.connect_timeout = Duration::from_secs(20);
        assert_eq!(
            validate_client_policy(&candidate),
            Err(ClientPolicyError::ConnectExceedsTotal)
        );
    }

    #[test]
    fn client_policy_rejects_plaintext() {
        let mut candidate = policy();
        candidate.https_only = false;
        assert_eq!(
            validate_client_policy(&candidate),
            Err(ClientPolicyError::PlaintextAllowed)
        );
    }
}
