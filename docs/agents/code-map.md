# Agent code map

Use this map to find a change's entry point, connected boundaries and existing tests. It describes implemented code through the completed local scope of issue #4; provider integration, paper/live trading and release qualification remain future work. Component contracts hold detailed behavior; this map is a navigation aid.

## Start with the task

1. Read root [AGENTS.md](../../AGENTS.md), the [product specification](../../METTEN_DESIGN_DRAFT_2026-11-12.md) and the issue's acceptance criteria. For engine/desktop changes, read [ADR 0001](../adr/0001-rust-engine-and-tauri-boundaries.md). For issue #4 behavior, start at its [current evidence index](../verification/issue-4.md#current-evidence-index), which identifies the operative design and latest corrections.
2. Pick a route below. Read the named source and owning contract, then trace callers and the relevant tests before editing. The tables locate candidate checks; they do not authorize native, credential or trading operations.
3. Use [verification guidance](verification.md) for bounded reads, historical evidence and staged checkpoints; [CONTRIBUTING.md](../../CONTRIBUTING.md#set-up-and-check-changes) lists the current check commands. Reuse unchanged evidence within its recorded limits.

## Find code and tests

Paths are relative to the repository root. Names are search landmarks, not a second API specification.

| Task | Source and landmarks | Tests and owning contract |
| --- | --- | --- |
| Session controls, recording failure, stopped reopen | [lib.rs](../../crates/engine/src/lib.rs): `Engine`, `request`, `snapshot`, `stop_and_save`; [storage.rs](../../crates/engine/src/storage.rs): `Recorder`, `append`, `latest` | [session.rs](../../crates/engine/tests/session.rs), embedded `lib.rs::tests`; [engine source contract](../../crates/engine/src/README.md) |
| Strategy JSON, validation, explanation or import errors | [strategy.rs](../../crates/engine/src/strategy.rs): `ValidatedStrategy`, `parse`, `validate`, `rule_error`, `explain` | [strategy.rs tests](../../crates/engine/tests/strategy.rs); [format reference](../strategies/reference.md), [engine source contract](../../crates/engine/src/README.md) |
| Strategy authoring guides or teaching example | [Strategy guides](../strategies/README.md), shared format reference above and [learning.json](../../examples/learning.json) | Public strategy tests above; strategy guides own the human/agent authoring instructions and example |
| Controlled strategy evaluation, history or resource limits | [strategy.rs](../../crates/engine/src/strategy.rs): `Evaluator`, `begin`, `process_slice`, `snapshot`; use the internal landmarks below | Public strategy tests above and embedded `controlled_failure_tests`; [issue #4 evidence index](../verification/issue-4.md#current-evidence-index) links the processing/memory proofs and separate release-hour fixtures |
| Saved strategy versions, schema migration or compatibility | [storage.rs](../../crates/engine/src/storage.rs): `Recorder::open`, `save_strategy`, `strategy_versions`, `strategy_detail`; [lib.rs](../../crates/engine/src/lib.rs): `Engine::strategies` | [strategy_storage.rs](../../crates/engine/tests/strategy_storage.rs); [engine source contract](../../crates/engine/src/README.md) |
| Request/response shape or sanitized strategy failures | [lib.rs](../../crates/engine/src/lib.rs): `decode_strategy_request`, `StrategyRequest`, `StrategyResponse`, `StrategyFailure`; [native main.rs](../../src-tauri/src/main.rs): `decode_strategies`, `strategies`; [contract.ts](../../src/contract.ts) | Strategy storage tests above, native `permissions_tests` and [strategy.test.mjs](../../tests/strategy.test.mjs); [desktop source contract](../../src-tauri/src/README.md), [interface contract](../../src/README.md) |
| Desktop startup, command admission or Quit | [native main.rs](../../src-tauri/src/main.rs): `Workspace`, `StrategyAdmission`, `admit_strategy`, `begin_quit`, `main` | Embedded `permissions_tests`; [desktop source contract](../../src-tauri/src/README.md), bounded packaged records under [verification](../README.md#interface) |
| Dummy credential isolation or operation handling | [credential.rs](../../src-tauri/src/credential.rs): `Credentials`, `Bootstrap`; native `decode_credential` and `credential`; UI credential handlers in [main.ts](../../src/main.ts) | Embedded credential/native tests and [credential.test.mjs](../../tests/credential.test.mjs); [desktop source contract](../../src-tauri/src/README.md), [issue #3 evidence](../verification/issue-3.md) |
| Screen refresh, controls, strategy editor or failure wording | [main.ts](../../src/main.ts): `refresh`, `control`, `changedText`, `loadStrategies`, `inspectStrategy`; [view.ts](../../src/view.ts): `presentation`, `strategyFailure`; [contract.ts](../../src/contract.ts) defines the snapshot type; callers use the checked helper implementation | [view.test.mjs](../../tests/view.test.mjs), [refresh.test.mjs](../../tests/refresh.test.mjs), strategy/credential interface tests above; [interface contract](../../src/README.md), [test boundaries](../../tests/README.md) |
| Layout, labels or frontend assets | [index.html](../../index.html), [style.css](../../src/style.css); [assets contract](../../assets/README.md) and [brand contract](../../assets/brand/README.md) locate maintained asset sources/exports | [Interface contract](../../src/README.md); VM checks do not render layout or establish accessibility. Desktop icons belong to the desktop package below |
| Desktop permissions, command registration or packaging | [build.rs](../../src-tauri/build.rs), native `main` registration, [main capability](../../src-tauri/capabilities/main.json), maintained command TOMLs in `src-tauri/permissions/autogenerated/`, [tauri.conf.json](../../src-tauri/tauri.conf.json), [desktop manifest](../../src-tauri/Cargo.toml) | Native `permissions_tests`; [desktop package contract](../../src-tauri/README.md) owns permissions, generated artifacts, icons and build boundaries |
| Verification tooling or CI | [check-staged.py](../../scripts/check-staged.py), [verify-credential.swift](../../scripts/verify-credential.swift), [checks.yml](../../.github/workflows/checks.yml) | [check-staged.test.py](../../tests/check-staged.test.py), [credential-runner.test.py](../../tests/credential-runner.test.py); [tool contract](../../scripts/README.md), [verification guidance](verification.md) |
| Dependencies, compiler or frontend build configuration | Root [package.json](../../package.json), [package-lock.json](../../package-lock.json), [Cargo.toml](../../Cargo.toml), [Cargo.lock](../../Cargo.lock), [rust-toolchain.toml](../../rust-toolchain.toml), [tsconfig.json](../../tsconfig.json); [engine manifest](../../crates/engine/Cargo.toml), [vite-env.d.ts](../../src/vite-env.d.ts) | [Repository setup](../../README.md), [engine package contract](../../crates/engine/README.md), desktop package/CI above. Toolchain changes also require the resource-proof review described in verification guidance |

## Follow connected boundaries

- **Preview/save/list/detail:** screen handlers → native `strategies` admission/decoding → engine strategy API → validator or recorder → typed response and literal screen rendering. A protocol change reaches Rust types, TypeScript types, callers and boundary tests. The controlled evaluator is separate from this desktop workflow.
- **Start/Pause/Advance:** screen `control` → native `control` → engine request/recorder → `snapshot` → `refresh`/`presentation`. Request acceptance and confirmed recording are separate states.
- **Quit:** native `begin_quit` waits for admitted strategy work, then credential operations, then asks the engine to stop and save. Use the desktop contract for lock/order and failure behavior.

## Navigate the large strategy module

Search for these symbols rather than relying on cached line numbers. Read bounded ranges around matches and follow the called helpers.

| Concern | Landmarks in `crates/engine/src/strategy.rs` |
| --- | --- |
| Input grammar, decimals and validation | `Document`, `Decimal`, `StrictJson`, `ValidatedStrategy`, `resolve_measurement`, `validate_option` |
| Explanation and validation-time history costs | `explain`, `Requirements`, `history_bytes` |
| Controlled evidence and lifecycle | `RunContext`, `AdvanceBatch`, `EvidenceChange`, `Evaluator`, `SliceControl`, `SliceResult` |
| History and incremental calculation | `History`, `Series`, `Pending` |
| Runtime memory ownership and admission | `HeapBytes`, `compilation_bytes`, `register_to`, `allocate_new`, `series_bytes` |
| Work accounting and independent resource probes | `fixed_work`, `tree_work`, `heap_probe`, `controlled_failure_tests` |

For example, `rg -n 'impl ValidatedStrategy|impl Evaluator|fn history_bytes|mod controlled_failure_tests' crates/engine/src/strategy.rs` finds the main sections. Use the evidence index for proof assumptions instead of inferring a universal limit from a passing fixture.

## Keep navigation current

When moving an entry point or adding a maintained boundary, update its route here and its owning contract. The [EGM coverage map](../egm/README.md#interface) checks inventory against ownership; the [documentation/storage map](../README.md#where-files-belong) decides where documents belong. Private reports and temporary notes use the locations defined there and in [SECURITY.md](../../SECURITY.md); [.gitignore](../../.gitignore) excludes the entire root `private-security/` directory.
