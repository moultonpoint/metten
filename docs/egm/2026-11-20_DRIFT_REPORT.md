# Daily EGM documentation drift report — 20 November 2026

Pass: full maintained repository, recorded 2026-11-20T10:52:29.885837-06:00 (America/Chicago).
Accountable lead: Codex. **Drift found: three current documentation finding groups.**
This is a repository-wide static drift audit using fresh reads and identity-verified prior inspection, not fresh acceptance, security, resource or release certification.

## Scope, authority and published-guidance baseline

- Connor explicitly expanded the recurring task to the entire maintained repository on 20 November, then requested this full audit now. That supersedes the former issue #4 exclusion in the local coverage map, old prompts and historical reports. Issue #4, its shared-file additions and all other maintained components are included. No foundation projection restriction applies to this pass.
- Selected working copy: `/Users/connormoulton/Desktop/metten`; code HEAD **868609baffc25f06ecb43d8760e7cdd60f684a68**. Tracked working files are unchanged against HEAD; the four pre-existing untracked reports dated 15/16/17/19 November were preserved.
- Authoritative remote: `origin = https://github.com/moultonpoint/metten.git`. Fresh remote queries at the start of this full pass (approximately 10:50 CST) and before delivery (10:52 CST) resolved `refs/heads/main` to **8bb7aac3704c9b197e58e10ed9256483cbf9f2e8**. Objects were already available; no fetch or tracking-ref update.
- Published root AGENTS.md and its verification/discovery procedure were loaded at that same revision earlier in this ongoing task. Applicable published maintainability, code-map, issue-workflow, domain and triage guidance was consulted, alongside local instructions/contracts. Published and local trees contain only root AGENTS.md. No new session or delegate was created. This session's guidance was refreshed; no delivery claim is made for unrelated sessions.
- Published navigation describes later implementation than this selected checkout. It supplies current process guidance; it does not make later default-branch features requirements already implemented in this older selected revision. This report audits the owner's selected working copy, not all code now on remote main.
- Used the installed `egm-drift-report` skill at `/Users/connormoulton/.codex/plugins/cache/egm/egm/0.1.1/skills/egm-drift-report/SKILL.md` and its bundled `docs/codex.md`. Source inspection was sequential. No bootstrap, design-review workflow or delegation.
- Inventory: **240 distinct tracked plus nonignored untracked paths** before this report, including hidden configuration. Captured SHA-256 identities at the start and again immediately before writing; all 240 were unchanged.
- Implemented scope is MIT licensing, the persisted controlled non-trading session, isolated dummy credentials, format-2 controlled rule evaluation and immutable local versions, with supporting tools/configuration/CI/assets. Connected sources, integrated paper/live execution, financial accounting/recovery, backups and signed distribution remain future product scope in this checkout. Their absence is not a completed-scope regression.
- Only this dated report is written. No fixes, staging/commit/push, fetch/pull/reset/checkout, tracker or schedule mutation, tests/builds, browser/app/container launch, credential/account/private-runtime access or trading.

## Evidence method and coverage reconciliation

The earlier interrupted foundation pass in this same conversation freshly inspected retained session engine/storage, public session tests, native credential implementation/embedded checks, frontend controls/presentation, supporting tests, both verification scripts, manifests/configuration, CI/dependency automation, component contracts and relevant verification records. HEAD and tracked contents remain unchanged. That work is reused here, without representing the interrupted audit as a completed report.

For issue #4 and the full shared boundaries:

