//! Integrations.
//!
//! Owns external adapters, including the payment and MRA EIS boundaries
//! (Phase 1 specification, §5.12). External systems are untrusted trust
//! boundaries; provider representations remain behind adapters.
#![forbid(unsafe_code)]
