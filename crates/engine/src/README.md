# Engine session and recording

## Overview

This directory implements the controlled, non-trading foundation session. It has a manually advanced test clock and durable session history; it does not implement the product's market clock, purchases, accounting or recovery of exchange orders. The architectural boundary follows [ADR 0001](../../../docs/adr/0001-rust-engine-and-tauri-boundaries.md).

Scope includes completed issue #4 controlled rules and local strategy versions. The [shared reference](../../../docs/strategies/reference.md) owns the exact format-2 grammar and meanings; the [operative design](../../../docs/proposals/issue-4-egm/strategy-rules-design.md) and [evidence index](../../../docs/verification/issue-4.md#current-evidence-index) define the controlled developer boundary and its version-bounded proof.

## Interface

| Boundary | Contract |
| --- | --- |
| [lib.rs](./lib.rs): `Engine::open(path)` | Opens the caller-selected local nonsecret workspace, reads prior history, preserves its latest tick, and commits a stopped opening record before returning an engine. Returns `Result<Engine, String>`. |
| `Engine::request(Action)` | Queues `Start`, `Pause` or `Advance`. `Ok(())` confirms acceptance into the queue, not saving. Read `Snapshot.pending`, `recording_error` and `history` for the result. |
| `Engine::snapshot()` | Clones the current `Snapshot`: effective `running`, pending request count, recording error, committed history and effective writer settings. |
| `Engine::stop_and_save()` | Stops effective control immediately, then blocks for the saved stopped boundary. Returns a visible error on failure and permits retry. Call away from the desktop event thread. |
| `Action`, `Status` | `Action` deserializes/serializes only `start`, `pause`, `advance`; `Status` serializes `stopped`, `running`, `paused`. No arbitrary SQL, paths, shell or order action is accepted by `Action`. |
| `Record`, `WriterSettings` | `Record` exposes `id`, `status`, `tick`, `reason`, `recorded_at`; `WriterSettings` exposes `journal_mode`, `synchronous`, `fullfsync`, `checkpoint_fullfsync`, `sqlite_version`. These and `Snapshot` serialize for callers. |
| [storage.rs](./storage.rs): `Recorder::{open, latest, append}` | Crate-private SQLite adapter. `open` returns a recorder and effective settings; `latest` returns newest committed session rows; `append` commits a session record and reads history back. It is not a public desktop API. |
| `lib.rs`: `decode_strategy_request`, `StrategyRequest`, `StrategyResponse`, `StrategyFailure` | Strict preview/save/list/detail protocol with bounded text and positive decimal-string IPC IDs; finite errors do not echo submitted content. Public Rust requests use native integer IDs. |
| `Engine::strategies`, `strategies_unless_quitting` | Preview validates without writing; save requires exact reviewed/current bytes and revalidates. The shared recorder mutex serializes version operations and session recording; save rechecks quitting under storage admission. Responses distinguish rule errors, confirmed save, paged summaries and detail. |
| [strategy.rs](./strategy.rs): `ValidatedStrategy`, `Evaluator` | Validation produces exact JSON, generated explanation and requirements. `new`, `begin`, `process_slice` and borrowed `snapshot` expose controlled evaluation only, without providers or desktop session execution. |
| `storage.rs`: `save_strategy`, `strategy_versions`, `strategy_detail` | Append complete immutable metadata; page newest-first summaries within 50 rows/32 KiB; detail preserves legacy/unsupported content with explicit status and a 1 MiB response limit. |

## Invariants

