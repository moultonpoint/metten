# Interface and credential-runner regression checks

## Overview

These checks cover presentation, refresh ordering, credential-input handling, local strategy flow, staged snapshots and the verification runner's process recovery. They use controlled inputs rather than exchanges or trading accounts. They do not prove the complete acceptance requirements in the [product specification](../METTEN_DESIGN_DRAFT_2026-11-12.md). Controlled strategy engine/storage tests are owned by the [engine contract](../crates/engine/README.md); local packaged evidence is indexed in [issue #4](../docs/verification/issue-4.md#current-evidence-index).

## Interface

| Test file and entry point | Boundary exercised |
| --- | --- |
| [view.test.mjs](./view.test.mjs): `pending and failed recording never claim saved success or permit another event` | Calls exported [presentation](../src/view.ts) with saved, pending, and failed snapshots; checks recording wording and relevant controls. |
| [refresh.test.mjs](./refresh.test.mjs): failed-refresh test and four older/newer-result cases | Reads [main.ts](../src/main.ts), strips TypeScript/imports, and runs it in a VM with fake elements, timers, and `invoke`. Checks unavailable-state messaging, continued Pause access, no automatic stop request, and protection against stale successes/failures overwriting a newer Pause result or failure. |
| [credential.test.mjs](./credential.test.mjs): credential-submission test | Runs the same entry point with fake elements and native calls. Submits one dummy save, checks immediate input clearing and button disabling, rejects with a marker-bearing unexpected error, and checks that the marker is hidden and buttons recover. |
| [ui-harness.mjs](./ui-harness.mjs): `harness(t, replies)` | Shared non-entry VM bootstrap for strategy, refresh and credential suites. Loads the real entry point and presentation exports; each fixture owns elements, call log, deferred replies and timer callbacks. Optional per-command reply functions control the current refresh scenarios. |
| [ui-harness.test.mjs](./ui-harness.test.mjs) | Checks fresh-fixture isolation, finite startup, deferred replies and manual polling. A disposable child test proves an unknown command fails its owning test even when its rejection is caught. |
| [credential-runner.test.py](./credential-runner.test.py): `CredentialRunnerRecovery.test_progress_is_visible_before_exit_and_late_exit_is_recovered` | Compiles [verify-credential.swift](../scripts/verify-credential.swift) using `swiftc`, runs `--self-test`, observes progress before exit, releases the controlled gate, and checks the runner's recovery/output markers and successful exit. |
| [strategy.test.mjs](./strategy.test.mjs): entry-point harness | Uses fake DOM/native responses to check exact reviewed bytes, stale replies, literal rendering, edit-copy, duplicate-save blocking, Busy and uncertain-save/refresh separation. |
| [check-staged.test.py](./check-staged.test.py): `StagedCheckpoint` | Exercises [check-staged.py](../scripts/check-staged.py) in disposable Git repositories: staged-only export, missing dependencies, whitespace/command failures, cleanup and unchanged source index/worktree. |

## Invariants

- The shared bootstrap returns finite stopped-snapshot and initial empty-version-list replies, resolves controls, and defers later strategy/credential calls until the test settles them. All native calls record command/arguments; unexpected command names reject and fail the owning test through its teardown assertion, outside production catch handlers. Poll timers are recorded, never scheduled; `flush` drains promise continuations with `setImmediate`.
- VM tests replace the DOM, timers, and native calls. They exercise real entry-point logic but neither render a browser nor launch Tauri. The credential test covers one unexpected save failure; it does not test every credential operation or native error prefix.
- Refresh coverage includes all four combinations of an older success/failure and newer success/failure. A failed read must not be represented as proof that the engine stopped, and a stale result must not replace newer display state.
- The credential-runner Python regression is process-only: its `--self-test` does not launch Metten or access Keychain. It checks visible progress, retained pending children, late exit/channel cleanup, unsuccessful child status preservation, and noisy-child output handling through explicit runner markers. It does not independently establish packaged-app credential protection.
- The [package check script](../package.json) selects `tests/*.test.mjs`; `ui-harness.mjs` is maintained support code and is not a test entry point. The Python files are separate checks, not part of `npm run check`. These checks do not cover stylesheet layout, actual assistive technology, native persistence, live exchange acceptance, or fills.

## Side Effects

- All three VM suites use the shared bootstrap to read the entry-point source and mutate only fake element objects in isolated VM contexts; native commands and timer scheduling are mocked. The presentation check calls the pure function directly.
- The harness unknown-command regression starts one bounded Node child with synthetic inline test code; it launches no application and writes no source.
- The staged-helper regression creates disposable Git repositories and invokes the helper with controlled Python commands. It checks temporary-export removal and source preservation; it does not stage or commit this repository.
- The credential-runner Python test writes a temporary Swift binary and progress log, starts disposable child processes, sends a gate byte over stdin, and removes its temporary directory. On a blocked cleanup path, it kills only the process group it created and waits for exit.

## Verification

Run from the repository root:

```sh
npm run check
python3 tests/credential-runner.test.py
python3 tests/check-staged.test.py
```

The JavaScript entry-point checks require Node's `stripTypeScriptTypes`. Both Python checks require Python 3; the staged-helper check also needs Git, and the credential-runner check needs `swiftc`. Credential-runner compilation and child waits have explicit timeouts. These checks do not launch a browser or native application or access credentials.

## Sources checks

`sources.test.mjs` runs the actual entry point with controlled native replies and
timers: active-only serial reads, disconnect/selection stale replies, immediate
provider secret clearing, literal output, truthful unsupported data and revisions
above 2^53. Existing session/strategy/dummy suites remain included by npm check.
Native permission/command tests use the actual Tauri authority resolver and
bounded decoders. Native source tests use the real coordinator/worker/decoder,
zero-network read counts, fixture publication completion, scoped SID retirement
and production-transport refusal. Receiver tests exercise real Unix sockets,
framing/deadlines, duplicate sequence, dispatch-before-ack, EOF/Quit and late
completion. Process-only runner checks remain separate from packaged execution.
