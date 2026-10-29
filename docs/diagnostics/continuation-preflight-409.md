# Native continuation 409: sanitized incident and diagnostic gap

## Scope

This report accompanies a **diagnostics-only change**, not a claim that the
production continuation failure has been permanently fixed. All reproduction
inputs are synthetic. No user conversation, image, encrypted reasoning, account
credential, database, original log, or actual client identifier is included.

The affected installation ran Cockpit Tools **1.3.75**, native engine, on Windows
x64. The inspected release and upstream `main` resolved to
`2cfe5e51314bfc0bf70bdda1ba08aed82c7dc8bf` on 2026-10-29.

Related symptom: #2844 (reported against 1.3.70). That report includes an earlier
429, whereas **this incident does not establish a preceding 429**. This PR must
not close that issue as a confirmed root-cause fix. #2899 addresses different
history-compatibility cases; this change does not relax those checks.

## Observed timeline

All times below use UTC+08:00. Identifiers are deliberately replaced by labels.

| Time | Evidence | Result |
| --- | --- | --- |
| 2026-10-27 13:06:38 | Latest successful request before the reported failures | HTTP 200 using owner A |
| 2026-10-28 02:40:34–02:40:57 | Client turn metadata | Failed with the exact local 409 below |
| 2026-10-29 16:34:15–16:34:37 | Client turn metadata | Same 409; retries did not recover the turn |
| 2026-10-29 approximately 17:55–18:00 | Later read-only configuration/UI snapshot | A remained in the service pool and caller scope, was not paused or model-excluded; displayed quota was 100% for 5h and 53% for the weekly window |
| 2026-10-29 18:13:56–18:15:36 | Successful client turn metadata | Item types included context compaction, a user message, and an assistant message |
| 2026-10-29 18:15:30.737 and 18:15:36.978 | Two target-thread request records | HTTP 200, one attempt each, still using owner A |
| After the minimal retry | User confirmation | The original conversation worked again |

Exact error text, with the machine-specific endpoint replaced:

```text
unexpected status 409 Conflict:
The connection's account is no longer available; reconnect with full input,
url: http://127.0.0.1:<local-port>/v1/responses
```

The diagnostic work did not perform an application restart, binary replacement,
configuration change, account exclusion, index repair, conversation edit, or
identity database edit. Recovery must not be attributed to a fix performed by
the diagnostic work.
The later availability of A and the client's compaction are observations, not
proof of which event caused recovery.

## Evidence recovered and its limits

Read-only queries were scoped to the affected thread before returning metadata.
The request diagnostics initially contained 94 successful requests for that
thread, but no matching records for the two failed client turns. The later two
successful requests were independently found and matched to the same owner.

The exact API and application log files for 2026-10-28 and 2026-10-29 were checked
locally for the affected thread/turn identifiers and the precise binding error.
There were no matches. Unrelated log entries were not exported.

The **last successful pre-failure checkpoint**, not the failing request payload, had:

| Structural check | Result |
| --- | --- |
| Ordered items | 371 |
| Decoded item bytes | 81,660,353 |
| Item integrity checks | 371/371 passed |
| Repeated call IDs | 0 |
| Additional results for an already-seen call ID | 28 |
| Results without a matching preceding call/type | 0 |
| Calls missing a result | 0 |
| Standalone recognized host results | 4 |
| Items containing encrypted content | 75 |

The archive existed; absence from the hot cache is expected for this size
(8 MiB per hot checkpoint, 64 MiB total hot budget). It is not evidence of lost
history. The statistics do **not** prove that the failing request contained the
same items or that repeated results were its only rebuild blocker.

Payload logging was disabled. Enabling it afterwards cannot reconstruct a
request that was never retained, and it was not enabled during this audit.
There is no complete failed-request capture to upload.

## Confirmed code paths in the inspected revision

1. `codex_local_access_native_routes.rs`: `account_binding_unavailable` is a
   local selection failure when a hard continuation owner is absent from the
   eligible candidates but other candidates remain. An account can be filtered
   for several reasons; the error alone does not identify which reason applied.
2. `codex_native_api/mapping.rs`: this error can trigger cross-account rebuilding
   only when a safe rebuild plan exists. Otherwise it returns before dispatch.
3. `identity_rebuild.rs::paired_tools`: a second result with the same call ID
   fails the strict replay check. Existing `multi_result_history_tests.rs`
   explicitly separates same-owner pass-through from proof of safe replay.
4. `http.rs`: a failed `mapping::prepare` calls `tracker.fail` before routing
   metadata or an upstream attempt is committed.
5. `lifecycle.rs`: zero-dispatch failures skip `attempt_error`, discard ordinary
   diagnostics in `finish_diagnostic`, and are gated out of persistence. Existing
   dispatch tests intentionally protect the absence of upstream usage/health
   outcomes for requests that were never sent.

The last point explains a reproducible **observability gap**, not a unique
production routing root cause. It also explains why searching the request
database cannot be expected to recover a complete record of this local 409.

## Change proposed here

Add a separate, small continuation-preparation rejection event to the existing
local API log. Keep it separate from `RequestReport`, ordinary payload
diagnostics, quota accounting, account health, and routing feedback.

The event is limited to authorized, zero-dispatch, explicitly classified
continuation failures. It contains machine-generated correlation metadata and
bounded error categories, not request/response bodies or raw identities.
Session/thread correlation uses process-scoped salted fingerprints; they are
not portable identifiers and cannot correlate across application restarts.

The patch deliberately does **not**:

- disable history validation, remove/merge tool results, or strip encrypted data;
- reassign an old response to a different owner;
- bypass real quota/cooling/authorization limits;
- create an upstream attempt, usage record, or account-health penalty;
- change the externally visible 409 or claim automatic failover is now fixed;
- enable payload capture or upload any diagnostic data.

## Remaining unknowns

- Why owner A was filtered at each failure instant.
- The exact body/continuation anchors of those failed requests.
- Which condition prevented a safe rebuild plan on each attempt.
- Whether quota recovery, changed input after compaction, or another state
  transition enabled the later successful continuation.

Future work can add precise eligibility/rebuild reason codes without exporting
history. Such work must preserve ownership and replay safety. This report does
not substitute the earlier successful checkpoint for the missing failed input.

## Reproduction and verification

The accompanying Rust tests use synthetic callers, identities, messages and
loopback services with temporary storage. A synthetic Host returns a controlled
409 through the real HTTP prepare-to-fail path; this verifies metadata-only
observation and the absence of fabricated upstream outcomes, not the production
account-filtering cause. See the PR's validation results for the exact commands and executed
test counts. No real user history was replayed and no live model call was made.
