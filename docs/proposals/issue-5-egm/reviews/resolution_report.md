# Independent review resolution — 2026-11-14

**Status: resolved — eight accepted document corrections applied.**

**Target:** [market-data design](../market-data-design.md), full file. **Findings:** [consolidated independent review](consolidated_review_report.md). Connor instructed “Fix them with a sub agent.” That is authority to apply these targeted document corrections; implementation remains explicitly prohibited. The subagent applied the corrections directly under that instruction rather than reopening the resolution skill's approval gate.

## Version and scope

Original reviewed target: 586 lines; SHA-256 `a68ea14df9692889230110fca306fa9e03eba3edfd7c81816f2c3ed4c21f0fc2`.

Revised target: 599 lines; SHA-256 `21e2926329f67fef05b264dcb8a0dfca0455aaa697c753bfc4f2d95b19fe4bc7`. Detailed Implementation now starts at line 463.

Only the design and this resolution record were edited by the fixing subagent. All historical review reports, original hashes and original line anchors remain unchanged. This record supplies revised anchors. No implementation, dependency, configuration, app, credential, account or trading operation occurred. No new independent clean-review or runtime acceptance claim is made.

## Accepted corrections

Each A1–A8 finding is categorized **PROPOSE FIX**, accepted by the owner's fix instruction, and applied. Prior consolidated severity is preserved: A1–A4 Major, A5–A8 Nit.

| Finding | Revised target anchors | Discrepancy and rationale | Applied edit and required implementation evidence |
| --- | --- | --- | --- |
| A1 — Major | 113, 298, 308, 335, 579 | A subscription gap could leave the other retained market incorrectly usable/ready. | Both books lose usability and successor readiness on an unexplained subscription gap. Each needs verified replacement state in the current recovery generation; a new gap revokes both again. Recovery is coalesced/budgeted. Test A:41 → missing B:42 → A:43, per-book recovery, old-generation work and prohibited promotion. The all-market rule is a conservative inference from real-time sequence scope, not a conflated-feed instruction misquoted as real-time. |
| A2 — Major | 362–368, 486, 522, 579 | Key-ID-specific items left restart selection and mutation targets unspecified. | One fixed generic-password item per environment: service `com.metten.provider-credential`, account `kalshi:demo` or `kalshi:production`; strict protected version-1 JSON with key ID/material. Key ID is data, not selector. Save refuses existing; Replace requires existing and atomically replaces the whole record after shared-generation revocation; Remove confirms deletion or reports Missing/failure; Status/Connect use that same fixed item. Failed mutation leaves connections stopped; ambiguous outcomes need readback. Test save A → replace B → reopen → select B, missing/refused/failed lookup and failed replacement/removal. No SQLite or selector registry. |
| A3 — Major | 356, 370–372, 524, 579 | Shutdown required a deadline but had no finite setting or completion-failure policy. | One initial 5-second monotonic total provider/source shutdown deadline. Cancel waits/retries/admission, request close and worker exit, then on expiration drop transports/cancel async work while retaining handles. Require observed completion before save/exit. Fail-stopped message reports intake stopped while shutdown is unconfirmed. An explicit later Quit rechecks existing handles; resetting the general Quit guard cannot clear the source-stop latch or reconnect. Explicit Connect can clear the failure/latch only with no active Quit attempt and every old handle confirmed finished; otherwise Still stopping. Keychain prompts retain their separate existing guarded behavior. Test a nonresponsive peer, deadline outcome, retained ownership and no later intake/reconnect. |
| A4 — Major | 523–524, 545–546, 570–577 | Existing descriptor-3 observer accepts only dummy expectations; provider dispatch was absent from the plan. | One validated mode-specific receiver in existing files. Optional bootstrap mode selects sources only after existing identity/store checks; missing mode preserves dummy. Bootstrap cap/deadline stays 4096 bytes/2 seconds. Source envelopes are strict, length-prefixed JSON capped at 256 KiB, with exact session, canonical decimal-string u64 sequence, fixed kind/provider and a UTF-8 payload string capped at 128 KiB. Only one outstanding message; bounded actual decoder/source dispatch acknowledges processed accepted/invalid state, not health. Protocol/timeout failure revokes source evidence and never enables production. Test wrong mode/bootstrap, fields/provider/session/sequence/size, dispatch/ack/EOF and pending Quit. Default dummy shape, numeric sequence, cap and meaning stay unchanged. |
| A5 — Nit | 206 | Rotating diagnostics conflicted with the chosen in-memory scope. | Benchmark now exercises bounded in-memory diagnostic publication with UI and strategy contention. No rotating-file feature added. |
| A6 — Nit | 4, 19, 35, 80–82, 249, 265, 381 | Earlier drafting instructions described decisions/inventory as unfinished. | Point to selected architecture, completed inventory and dated review/resolution. Keep actual capability, compatibility and calibration gates open. |
| A7 — Nit | 492 | “Never slows” was an unmeasured absolute claim. | Local reads use published state and never start provider intake; their effect on intake delay must be qualified under the prescribed contention workload. |
| A8 — Nit | 407 | Hypothetical mistaken completion was framed as inevitable. | State directly that learning-example completion alone does not satisfy other named-source requirements. |

F3's contextual performance/edge caveat is retained as requested. P2's optional paper/live-fill caveat is retained; no authority boundary was weakened. Neither is an unaddressed required correction. No CONTEST or ESCALATE item was needed; zero accepted corrections remain unaddressed.

## Validation and remaining limits

Read the complete target and consolidated report with bounded rereads after truncated output; read the EGM resolution skill and Codex integration. Inspected actual native credential bootstrap, framing, observer, fixed dummy store operations, guarded Quit, and runner send/ack code. Checked cached official book sequence/recovery passages, distinguishing real-time sequence scope from conflation-specific wording. No fresh provider research was needed.

Checked all local Markdown links in the revised target and this report, paired fences, heading order, table column counts and trailing whitespace. Searched for the replaced stale phrases and for contradictory key-ID item selection. Required evidence cases and affected inventory rows now carry the four contracts. Recomputed target hash and line count after edits. Historical review file fingerprints were not changed by this subagent.

These are document checks, not implemented behavior tests. Provider entitlement/rights, source completeness/validity rules, dependency compatibility and performance calibration/qualification remain explicit gates. The 5-second shutdown setting and packaged-fixture limits are initial design choices to validate, not measured guarantees. No build, runtime test, app/browser launch, account call or trading action was performed.

The lead checked the revised contracts against A1–A8 and corrected the unconfirmed-shutdown message to describe stopped intake rather than claim completed connection closure. Final checks independently confirmed the revised target hash above, all eight finding references, local links, fences, heading hierarchy, table widths and whitespace. A before/after fingerprint comparison confirmed no additions, removals or content changes in the engine, native, frontend, test or tool source trees. This lead check is not a new independent Goldfish review.
