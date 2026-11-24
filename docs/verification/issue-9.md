# Issue #9 evidence: checkpoints and purchasing safety through feed interruptions

## Current evidence index

**Baseline.** Branch `feat/9-feed-interruptions` from published `main` `55e5c9cb5f441bab10ba336fa097bef104aceea9` (remote re-queried 2026-11-24T04:06Z, unchanged). Approved scope and owner decisions: [issue #9 start comment](https://github.com/moultonpoint/metten/issues/9). Behaviour contract: [engine feed-interruption section](../../crates/engine/src/README.md#feed-interruptions-issue-9).

**What changed.** (1) Kalshi loss blocks purchases through its own `Fence::source_lost`; after the loss is applied, a run continues in the same market without Resume once fresh evidence qualifies. (2) Late results (all-retired-market Update/Metadata, older-generation Book) are refused with non-fencing `stale_result` without consuming the ordinal. (3) Defect found by test 4 and fixed: a market promoted late after a reconnect across its opening was recorded as participating; entry now needs its exact-opening boundary in the applied batch.

| Criterion clause | Evidence (test name; file) | Status |
| --- | --- | --- |
| **AC1** Apply required book changes and checkpoint observations before latest-only UI summarization | `burst_in_one_millisecond_latches_a_transient_crossing_and_keeps_its_side` (engine `tests/interruptions.rs`); native `decoder_burst_latches_a_transient_crossing_and_kalshi_loss_blocks_the_candidate` (`src-tauri/src/providers/tests.rs`): three real Kalshi frames in one ms reach the evaluator in order; the middle crossing latches YES at its own transaction although the last frame alone would latch NO. The worker applies each frame to `SourceState` before publishing its latest-only view (`providers/source.rs`, unchanged). | Covered |
| AC1 Detect sequence gaps instead of silently losing events | Existing, unchanged: `shared_book_gap_revokes_both_and_stale_completion_cannot_restore_either_or_brti` (providers/tests.rs), `books_are_replaced_and_shared_gap_discards_both` (kalshi.rs). Coinbase/Kraken limits below. | Kalshi covered; display-only sources limited |
| AC1 Detect overload instead of silently losing events | Existing, unchanged: `full_intake_revokes_without_waiting_and_obsolete_results_stay_revoked`, `socket_intake_waits_within_age_limit_and_remains_cancelable` (providers/tests.rs). | Covered |
| **AC2** Reconnect with fresh named-source state | `mid_window_reconnect_keeps_the_latch_and_continues_only_on_fresh_qualifying_evidence`: pre-loss book refused (`stale_result`); new-generation data cannot buy before metadata is reverified (`objective_blocked`). Updated `public_engine_reconnect_never_regrants_a_spent_book` (tests/paper.rs) no longer calls Resume and still never regrants a spent book. | Covered |
| AC2 Order reconciliation through reconnect | `unconfirmed_partial_retry_through_a_reconnect_is_never_resent_then_completes_the_remainder` (tests/recovery.rs): 4 of 10 confirmed, retry outcome unknown, reconnect, fresh qualifying book: zero resends; once the venue record appears the next input buys exactly the remaining 3, no Resume. Existing: `lost_reply_is_checked_then_reconciled_without_a_second_send`, `recovery_keeps_newer_pause_and_stop_and_resume_waits_for_the_check`. | Covered |
| AC2 Continue in the same window only when rules still qualify | Same mid-window test, `qualifies = false`: no purchase. | Covered |
| AC2 …and no outcome is uncertain | Recovery test above; a rejected revocation keeps the block: `revocation_rejected_before_it_is_applied_keeps_purchases_blocked_until_resume` (`range`, `stale_input`). | Covered |
| AC2 Preserve checkpoints/sides; never assume one in the gap; reset at the next market | Mid-window test: latch evidence (`checkpoint_latch`, side yes at 1000) unchanged and the opposite side bought. `reconnect_across_a_market_boundary_never_enters_the_late_window`: no participation, latch or purchase in the late window (failed before the entry fix). | Covered; defect fixed |
| AC2 User controls are not cleared by recovery | `user_pause_stop_and_quit_win_over_source_recovery`; `lib.rs::tests::pause_and_busy_revocation_fence_qualifying_input_before_worker_handles_it` (now asserts the loss sets `source_lost`, not `paused`). | Covered |
| **AC3** Deadlines and boundaries handled without new quotes | `silence_expires_data_passes_the_deadline_and_closes_the_market_from_advance_alone` (control lease buys, expired lease does not; Advance alone closes at 901000); native `paper::tests::playback_silence_expires_quotes_passes_the_deadline_and_closes_without_new_quotes` (playback `Driver`, one-second synthetic ticks, fails if the quote does not expire). | Covered (caller-supplied time) |
| AC3 Clock changes do not replay missed purchase ticks | `clock_jumps_never_replay_missed_purchase_ticks_or_enter_late`: one jump past the window with a stored qualifying book buys nothing; the same jump ending inside the window buys once. Existing: `invalid_source_clock_is_rejected_before_pending_receipt_and_can_be_corrected` (backward clock). | Covered |
| AC3 …or bypass full-boundary start | Same test: a jump over the opening and a late Boundary never enter; also the boundary-spanning reconnect test. | Covered |
| AC3 Sleep/wake | Out of scope by owner decision (no detector change); engine time comes only from inputs. | Not evidenced |
| **AC4** Burst updates, transient crossings | AC1 tests. | Covered |
| AC4 Delayed/stale results from an old subscription or market | `stale_results_are_refused_without_consuming_the_ordinal_and_unknown_or_mixed_stop_the_run`: retired Update, retired Metadata and old-generation Book each refused, then a valid input succeeds at the same ordinal; newer generation keeps `range`; unknown-market Metadata/Update and a mixed batch still stop the run. Existing: `public_engine_retry_checks_live_event_token_and_owned_input_capacity_before_receipt` (old event token). | Covered |
| AC4 Silence | AC3 tests. | Covered |
| AC4 Slow recording | `lib.rs::tests::pause_during_a_long_jump_and_a_slow_dispatch_marker_is_serialized_with_dispatch`, both orderings, during a 60 s jump: Pause before the marker cancels the intent and sends nothing; Pause while the marker is being recorded waits for the fence, the order is sent once, and later qualifying evidence sends nothing. Existing: `brief_write_lock_during_venue_record_still_confirms_the_purchase`. | Covered |
| AC4 Pause remains responsive; controls serialized with dispatch | Same barrier test (Pause returns while the jump is in flight); existing `pause_and_busy_revocation_fence_qualifying_input_before_worker_handles_it`, `queue_full_revocation_survives_same_go_retry_and_only_fresh_go_restarts` (lib.rs), `public_engine_later_pause_survives_an_older_resume_completion` (tests/paper.rs), native `pause_after_native_resume_admission_survives_delayed_dispatch` (src-tauri/src/paper.rs). | Covered |
| Known limit: request in flight when the feed is lost | `lib.rs::tests::feed_loss_while_a_user_request_is_in_flight_stops_the_run_and_sends_nothing`. | Documented limit |

## Commands and results

Run on the branch worktree (macOS, pinned toolchain), with `cargo` from `~/.cargo/bin`.

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | Passed (no changes). |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed, no warnings. |
| `cargo test --workspace --locked` | Passed: 344 passed, 0 failed, 3 ignored (ignored tests were already ignored on `main`). Native `metten` 138 passed/1 ignored; engine lib 32, `interruptions` 8, `paper` 31, `paper_storage` 8, `recovery` 12, `session` 6, `sources` 36, `strategy` 63/2 ignored, `strategy_storage` 10. |

Not run, because nothing they cover changed: frontend `npm run lint/check/build` and VM tests (no `src/` or `tests/*.mjs` change), Python runner tests, the release-mode strategy fixtures (`strategy.rs` unchanged) and `npm run tauri build` / the packaged `--paper-check` (no packaging or check-protocol change). The native playback test runs the playback `Driver` under `cargo test`, not a built app bundle.

Regression evidence: before the entry fix, `reconnect_across_a_market_boundary_never_enters_the_late_window` failed with participation `["KXBTC15M-PAPER-B", "KXBTC15M-PAPER-A"]`. With the quote lease not expiring, the native playback test failed (cash `9736870000`, a purchase), so it detects expiry. The rejected-revocation test also passes on the old code, where the block came from `paused`; it guards that the new flag keeps that safety.

## Limits

- The engine has no timers. Deadlines, expiry, boundaries and rollover advance only when the caller supplies inputs; the packaged clock is synthetic (20×). A real sleep/wake or clock-change detector is not part of this evidence.
- Native feed handling (decoder, intake, gap/overload revocation) and engine recovery are evidenced separately and never joined: the live feed is not connected to paper runs.
- A feed loss that lands while any request is in flight, including the user's own Pause, stops the run. Nothing is sent, but the market is discarded.
- A large clock jump is processed in one pass while holding the recorder; Pause still fences at once, but other requests may see `busy` meanwhile. Duration is unmeasured.
- A level checkpoint true in the fresh post-reconnect snapshot can latch from it (treated as observed now, not inferred during the gap).
- A book received after reconnect but before metadata reverification may buy once metadata is reverified, if it is still valid, never spent and matches the evaluated ask (all of it post-reconnect evidence).
- A late *unverified* Metadata result for a retired market is admitted as a source loss before the worker can see it is stale; it is then refused as `stale_result` and, per the approved rule, its block stays until Resume or a later applied loss.
- Coinbase misses a skipped trade ID once a later trade advances its watermark; Kraken has no gap detection. Both sources are display-only; the owner accepted this limit for #9.
- No paper result proves live acceptance or fills.

## Corrections to the reviewed plan found during implementation

- An older-generation Book under the current context was already refused non-fatally (`range`, from `Input::canonical`), not fatal; it is now `stale_result`. A newer generation keeps `range`, which is non-fatal today.
- Unverified current metadata surfaces in the market receipt as `objective_blocked`, not `unverified_metadata`.
- The suspected participation defect (test 4) was real and is fixed in `paper.rs:Run::input`.
