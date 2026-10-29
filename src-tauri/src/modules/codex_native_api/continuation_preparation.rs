//! A narrow metadata allowlist for local continuation failures, never payload diagnostics.
use std::{sync::LazyLock, time::Duration};

use sha2::{Digest, Sha256};

use super::{
    contract::{ContinuationPreparationRejection, ServiceError},
    diagnostics::DiagnosticContext,
};

// This secret is never persisted or logged. It prevents low-entropy client IDs
// from being recovered with a dictionary and limits correlation to one process.
static FINGERPRINT_SALT: LazyLock<[u8; 32]> = LazyLock::new(rand::random);

fn fingerprint(domain: &[u8], identity: Option<&str>) -> Option<String> {
    identity.map(|identity| {
        let mut hash = Sha256::new();
        hash.update(*FINGERPRINT_SALT);
        hash.update(b"continuation-preparation-v1\0");
        hash.update(domain);
        hash.update(b"\0");
        hash.update(identity.as_bytes());
        format!("{:x}", hash.finalize())
    })
}

fn millis(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

pub(super) fn rejection(
    context: &DiagnosticContext,
    elapsed: Duration,
    error: &ServiceError,
) -> Option<ContinuationPreparationRejection> {
    // Exact internal codes only. Never emit an upstream-supplied message,
    // arbitrary error string, generic routing/quota failure or cancellation.
    let code = match error.code {
        "account_binding_unavailable" => "account_binding_unavailable",
        "continuation_owner_mismatch" => "continuation_owner_mismatch",
        "continuation_owner_unavailable" => "continuation_owner_unavailable",
        "continuation_history_incomplete" => "continuation_history_incomplete",
        "continuation_history_too_large" => "continuation_history_too_large",
        "continuation_not_supported" => "continuation_not_supported",
        "previous_response_not_found" => "previous_response_not_found",
        "upstream_state_caller_mismatch" => "upstream_state_caller_mismatch",
        "upstream_state_owner_changed" => "upstream_state_owner_changed",
        "upstream_state_owner_conflict" => "upstream_state_owner_conflict",
        _ => return None,
    };
    Some(ContinuationPreparationRejection {
        request_id: context.request_id.clone(),
        started_at_ms: context.started_at_ms,
        elapsed_ms: millis(elapsed),
        status: error.status,
        code,
        retry_after_ms: error.retry_after.map(millis),
        thread_fingerprint: fingerprint(b"thread", context.client_thread_id.as_deref()),
        session_fingerprint: fingerprint(b"session", context.client_session_id.as_deref()),
    })
}
