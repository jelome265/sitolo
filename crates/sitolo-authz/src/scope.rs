//! Scope boundaries and scope intersection logic.

use crate::id::{BranchId, RegisterId, WarehouseId};
use serde::{Deserialize, Serialize};

/// Organizational scope dimension.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Scope {
    /// Organization-wide scope (applies to all descendant branches/warehouses/registers).
    Organization,
    /// Scope restricted to a specific branch.
    Branch(BranchId),
    /// Scope restricted to a specific warehouse.
    Warehouse(WarehouseId),
    /// Scope restricted to a specific register.
    Register(RegisterId),
}

impl Scope {
    /// True if `self` (the granted scope) covers or equals `target` scope.
    pub fn covers(&self, target: &Scope) -> bool {
        match (self, target) {
            (Scope::Organization, _) => true, // Org-wide grant covers all descendant scopes
            (Scope::Branch(b1), Scope::Branch(b2)) => b1 == b2,
            (Scope::Warehouse(w1), Scope::Warehouse(w2)) => w1 == w2,
            (Scope::Register(r1), Scope::Register(r2)) => r1 == r2,
            _ => false,
        }
    }
}
