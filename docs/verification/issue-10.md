# Issue #10 verification

## Baseline

The implementation worktree starts from published `main` at `2e2795015aa9827344fcc24f224943dc9b81fbf2`. The original checkout at `868609b` contained unrelated documentation edits and drift reports. The implementation is on `feat/10-daily-limits` in a separate worktree. [Issue #10](https://github.com/moultonpoint/metten/issues/10) · [Issue #8 acceptance](https://github.com/moultonpoint/metten/issues/8#issuecomment-7110860281)

The reviewed baseline includes the published entry instructions, discovery procedure, maintainability standard, code map, issue workflow, product specification, ADR 0001, and the engine and desktop paper contracts. The published tree has no nested agent instructions. [Published baseline](https://github.com/moultonpoint/metten/tree/2e2795015aa9827344fcc24f224943dc9b81fbf2)

## Acceptance evidence

| Criterion | Local evidence |
| --- | --- |
| Full worst-case cost with fees. Skip an unaffordable objective at its original size. | `daily_limit_refuses_the_whole_first_objective_at_the_configured_ceiling` confirms zero recorded attempts even when the offered price fits. The packaged check verifies a $2.632 reservation for ten contracts under the conservative fee bound and the actual confirmed $2.6313 debit. The controlled-live worker test refuses stale funds and a smaller external balance. |
| Only Metten-owned settled live results, after fees and by market close UTC. | `live_daily_limit_uses_workspace_results_and_sticky_rollover_permissions` first settles a paper result on the same UTC day, then verifies the live scope has zero settled net. It records the live loss after fees and applies the currently authorized live limits across runs. `delayed_settlement_releases_pending_risk_but_keeps_the_market_close_day` exercises prior-day attribution, terminal replay, and a contradictory report. |
| Pending exposure, partials, restart, and changes between runs. | The controlled-live test carries the first run's confirmed holding into a fresh run and rejects a backward clock on reopen. The delayed-result test observes the debit after midnight before settlement. `reopen_applies_a_final_result_deferred_while_a_purchase_was_unknown` posts the final holding once with no extra venue send after reconciliation. Existing tests cover terminal outcomes and migration with retained records. The bundle verifies a 100-of-101 partial. |
| Finish the current market and enforce stop and resumption precedence. | The controlled-live worker test posts an exact-threshold loss at 23:50 while the 23:45 window is active. It contrasts midnight entry with opt-in, without opt-in, and with Pause. It confirms that reopening cannot restore a live run's input context. Existing checks cover manual Stop, Quit, and feed revocation. |
| Now and History distinguish settlement, pending risk, and block reason. | The frontend test checks separate settled net, holdings, attempt reservations, and stop text. History retains threshold refusals apart from participation and preserves the verified transaction state. The headless bundle validates real native decoding and committed SQLite amounts and refuses live selection and injected financial facts. |

[Engine contract](../../crates/engine/src/README.md#daily-risk-and-utc-rollover-issue-10) · [Paper guide](../strategies/paper.md)

## Local check results

The final remote query at 2026-11-24 06:40 UTC returned the same published `main` commit.

| Check | Result |
| --- | --- |
| `cargo test --workspace --locked` | 351 passed, zero failed, three previously ignored. |
| `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and Git whitespace check | Passed. |
| `npm run lint`, `npm run check`, and `npm run build` | Passed. The frontend suite ran 50 tests. |
| `python3 tests/paper-runner.test.py` | Seven passed. |
| `npm run tauri build -- --bundles app` and `python3 scripts/verify-paper.py target/release/bundle/macos/Metten.app` | Passed on the freshly built Mac bundle. The runner verified native command responses and persisted receipts after the process exited. |

## Proof limits

The workspace-wide live path in these checks uses controlled input inside the engine crate. The packaged app uses synthetic paper inputs. The fixed desktop example ends after its prepared schedule, so the engine tests exercise next-day resumption. Real Kalshi funding, fees, settlement identity, and execution need the integration in [#12](https://github.com/moultonpoint/metten/issues/12). Qualification belongs to [#11](https://github.com/moultonpoint/metten/issues/11). The packaged check runs without a WebView and the frontend tests use a controlled document. GUI behavior remains unverified.
