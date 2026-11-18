# Issue #5 independent design review

**Outcome: revise four focused design contracts before handing the full plan to an implementing agent.** The chosen Rust/Tokio direction and current/next-market preparation do not need redesign. The review found missing decisions around shared-feed recovery, saved-key selection, shutdown and the packaged test channel. These can be resolved inside the planned scope and file inventory.

The owner requested a consolidated review and explicitly prohibited implementation. Ten fresh-context reviewer runs completed across all five EGM stages. No source code, dependencies, settings, credentials, application processes or trading state were changed. The target design remained unchanged throughout. This report records findings; it does not apply them or authorize work.

## What this means for the project

The plan provides a usable direction and bounded investigations. Its explicit research and calibration gates are appropriate: provider guarantees, rights/access, source-validity rules and measured performance are not established by approving a document. Those gates must continue to withhold unsupported claims.

It is not yet a complete coding handoff. Four decisions could otherwise be made differently by the next agent, changing whether an upcoming market is trustworthy, which account identity is selected, how Quit behaves, or whether the required packaged test can run. Resolve them in the design first, then recheck only the affected contracts. Implementation remains unapproved.

No reviewer found a need for another runtime, a generic provider platform, a second history engine or an extra storage system. Existing evaluator evidence, immutable strategy lookup, native operation admission and guarded shutdown are appropriate reuse points. No performance or uptime result follows from that assessment.

## Required design corrections

### A1 — Major: recovery must cover every market whose shared feed may have lost an update

**Design locations:** lines 108–115, 294–306 and 331–337. **Raw findings:** D1, M1.

Current and upcoming market books share a Kalshi subscription. The plan refers to an affected book or stream, but does not say what happens when a missing message cannot be assigned to one market. For example, current-market message 41 arrives, upcoming-market message 42 is lost, then current-market message 43 reveals the gap. Repairing only the market that revealed it could leave the upcoming market incorrectly marked ready.

**Smallest correction:** define the subscription as the continuity boundary for an unexplained sequence gap. Revoke usability for all retained books on that subscription and remove upcoming-market readiness. Restore each retained book only from verified replacement state. Add one controlled case where the missing update belongs to the other market. This uses the existing generation and rebuild mechanisms; it does not require another socket or service.