- The expanded 14 November audit, [authority/identity and coverage record](2026-11-14_DRIFT_REPORT.md#authority-identity-and-method), recorded full sequential inspection at **96ad37ab547751f1f3b4b55d392bc42be3563e9c**, including all 7,192 strategy source lines and 3,043 public strategy-test lines, storage tests and entire shared implementation. Its observations and later corrections remain version-bound evidence, not independent proof newly obtained today.
- Compared **137 maintained code/test/tool/asset paths** against their blobs at 96ad37a. **134 match byte-for-byte**. Only `crates/engine/src/strategy.rs`, `crates/engine/tests/strategy.rs` and `scripts/verify-credential.swift` differ. No code path is silently omitted because it belongs to issue #4.
- The two strategy files match the exact final identities in the [F12 diagnostic correction and Sol review](2026-11-14_DRIFT_REPORT.md#f12-diagnostic-correction-and-sol-review--2026-11-14t104821737765-0600): source **330994592799c22bb12685dbe55a5c4ebc1b0ac3f5765f26222b168a6a760575**, tests **7e4476d008eeb2e26d43cb3a50305a5a6a23e3b3636e0e79d3ff45cf58b7a540**. Read the complete two-file diff from the expanded-audit baseline, current grammar/error/planning sections and added public regression assertions. Reuse unchanged evaluator/history/continuation/resource inspection with these precise limits; no fresh whole-module line-by-line reread or proof recomputation is claimed.
- The operative design is unchanged at SHA-256 **9b5f1d2738fad3f77c99d70ea3cfe6e7436b370e07a5056daa08dc23f3fc872d**; learning JSON is unchanged at **a79bd747078601cd16bc292c7896cdcb9f2fe3db0feebf412284a50a2bd50aa5**. Read current approval/phase/grammar/acceptance sections and issue-4's current index/latest corrections. Other unchanged design sections retain the expanded audit's inspection coverage.
- The Swift verifier was freshly read in full in this ongoing task, including process self-test, bundle-derived storage paths, retained-child cleanup and native runner. Its changes are separately owned by issue #31; they are not inferred covered by the older strategy audit.
- Previously withheld shared intervals were read without the old filter: engine strategy protocol/version/storage code, native admission/quit/decoder/tests/build/capability/permission, frontend editor/types/helpers and strategy VM tests. Shared contracts, root status, glossary, sequence and product/ADR additions were inspected in their current form.
- Both lockfiles, credential.rs, session.rs, 103 non-README asset files and two desktop icons match foundation **b1eb58c25fed5a2970bd5c40db9c05f673ff8d71** byte-for-byte: 109 comparisons, no differences. Asset coverage is ownership/usage/provenance/inventory, not render/export or legal recertification.
- Active-document relative file/directory link check: **334 checked, zero missing targets**. This excludes historical proposals/reports/evidence bodies and does not validate heading anchors or external URLs.

| Maintained group | Current ownership and evidence |
| --- | --- |
| Root manifests, lockfiles, toolchain, TypeScript configuration, ignore rules | README / CONTRIBUTING / product and ADRs; fresh config comparison plus lock identities |
| Engine session, strategy, storage and embedded/public/integration tests | Three engine contracts, operative issue-4 design/reference and evidence; fresh boundary reads plus identity-verified complete prior inspection |
| Desktop main/credential/build/configuration, capabilities and four generated permission TOMLs | Both desktop contracts, issues 2/3/4/31; fresh retained/full additional intervals and verified source correspondence |
| Frontend entry/types/view/declaration/HTML/CSS/Vite declaration | src contract and strategy guides; fresh control/editor/error/paging/inspection comparisons; unchanged baseline coverage reused |
| All JavaScript and Python regressions | tests contract; current strategy VM test and retained refresh/public/storage evidence; no tests executed |
| Both scripts | scripts contract and verification guidance; current source read, historical results bounded to recorded versions |
| Foundation workflow, CodeQL, Dependabot and issue/PR templates | CONTRIBUTING, issue workflow and corresponding evidence; fresh static configuration reads; F1 navigation omission |
| Static brand kit and desktop icons | Assets/brand/package contracts; 109-comparison identity set includes all non-README asset files; no visual recertification |
| Product, two accepted ADRs, root/agent/component docs and strategy guides/example | Current contracts plus evidence/approval chain; F1–F3 below |
| Verification records and dated reports | Historical evidence, current indexes and applicable later resolutions; older chronology not exhaustively reread |
| Proposal tree | Historical design/review/source-capture ownership; operative issue-4 contract selected through approval/current evidence, not every archived draft made binding |

## docs/egm/README.md

- **F1 — incomplete central inventory/navigation.** Lines 24 and 26 omit `.github/workflows/codeql.yml` and `docs/verification/{issue-29,issue-30,pr-33,pr-34}.md`. These five maintained paths were found independently and included in this audit. CONTRIBUTING and individual records supply substantive ownership, so this is incomplete navigation, not wholly undocumented scanner behavior. Add CodeQL and the four evidence records to the existing ownership rows.
- **F3 — obsolete daily-audit authority.** Lines 5 and 32 and the daily route at lines 56–65 continue to direct recurring reports to exclude issue #4 and withhold mixed-file additions. Connor's 20 November instruction and the saved recurring prompt now require the entire maintained repository on every run. This conflict can mislead a future reader into a partial audit. Update the operative daily route to the full-repository scope; preserve earlier exclusions as dated history, not present instructions. This audit followed the owner's newer instruction.
- **F3 related resolution navigation:** the overview's expanded-pass summary still calls the private-reporting destination unresolved and refers to a resource-error diagnostic gap, although the dated report later records F12's diagnostic correction and F8's reporting-route resolution. Link those final resolution sections and distinguish today's repository-owner link lag (F2) from the older resolved destination-selection problem.
- Previously missing Dependabot, issue-28/31, staged-helper/test, strategy/example/permission ownership and expanded component contracts are present. Those older findings are resolved and are not reopened.

## docs/README.md

- **F1 follow-through:** lines 13–15 explicitly navigate issues 28/31/2/3/4 but omit dependency/scanner and pin-review records 29/30/PR-33/PR-34. Generic verification-directory ownership exists at line 29. Add the current evidence links alongside the existing entries.
- **F3 follow-through:** line 11 explicitly says the recurring Daily Drift Report excludes issue #4. Align this active navigation with Connor's 20 November full-repository scope and the updated scheduled prompt.

## docs/agents/issue-tracker.md

- **F2 — active checkout links/identifier lag repository identity.** Line 3 identifies `moultonlavah/metten`; Queue/filter/view links at lines 61–63 use the former owner. Configured origin resolves `moultonpoint/metten`, and the freshly pinned published workflow uses the current repository/Queue destinations. Stale local instructions can direct agents at an obsolete filter or destination.
- Recommended correction: integrate the already-published destination changes in separately authorized work. This is selected-checkout lag, not evidence that a new upstream repair is needed. Hosted Project settings and redirects were not tested. Dated setup URLs remain historical.

## README.md

- **F2:** active Issues links at lines 13 and 23 use the former repository owner. Align current navigation with the published destinations during authorized integration.
- No other drift detected in current setup, non-trading/session/strategy boundaries or root manifest/toolchain contracts.

## CONTRIBUTING.md

- **F2:** line 7's active open-issues link uses the former repository owner. Align through authorized integration.
- No other drift detected in inspected check commands, manual review/merge requirements, weekly dependency configuration, fixed compiler or scanner/tool limitations. Full-hour strategy checks are present; the earlier omission is resolved.

## SECURITY.md

- **F2:** line 7's active private-reporting URL uses the former owner, while origin and published guidance use moultonpoint. Reconcile the current link through authorized integration; this audit did not prove a broken or working redirect, inspect settings or send/access a report.
- The original F8 reporting-destination decision and private-storage organization were resolved in the 14 November follow-up. They are not reopened by this later active-link lag.

## .github/ISSUE_TEMPLATE/bug_report.md

- **F2:** line 24's private-reporting URL uses the former owner. Include this active entry in the same destination refresh.
- No other drift detected in inspected intake/evidence/authority boundaries.

## .github/ISSUE_TEMPLATE/work_proposal.md

No drift detected.

## .github/pull_request_template.md

No drift detected.

## docs/agents/maintainability.md

- **F2:** line 41's active adoption-review issue link uses the former owner; published guidance uses moultonpoint.
- No other drift detected in the applicable ownership/change/evidence recipe. This report is not the separate adoption trial or changeability audit.

## docs/implementation-plan.md

- **F2:** active issue-sequence links at lines 13–31 use the former owner. Include the sequence in authorized destination integration, preserving issue identities/dependencies and keeping historical evidence unchanged.
- No other drift detected in the implementation-stage/authority distinctions. Live tracker status was not queried.

## METTEN_DESIGN_DRAFT_2026-11-12.md

No drift detected within implemented scope. Prior stale security/integration/format wording is corrected; later financial/provider/release requirements remain planned. External research references remain historical and were not revalidated.

## docs/adr/0001-rust-engine-and-tauri-boundaries.md

No drift detected in the selected implementation's engine/desktop/provider responsibility boundaries. The evidence update is bounded to the original proofs.

## docs/adr/0002-mit-license.md

No drift detected.

## AGENTS.md

No drift detected in applicable authority, instruction discovery and component ownership. Newer explicit owner scope governs this audit.

## docs/agents/verification.md

No drift detected against current helper/failure handling and applicable evidence reuse. No fresh tests or packaged checks were run. Older GUI prerequisites do not override standing unattended-verification instructions.

## docs/agents/code-map.md

No drift detected in the selected checkout's maintained routes, including strategy protocol/evaluator and current test owners. Published navigation for later code was not imported as implemented local scope.

## docs/agents/domain.md

No drift detected.

## docs/agents/triage-labels.md

No drift detected in document correspondence. Hosted labels were not inspected.

## GLOSSARY.md

No drift detected, including Input, Measurement, Evidence and coverage, Purchase option and Format meaning.

## crates/README.md

No drift detected. The former issue-4 omission is resolved in this current contract.

## crates/engine/README.md

No drift detected. Current dependency/API/test/storage ownership includes the completed local strategy scope.

## crates/engine/src/README.md

No drift detected in public session/strategy fields, controlled evaluation, storage revision 3, shared serialization, compatibility/status, paging, response limits and documented side effects, using fresh reads plus the verified prior inspection described above. The diagnostic fix changes the accepted_form Rust representation to Cow while preserving its serialized string contract. No independent resource recertification was performed.

## src-tauri/README.md

No drift detected in current package/configuration/icon/command/permission coverage. Generated strategy permission output is included; the former issue-based exclusion is removed for this audit.

## src-tauri/src/README.md

No drift detected in current Workspace, strict decoders, one-operation admission, RAII release, shutdown ordering, credential boundary and documented tests. Full shared registration/admission/quit interactions were included through fresh reads and byte-verified prior inspection.

## src/README.md

No drift detected in current types/helpers, exact reviewed-text workflow, literal rendering, stale file/preview/list/detail handling, confirmed-save/list-refresh distinction and error guidance. Earlier test-count and strategy omissions are resolved. Rendered layout/accessibility behavior remains unverified.

## tests/README.md

No drift detected in maintained test selection and described boundaries, including strategy VM and staged-helper tests. Source inspection and reused historical evidence do not imply test execution today.

## scripts/README.md

No drift detected against the two scripts, dedicated regressions and bounded issue-31 corrections. Current runner source was freshly read; no helper or native runner was executed.

## .github/workflows/checks.yml

No drift detected in current pinned actions, macos-15/Node-24/Rust-1.100.0 configuration, restricted permissions, both Python checks, frontend/Rust checks, release full-hour target and Mac bundle build. No hosted run was checked.

## .github/workflows/codeql.yml

No drift detected against CONTRIBUTING's documented language/runner coverage, paired init/analyze pin, no-build mode, job-scoped result upload and Rust diagnostics. Swift remains unscanned as documented. F1 records missing central navigation; current hosted enforcement/security state was not checked.

## .github/dependabot.yml

No drift detected in three root ecosystems, weekly schedules and minor/patch version-update groups. Settings/activation evidence remains historical.

## docs/strategies/README.md

No drift detected against the local controlled-rule/version capability and linked guides/example.

## docs/strategies/authoring.md

No drift detected in the learning rule's thresholds, ordering, half-open intervals, remembered side, reviewed-byte save/edit-copy and source/trading limitations. Current guide inspection is supplemented by unchanged guide/example identities and the expanded audit.

## docs/strategies/agent-authoring.md

No drift detected in public parser/requirements/strategy request-response API names, the disposable-harness recipe or described scope. The recipe was read, not executed.

## docs/strategies/reference.md

No drift detected in current grammar, fields/variants, arithmetic/clock, evidence, support limits, paging/status/legacy/error contracts using fresh relevant-section reads and unchanged full-reference prior inspection. The complete teaching examples retain identity-verified prior coverage; no fresh example execution or provider validation.

## docs/proposals/issue-4-egm/strategy-rules-design.md

No drift detected against its operative implemented requirements and subsequent recorded corrections. Current design identity matches the expanded audit; approval/grammar/phase/acceptance portions were inspected afresh. Earlier alternatives, continuation prompts and design-session “not implemented” statements remain historical, not current failures.

## docs/verification/issue-4.md

No drift detected as historical phase-1/2/3 and review-correction evidence. Its current index points to the operative design and later results. The older final evaluator hash is superseded by the diagnostic follow-up's exact matching current hash in the 14 November report. Historical pending GitHub/remote/native statements are not treated as present tracker/runtime state. No fresh resource, package or fill proof is supplied.

## docs/verification/issue-2.md

No drift detected as bounded historical evidence; the later post-review checkpoint supersedes earlier incomplete behavior.

## docs/verification/issue-3.md

No drift detected as bounded historical dummy-only evidence. Finished verification supersedes earlier prompt/cleanup failures; no current process-state claim is inferred.

## docs/verification/issue-28.md

No drift detected as historical action-pin/update-route evidence. Original monthly/actions-only wording does not override later weekly three-ecosystem policy.

## docs/verification/issue-29.md

No drift detected in inspected dependency configuration/settings/proposal evidence correspondence. Historical settings and job observations were not refreshed. F1 records navigation omission.

## docs/verification/issue-30.md

No drift detected in inspected current index/final enforcement observations and retained limits. Missing/failed/incomplete/skipped Rust blocking and positive eligibility remain dated hosted evidence, not a fresh check. Neutral-Rust-job/external-fork limits remain explicit. Older chronology was not exhaustively reread. F1 records navigation omission.

## docs/verification/issue-31.md

No drift detected in current checklist/runner ownership, retained cleanup limitations and history-reader connection correction. Signature/distribution limits remain limitations, not passed acceptance. No private supporting records were opened.

## docs/verification/pr-33.md

No drift detected between recorded setup-node pin and selected Foundation configuration. Pending-at-push and historical hosted results retain their original source bounds. F1 records navigation omission.

## docs/verification/pr-34.md

No drift detected between recorded checkout pin and selected Foundation configuration. Its preservation of CodeQL's distinct older checkout is explicit. F1 records navigation omission.

## assets/README.md

No drift detected within maintained inventory/usage ownership. Product branding remains staged.

## assets/brand/README.md

No drift detected within usage/provenance and unchanged full asset inventory. No visual/export recertification.

## assets/brand/specifications.json

No drift detected against inspected palette, responsive roles and minimum-size contracts. Device/print/platform certification is not supplied.

## assets/brand/licenses/font-source.json

No drift detected in provenance-path/no-font-software-bundled correspondence. External availability or legal rights were not revalidated.

## assets/brand/licenses/Clash-Grotesk-FFL.txt

No drift detected within identity-reused provenance scope. License text was not freshly reread or legally certified.

## assets/brand/licenses/Inter-OFL.txt

No drift detected within identity-reused provenance scope. License text was not freshly reread or legally certified.

## LICENSE

No drift detected against ADR 0002 and package license metadata. Brand/font rights remain separate.

## Historical proposals and dated reports

No additional behavioral requirement inferred from archived drafts, reviews, source captures or dated observations. Their paths are covered by proposal/report ownership and preserved. This pass does not claim an exhaustive new read of every historical body. Operative issue-4 requirements were selected through the evidence/approval chain; no issue-based implementation exclusion remains.

The expanded 14 November report and its later appendices distinguish resolved findings from current drift: former component/test/helper/format omissions, F12 memory-error detail and original F8 private-reporting setup are resolved. Today's F1/F2/F3 describe the selected checkout's current navigation/link/scope inconsistencies. Earlier daily exclusion reports remain historically partial and are not retroactively upgraded.

## Verification limits and final disposition

- **No fresh acceptance tests, other tests or builds ran.** No browser/native app/container was launched, and no credential/account/private-runtime record was accessed.
- This covers every maintained component by fresh inspection or exact-identity prior inspection as recorded above. It does not mean every source/history line was freshly reread. Fingerprints establish correspondence, not runtime correctness or proof completeness.
- Current GitHub issues/Projects/settings/alerts, external redirects, upstream action sources, remote CI and private reports were not inspected. Remote access was limited to published-guidance discovery/refresh.
- Actual native GUI/accessibility behavior, package/signature/update continuity, physical power-loss durability, complete adversarial security, current performance/capacity, provider access, paper/live acceptance and fills remain outside this static audit.
- The only detected current divergences are documentation finding groups F1/F2/F3. No additional missing completed-scope feature, inaccurate maintained API field, logic shortcut or architectural divergence was established by this pass. This is a drift-found result, not a clean or acceptance pass.
- Smallest recommended next action: separately authorized documentation integration for current repository destinations, complete coverage/evidence navigation and the newly authorized full daily scope. This report applies no corrections and creates no tracker work.

Before creation, all 240 starting identities and HEAD were unchanged. No report existed for 20 November at the beginning of this pass. The post-write preservation result is appended below.


Final observed preservation check — 2026-11-20T10:54:34.193517-06:00: all 240 pre-existing paths retain their starting SHA-256 identities; the sole new path is this report (241 total). HEAD remains 868609baffc25f06ecb43d8760e7cdd60f684a68. Published main remained 8bb7aac3704c9b197e58e10ed9256483cbf9f2e8 at delivery refresh. Before this append: 25698 bytes; SHA-256 6034e25e8104289001f1adbc2f1f3873e5a18b9e86a6b2c9d5c8815191146d4a. Exact previous prefix is verified by the append operation.

---

## Owner-authorized documentation corrections — 2026-11-20T14:27:09.620860-06:00

Connor requested correction of the three finding groups after the full audit. **F1, F2 and F3 are resolved locally in this working copy.** The original observations above are preserved; this appendix supplies their current disposition.

- **F1:** Added CodeQL to the EGM CI ownership row, included issue-29/30 and PR-33/34 verification files in its documentation inventory, and linked those four records from docs/README.md. Existing contracts remain the behavior/evidence owners.
- **F2:** Updated active repository destinations to moultonpoint in root README, CONTRIBUTING, SECURITY, bug intake, implementation-plan issue links, maintainability's review link, and issue-tracker's current repository/Queue/view definitions. The dated issue-tracker setup paragraph and historical evidence URLs remain byte-for-byte unchanged. Existing published destination corrections were applied without importing newer application code.
- **F3:** Replaced the obsolete issue-4 daily exclusion with a full maintained-repository route for every day, including weekends, at 09:00 America/Chicago. Both maps now include issue #4, shared files and future maintained additions, require a fresh inventory and explicitly retain private/runtime/generated exclusions and coverage limits. Linked the actual F12/F8 resolution appendices rather than presenting those old findings as outstanding.

Scope: nine existing documentation/template files changed; no application source, tests, dependency/build/CI configuration, older reports or historical proof records changed. No schedule, tracker, branch/index or GitHub mutation. Edits remain uncommitted.

Verification: the focused consistency check passed for exact nine-document scope, F1 inventory/navigation, F2 destinations/historical setup preservation and F3 full daily coverage. All **106 relative file/directory links and Markdown heading anchors** in the nine changed documents resolved. `git diff --check` passed; final diff was reviewed for scope and unrelated changes. All other **232 of 241** pre-existing maintained paths retained their starting SHA-256 identities immediately before this append, including this report's original contents; no files were added or removed during correction.

Guidance/code baseline: selected HEAD remains 868609baffc25f06ecb43d8760e7cdd60f684a68; fresh origin/main query before correction and at delivery returned 8bb7aac3704c9b197e58e10ed9256483cbf9f2e8. Previously loaded immutable root guidance/linked procedures remain applicable and unchanged. No extra application checks were required for these documentation-only edits.

Limits: no tests/builds, browser/native app/container launch, credentials/private records, external-URL/redirect or hosted settings verification, publication or trading. This correction establishes local documentation consistency, not new application acceptance or proof that the next scheduled run has completed. The recurring configuration was already updated and verified earlier in this conversation; this correction did not change it.

Before this append: **26193 bytes**, SHA-256 **69047d876116813c1743112507d2403ee8de699c0d437724e5b7f147ed6343f9**. The append operation verifies the exact prior byte prefix afterward.
