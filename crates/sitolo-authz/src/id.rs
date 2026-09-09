//! Opaque identifiers for Tenancy and IAM.

use serde::{Deserialize, Serialize};
pub use sitolo_auth::id::SecurityVersion;

use crate::error::AuthzError;

macro_rules! tenancy_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, AuthzError> {
                let value = value.into();
                if value.is_empty()
                    || value.len() > 128
                    || !value.bytes().all(|b| {
                        b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':')
                    })
                {
                    return Err(AuthzError::InvalidIdentifier);
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

tenancy_id!(OrganizationId);
tenancy_id!(BranchId);
tenancy_id!(MembershipId);
tenancy_id!(InvitationId);
tenancy_id!(RoleId);
tenancy_id!(WarehouseId);
tenancy_id!(RegisterId);
tenancy_id!(BusinessEntityId);
tenancy_id!(OwnershipTransferId);
tenancy_id!(ScopeGrantId);
