# Implementation readiness — 2026-11-14

Active [target](../market-data-design.md) lines 457–586, with earlier architecture as context. [Frozen inputs](inputs.md). Three independent fresh-context reviewers completed. They did not read other formal-review reports. No implementation or target changes.

## Implementation lead

**R1 — Major — lines 465, 473–484, 516–518; architecture 360–364:** saved credential identity, restart lookup and changed-key-ID replacement are unspecified. Connect accepts a source but no credential selector; status/remove also lack a deterministic target-selection rule. Define one configured identity per environment, durable protected lookup, replacement/removal outcomes and failures. Test restart, changed ID, failed replacement/removal and shared Kalshi/BRTI revocation. This fits existing files and needs no SQLite migration.

**R2 — Major — lines 465, 509, 518, 566; architecture 189, 354–366:** a shutdown deadline is required but absent from the numerical profile. Socket close/join can therefore wait indefinitely or require an undocumented termination policy. Define a finite monotonic provider/source shutdown deadline, closure/cancellation actions, observed task completion and a visible stopped failure if termination cannot be confirmed. Preserve the separately guarded, unkillable native Keychain prompt. Test a nonresponsive peer during Quit, deadline outcome and no subsequent intake/reconnect.

The exact file inventory is sufficient for a write allowlist. Existing phases/tests are useful. No extra abstraction, file split or exhaustive public signature is needed to remove ordinary private coding choices. Provider capability research and calibration are legitimate explicit investigations, not fabricated claims of current support; passing them remains necessary for full acceptance.

Read: complete persona, readiness standard, target and inputs; code map, ADR, engine/native contracts, cached issue; bounded engine evidence/source symbols, native/compiler manifests and product excerpts. No builds, tests, external/account checks or edits.

## Codebase-details auditor

**B1 — Major — lines 517, 539, 562–564:** the packaged provider-fixture receiving path is undefined. Existing `Credentials::setup()` takes descriptor 3 and transfers it exclusively to `observe()`. That observer accepts only strict `Expectation` messages, `phase == local-use`, one pending dummy value and 4096-byte framing; other input permanently fails the channel. Exposing disposable-store access does not add a provider dispatcher. Specify one receiving owner, validated source-mode activation, bounded provider envelope, sequence/acknowledgment rules, dispatch, failure and shutdown. Preserve default dummy messages/limits. Test wrong mode, oversize, duplicate sequence, invalid bootstrap and inability to enable production networking. No general-purpose transport framework is required.

Source anchors: `src-tauri/src/credential.rs:40`, `:86`, `:260`; `scripts/verify-credential.swift:51`, `:266`; `src-tauri/src/main.rs:250`.

**B2 — Major — lines 473–484, 516:** same saved-credential selection gap as R1. No existing provider registry/active selector can fill it: current dummy adapter has one hardcoded identity. Define one protected configured item per environment or an explicit bounded selector policy and make commands, failure handling and generation revocation consistent.

No verified false existing-interface or check-command claim found. Inspected evidence/batch/slice and ingress limits, immutable detail lookup, admission/Quit, command/permission pattern and test commands match current source. Proposed Create files were correctly treated as future work.

Read: complete persona, readiness standard, target, inputs, code map, verification guidance and ADR; relevant product/component/tool/test contracts; manifests/build/capability/CI; native commands/Quit/credential bootstrap-observer-store; engine strategy lookup/admission/evidence; runner framing/bootstrap and process self-test; Python runner regression; representative frontend/harness code and callers. Not inspected in full: remaining evaluator algorithms/resource proofs; complete storage/session; every frontend test; full runner Accessibility/Keychain cleanup; generated permission TOMLs; styling/assets; historical packaged evidence. No execution, browser/app launches, account or provider calls.

## Style reviewer

**P1 — Major as reported — line 486 — CALIBRATE:** “never slows” provider intake is an absolute unmeasured performance claim. Keep zero provider calls from refresh, but say it reads published local state and require qualification of contention under the named workload. Earlier benchmark requirements already acknowledge contention.

**P2 — Nit — line 584 — DROP proposed:** the statement that paper results do not prove live fills is outside this read-only slice and repeats scope exclusions. Retain actual account/trading authority and acceptance distinctions. Lead will decide whether this optional trimming merits a required change.

Read: complete style persona, inputs, detailed range and relevant earlier scope/publication/performance passages. No code, external-source or execution verification.

These are readiness judgments, not passing runtime evidence. All four underlying material concerns remain unresolved in the frozen target: shared-subscription recovery, saved credential selection, bounded shutdown, and packaged fixture receiving protocol (the first was identified in the feature critique).