- `Engine::open` always starts stopped, including after a previously recorded running state. Its controlled clock resumes the latest saved tick, not automatic running activity.
- `Engine::enqueue` rejects duplicate Start, Advance while not running, and Start/Advance while a write is pending. `mpsc::sync_channel(8)` bounds requests; `try_send` refuses an unavailable/full queue.
- `Engine::enqueue` makes Pause and Stop effective before recording. `Shared.generation` prevents an older Start completion from undoing a later pause. Effective control can therefore be stopped while the newest saved `Record.status` is still running.
- The recorder worker publishes refreshed history only after `Recorder::append` commits and reads it back. A recording error sets `running = false`, retains the last confirmed snapshot history and blocks further control requests; Stop remains available to retry saving. A failed operation is not proof that nothing reached disk.
- `Recorder::open` holds `File::try_lock` on `path.with_extension("lock")` for its lifetime, preventing another cooperating engine from opening the same workspace. It uses a 250 ms SQLite busy timeout.
- `Recorder::open` sets and reads back WAL, `synchronous = 2` (FULL), `fullfsync = 1` and `checkpoint_fullfsync = 1`; mismatches refuse opening. Storage revision 3 migrates revisions 0/1/2 atomically under the workspace lock, preserving prior rows with nullable metadata; newer revisions refuse opening.
- Foundation table `session_events` is `STRICT`: `id INTEGER PRIMARY KEY`, `status TEXT` restricted to `stopped/running/paused`, nonnegative integer `tick`, required text `reason`, and a UTC `recorded_at` default from SQLite `strftime`. `Recorder::latest` orders by descending `id` and limits the returned view to 100 rows; it does not delete older rows.
- `Action::Advance` adds exactly one controlled second with checked arithmetic; overflow becomes a recording error rather than a wrapped tick. This is not the product's next-full-market start behavior.

- New strategy rows retain exact JSON, format 2, meaning `metten_rules_2`, generated explanation, display name and UTC time. No-update/no-delete and complete-insert guards protect versions; legacy explanations are never reconstructed as historical truth.
- The controlled evaluator uses six-place decimals and a one-millisecond calculation clock. The support profile bounds graph/history and total planned memory; concrete registration and owned ingress are admitted before use. Each processing slice is bounded to 4096 algorithmic record primitives under the recorded proof convention, not a wall-clock latency guarantee. Only Complete supplies a current candidate; Pending supplies none, pre-mutation rejection returns idle, and Stop/internal failure are terminal. See the linked design/evidence for ownership and caller restrictions.

## Side Effects

- `Recorder::open` creates/opens a local lock file and SQLite database, configures SQLite, and initializes the foundation `session_events` table inside `BEGIN IMMEDIATE`. SQLite manages WAL auxiliary files. The same transaction adds strategy metadata/guards and sets revision 3 only on commit.
- `Engine::open`, accepted controls and successful `stop_and_save` append session events. `Recorder::append` uses a transaction with bound SQL parameters, then reads committed history. Foundation code has no history deletion path.
- `Engine::open` starts the `metten-recorder` thread; control requests cross a bounded channel and share snapshot state through a mutex. `Engine::drop` closes the sender and joins that worker; Drop does not promise a saved stopped record.
- `stop_and_save` can wait on the recorder; failure reports that Quit was canceled. On a successful stop the worker exits and later controls are rejected.
- The inspected foundation performs no network, credential, exchange or real-money operations. Filesystem settings and process-restart tests are not proof of physical power-loss durability.

## Verification

From the repository root, the existing focused checks are:

```sh
cargo test -p metten-engine --test session --locked
cargo test -p metten-engine --lib pause_stays_effective_after_start_saves_before_pause_saves --locked
```

[Session tests](../tests/session.rs) cover public controls, real SQLite lock failure, single workspace ownership, stopped reopen, child-process exit recovery, action deserialization, and failed quit/retry. `lib.rs::tests::pause_stays_effective_after_start_saves_before_pause_saves` controls the precise Start/Pause recording race with a test-only barrier.

Strategy checks are `cargo test -p metten-engine --locked` and the separately run release hour fixtures, `cargo test -p metten-engine --release --locked --test strategy -- --ignored --nocapture`. Public [strategy tests](../tests/strategy.rs) cover rule behavior; [storage tests](../tests/strategy_storage.rs) cover migration, preserved records, bounded protocol and write failures. Embedded strategy tests check resource accounting, continuation/Stop and controlled failure paths. No tests or builds were run for this documentation-only change. [Issue #2 verification](../../../docs/verification/issue-2.md) records earlier local results and their limits; it is historical evidence, not a fresh result for all current files.
