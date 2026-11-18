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

Strategy checks are `cargo test -p metten-engine --locked` and the separately run release hour fixtures, `cargo test -p metten-engine --release --locked --test strategy -- --ignored --nocapture`. Public [strategy tests](../tests/strategy.rs) cover rule behavior; [storage tests](../tests/strategy_storage.rs) cover migration, preserved records, bounded protocol and write failures. Embedded strategy tests check resource accounting, continuation/Stop and controlled failure paths. [Issue #2 verification](../../../docs/verification/issue-2.md) records earlier local results and their limits; it is historical evidence, not a fresh result for all current files. Current source/evidence changes and actual checks are indexed in [issue #5](../../../docs/verification/issue-5.md#current-evidence-index).

## Synchronous source evidence

[sources.rs](sources.rs) owns received source facts only. It performs no networking, credential access, provider decoding, book reconstruction, persistence, strategy arithmetic or order submission. The existing evaluator still owns historical cells and strategy state. Foundation Start/Advance is not connected to these sources. [Source profiles](../../../docs/sources/README.md) retain U1–U7; controlled leases/coverage never establish provider support.

### Interface and ownership

- `ValidatedStrategy::source_requirements()` derives declared inputs, selected-side selectors, startup history and missing implicit asks from the validated strategy. One structured implicit-ask definition in strategy.rs supplies its meaning; generated explanation bytes remain unchanged. The source constructor borrows that strategy and retains only requirements/descriptors, not another compiled graph.
- `SourceState::new` accepts at most two exact market descriptors, 128 concrete bindings and per-binding trusted capability facts. Both owners use the same crate-private descriptor/scope checks. Requirement records keep selected_side; registered binding descriptors resolve concrete YES/NO. Readiness requires the required sides and applicable history. Source registration is not evaluator memory/catalog admission. Constructor market-count and borrowed identity checks precede any identity copy; rejected oversized input receives no proportional allocation.
- `apply(Envelope)` accepts one controlled-clock event. Connection/discovery/metadata/update work carries a token captured when that work begins. Never reacquire a token to legitimize a late completion. Kalshi connection events use the Book token; Coinbase/Kraken use their matching stream tokens. Metadata/discovery use Metadata. Acknowledgment creates no observation or validity. Defaults leave current validity, observations and history unavailable; incoming events cannot enable capabilities.
- `SourceEffect.batch` contains existing evaluator evidence. One coherent update stays one transaction across every binding, including value/validity pairs. Initialization and unsupported-observation mappings use Resynchronize; genuine monotone new numbers can produce Observation, including equal prices. Replay adds no observation; conflicting identity/value invalidates the stream. Provider-specific identity/deduplication remains an adapter responsibility.
- `snapshot(now)` derives a complete local view, with values/age, capability limitations, history and market preparation separate. Backward time returns invalid_request; an over-cap view returns resource_limit with no partial result. It makes no provider request or calculation. The conservative 256-KiB construction charge includes records, labels/scalars, doubled escaping allowance for text and current/next metadata. A never-received value has the distinct awaiting_data reason; expired received data reports freshness loss, and explicit access refusal remains separate. Capability limitations remain separate facts. A later native wire projection must preserve the approved decimal-string counter contract; no new IPC command is implemented here.
- `stamp(binding_ids)` captures all dependencies supplied by the controlled consumer. An effect's stamp covers its touched bindings, not an arbitrary strategy result: consumers must capture the complete decision dependency set themselves. `is_current` rejects another owner, changed binding/stream generation, metadata revision, retirement/rebinding or Stop. It does not certify freshness or extract production per-decision dependencies.

### Timing, invalidity and recovery

Clock fields are monotonic milliseconds since Go. The consumer supplies the last completed evaluator frontier, receipt time, accepted time and received-through watermark. Receipt <= accepted <= received-through; accepted cannot precede the completed frontier or the last accepted source time. Transactions retain increasing local numbers and nondecreasing time; provider UTC is provenance, never a cross-provider ordering clock.

`invalidate(scope, reason, at, known_gap)` revokes generations immediately, clears current/history readiness, and keeps last display values labeled unusable. Shared book loss affects both current/next books and successor readiness. Logical-source failure affects its bindings; Kalshi connection loss includes BRTI. Revoking metadata also revokes all retained market preparation and its dependent book/trade generations, even for ask-only strategies with no reference binding. Active-window effects carry Admission::Blocked("unverified_metadata") until metadata is reverified, so a new book snapshot alone cannot restore a candidate. Underlying streams outside that loss scope retain their own evidence. Only an explicit substantiated gap interval produces Gap; unknown loss creates no guessed coverage. Known late contradictions go into a new batch, where the evaluator's per-batch initial-frontier scan invalidates retained dependent history. Fresh initialization restores present state, not the missing cells. An invalid_value or inconsistent_evidence apply error has already revoked source generations, but supplies no engine batch. It blocks all ordinary apply calls until the consumer explicitly calls invalidate for the failed stream and delivers that returned revocation to the evaluator; fresh initialization follows only after that delivery completes. If revocation cannot be admitted, keep the consumer blocked or stop it. Lease-only renewal after source loss returns invalid_request; a healthy substantiated renewal without loss is accepted and adds no observation.

An already-ready market retains its preparation time when a validated coverage-only
update extends its existing proof through overlapping/adjacent intervals. Readiness
is still checked at the new clock, including current-value expiry. New readiness,
disjoint coverage, source loss and expired values cannot backdate preparation. The
public `continuous_coverage_preserves_prior_readiness_without_waiving_new_or_expired_readiness`
regression covers all four cases through SourceState and Evaluator.

Source receipt and evaluator completion are separate. The controlled consumer owns one effect/batch, calls begin, and drains process_slice before admitting more ordinary evidence. begin checks private metadata plus ingress against the existing 8-MiB/4096 allowance. Rejection requires invalidating touched source claims and delivering applicable revocations before initialization/recovery; never send revocations for refused/unregistered bindings. Invalidation during Pending makes its result obsolete before continuation. Unexpected Busy is protocol misuse: stop source owner and evaluator, mark pending/refused results unusable, and replace both only through explicit recovery. Stop/failure retains cancelled evaluator ownership until destruction, preserving sliced cleanup. The engine owns no transport coordinator; native read-only source intake remains
separate from a production financial evaluator.

Stream/binding **generations** revoke old work. A binding **incarnation** distinguishes reused numeric IDs after retirement; a process-local checked **owner identity** distinguishes new source owners. A **stamp** records the dependency generations/incarnations and relevant metadata revision. A **lease** is substantiated current validity; **capabilities** are separately trusted permission to interpret current/observation/history evidence. A **coverage frontier** summarizes contiguous received proof for readiness, merging only overlapping/adjacent intervals in either direction and retaining the newest interval when disjoint; it is not a second historical calculation store. These are module mechanics; domain terms remain in the root glossary.

### Atomic evaluator evidence

A registered binding may have one Observation OR Resynchronize plus one Validity in the same transaction. Duplicate values, duplicate validities, duplicate gaps, observation+resynchronization and gap+another change to the same binding reject before mutation with the existing `unregistered_binding` duplicate-member code. A gap on one binding may coexist with another binding's update. All members apply before conditions. An invalid/expired paired validity clears the paired retained value before conditions in either member order, preserving observation high-water marks; later lease/replay cannot revive it.

Standalone evaluator Validity retains its established behavior: it changes eligibility without clearing raw value; renewing it never erases an invalid historical cell or certifies coverage. The stronger source-loss initialization policy belongs to SourceState. after_checkpoint_update remains evidence of a later accepted observation identity, separately from current price/ask freshness. Historical [issue #4 evidence](../../../docs/verification/issue-4.md#current-evidence-index) describes the old single-member restriction; [issue #5 evidence](../../../docs/verification/issue-5.md) owns this narrow superseding rule, new regressions and affected resource derivation.

### Markets and limits

`SourceState::market_ids()` borrows the retained IDs in the same order as
`snapshot().markets`, including unready markets; it grants no validity/readiness.
Native subscription membership uses this narrow read without cloning binding views.
Registration and retirement are visible immediately from the same source owner.

Market descriptor open/close times are signed offsets from Go; elapsed source clocks remain unsigned. Both registration owners accept a valid current window opened before zero. Source schedule comparisons widen both types to i128, so negative times cannot wrap. That current window may display after verification but emits no late entry; only a properly prepared next full window can emit its exact-opening boundary. Windows closed before Go are unusable and retire normally; observations/history still begin at or after Go. Metadata and initial data are verified separately. A prepared successor needs exact identity/schedule, verified unchanged metadata, current-generation data and any required history. Per-market readiness records when all source prerequisites became ready, observing expiry before a same-time renewal. The first admitted boundary requires that readiness strictly before opening; first data, metadata verification or renewal restoring readiness at opening waits for a later full window. A display-only late promotion never counts as the first entry. After emitting its first eligible boundary, the source retains the existing startup meaning during active-run recovery; emission remains received source state, and evaluator completion is still the consumer's responsibility. An on-time boundary expires the old market before promoting a ready successor once. Duplicate notifications emit no second evaluator boundary. Verification of metadata for an active [open, close) market updates display identity immediately, even without another Boundary event; future or closed metadata does not select that market. This display update needs no data-readiness claim and emits no late/backdated entry. Boundary entry/deduplication is recorded independently of display identity, so an already displayed market can enter once at an otherwise eligible exact opening. The source gate complements the evaluator's history-ready clock; zero-history evaluator readiness alone cannot establish source readiness. Boundary and Advance retire expired source records even when a market was never ready, releasing slots for later discovery. Retirement preserves the chosen series identity and underlying bindings, releases old contract facts and makes old stamps fail; evaluator history stays with its exact contract. Repeated source-only handoffs do not promise unlimited evaluator registrations.

`SourceEvent::ReplaceMarket` handles only a retained same-identity schedule
correction, with the captured Metadata token and exact expected old descriptor.
An identical retry is a no-op. A genuine correction invalidates market metadata,
book/trade generations and result stamps before accepting a nonoverlapping valid
900000-ms replacement. The two retained slots and binding IDs stay fixed; the
replacement is unverified until fresh metadata under the new token arrives.
Invalid/over-cap candidates leave preparation revoked and require explicit
revocation delivery after an error. Expected-old/token mismatches admit no change.
An entered market correction stops the source context. A late verified descriptor
can change display, never backdate entry. An attached controlled evaluator cannot
mutate its catalog: invalidate its full result stamp, deliver revocations where
possible, stop it with `SliceControl::Stop`, and explicitly create a fresh pair/
epoch for new descriptors. Keep the old owned history/checkpoints as stopped
evidence; there is no uninterrupted-evaluation promise.

Replacement adds no retained collection or asynchronous work. Two bounded input
descriptors coexist transiently with the old retained slot; identity length and
capacity checks precede copies, with the existing two-market schedule scan. The
original <=1-MiB source owner/input/effect/view argument gains at most two512-byte
input descriptors, within its existing input allowance. It reuses the existing
market invalidation effect/counter headroom and does not enlarge evaluator
admission, change history/arithmetic or introduce an engine network dependency.

Source storage is bounded by 34 requirement records (capacity at most 64), two market records (capacity at most four), 128 concrete bindings, six stream slots, three connections, 128-byte provenance/machine identities and 64-byte input IDs. Update and coverage vectors each admit at most 128 entries/capacity. One effect has at most 256 coherent value/validity members or two revocation transactions of at most 128 members each, with existing ingress accounting before acceptance. Caller-retained clones are outside this one-outstanding-effect/one-view contract; no effect queue, feed archive or history series exists here. Integer counters fail closed. Full type/ownership charges and check limits live in the issue-5 evidence record.

Focused checks: `cargo test -p metten-engine --locked --test sources`, all ordinary engine tests, fmt/clippy and the unchanged release-hour fixtures. Public [source tests](../tests/sources.rs) exercise the actual source/evaluator seam. Internal source-counter and evaluator mask/allocation/work probes cover invariants not visible through public outcomes. None proves runtime delivery, source rights or live fills.

### Fixed source display catalog (step 5)

`SourceState::display()` initializes the eight fixed requirement records for
read-only named-source setup without fabricating a strategy or evaluator. It
shares bounded market/binding registration with `new`; current/next metadata is
still discovered independently. Three underlying bindings exist initially;
contract/reference bindings are registered with the two exact market slots.
`requirements()` exposes borrowed immutable records to the native owner.
Capabilities remain unsupported by default; a zero startup lookback is not proof
of complete history. Native projections must check history capabilities as well
as the existing zero-lookback readiness fact. Strategy grammar, arithmetic,
financial decisions and SQLite revision 3 are unchanged. The public display
catalog regression is in `crates/engine/tests/sources.rs`.
