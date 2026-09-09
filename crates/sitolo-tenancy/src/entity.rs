//! Additional tenancy entities (BusinessEntity, Warehouse, Register).

use serde::{Deserialize, Serialize};
use sitolo_authz::{BranchId, BusinessEntityId, OrganizationId, RegisterId, WarehouseId};

/// Legal / Commercial Business Entity (§3.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusinessEntity {
    pub id: BusinessEntityId,
    pub organization_id: OrganizationId,
    pub legal_name: String,
    pub registration_number: Option<String>,
    pub tax_identifier: Option<String>,
}

/// Inventory holding facility (§3.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Warehouse {
    pub id: WarehouseId,
    pub organization_id: OrganizationId,
    pub branch_id: Option<BranchId>,
    pub name: String,
    pub active: bool,
}

/// Point-of-sale operational station (§3.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Register {
    pub id: RegisterId,
    pub organization_id: OrganizationId,
    pub branch_id: BranchId,
    pub name: String,
    pub active: bool,
}
