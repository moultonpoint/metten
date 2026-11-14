# Rust library crates

## Overview

This subtree currently contains the `metten-engine` Rust library. The [workspace manifest](../Cargo.toml) also includes `src-tauri` outside this directory. [ADR 0001](../docs/adr/0001-rust-engine-and-tauri-boundaries.md) assigns authoritative behavior to the independent engine and leaves the desktop interface to request actions and display state.

This map covers the non-trading foundation and completed issue #4 controlled rules and local strategy versions. The engine contract links their source boundaries and historical evidence. The [product specification](../METTEN_DESIGN_DRAFT_2026-11-12.md) describes the intended finished application, not capabilities already supplied by this subtree.

## Interface

| Subdirectory | Boundary |
| --- | --- |
| [engine](./engine/README.md) | `metten-engine` exposes `Engine::{open, request, snapshot, stop_and_save}` and typed session controls/state, plus `Engine::strategies` and controlled `ValidatedStrategy`/`Evaluator` interfaces. [Source details](./engine/src/README.md) cover session and immutable strategy-version persistence. |

## Invariants

- `Engine::open` starts stopped and preserves saved session history and the controlled clock across reopen.
- `Engine::request` queues work; `Snapshot` separates effective control from confirmed history. Pause remains effective while earlier recording completes.
- `Recorder::open` enforces one cooperating workspace owner and verifies SQLite writer settings. Failed recording blocks further Start/Advance rather than reporting unconfirmed changes as saved.
- The foundation library is independent of Tauri, credentials and exchange transport. The [implementation plan](../docs/implementation-plan.md) tracks later trading capabilities separately.

## Side Effects

- `Engine` starts a recorder thread and uses a bounded queue and shared snapshot state.
- `Recorder` creates/opens a local SQLite database and lock file and appends `session_events` and immutable `strategy_versions`; SQLite manages auxiliary write files. The inspected foundation performs no network or financial operations.

## Verification

From the repository root, use the existing foundation checks:

```sh
cargo test -p metten-engine --test session --locked
cargo test -p metten-engine --lib pause_stays_effective_after_start_saves_before_pause_saves --locked
```

See the [engine verification map](./engine/README.md) and historical [issue #2 evidence](../docs/verification/issue-2.md). See also the [issue #4 evidence index](../docs/verification/issue-4.md#current-evidence-index) for controlled rule/version checks. No tests or builds were run for this documentation-only update; focused session targets alone do not establish strategy acceptance.
