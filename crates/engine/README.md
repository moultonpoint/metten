# Metten engine crate

## Overview

`metten-engine` is the Rust library for a controlled, persisted non-trading session. It provides desktop-independent controls and saved history as the first implementation slice of the [Gen 1 specification](../../METTEN_DESIGN_DRAFT_2026-11-12.md). Future product behavior belongs to the [implementation sequence](../../docs/implementation-plan.md); it must not be inferred from this foundation.

This contract also covers completed issue #4: controlled rule validation/evaluation and local immutable versions. The [shared reference](../../docs/strategies/reference.md) defines format-2 meanings; [issue #4 evidence](../../docs/verification/issue-4.md#current-evidence-index) records bounded local checks. The evaluator is separate from desktop session execution.

## Interface

- [Source contract](./src/README.md): `Engine::{open, request, snapshot, stop_and_save}`, `Action::{Start, Pause, Advance}`, `Status::{Stopped, Running, Paused}`, `Snapshot`, `Record`, and `WriterSettings`, plus `ValidatedStrategy`, `Evaluator`, strategy request/response types and version operations.
- [Cargo manifest](./Cargo.toml): library package `metten-engine`, version `0.1.0`, Rust edition 2024, minimum Rust `1.100`. Runtime dependencies are `rusqlite` with bundled SQLite, `serde` and `serde_json`; `tempfile` supports tests.
- [Session tests](./tests/session.rs) exercise the public library against temporary real SQLite files. [Strategy tests](./tests/strategy.rs) cover the public controlled rule boundary; [storage tests](./tests/strategy_storage.rs) cover immutable versions, compatibility, migration and request limits.

## Invariants

- `Engine::open` returns stopped with a committed opening record; reopening preserves the controlled clock and saved history without automatically resuming.
- `Engine::request` acceptance is not save confirmation. `Snapshot.running` is effective control state; `Snapshot.history` contains only the last successfully read committed rows, newest first.
- `Engine::enqueue` stops effective control immediately on Pause/Stop, rejects invalid transitions, and blocks Start/Advance during pending or failed recording. A generation guard preserves a later pause through an earlier Start completion.
- `Recorder::open` requires one cooperating workspace owner and verified SQLite writer settings. `Recorder::append` commits session records transactionally; recording errors stop control and retain the last confirmed history.
- The dependency boundary in [ADR 0001](../../docs/adr/0001-rust-engine-and-tauri-boundaries.md) remains intact for this foundation: the engine has no Tauri or provider dependency. Session events are not paper fills, live fills or financial truth.

## Side Effects

- Opening creates/opens a local SQLite workspace and lock file; SQLite may create auxiliary files. `Recorder` reads/writes `session_events` and appends immutable `strategy_versions`, whose contract is specified under [source](./src/README.md).
- The engine owns a recorder thread and bounded control queue. Controls append records; `stop_and_save` waits for durable recording confirmation, while Drop only closes/joins the worker.
- The foundation makes no network calls and handles no credentials or money. Pure rule evaluation uses controlled in-memory evidence; preview writes no version, while save appends a revalidated version.

## Verification

Run from the repository root:

```sh
cargo test -p metten-engine --locked
cargo test -p metten-engine --release --locked --test strategy -- --ignored --nocapture
```

The [source contract](./src/README.md) maps checks to behavior. [Issue #2 evidence](../../docs/verification/issue-2.md) records prior local tests and packaged-app checks, with their distribution and physical power-loss limits. Its pending-CI and review dispositions describe that historical checkpoint, not current PR status. No tests/builds were rerun for this documentation change. The issue #4 record distinguishes controlled rules, local packaged import proof and later unproven provider/trading integration.
