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

/// Field names whose values must never appear in emitted authentication text.
/// Phase 3 specification, §40 and §62: authentication-specific fixtures are
/// enforced by CI, and this list is the fixture grammar.
pub const AUTH_FORBIDDEN_FIELDS: &[&str] = &[
    "password",
    "otp",
    "totp_secret",
    "recovery_code",
    "refresh_token",
    "access_token",
    "id_token",
    "authorization",
    "authorization_code",
    "client_secret",
    "code_verifier",
];

fn is_field_boundary(byte: u8) -> bool {
    !(byte.is_ascii_alphanumeric() || byte == b'_')
}

fn field_at(bytes: &[u8], at: usize, name: &str) -> bool {
    let end = at + name.len();
    if end > bytes.len() || !bytes[at..end].eq_ignore_ascii_case(name.as_bytes()) {
        return false;
    }
    let before_ok = at == 0 || is_field_boundary(bytes[at - 1]);
    let after_ok = end == bytes.len()
        || bytes[end] == b'='
        || bytes[end] == b':'
        || (bytes[end] == b'"' && end + 1 < bytes.len() && bytes[end + 1] == b':');
    before_ok && after_ok
}

fn value_end(bytes: &[u8], start: usize) -> usize {
    let mut i = start;
    // Quoted value: consume through the closing quote.
    if i < bytes.len() && bytes[i] == b'"' {
        i += 1;
        while i < bytes.len() && bytes[i] != b'"' {
            i += 1;
        }
        return (i + 1).min(bytes.len());
    }
    // Unquoted value: stop at a structural delimiter.
    while i < bytes.len()
        && !matches!(
            bytes[i],
            b' ' | b'\t' | b'\n' | b'\r' | b'&' | b';' | b'"' | b'\'' | b'}'
        )
    {
        i += 1;
    }
    i
}

fn eol_end(bytes: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < bytes.len() && !matches!(bytes[i], b'\n' | b'\r') {
        i += 1;
    }
    i
}

/// Masks authentication field values in arbitrary bounded text.
///
/// Both explicit structured fields and accidental debug formatting are covered
/// (Phase 3 specification, §40: the difference is architectural, not
/// stylistic). `name=value`, `name: value`, and `"name":"value"` shapes have
/// their value replaced with `REDACTED`; `Bearer ...` credentials are
/// likewise replaced.
pub fn sanitize_authentication_text(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    while i < bytes.len() {
        // Bearer credential masking.
        if bytes[i..].len().ge(&7) && bytes[i..i + 7].eq_ignore_ascii_case(b"Bearer ") {
            out.push_str("Bearer ");
            let end = value_end(bytes, i + 7);
            out.push_str(REDACTED);
            i = end;
            continue;
        }
        let mut replaced = false;
        for name in AUTH_FORBIDDEN_FIELDS {
            if field_at(bytes, i, name) {
                out.push_str(&input[i..i + name.len()]);
                let mut j = i + name.len();
                // Consume the `=`/`:` separator and any spaces/quote after it.
                while j < bytes.len()
                    && (bytes[j] == b':'
                        || bytes[j] == b'='
                        || bytes[j] == b' '
                        || bytes[j] == b'"')
                {
                    out.push(bytes[j] as char);
                    j += 1;
                }
                let value_end_at = if name.eq_ignore_ascii_case("authorization") {
                    eol_end(bytes, j)
                } else {
                    value_end(bytes, j)
                };
                out.push_str(REDACTED);
                i = value_end_at;
                replaced = true;
                break;
            }
        }
        if !replaced {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
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

    // Phase 3 specification, §40/§78: CI must prove authentication credentials
    // never appear in emitted events, covering both structured fields and
    // debug/error formatting shapes.
    #[test]
    fn authentication_fixtures_are_all_masked() {
        let fixtures = [
            (
                "auth header",
                "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.payload",
                "eyJhbGciOiJIUzI1NiJ9",
            ),
            (
                "query form",
                "POST /token refresh_token=r3fr.esH.value ok",
                "r3fr.esH.value",
            ),
            (
                "login body",
                "user submitted password=hunter2!x now",
                "hunter2!x",
            ),
            ("otp field", "challenge otp=123456 pending", "123456"),
            (
                "callback code",
                "callback authorization_code=SplxlOBeZQQYbYS6WxSbIA done",
                "SplxlOBeZQQYbYS6WxSbIA",
            ),
            (
                "client secret",
                "config client_secret=cs_live_9f3aa secret",
                "cs_live_9f3aa",
            ),
            (
                "totp seed",
                "enrollment totp_secret=JBSWY3DPEHPK3PXP issued",
                "JBSWY3DPEHPK3PXP",
            ),
            (
                "recovery",
                "recovery_code=a1b2-c3d4-e5f6 redeemed",
                "a1b2-c3d4-e5f6",
            ),
            (
                "json form",
                r#"{"password":"plain-text-pw","user":"u1"}"#,
                "plain-text-pw",
            ),
            (
                "access token json",
                r#"{"access_token":"at.abcdef","id_token":"idt.xyz"}"#,
                "at.abcdef",
            ),
            (
                "pkce verifier",
                "exchange code_verifier=dBjftJeZ4CVP-mB92K27uhbUJU1p1rwW4jN3B9w",
                "dBjftJeZ4CVP-mB92K27uhbUJU1p1r",
            ),
        ];
        for (label, input, secret) in fixtures {
            let out = sanitize_authentication_text(input);
            assert!(!out.contains(secret), "{label} leaked credential: {out}");
            assert!(out.contains(REDACTED), "{label} was not redacted: {out}");
        }
    }

    #[test]
    fn safe_text_passes_through() {
        assert_eq!(
            sanitize_authentication_text("method=password platform=android"),
            "method=password platform=android"
        );
    }
}
