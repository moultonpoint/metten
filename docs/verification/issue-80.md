# Issue #80: run context after a summary-read failure

## Scope and baseline

Issue [#80](https://github.com/moultonpoint/metten/issues/80), “Loss report is not counted when the run context was never published,” owns acceptance. Owner authorized a separate worktree from current main, preserving the older `codex/daily-drift-scope` checkout and its unfinished documentation. The issue has no body/native blockers, children or parent; its prepared product/medium-priority brief authorizes controlled local engineering only. No credentials, providers, accounts or real-money actions are involved.

Authoritative guidance baseline: remote `origin` = `https://github.com/moultonpoint/metten.git`, default `refs/heads/main`, freshly queried with `git ls-remote --symref origin HEAD` on 27 November 2026 at approximately 03:11 UTC, commit `ccca986076e4bc642ceab54b2312c56580aa56c3`. Selected implementation code starts at that same commit on `fix/80-paper-context`, initially clean. The original checkout remains at `40f95e0f66bc1ce3865608fb5eb6faa887054044` with its original unrelated changes.

Read from the pinned published tree: root `AGENTS.md`; `docs/agents/verification.md`, `maintainability.md`, `issue-tracker.md`, `code-map.md`, `triage-labels.md`, `domain.md`; `CONTRIBUTING.md`; product specification; ADR 0001; engine package/source contracts; `.github/workflows/checks.yml`. Published nested instruction discovery found only root `AGENTS.md`; the new worktree has the same instructions. Local older guidance was also inspected; published guidance supplies the current paper paths and checks. Owner instructions take precedence on unattended/GUI verification. No missing applicable repository guidance identified. This is a task baseline, not an installation or activation claim about other sessions.

## Expected change path

The existing paper worker in `crates/engine/src/lib.rs` remains the sole publisher of admission context and retired-market projection. A successful reply with a current context already carries its committed summary; use it if the worker's extra latest-summary read fails. Historical replies without an open context must not replace the active run. The normal successful-read path, source-state owner, admission fence, order dispatch, storage schema and external API remain unchanged.

Behavior checks use the existing public `Engine::paper_*` interface and shared controlled paper driver. A test-only one-shot storage read failure targets `Recorder::paper_latest` after a successful Start. Check a loss refused before apply, persistence of the purchase block despite healthy later inputs, explicit Resume/new-run recovery and actual purchase outcomes. Check a valid applied loss under the same fault and historical replay isolation. Existing engine tests cover successful-read behavior and connected callers; native and packaged paper checks cover integration. No new production dependency or interface is needed.

## Evidence

The following focused command passed all three tests after the fix:

```sh
/Users/connormoulton/.cargo/bin/cargo test -p metten-engine --locked --lib summary_read_failure -- --nocapture
```

Before the fallback existed, `refused_loss_after_start_summary_read_failure_blocks_until_resume_or_new_run` failed at its first `paper_effective().0` assertion. This reproduced the missing admission block after the injected storage error. After the fix, the test reaches qualifying books and confirms zero attempts and unchanged cash while blocked. Both reset cases then confirm one purchase of 10 contracts. The valid-loss test confirms applied generation/token changes, then reconnects and verifies another applied loss prevents a qualifying purchase. The historical-replay test preserves the current control target and proves that its refused loss still blocks admission.

## Independent review

Candidate staged tree `286fc073aee988e7f4263007e0a96c9d4e18f7c0`, compared to code/guidance baseline `ccca986076e4bc642ceab54b2312c56580aa56c3`. Both fresh GPT-6.1 Sol reviewers independently discovered the published root entry and applicable linked guidance. Neither found a concrete finding. Standards checked publication ownership, error fencing, late revocation, durable dispatch and the twelve required smell heuristics. Spec independently fetched #80, checked its requirements against connected flows and reran all three focused tests successfully. Standards was source-only and relied on the supplied issue baseline. Spec did not independently rerun the pre-fix reproduction or full regression. No production changes followed these reviews.

The required code-review skill calls for two independent Sol reviewers plus GLM Standards and DeepSeek Spec. T3 capability discovery reports both free provider instances disabled (`canRunChildTask: false`); these two roles are unavailable, with no substituted model or free dispatch. Record completed reviews and verification below before delivery.


## Tracker and remaining gates

The issue is assigned to `moultonpoint` and remains open. [Start/evidence comment](https://github.com/moultonpoint/metten/issues/80#issuecomment-7208866698). Project access returned missing `read:project`, so its stage cannot be verified or changed. Native blockers and children were empty, and the parent endpoint returned “No parent issue found.” No account-wide definitions or access settings changed.

The owner has been asked whether the two Sol reviews can satisfy this task's review gate. The request cites the exact four-role requirement in `/Users/connormoulton/.codex/skills/code-review/SKILL.md`. Until answered, the four-role review remains incomplete. All local Foundation checks passed as recorded below. A committed candidate can be published for exact-head hosted checks while the review-roster decision remains pending. Hosted CI is not established by the local run; its result belongs in the PR/issue record. Existing ignored Keychain checks require their own authority and are not selected by this change. GUI and live behavior are unverified.


## Final local regression

After both available independent reviews completed, the unchanged production source passed:

- `python3 tests/credential-runner.test.py`, `python3 tests/paper-runner.test.py` and `python3 tests/check-staged.test.py`.
- `npm run lint` with zero warnings; `npm run check` with TypeScript validation and all 50 frontend tests; `npm run build`.
- `cargo fmt --all --check`.
- `cargo test --workspace --locked`: 355 passed, zero failed. Three tests were ignored by default. The two controlled history tests passed separately below; the isolated Keychain test was not selected.
- `cargo test -p metten-engine --release --locked --test strategy -- --ignored --nocapture`: both controlled full-hour history checks passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`.

Frontend correctness lint, TypeScript validation and Rust lint are wired and passed. No new or missing lint/typecheck gate was introduced. `npm run tauri build -- --bundles app` and `python3 scripts/verify-paper.py target/release/bundle/macos/Metten.app` also passed. The fresh bundle confirmed native decode/dispatch, shared learning evaluation, finite full/partial fills, exact fee/settlement, numeric identity/history, stopped reopen, settlement idempotency and lost-reply reconciliation through the synthetic protocol. No window, credentials or provider connection was used. Local source/document link checks found no missing targets. Whitespace checks passed. Delivery `git ls-remote --symref origin HEAD` refresh at `2026-11-27T03:24:16Z` still returned `ccca986076e4bc642ceab54b2312c56580aa56c3`; there was no changed guidance to re-review. Only evidence text changed after the reviewed candidate, so passing behavior checks remain applicable.


## Standards

- GPT-6.1 Sol: zero supported findings. The fallback stays with the existing worker, retains successful-read precedence and does not add a persistence or purchase-authority path. Test-only injection remains with storage; the source contract documents the behavior. No baseline smell finding. This was a source review rather than an independent full-suite run.
- GLM: unavailable. Its configured free provider is disabled; no task was dispatched and no substitute was used.

## Spec

- GPT-6.1 Sol: zero supported findings. The current-context fallback, historical-reply exclusion, refused-loss block, reset behavior and valid-loss handling satisfy #80. This reviewer independently ran all three focused tests, but not the full suite or pre-fix reproduction.
- DeepSeek: unavailable. Its configured free provider is disabled; no task was dispatched and no substitute was used.

Review summary: zero supported Standards findings and zero supported Spec findings from the two completed Sol reviews. Both free-model roles remain unavailable, so the four-role review is incomplete unless the owner accepts the reduced roster.
