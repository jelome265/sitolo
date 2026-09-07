//! Sitolo API binary composition root.
//!
//! Phase 0–2 audit (F-001/F-020): exactly one application bootstrap path
//! consumes the Phase 2 platform contracts. `main` only orchestrates
//! build → serve → shutdown; all composition lives in [`bootstrap`].
#![forbid(unsafe_code)]

pub mod bootstrap;
pub mod serve;
pub mod shutdown;
pub mod state;
