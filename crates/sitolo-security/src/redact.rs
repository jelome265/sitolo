//! Redaction primitives.
//!
//! Redaction is defense in depth, not the primary data-classification
//! mechanism (Phase 2 specification, §20). The safe path is: never build an
//! event containing secret material in the first place. When an event must be
//! emitted and it *might* contain forbidden data, we redact the forbidden data
//! and, on redaction failure, refuse to emit the raw payload.

use std::fmt::Write;

/// Budgets that bound redaction and serialization work so hostile input cannot
/// become an allocation-blowup path (Phase 2 specification, §28, §47-§49).
#[derive(Debug, Clone, Copy)]
pub struct RedactLimits {
    pub max_input_bytes: usize,
    pub max_output_bytes: usize,
}

impl Default for RedactLimits {
    fn default() -> Self {
        RedactLimits {
            max_input_bytes: 64 * 1024,
            max_output_bytes: 8 * 1024,
        }
    }
}

/// A fixed ASCII placeholder used whenever redacted content appears.
pub const REDACTED: &str = "<redacted>";

/// Outcome of attempting to redact a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedactionOutcome {
    /// Redaction succeeded; `buf` holds the safe text.
    Ok(String),
    /// The value was unsafe and redaction was intentionally not attempted.
    Refused,
    /// The input exceeded configured bounds; the safe protocol is to emit a
    /// smaller safe failure event rather than the raw payload.
    BoundedDrop,
}

/// A minimal, deterministic string redactor.
///
/// Given a set of exact byte sequences that must never appear in a sink, this
/// replaces every occurrence with `<redacted>` so downstream code cannot leak
/// them by accident (Phase 2 specification, §46, Appendix X).
#[derive(Debug, Clone)]
pub struct SecretRedaction {
    secrets: Vec<Vec<u8>>,
    limits: RedactLimits,
}

impl SecretRedaction {
    /// Configures a redactor over the supplied secret byte sequences.
    pub fn new(secrets: impl IntoIterator<Item = Vec<u8>>, limits: RedactLimits) -> Self {
        SecretRedaction {
            secrets: secrets.into_iter().collect(),
            limits,
        }
    }

    /// Redacts a CSV-style cell (or any bounded text).
    pub fn redact(&self, input: &str) -> RedactionOutcome {
        if input.len() > self.limits.max_input_bytes {
            return RedactionOutcome::BoundedDrop;
        }
        let mut out = String::with_capacity(input.len());
        let bytes = input.as_bytes();
        let mut i = 0usize;
        // Greedy earliest-match scan. Forbidden sequences are short; linear
        // scan over `secrets` is acceptable under configured bounds.
        while i < bytes.len() {
            let mut matched: Option<(usize, usize)> = None; // (start, end)
            for s in &self.secrets {
                if s.is_empty() {
                    continue;
                }
                if bytes[i..].len() >= s.len() && &bytes[i..i + s.len()] == s.as_slice() {
                    matched = Some((i, i + s.len()));
                    break;
                }
            }
            match matched {
                Some((_, end)) => {
                    let _ = out.write_str(REDACTED);
                    if out.len() > self.limits.max_output_bytes {
                        return RedactionOutcome::BoundedDrop;
                    }
                    i = end;
                }
                None => {
                    out.push(bytes[i] as char);
                    i += 1;
                }
            }
        }
        RedactionOutcome::Ok(out)
    }
}

/// A placeholder buffer type kept to match the public surface used by callers
/// who want a reusable write target. Prefer [`SecretRedaction::redact`].
#[derive(Debug, Default)]
pub struct RedactBuf(String);

impl RedactBuf {
    /// Applies `redaction` to `input`, returning whether it succeeded and was
    /// bounded; on success the buffer holds the safe text.
    pub fn apply(&mut self, redaction: &SecretRedaction, input: &str) -> bool {
        match redaction.redact(input) {
            RedactionOutcome::Ok(s) => {
                self.0 = s;
                true
            }
            // Refused: redaction was intentionally not attempted; the buffer
            // must not carry raw content, so it holds the safe marker.
            RedactionOutcome::Refused => {
                self.0 = REDACTED.to_string();
                true
            }
            RedactionOutcome::BoundedDrop => false,
        }
    }

    /// The current safe text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn redactor() -> SecretRedaction {
        SecretRedaction::new(
            vec![
                b"TEST_SECRET_DATABASE_001".to_vec(),
                b"TEST_SECRET_PAYMENT_002".to_vec(),
            ],
            RedactLimits::default(),
        )
    }

    #[test]
    fn redacts_exact_secret_sequences() {
        let r = redactor();
        let out = match r.redact("connect with TEST_SECRET_DATABASE_001 now") {
            RedactionOutcome::Ok(s) => s,
            other => panic!("unexpected {other:?}"),
        };
        assert!(!out.contains("TEST_SECRET_DATABASE_001"));
        assert!(out.contains(REDACTED));
        assert!(out.contains("connect with"));
    }
}