**Evidence qualification:** Kalshi documents real-time sequence numbers at subscription scope. The conservative all-affected-market recovery requirement is an inference from that scope. Its explicit all-market snapshot instruction appears under the conflated-feed section; the plan selects real-time mode, so that instruction must not be presented as a real-time quotation. Lifecycle sequencing should follow its own verified protocol contract before unrelated payloads are discarded. [Official book contract](https://docs.kalshi.com/websockets/orderbook-updates).

**Resolution evidence:** a missing update cannot leave either potentially affected book usable; the upcoming market cannot be promoted until its replacement evidence is established. Late messages and repeated recovery requests retain the existing identity/budget rules.

### A2 — Major: choose one saved credential deterministically after replacement and reopening

**Design locations:** lines 360–364, 473–484 and 516–518. **Raw findings:** D2, M2, R1, B2.

The design stores protected items by environment and key ID, allows the ID to change, and keeps connection choices in memory. Connect selects a source without a key selector. After saving A, replacing it with B and reopening, the plan does not say whether both items remain, which item Connect uses, or which item Status/Remove targets. The existing dummy credential has one fixed identity and supplies no provider selection registry.

**Smallest recommended correction:** one fixed protected credential item per provider/environment containing the key ID and key material. Specify save-versus-existing behavior, atomic replacement, restart lookup, removal and finite failure outcomes. Changing/removing that credential must revoke the shared Kalshi/BRTI connection as already required. This keeps selection in protected storage and needs no SQLite migration or multiple-key feature.

**Resolution evidence:** controlled save A → replace with B under a changed ID → reopen → explicit Connect selects B. Failed replacement preserves the declared previous-item behavior; failed removal cannot falsely report success. Missing/refused/failed lookup is visible and does not start a connection. Commands and status follow the same single-item policy.

### A3 — Major: finish the network shutdown deadline and failure contract

**Design locations:** lines 189, 354–366, 465, 509, 518 and 566. **Raw finding:** R2.

The design requires a separate shutdown deadline but supplies numerical limits only for requests, handshakes, acknowledgments and initial/replacement data. It orders socket closure and task/worker joins without defining when a stalled close becomes a visible failure or what happens next. An implementing agent must otherwise choose between an indefinite wait and an undocumented termination policy.

**Smallest correction:** choose a finite monotonic deadline for the provider/source shutdown phase. State which closure/cancellation actions occur, how actual completion is observed, and how the app remains stopped with a specific failure if termination cannot be confirmed. Preserve the existing guarded native Keychain prompt exception: timing out a waiter cannot kill an already running native operation.

**Resolution evidence:** a nonresponsive controlled peer during Quit produces the declared deadline outcome; no later reconnect or intake occurs; successful exit is reported only after the required owned work has actually finished. Do not add a general task-management framework.

### A4 — Major: define how packaged tests deliver simulated feed messages

**Design locations:** lines 517, 539 and 562–564. **Raw finding:** B1.

The plan promises a packaged source walkthrough over the existing private verification channel. Current code hands descriptor 3 exclusively to a credential observer. It accepts only strict dummy-expectation messages, allows one pending dummy value, has a 4096-byte frame cap, and fails on other message types. Exposing verified disposable-store access does not create the required source-message receiver.

**Smallest correction:** specify one receiving owner and a bounded source-verification extension using the existing planned files. Define activation after validated bootstrap, the permitted message envelope/size, sequencing and acknowledgments, dispatch to the real provider decoder seam, and failure/shutdown behavior. Preserve the default dummy mode's existing accepted messages and limits. No general-purpose transport layer is necessary.

**Resolution evidence:** valid synthetic source messages exercise the actual decoder/source path. Wrong-mode messages, invalid bootstrap, oversize frames and duplicate sequences fail as declared. The test path cannot select real provider networking, and the preserved dummy checks continue to pass when implementation eventually occurs.

Source evidence: `src-tauri/src/credential.rs` bootstrap/observer/framing; `src-tauri/src/main.rs` setup; `scripts/verify-credential.swift` sender/expectation acknowledgment. The lead independently reread those boundaries after receiving the finding.

## Small wording corrections

| ID | Priority | Target | Smallest change |
| --- | --- | --- | --- |
| A5 | Nit | 206 versus 494 | Replace diagnostic rotation in the benchmark with bounded diagnostic publication/update. The plan already selects in-memory diagnostics; do not add rotating files. |
| A6 | Nit | 35, 80–82, 249, 265, 375 | Replace earlier drafting instructions with references to decisions and inventory now present. Keep genuine provider-proof and calibration gates unresolved. |
| A7 | Nit | 486 | Replace the absolute claim that screen reads never slow intake with a published-local-state requirement and the existing contention qualification. Preserve zero provider calls from refresh. |
| A8 | Nit | 401 | Say learning-example completion does not satisfy the other named-source requirements, rather than predicting it would be mistaken for completion. |

F1 and P1 were reported as Major by style reviewers. The lead consolidates them as Nits: each is a local wording conflict or overclaim whose intended implementation and required measurement are already stated elsewhere. Fix them, but they do not justify architecture changes.

Two additional editorial suggestions are not required remediation. F3 proposes deleting the contextual statement about performance/edge guarantees; retain it because it answers the owner's expressed expectations and accompanies concrete failure requirements. P2 proposes deleting the out-of-scope paper/live-fill caveat; trimming it is optional and does not affect readiness. Preserve the actual account/trading authority boundary.

## Review completion and evidence

| Stage | Reviewer | Status | Result record |
| --- | --- | --- | --- |
| 5 — initial zoom-out | Skeptical architect | Completed | [Initial report](zoom_out_report.md): Z1; independent reuse searches |
| 5 — initial zoom-out | Inventive catalyst | Completed | [Initial report](zoom_out_report.md): no false compromise detected |
| 6 — document-only comprehension | Comprehension | Completed | [Complete paraphrase and findings](comprehension_report.md): C1–C2 |
| 7 — feature critique | Design critic | Completed | [Critic report](critic_report.md): D1–D2 |
| 7 — feature critique | Structural | Completed | [Critic report](critic_report.md): no structural findings |
| 7 — feature critique | Style | Completed | [Critic report](critic_report.md): F1–F4 |
| 8 — intermediate zoom-out | Skeptical architect | Completed | [Intermediate report](intermediate_zoom_out_report.md): M1–M4 and prior-finding dispositions |
| 9 — readiness | Implementation lead | Completed | [Readiness report](implementation_readiness_report.md): R1–R2 |
| 9 — readiness | Codebase-details auditor | Completed | [Readiness report](implementation_readiness_report.md): B1–B2 |
| 9 — readiness | Style | Completed | [Readiness report](implementation_readiness_report.md): P1–P2 |

All required persona, target and supplied-standard reads were reported completed; bounded rereads recovered truncated required output where identified. Native final reports were received before synthesis. Source checks were bounded to the review question; per-review records state their uninspected areas. No failed or missing reviewer was treated as a clean result.

Execution was one uninterrupted, review-only pass through the five stages, with parallel reviewers within stages. No remediation loop, second clean round or convergence claim is made. The comprehension reviewer read the target alone and its persona, without supporting context. Other reviewers received the declared local baseline and cached linked sources. Native `fork_turns=none` excluded the parent conversation; exclusion of host/project instruction files was not independently tested.

### Traceability matrix

Every raw finding maps to an action or an explicit disposition; repeated findings are not counted as additional problems.

| Raw finding | Consolidated disposition |
| --- | --- |
| Z1 | A5 |
| C1 | A5 |
| C2 | A6 |
| D1 | A1, with real-time/conflated evidence qualification |
| D2 | A2 |
| F1 | A5; raw Major reduced to Nit for stated reason |
| F2 | A6 |
| F3 | Retain contextual caveat; deletion not required |
| F4 | A8 |
| M1 | A1; corroborates D1 and narrows source attribution |
| M2 | A2; corroborates D2 |
| M3 | A5; duplicate/priority disposition |
| M4 | A6 |
| R1 | A2; independent readiness confirmation |
| R2 | A3 |
| B1 | A4 |
| B2 | A2; independent source-backed confirmation |
| P1 | A7; raw Major reduced to Nit for stated reason |
| P2 | Optional trimming only; no required readiness action |

## Version and limits

Reviewed target: `docs/proposals/issue-5-egm/market-data-design.md`, 586 lines. Feature range 1–456; implementation range 457–586. SHA-256 `a68ea14df9692889230110fca306fa9e03eba3edfd7c81816f2c3ed4c21f0fc2`. Repository baseline `47d0f384669f38aaf76e3c5bc50060836b235f02`; proposal documentation was uncommitted. [Inputs](inputs.md) records nine direct local references and 26 linked source captures. Raw source pages remain in ignored local planning, with checksums, rather than adding vendor HTML to maintained documentation. Public source URLs remain in the manifest; the private issue is supporting tracker context.

This review created only review records. The design's earlier statement that formal review had not occurred remains frozen as part of the reviewed version; this dated report records the subsequent completed review.

No builds, runtime tests, browser/app launches, actual accounts, real feeds, paid access or trading were exercised. Entitlements, retention/use rights, complete-history guarantees, source-validity limits, dependency compatibility and measured latency/capacity remain the plan's explicit outstanding gates. The report supports a focused design revision, not a claim of implementation or operational readiness.
