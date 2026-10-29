# Validation of continuation preparation diagnostics

Executed on 2026-10-29 (UTC+08:00), Windows x64, Rust 1.99.1.
Base: `2cfe5e51314bfc0bf70bdda1ba08aed82c7dc8bf` (1.3.75).

## Results

| Suite / binary filter | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| `modules::codex_native_api::lifecycle::tests` | 28 | 0 | 0 |
| `continuation_preparation_http_tests` | 1 | 0 | 0 |
| `multi_result_history_tests` | 2 | 0 | 0 |
| `native_pinned_cooling` | 2 | 0 | 0 |
| **Total targeted tests** | **33** | **0** | **0** |

Nine are new tests: eight lifecycle boundary/privacy tests and one real HTTP
prepare-to-fail-path test with a synthetic Host. The existing tests verify that
no upstream outcomes are fabricated, prior real 429 outcomes remain attributed
correctly, multi-result history pass-through is unchanged, and temporary owner
cooling retains its existing semantics. This is not the full repository test suite.

Actual test output is in [continuation-preflight-409-tests.txt](continuation-preflight-409-tests.txt).
`git diff --check` and `rustfmt --edition 2021 --check` on the three new Rust
files passed. The full library-test compilation succeeded, with 346 compiler
warnings; unrelated warnings were not changed. No release build, Go sidecar
test, GUI automation, or live-account test is claimed.

## How it was run

The library-test binary was built from the final patch using:

```text
cargo test --locked -p cockpit-tools --lib --no-run --message-format=json-render-diagnostics
```

Each filter in the table was run against that binary with `--test-threads=1`.
The corresponding standard Cargo invocation is:

```text
cargo test --locked -p cockpit-tools --lib <filter> -- --test-threads=1
```

On this Windows host, a byte-identical copy of the Cargo-produced test executable
was run with an adjacent Common Controls v6 manifest; no executable patch or
system DLL change was performed. The final compiled test executable SHA256 was:

```text
4BB4BDA5312F196ADFC538980F55F46BD571DC3C621B886B386073647318A449
```

The existing installed Go sidecar was copied into the build resource location
solely to satisfy the build script, with `COCKPIT_SKIP_CLIPROXY_BUILD=1`; it was
not run, rebuilt, or deployed. Its build-resource SHA256 was:

```text
2AA9FCC57E652BECEDBCDB75E360768159FA35F122BFBC4F2DF9147D993924ED
```

Cargo caches, target output, test data and temporary files were confined to a
dedicated task directory. `COCKPIT_TOOLS_TEST_DATA_DIR`, `COCKPIT_TEST_DATA_DIR`,
`COCKPIT_TOOLS_DATA_DIR`, `TEMP` and `TMP` were directed to isolated test paths.
Live-test opt-in variables were unset. Synthetic upstreams bound only to
`127.0.0.1:0` and the test HTTP clients disabled proxies.

## What these tests do not prove

- They do not reconstruct the missing production failed-request body.
- They do not identify why the original account was ineligible at the time.
- They do not prove cross-account replay compatibility of multi-result or
  encrypted histories.
- They do not establish that quota recovery or compaction alone caused recovery.
- They do not prove that every future 409 is fixed. The patch adds local
  evidence for a narrow class of future preparation failures.
