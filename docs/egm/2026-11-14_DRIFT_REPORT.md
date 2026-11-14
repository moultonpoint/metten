# Daily EGM documentation drift report — 14 November 2026

Audit time: 2026-11-14 09:04:29 CST (America/Chicago). Accountable lead: Codex. Result: **documentation drift found; current mixed-file coverage remains limited**, not an acceptance pass.

Scope: the maintained repository outside issue #4, using [the coverage map](README.md), foundation `b1eb58c25fed5a2970bd5c40db9c05f673ff8d71`, and current local HEAD `96ad37a` (initially clean working tree). Read the installed `egm-drift-report/SKILL.md` and its `docs/codex.md`; processed source reads sequentially. No bootstrap, design review, delegation or scheduling was performed.

The completed slice is MIT licensing, controlled persisted non-trading sessions, and isolated dummy-credential integration. Provider connections, market-clock runs, paper/live orders, accounting, backups, distribution and full Gen 1 acceptance are future scope, not missing completed-slice features. Historical observations and already-resolved findings are not new behavioral requirements.

## Coverage and verification limits

- Compared the 222 maintained paths listed by local Git (tracked plus nonignored untracked paths before this report) against the coverage map. Included hidden maintained configuration, tests, manifests, locks, build/CI, native command permissions, tooling and static asset ownership. Ignored private planning/research, runtime data, dependencies, generated schemas and build output were not read.
- For the ten named mixed files, read only current lines matching the foundation through a sequential diff projection; changed intervals were suppressed. Checked retained foundation command names and capability grants by allowlisted membership, without assessing the extra command. Current references include `lib.rs:172–594`, `storage.rs:11–81,238–277`, `main.rs:20–43,123–174,176–269,407–438`, and frontend foundation segments. These are discontinuous retained portions: inserted recorder sharing, schema, initialization, command registration and UI changes are excluded. Textual retention alone cannot establish that excluded additions leave all foundation behavior unaffected. Their interactions remain **incomplete evidence**, not proof of no drift in the complete working copy.
- Extended the exclusion to newly identified issue #4 paths: `crates/engine/tests/strategy_storage.rs`, `tests/strategy.test.mjs`, `docs/strategies/`, and `docs/verification/issue-4.md`, plus additional mixed-file additions in `src/view.mjs`, `src/view.d.mts`, `tests/refresh.test.mjs`, `.github/workflows/checks.yml`, `GLOSSARY.md` and the root README's strategy-facing text. Classification used paths, foundation diff metadata and local commit subjects; no issue #4 acceptance is inferred from its merged status. Proposals, the strategy implementation and dedicated tests, learning example and strategy permission artifact were not opened.
- Read-boundary exception: broad reads of files the map described as wholly in scope incidentally returned strategy-related additions in the view helper/declaration, refresh harness, CI and glossary. Those additions were not evaluated for drift or approved. Subsequent coverage treats them as excluded. This pass therefore cannot claim perfect avoidance of all excluded text; future passes should extend projection filtering to these mixed files before reading them.
- Directly read the foundation session tests, credential adapter and embedded tests, Swift verifier and Python regression, presentation/credential tests, staged-check helper/tests, supporting configuration and contracts. Byte comparison confirmed the session tests, credential adapter, Swift verifier, credential-runner test, presentation test, credential UI test, desktop manifest/config and both lockfiles are unchanged from the foundation. npm lock root requirements match `package.json`. No fresh dependency audit was performed.
- Static coverage included 107 asset/ownership paths, their ownership/usage specifications, provenance/license records, referenced paths and desktop PNG dimensions. Existing static files are unchanged from the foundation except the added assets README. No SVG/raster/PDF rerendering, visual recertification, font-rights certification or platform appearance check was performed.
- Reused only the version-bounded, historical foundation and dummy-only evidence in issues #2/#3. **No fresh acceptance tests, tests, builds, browser/app/container launches, credentials, accounts, provider calls, trading or remote CI checks were run.** No GitHub or public-link requests were made; repository-link findings establish local inconsistency, not HTTP failure or whether redirects/private reporting are enabled.
- This report is the sole intended write. No fixes, staging, commits, pushes, fetches, branch changes or tracker mutations. No private runtime records or secret values are included.

## METTEN_DESIGN_DRAFT_2026-11-12.md

- **Stale security implementation/evidence statement (line 104):** “No security implementation, audit, or credential access test has occurred” contradicts the unchanged dummy credential implementation in `src-tauri/src/credential.rs:260–430`, its focused tests, and the finished dummy-only packaged evidence in `docs/verification/issue-3.md:7–48`. The header already acknowledges that bounded proof. Keep the lack of a full independent security audit explicit, but distinguish it from implementation and credential tests that did occur. This is documentation drift, not evidence that trading is safe or that real credentials were tested.
- **Residual broad future-evidence wording (line 192 and section 16):** the unresolved-evidence list still groups the implementation/build plan and Rust/Tauri integration proof as outstanding; section 16 still says a small integration proof “must validate” the direction. The maintained implementation plan, issue #2/#3 records and ADR 0001's evidence update now supply the bounded initial proof. Clarify the remaining full-product/distribution proof without implying the initial developer integration has never happened. The prior report's header/section-2 corrections remain resolved.

## docs/adr/0001-rust-engine-and-tauri-boundaries.md

No drift detected in the inspected foundation boundary. The 13 November evidence update resolves the initial report's stale-proof finding; full-product and distribution limitations remain applicable.

## docs/adr/0002-mit-license.md

No drift detected.

## README.md

No drift detected in the inspected foundation/setup/configuration portions. Strategy-facing additions and their claims are excluded; this is not a review of the complete current README.

## crates/README.md

No drift detected in the inspected retained foundation portions. Mixed-file interaction limits above apply.

## crates/engine/README.md

No drift detected in the inspected retained foundation portions. Runtime dependency changes and strategy persistence are excluded.

## crates/engine/src/README.md

No drift detected in the inspected retained session control, recording and test portions. Changed recorder sharing and schema/open behavior cannot be certified by this pass.

## src-tauri/README.md

No drift detected in the inspected foundation configuration, generated permissions, command handling and dummy adapter portions. Whole-command registration and mixed initialization remain outside the complete-coverage claim.

## src-tauri/src/README.md

No drift detected in the inspected foundation/dummy portions. Inspected `decode_credential`, `credential`, retained shutdown handling, `Credentials` and embedded tests; excluded shared strategy changes prevent a full current bridge assessment.

## src/README.md

- **Outdated test count (line 36):** it says `npm run check` runs three JavaScript test files, while `package.json:9` selects `tests/*.test.mjs` and the maintained inventory contains four matching paths. The additional issue #4 test is excluded from evaluation; its filename alone establishes that the count is stale. Describe the selection pattern or explicitly bound the count to foundation tests.

## tests/README.md

- **Incomplete maintained-test inventory (lines 5–15):** it introduces “These four files” and omits the in-scope `tests/check-staged.test.py`. That file's `StagedCheckpoint` checks staged-only export, unchanged source index/worktree, missing dependencies, whitespace failure, command failures/cleanup and inherited-index removal. `scripts/README.md:3–5`, `docs/agents/verification.md:11–30` and CI line 20 already reference it; the owning test contract should cover its interface, effects and limits. No execution result is inferred from reading those assertions. The dedicated strategy test remains excluded.

## scripts/README.md

No drift detected. The staged-check addition is documented here and in the verification guide; the separate coverage-map/test-contract omissions are recorded under their owning documents.

## .github/workflows/checks.yml

No drift detected in the inspected in-scope workflow steps: current `macos-15`, Node 24, pinned Rust, locked Cargo checks, frontend checks, native bundle build and the two Python regressions. The added issue #4 acceptance step is excluded. No claim is made about current hosted execution or success. Historical `macos-14` wording in issue #2 is not a current requirement.

## assets/README.md

No drift detected within static ownership/inventory coverage. Rendered output is not recertified.

## assets/brand/README.md

No drift detected within inspected ownership/usage records and unchanged asset inventory. `specifications.json`, font provenance/licenses, master/export paths and desktop icon ownership were accounted for; historical render claims remain historical.

## assets/brand/specifications.json

No drift detected within ownership/usage coverage; deferred physical, print and product-integration work is not a foundation regression.

## assets/brand/licenses/font-source.json

No drift detected within local provenance-record coverage. External font source availability and rights were not newly verified.

## assets/brand/licenses/Clash-Grotesk-FFL.txt

No drift detected within the provenance/no-font-software-bundled contract. No fresh legal certification is implied.

## assets/brand/licenses/Inter-OFL.txt

No drift detected within the provenance/no-font-software-bundled contract.

## docs/README.md

No drift detected in the inspected foundation documentation map. Newly added strategy documentation/evidence is excluded rather than treated as missing foundation documentation.

## docs/egm/README.md

- **Inventory no longer exhaustive (lines 17–23):** the maintained-file table does not assign `scripts/check-staged.py` or `tests/check-staged.test.py` to contracts, although both exist and are referenced in the script/verification guides and CI. Its documentation row also says three `docs/agents/*.md`, but four exist, including `verification.md`. Update the inventory in separate authorized work; the newly documented helper is not undocumented everywhere, but this coverage map's exhaustive claim is stale.
- **Mixed-file exclusion list has not kept up (lines 18–20,29–44):** it assigns all of `src/view.mjs`, `src/view.d.mts` and the regression tests to foundation coverage, but foundation diff metadata shows additions/changes there associated with the excluded strategy work. CI/glossary also have later additions. Explicitly extend the mixed-file boundary so a repeat audit does not inspect issue #4 just because these files were formerly wholly in scope. Until ownership and safe slices are established, their changed portions are a coverage limitation. New dedicated issue #4 files listed above remain excluded under the existing standing instruction.

## AGENTS.md

No drift detected. The root README remains the project page; no `.github/README.md` exists in the maintained inventory. The evidence-navigation guide is linked.

## CONTRIBUTING.md

- **Repository destination inconsistency (line 7):** the open-issues link uses `connorreganmoulton-web/metten`, while local `origin` is `https://github.com/moultonlavah/metten.git` and root README links use `moultonlavah/metten`. Align the maintained contribution entry point; no redirect or remote failure was tested.
- **Incomplete current in-scope CI command list (lines 13–23):** it introduces the commands as the current automated workflow, but omits `python3 tests/check-staged.test.py`, now present at `.github/workflows/checks.yml:20`. This does not authorize running the check in this audit. The excluded issue #4 workflow addition is not assessed.

## SECURITY.md

- **Repository destination inconsistency (line 7):** private vulnerability-reporting link uses the older repository owner while local origin/current root project links use `moultonlavah/metten`. Because this is the private reporting entry point, reconcile its intended destination in separate work. No report was submitted and availability/redirects were not tested; a broken reporting channel is not established.

## GLOSSARY.md

No drift detected in the foundation/product terms through “Trading day.” Later strategy-specific additions are excluded from this pass.

## LICENSE

No drift detected.

## docs/implementation-plan.md

- **Repository destination inconsistency (issue sequence, lines 15–33):** implementation-tracker URLs use the older `connorreganmoulton-web/metten` path rather than local origin/current root project links. Remote redirects and current issue status were not checked. Sequence entries describe planned dependencies, not independently verified current blocker state; issue #4 implementation/proposals remain excluded.

## docs/agents/issue-tracker.md

- **Stale repository identifier (line 3):** its explicit tracker target is `connorreganmoulton-web/metten`; local origin is `moultonlavah/metten`. This guide tells agents to use explicit repository arguments, so the inconsistency can direct later tracker commands to the older identifier even though it may redirect. No remote behavior was tested.

## docs/agents/triage-labels.md

No drift detected. No live label state was queried.

## docs/agents/domain.md

No drift detected.

## docs/agents/verification.md

No drift detected in the inspected staged-helper/evidence-navigation guidance. The issue #4 index link is an excluded pointer, not inspected evidence.

## docs/verification/issue-2.md

No drift detected. Treated as historical observations with its later fixes/checkpoint superseding earlier incomplete local evidence. Old command counts, CI runner labels and pending-remote dispositions are not current implementation contracts or current GitHub status.

## docs/verification/issue-3.md

No drift detected. The finished local dummy-only result supersedes earlier failures and incomplete cleanup/prompt observations. Exact bundle/version limits and the later runner-only fix remain explicit; no fresh acceptance or current process-state inference is made.

## docs/egm/2026-11-13_DRIFT_REPORT.md

No drift detected as a historical observation. Preserved original findings, resolution notes and the later partial-check limitation. The two recorded header/ADR corrections remain resolved; the residual specification wording identified today is distinct. Its former `.github/README.md` section documents an earlier state, not a requirement to recreate that file.

## Remaining coverage disposition

All inventoried non-#4 components have an owning contract or an explicit map/test-contract finding above. No confirmed missing completed-scope feature, incompatible foundation field/symbol, new foundation logic shortcut or engine/desktop architectural divergence was established in the inspected retained code. This is **not** a no-drift result for changed mixed-file interactions or excluded content. The smallest next action is separate documentation maintenance for the findings; issue #4 coverage requires separate owner authorization. No fixes were performed.

---

## Council planning continuation — 14 November 2026

Connor explicitly authorized adding completed issue #4 to drift scope after the original pass. The earlier exclusions and findings above remain historical. This appendix records a plan review, **not an expanded audit result or completed correction pass**. The original report remains partial until the expanded audit is executed. Council scores assess the plan only.

# Revised plan v1 — include completed issue #4
Role: Lead a bounded documentation drift audit and prepare the smallest correction plan.
Authority: Connor explicitly added completed issue #4 to drift scope on 14 November 2026. This supersedes the earlier exclusion for inspection. It does not authorize trading, credentials, builds, tests, publication or source fixes. Current council task reviews this plan and the existing report; only append planning/review records to the dated report. Do not execute documentation corrections during council planning review.

1. Append a new timestamped audit pass to docs/egm/2026-11-14_DRIFT_REPORT.md; preserve the original limited pass and prior resolution history. Record current HEAD and inventory. Use the existing coverage map as a starting point, explicitly overriding its issue #4 exclusion under the new owner instruction.
2. Audit issue #4 sequentially using EGM drift procedure: start with docs/verification/issue-4.md current evidence index and its latest completion/review-correction records; read docs/strategies/{README,authoring,agent-authoring,reference}.md, examples/learning.json, product specification, accepted ADRs and the accepted issue #4 contract/design pointed to by evidence. Treat proposals and old review records as historical unless completion evidence identifies an operative contract. Compare all current strategy engine/model/validation/evaluation/resource-bound code, storage/schema/version APIs, desktop commands/permissions/build registration, UI/types/helpers, tests and CI to those contracts. Read shared files in full now that issue #4 is included. Compare file inventory back to contract ownership. Reuse the first pass's unchanged non-#4 inspections, but recheck every previously suppressed shared interval; unchanged coverage must be tied to actual HEAD/file identity, not merely an old passing result. List any unread/uncertain area as a coverage gap; never label it clean.
3. Report differences per inspected document with source locations, or scoped No drift detected. Completed issue #4 includes local strategy import/preview/explanation, immutable versions, public rule evaluation with controlled inputs, authoring docs and recorded local acceptance; actual provider access, integrated paper/live runs and later product work remain future scope. Separate local validation from provider/trading readiness, historical failures from latest completion, and documentation mismatch from possible behavior defect. Do not copy a potentially incorrect implementation into a contract to silence drift.
4. Prepare a finite documentation-only correction list after the expanded audit. Known candidates: stale bounded-evidence wording in product specification; current coverage/ownership in docs/egm/README.md and root/component documentation that still excludes completed issue #4; missing staged-helper ownership/test/check instructions; count wording replaced by selection patterns where practical; repository destinations aligned with local origin. Only update current normative docs; historical reports/proposals/verification snapshots retain their original text. Existing old limitation statements may remain historically true; do not globally replace every mention of issue #4 exclusion. A new substantive source or product-contract discrepancy is reported separately, not fixed in this session.
5. A later authorized correction pass may edit only the specific confirmed lines in the above current docs and their direct owning contract, plus append a resolution note to the dated report. Candidate files: METTEN_DESIGN_DRAFT_2026-11-12.md, docs/egm/README.md, README.md, crates/{README.md,engine/README.md,engine/src/README.md}, src-tauri/{README.md,src/README.md}, src/README.md, tests/README.md, CONTRIBUTING.md, SECURITY.md, docs/implementation-plan.md, docs/agents/issue-tracker.md, docs/README.md. Do not edit all candidates automatically: use only those with confirmed findings. Preserve every behavioral requirement, source file, test, config, generated artifact and unrelated change. Do not create per-directory scaffolding or a new tool.
Validation: compare each proposed/actual wording change to the named existing evidence/source, validate local relative links and inventory/ownership, scan the allowed diff for accidental scope expansion, verify only authorized report/docs changed. Remote destination consistency is not remote HTTP/private-reporting availability. No fresh tests/builds/native/browser/credential operations; prior execution evidence remains version-bounded.
Done: revised audit covers all maintained implemented components through issue #4 or explicitly lists gaps, and each confirmed documentation finding has a precise minimal correction or justified deferral. Council average is the arithmetic mean of all five actual scores in the same complete round; target >=9.0, at most three rounds with confirmed findings addressed between rounds. A score is confidence in the plan, not acceptance-test proof. Stop without claiming success if seats/evidence are missing or the threshold is not met. No staging/commit/push/PR/tracker writes/scheduling.

## Council round 1 — completed
All five reviewed the same original report and plan v1 independently; no source audit or implementation was delegated.

| Reviewer / connection | Plan score |
| --- | --- |
| GPT-6.1 Sol / ChatGPT subscription | 9.2/10 |
| Inkling 2 Small / OpenRouter Free | 7.0/10 |
| MiniMax M3.1 Flash Free / OpenCode Free Trial | 8.5/10 |
| GLM 5.3 Flash Free / OpenCode Free Trial | 9.0/10 |
| DeepSeek V4.1 Flash Free / OpenCode Free Trial | 7.5/10 |

Arithmetic mean: **8.24/10**. Five seats complete; no substitutions. This score applies only to plan v1.

### Lead adjudication of material findings
- **Confirmed, consolidated:** Sol's repository-intent finding and Inkling F2: local origin/current root links establish inconsistency, not intended private-reporting destination (original SECURITY.md finding). V2 step 4 defers that destination unless exact intent is already authorized; ordinary destinations are reconciled against project identity.
- **Confirmed as useful execution precision:** Inkling F1, DeepSeek R3: v1 already required recording inventory and comparing it back to ownership, but did not specify its derivation. The original report's coverage-map finding supports explicit Git-derived enumeration. V2 steps 1–2 make the map a cross-check, not inventory authority.
- **Confirmed as useful precision, not new work:** MiniMax R1/R3, GLM R1, DeepSeek R1/R4: v1 categories/file allowlist covered the agents count, two product-evidence statements and contribution checks but did not individually trace them. V2 step 4 names each and requires disposition of every original/expanded finding; no speculative edit is added.
- **Disproved as a v1 instruction conflict; clarified anyway:** MiniMax R2 claims v1 permits a foundation-only test-count fix, but v1 says replace count wording by selection patterns; the alternative appears in the historical report, not v1. V2 explicitly chooses tests/*.test.mjs and excludes that stale alternative.
- **Confirmed:** Inkling F3: preservation was required, but validation lacked a byte-preserved-prefix check. V2 validation makes append-only verification explicit.
- **Confirmed as clarification:** Inkling F4 and MiniMax R4: new owner scope supersedes old projection advice; current ownership/exclusion instructions must be replaced appropriately, while history stays intact. V2 authority and steps 2/4 state that distinction.
- **Confirmed as execution precision:** MiniMax R5 and GLM R2's stale-evidence concern: v1's file-identity requirement was correct but underspecified. V2 checks HEAD, local diffs and per-file identity before reuse, rereading changed/uncertain portions or recording gaps.
- **Disproved as necessary gate:** GLM R2's clean-tree precondition and DeepSeek's optional clean-tree gate. A report-only untracked file already exists; unrelated local work does not invalidate unchanged file bytes. V2 preserves concurrent changes and verifies relevant file identity instead. No reset/checkout/commit requirement is introduced.
- **Not established as an actual privacy permission:** MiniMax R6 says full shared-file reads could include generated/private paths; v1 refers to maintained source and inherits the original exclusions, not arbitrary runtime paths. V2 step 1 nevertheless repeats the exclusions explicitly to prevent ambiguity.
- **Confirmed:** GLM R3: record owner scope override in the executed audit pass, not just its plan appendix (v2 step 1).
- **Confirmed:** DeepSeek R2: expected completed issue #4 boundary needs source authority. The local current-evidence index at docs/verification/issue-4.md:1–29 identifies completed local phases and the operative approved phase sequence. V2 step 2 requires confirming/citing the boundary from current evidence and operative contract. It does not infer provider/trading readiness from completion.
- Optional suggestions were adopted only where they close these same gaps: concise ownership/status groups in the existing report, exact finding/file mapping and correction-time identity recheck. No new tracker, clean-tree gate, remote security check, code repair or test/build run was added.

# Revised plan v2 — expanded audit, then minimal correction proposal
Role: Accountable lead for a documentation drift audit through completed issue #4, guided by Ponytail full.
Authority and delivery: Connor explicitly added issue #4 on 14 November 2026. This authorizes its inspection and supersedes the old exclusion/projection-filter recommendation for the new pass. The current task is council review of this plan and the existing drift report. Only append planning/review records now; executing the expanded audit is the next phase, and correcting other documentation is a separate authorized pass. Scores assess plan quality only.

1. Establish the new pass and inventory. Append a timestamped pass to docs/egm/2026-11-14_DRIFT_REPORT.md when the audit executes; explicitly cite Connor's 14 November scope override, record current HEAD and working-tree status, and preserve original observations. Re-derive the maintained inventory from local Git: tracked plus nonignored untracked paths. The existing docs/egm/README.md is a cross-check, not the inventory authority. Exclude ignored/private research/planning, secrets, credentials, runtime records, .git internals, dependencies, generated schemas/build output and temporary resources. Include hidden maintained config. A maintained path lacking an owning contract is a finding.

2. Establish requirements and inspect source sequentially. Apply the installed egm-drift-report skill, discovered from available skills, including its Codex integration. Read METTEN_DESIGN_DRAFT_2026-11-12.md and accepted docs/adr/*.md. Start issue #4 at docs/verification/issue-4.md's current evidence index; follow its latest completion/review-correction records and operative approved contract/design. Read docs/strategies/{README,authoring,agent-authoring,reference}.md and examples/learning.json. Archived proposals/reviews/checkpoints explain history; they become requirements only when the operative approved contract establishes that authority. Merged/closed status alone is not acceptance evidence. The expected completed slice (local import/preview/explanation, immutable versions, controlled-input pure-rule evaluation, authoring docs and bounded local acceptance) must be confirmed and cited against that contract/evidence, not asserted as a substitute for it. Provider access, integrated paper/live trading and later release features remain future work unless those same authoritative records establish otherwise; do not silently redefine product scope.

Read current maintained issue #4 source/tests/config end to end in bounded sequential ranges: strategy models/validation/evaluation/resource bounds; engine public APIs and storage/schema; desktop commands/permissions/build registration; UI/types/helpers; tests and CI. Read previously mixed files in full now that issue #4 is included. This supersedes the earlier filter advice only for the new authorized scope; previously suppressed or incidentally exposed portions remain uncertified until read. Compare documentation to implementation AND maintained paths/logic back to documentation. No new auditor tool or scaffolding.

Reuse unchanged non-#4 inspections only after checking actual current file identity against the first-pass HEAD 96ad37a and its recorded foundation b1eb58c comparisons. Check HEAD, local changes and per-file diff/byte identity, including session tests, credential adapter, Swift verifier, credential-runner/presentation/credential-UI tests, desktop manifest/config and both lockfiles. Same HEAD alone is insufficient if files changed. If identity is different or uncertain, inspect the affected content or explicitly record the gap; preserve concurrent work, with no clean-tree gate, reset or checkout.

3. Write the expanded result. Section per inspected document, with specific divergence and source location or a scoped No drift detected. Carry forward unchanged prior observations by explicit evidence reference; distinguish completed foundation/issue #4 from future features, historical failures from latest finished evidence, and documentation errors from possible code/requirements defects. Maintain a concise path/group-to-contract coverage table with status or gap in the existing report, using package groups where sufficient. Any unread source, uncertain ownership or unverified interaction is a named gap, never a clean result. Preserve all earlier report text; do not rewrite the first pass as if it included issue #4. Do not copy an incorrect implementation into a contract to make drift disappear.

4. Prepare the smallest finite correction list. Map EVERY original and expanded finding to confirmed / disproved / unresolved, citing the current source/evidence and the proposed exact file/section or justified deferral. The audit, not these examples, determines the final list. Known report items to re-confirm:
- METTEN_DESIGN_DRAFT_2026-11-12.md:104: acknowledge implemented/tested dummy credential proof while retaining the missing full independent audit; line 192 and section 16: distinguish completed initial integration/build planning from outstanding full-product/distribution proof. Preserve all behavioral requirements.
- docs/egm/README.md: add staged-helper/test ownership, remove the fragile three-agents count, and replace the operative issue #4 exclusions with current ownership assignments. Update the repeated audit-request text too. Retain foundation ownership and audit cautions; historical reports/proposals keep their original wording.
- Current root/component READMEs and docs/README.md: amend only confirmed present-tense scope exclusions or omitted ownership now that issue #4 is included. No global replacement of historical exclusions.
- src/README.md:36: describe tests/*.test.mjs selection, not a foundation-only count. tests/README.md: include check-staged.test.py's purpose/effects/limits and complete current test ownership. CONTRIBUTING.md: add its omitted staged-check CI command. Do not edit scripts/README.md or docs/agents/verification.md merely to duplicate coverage already correct.
- Reconcile current ordinary project/tracker links in CONTRIBUTING.md, docs/implementation-plan.md and docs/agents/issue-tracker.md against intended project identity, using local origin/current project links as evidence. SECURITY.md's private-reporting destination remains a finding/deferral unless intended destination is established by existing exact owner authority; local origin alone is insufficient. No remote verification or private report submission.

5. Bound any later correction pass. Re-confirm findings against its starting HEAD/current content. Edit only confirmed lines in current owning docs named by step 4; conditional candidates are METTEN_DESIGN_DRAFT_2026-11-12.md, docs/egm/README.md, README.md, crates/{README.md,engine/README.md,engine/src/README.md}, src-tauri/{README.md,src/README.md}, src/README.md, tests/README.md, CONTRIBUTING.md, SECURITY.md (only if its destination condition is satisfied), docs/implementation-plan.md, docs/agents/issue-tracker.md and docs/README.md. New confirmed source/behavior/product-requirement defects or changes beyond these documentation boundaries are reported for separate bounded work; do not repair source, change requirements or expand the edit list silently. No new tools, dependencies, features, folder READMEs or unrelated cleanup. Append a resolution note only for corrections actually made; deferred findings remain open.

Validation and stop: Check changed wording against named existing evidence/source; local relative links and inventory/ownership; report diff is append-only with the preexisting report prefix byte-preserved; allowed-file diff/scope and concurrent-work preservation. No fresh tests/builds, browser/app/container launch, credential/account/private-runtime access, trading, network/GitHub operations, stage/commit/push/PR, scheduling or extra tasks. Existing execution evidence stays historical/version-bounded. Done means each maintained component through issue #4 has inspected coverage or an explicit gap, and each finding has a precise minimal correction or evidenced disposition. A partial audit remains partial. Council target: arithmetic mean >=9.0 from all five actual scores in the SAME complete round, maximum three rounds; no missing-seat average, acceptance proof or unanimity claim. Stop at the budget if unmet and report limitations.


---

Pre-append preservation baseline: 34157 bytes; SHA-256 cac6a1c1d8c05693f66ca63cb78eba9283e938ba0cb0442f90d0ac757d1ad6ab.

## Council round 2 — final result
All five seats completed a fresh review of revised plan v2. No model substitutions or failed/missing seats.

| Reviewer / connection | Round 1 | Round 2 |
| --- | --- | --- |
| GPT-6.1 Sol / ChatGPT subscription | 9.2/10 | 9.5/10 |
| Inkling 2 Small / OpenRouter Free | 7.0/10 | 9.1/10 |
| MiniMax M3.1 Flash Free / OpenCode Free Trial | 8.5/10 | 9.1/10 |
| GLM 5.3 Flash Free / OpenCode Free Trial | 9.0/10 | 9.4/10 |
| DeepSeek V4.1 Flash Free / OpenCode Free Trial | 7.5/10 | 9.0/10 |

Round 2 arithmetic mean: (9.5 + 9.1 + 9.1 + 9.4 + 9.0) / 5 = **9.22/10**. The requested 9.0 average is met for the reviewed v2 plan. This is not unanimous unconditional approval: Sol had no required corrections; Inkling and GLM recommended small clarifications; MiniMax and DeepSeek attached conditions. All reviewed the report/plan only; none verified source or acceptance. DeepSeek disclosed that its round-2 comparison did not reread v1; it assessed v2 and recorded dispositions, sufficient for the v2 plan score but not an independent reconstruction of round 1. Native Sol's seat identity is the requested tool target; the child did not independently verify subscription routing.

### Lead adjudication of remaining material comments
- **Confirmed, narrow clarification — DeepSeek F1:** "credentials" in v2 step 1 means credential secret values and private credential/runtime records, not maintained credential adapter source/tests/docs. The original report covers those maintained files; retain them in scope.
- **Not established as omissions — Inkling's mixed-file item and DeepSeek F2:** v2 step 2 already says read previously mixed files in full and step 4 requires every finding's disposition. Nevertheless the concrete read checklist below names the root README, GLOSSARY.md, view helper/declaration, refresh test and CI, along with the ten original mixed paths. No new component or code work is added.
- **Disproved as a generated-file collision — MiniMax finding 1:** v2 excludes generated *schemas/build output*, not every path containing "autogenerated". The original desktop contract explicitly calls maintained command permissions generated outputs; src/view.d.mts is a maintained declaration, not a generated schema. Keep both in the read checklist; do not weaken private/generated-output exclusions to exempt an arbitrary whole strategy set.
- **Disproved as an unhandled correction surface — MiniMax finding 2:** v2 step 5 expressly routes findings beyond its conditional edit list to separate bounded work; step 4 requires every finding's exact file/section or justified deferral. New findings in docs/strategies, examples/learning.json or GLOSSARY.md must be logged with that disposition. Their inspection is authorized; silently adding them to the edit list is not the smallest bounded plan.
- **Confirmed — MiniMax finding 3:** a byte-preserved-prefix check needs a baseline independent of Git tracking. Record report byte count/SHA-256 before each append and verify that prefix afterward. This continuation records its own baseline below; no report staging/commit is needed.
- **Not a present contradiction — Inkling's old disposition concern:** the original next-action paragraph was true under the former exclusion; v2 authority and steps 1/3 explicitly supersede it for the new pass without rewriting history. The new audit header should say which earlier scope/coverage disposition it supersedes and only claim expanded coverage actually inspected.
- **Confirmed as correction consistency — GLM:** the coverage map's repeated request still excludes issue #4 (read in this planning continuation). If updating that map's operative scope, re-confirm and update that repeated instruction in the same minimal change; otherwise the map would instruct the next audit to revert. This is not an unrelated edit or a reason to rewrite history.
- **Partly confirmed — DeepSeek F3:** the installed drift skill and Codex integration were actually read earlier in this session, so there is no current missing-skill blocker. Record their discovered paths again in the next pass for traceability. If unavailable, report blocked/incomplete procedural evidence rather than silently substitute or claim an EGM pass.
- **Confirmed — DeepSeek F4:** end-of-audit identity reconciliation makes the existing preservation principle checkable. Recheck HEAD, inventory and inspected-file identities before finalizing; re-inspect affected changed portions or mark a version/coverage gap. No clean-tree gate, reset, new monitor or extra tool is required.
- Optional disposition-location advice is adopted by keeping all finding dispositions in the same appended audit pass. Skill paths and baseline checks are recorded as execution details, not new proof or new work.

### Minimal execution notes for v2
These lead clarifications resolve the narrow comments above; they have not received a separate council score. No third round is needed for wording-only details after the requested five-seat mean was reached.

1. Before append, capture the report's existing byte count and SHA-256; after append compare exactly that prefix. Preserve original pass and all council history.
2. Scope includes maintained credential source/tests/docs, generated command permission TOMLs and src/view.d.mts; excludes secret values/private records and ignored/generated schemas/build resources.
3. The complete read checklist includes root README.md, GLOSSARY.md, src/view.mjs, src/view.d.mts, tests/refresh.test.mjs and .github/workflows/checks.yml, in addition to the ten original mixed files and dedicated issue #4 files. Nothing previously suppressed is certified by mere inclusion on this list.
4. Reconcile final HEAD/inventory/inspected-file identities with audit-start identities. Resolve affected reads or record explicit gaps before publishing an audit conclusion.
5. Keep every expanded finding and disposition in the new pass, including proposed edits beyond the current conditional list as deferred separate work. State the new owner-authorized scope supersedes only the earlier scope/next-action disposition, not the recorded observations.
6. Installed procedure read: /Users/connormoulton/.codex/plugins/cache/egm/egm/0.1.1/skills/egm-drift-report/SKILL.md; Codex integration: /Users/connormoulton/.codex/plugins/cache/egm/egm/0.1.1/docs/codex.md. Rediscover if installation changes; missing procedure evidence is reported plainly.

### Delivery status
**Prepared locally:** expanded issue #4 audit plan and council review record. **Not executed:** expanded audit or documentation corrections. The original audit is still partial; issue #4 inspection is now owner-authorized but not established by council scores. Only the dated report was appended during council planning; no source/design/current contract edits, tests/builds, app launches, accounts, remote operations, tracker writes or publication. No further council round or task is scheduled. Next bounded action is executing the expanded audit, then applying only the precise confirmed documentation correction list under the applicable authority.

---

# Expanded maintained-repository audit — 2026-11-14T10:23:24.828653-06:00

Accountable lead: Codex. **Documentation drift found. Static implementation inspection covers the maintained implemented components through issue #4, with the explicit evidence limits below. This is not fresh application acceptance or a security/resource recertification.** Documentation corrections have not been applied.

## Authority, identity and method

Connor's 14 November 2026 instruction explicitly includes completed issue #4, including shared files. For this pass it supersedes the first pass's issue-4 exclusions, projection filtering, mixed-file coverage disposition and statement that separate inspection authority was still needed. It does not supersede historical observations, rewrite council records, or authorize repairs/operations. V2's five-reviewer mean of 9.22/10 concerns the plan only; the lead clarifications were not separately scored. No council was rerun.

Working directory: `/Users/connormoulton/Desktop/metten`. HEAD at initial check, fingerprint capture (2026-11-14T10:19:07.594612-06:00) and pre-append reconciliation: `96ad37ab547751f1f3b4b55d392bc42be3563e9c`. Status throughout: only `?? docs/egm/2026-11-14_DRIFT_REPORT.md`. Inventory was independently derived using `git ls-files --cached --others --exclude-standard -z`: **223 distinct paths, including the existing report; 222 tracked paths**. Hidden workflow/ignore configuration is included. The map was a cross-check, not the inventory authority. No clean-checkout prerequisite was imposed.

Read the installed procedure at `/Users/connormoulton/.codex/plugins/cache/egm/egm/0.1.1/skills/egm-drift-report/SKILL.md`, its `../../docs/codex.md`, Ponytail full, repository AGENTS.md and docs/agents/verification.md. Source was read sequentially in bounded ranges; missing portions of truncated output were retrieved for the implementation/operative comparisons claimed here. Historical proposal text was not promoted into requirements merely because it was returned by a read.

Every tracked working-file byte was compared to its blob at the first report's HEAD `96ad37a`: **no differences**. A separate comparison to foundation `b1eb58c25fed5a2970bd5c40db9c05f673ff8d71` found **135 tracked paths byte-identical**. This verifies the first pass's foundation comparisons for session tests, credential adapter/embedded tests, Swift verifier, credential-runner/presentation/credential UI tests, desktop manifest/configuration, locks and static assets. Their unchanged first-pass inspections are reused, not inferred from HEAD alone. The staged helper and regression were read afresh. All previously suppressed shared implementation files were read in full.

Before this append: **41428 bytes; SHA-256 `1da231767d657feecbc352ade2c75042179f8e527adfa004550e9e493a521734`**. All 223 start fingerprints and inventory paths matched immediately before append. The final preservation check compares this exact byte prefix and rehashes every other maintained path.

## Completed-scope authority and historical evidence

The product specification and accepted ADRs remain authoritative. The operative issue-4 design is [strategy-rules-design.md](../proposals/issue-4-egm/strategy-rules-design.md), especially Detailed Implementation items 1–8 and ordered phases (lines 325–737). Its approval chronology is at lines 739–759. The [current evidence index](../verification/issue-4.md#current-evidence-index) identifies that sequence and later actual implementation authority/results.

Completion is established by the original local records, not issue/merge status: phase-1 integrated acceptance (issue-4.md:1014–1046), phase-2 implementation (1053–1125), guide-led authoring (1136–1258), and the later named native walkthrough and acceptance (1298–1377). The native record supersedes earlier targeting blockers and incomplete phase-3 tables. Subsequent review corrections (1401–1484) update evaluator semantics and affected resource evidence; their final source hashes match this checkout:

| Current file | SHA-256 |
| --- | --- |
| crates/engine/src/strategy.rs | `c74010cbd3171f20bb56668096d4f9c5e43d749e3ce60340c7f81411c304114f` |
| crates/engine/tests/strategy.rs | `e18c43cb9f1316731cb844b47fadfb0e3b72dda13236d5a06d8b8360171efa89` |
| Operative strategy-rules-design.md | `9b5f1d2738fad3f77c99d70ea3cfe6e7436b370e07a5056daa08dc23f3fc872d` |
| examples/learning.json | `a79bd747078601cd16bc292c7896cdcb9f2fe3db0feebf412284a50a2bd50aa5` |

The design's current hash differs from the older phase-1 design fingerprint; the current design was read directly, including its phase-2 registration correction. The older hash is not represented as current identity.

Completed issue #4 means supported format-2 validation, generated explanations, pure controlled-input evaluation, local preview/save/list/detail/edit-copy, immutable version storage/migration, guides and bounded local proof. Desktop Start/Pause still controls only the non-trading session. Provider capability/access, actual market collection, integrated paper/live execution, financial recovery, production capacity, release/security and actual fills remain future gates. Revision-3's older proposed contract is explicitly unapproved and conflicts with later adopted history/source/priority decisions; its larger vocabulary, market-only history and global-source gate are not operative requirements.

Issue #2/#3 evidence is reused only within its original source/version and dummy-only limits. The issue-4 record reports later passing checks and native observations; this audit does not rerun or independently certify those results, access their ignored private/disposable records, or establish current GitHub/CI/runtime state.

## Coverage and ownership reconciliation

| Maintained paths / behavior | Owning evidence or contract | Inspection disposition |
| --- | --- | --- |
| Root manifests, locks, toolchain, ignore/config | root README; CONTRIBUTING; product/ADRs | Current config read; lock identities verified; no dependency audit. |
| Engine lib.rs and storage.rs | engine READMEs; operative design items 3/4; issue-4 phase 2 | Entire files read, including recorder sharing, public exports/protocol, revision 3, guards, pagination, detail classification and save/quit handling. Current README ownership incomplete: F3. |
| strategy.rs, strategy.rs integration tests | operative design items 1–3/6/8; author reference; issue-4 proofs/corrections | All 7,192 source lines (including embedded tests/accounting) and all 3,043 public-test lines read sequentially. F12 below. |
| strategy_storage.rs; session.rs | design item 4/8; engine contracts; issues #2/#4 | Storage suite read in full; session suite identity-verified reuse. No tests executed. |
| Desktop main.rs, build.rs, capability and four maintained permission TOMLs | desktop contracts; design item 4 and registration amendment | Entire current files read, including decoder, one-operation guard, quit waiting, command registration and authority assertions. F3. |
| credential.rs and its embedded tests; desktop manifests/icons | desktop/credential contracts; issue #3 | Byte-identical foundation reuse; main.rs interactions freshly read. No secrets/private records accessed. |
| src main/types/view/declaration/CSS/Vite declaration; index.html | src contract; design item 4; authoring flow | All read in full. Stale-result handling, literal rendering, reviewed-byte flow, save/list separation and string IDs compared. F3/F4. |
| tests/*.test.mjs, credential-runner.test.py, check-staged.test.py | tests contract; design item 8; verification guide | Full refresh/strategy and staged-helper test reads; unchanged view/credential/runner tests reused by identity. F5. |
| scripts/* and CI | scripts README; verification guide; workflow | Helper read; Swift identity-reused; entire CI inspected, including ignored release-hour target. F6 contribution omission. |
| docs/strategies/*, examples/learning.json, GLOSSARY | operative design item 7; shared reference | All guides/reference/example/glossary read and compared to current types/validation/explanation/evaluation and UI. |
| Product, accepted ADRs, agent guides, contribution/security/implementation indexes | Existing owning documents | Current relevant text inspected; ordinary/private destination distinction retained. F1–F11. |
| assets/** and desktop icons | assets README, brand README/specifications/provenance/licenses; desktop contract | Inventory/ownership and verified-identity reuse only. No rendering/export or legal recertification. |
| docs/verification/*, dated EGM reports | Issue evidence/current-state navigation; EGM report ownership | Historical status/version-bounded records. Latest operative #4 completion/corrections read; earlier failures are not current blockers. |
| docs/proposals/** | Issue-4 approval/evidence chain and design artifact references | Current operative design inspected; other drafts, review captures, intake and source captures inventoried as historical evidence. Blanket current map exclusion is F2/F10. They are not all re-reviewed. |

The Git-derived inventory is fully assigned by these groups. Missing **current map/component ownership** is confirmed for completed strategy work and staged-helper tests; a path being described in a historical design does not make the current coverage map exhaustive. No additional maintained application file was found outside these groups.

## METTEN_DESIGN_DRAFT_2026-11-12.md

- **F1 — confirmed:** line 104 still says no security implementation/audit/credential test has occurred. The implemented dummy adapter and finished issue-3.md:7–48 contradict that absolute statement. Smallest correction: replace only that evidence sentence with bounded implemented/tested dummy-credential status and retain the outstanding full independent security review.
- **F1b — confirmed:** section 15's initial integration/build-plan statement (line 192) and section 16's initial proof wording (line 205) omit the completed limited integration. Link existing issue #2/#3 evidence and distinguish full-product, distribution and security proof still outstanding.
- **F11 — confirmed, expanded status omission:** section 4 says exact grammar remains engineering work (line 38), section 9 says source syntax is unresolved (line 101), section 10 leaves exact strategy schema/version identifiers open (line 136), and section 15 lists grammar plus SQLite schema without distinguishing completed rule/version work from future financial storage. Current format-2 grammar/meaning, local schema revision 3 and immutable versions are documented/implemented. Smallest correction: add narrowly scoped references/status qualifications at those statements and the top evidence pointer; preserve all future product obligations. Do not claim the general source/financial schema or full Gen 1 is complete.

## docs/adr/0001-rust-engine-and-tauri-boundaries.md

No drift detected within the implemented boundary inspected. The engine has no desktop/provider client dependency; native code requests engine operations and UI displays results. The existing evidence addendum remains bounded. No new ADR is needed for documenting completed #4.

## docs/adr/0002-mit-license.md

No drift detected within licensing selection/ownership. Unchanged LICENSE and asset-license limits remain separate.

## README.md

- **F3 — confirmed:** lines 70 and 81 still explicitly exclude/describe unfinished issue #4, despite the accurate early-development notice and existing guide links at the top. Replace just the operative baseline/scope language and add the strategy boundary/evidence link to the implementation map. Keep the controlled-session and no-trading limitations.

## crates/README.md

- **F3 — confirmed:** overview/interface/verification describe only the foundation and excluded ongoing #4 (line 7 onward). Add the already completed controlled rule/version boundary and link the engine's direct contract/reference/evidence; remove current exclusions without rewriting historical proof.

## crates/engine/README.md

- **F3 — confirmed:** lines 7–13 exclude source/tests/import/schema and describe serde_json's runtime promotion as excluded work. Cover current runtime serde_json, exported strategy request/results, parser/evaluator and strategy/storage test ownership. Keep the engine independent of transport and desktop session execution. Link existing contracts rather than duplicate the grammar.

## crates/engine/src/README.md

- **F3 — confirmed:** line 7 excludes strategy.rs and shared changes; schema guards/side effects/verification also stop at foundation. Add concise ownership of ValidatedStrategy/Evaluator, Engine::strategies(_unless_quitting), shared recorder serialization, revision-3 migration, immutable metadata/status/paging and the existing rule/storage checks. Link the fixed numeric/resource semantics and historical proof. Do not advertise algorithmic slices as real-time responsiveness.

## src-tauri/README.md

- **F3 — confirmed:** lines 12–15 and command/build/capability/permission rows omit completed strategies. Current build.rs registers four commands; capability grants allow-strategies; strategies.toml contains generated allow/deny declarations. Add that narrow command and native one-operation/quit boundary; retain generated-file and local-main-only restrictions. Replace current “unfinished issue #4 tree” evidence wording with exact recorded limitations.

## src-tauri/src/README.md

- **F3 — confirmed:** line 10 and interface/quit table omit StrategyAdmission/StrategyGuard, tagged decoder, blocking strategies command and quit's strategy wait. Add their current effects and existing boundary tests, linking engine/author reference. Credential operations remain dummy-only.

## src/README.md

- **F3 — confirmed:** line 7 and interface omit completed strategy editor/file/preview/save/list/detail/copy behavior, strategy types and strategyFailure in view.mjs/view.d.mts. Add concise ownership of these functions/state, stale-generation handling, literal rendering and uncertain-save/refresh separation; link the tutorial for user flow.
- **F4 — confirmed:** line 36 says three JS test files; package.json selects `tests/*.test.mjs`, which currently selects four. Use the selection pattern, not another count.

## tests/README.md

- **F5 — confirmed:** “These four files” and issue-4 exclusion (line 5) omit both tests/strategy.test.mjs and tests/check-staged.test.py. Add the two entries and their boundaries: controlled DOM/native-response strategy flow, and disposable staged-tree/index-preservation/failure/cleanup checks. Add the separate Python helper check in Verification. Use descriptive test groups/patterns rather than a fragile total. Do not present fake DOM checks as packaged acceptance.

## scripts/README.md

No drift detected in current helper ownership and verifier limits. Its staged-helper link, command and effect limits already exist; no duplicate correction needed.

## .github/workflows/checks.yml

No drift detected in inspected static command wiring. Includes both Python regressions, four-file JS selection through package.json, workspace checks, the release ignored-hour test target, Clippy and app bundle build. No hosted execution result inferred.

## docs/README.md

- **F10 — confirmed:** current map omits strategy guides, issue-4 evidence and verification.md; line 13 says no proposal is an approved implemented contract for the report. Add the guides/evidence/verification navigation and identify the operative design via its approval/evidence chain while keeping other proposals historical. Do not reclassify the whole proposal tree as requirements.

## docs/egm/README.md

- **F2 — confirmed:** title/overview/map, Explicit issue #4 exclusions, shared-file list and repeated audit request all enforce the old scope. Replace these operative exclusions with owner-authorized completed-#4 scope and assign strategy source/storage/tests/docs/example/permission and all shared paths.
- **F2b — confirmed:** map lacks scripts/check-staged.py and tests/check-staged.test.py and says three agent guides rather than the maintained group of four. Assign helper/test to existing script/test/verification contracts and use `docs/agents/*.md` rather than a fragile count.
- The original suggestion to extend projection filtering is **disproved as the next action under current authority**. Full shared-file inspection replaces it; historical reports keep their limitations intact.

## CONTRIBUTING.md

- **F6 — confirmed:** current workflow list omits `python3 tests/check-staged.test.py` and the release ignored-hour strategy command present in checks.yml. Add exactly those existing commands (or link the workflow as exhaustive authority and label any short list as examples).
- **F7 — confirmed:** line 7 uses the older repository owner; root project links and local origin consistently use moultonlavah/metten. Align this ordinary issues link. No redirect/HTTP failure is established.

## SECURITY.md

- **F8 — unresolved intended destination; confirmed local inconsistency:** line 7 names connorreganmoulton-web/metten while ordinary project links/origin name moultonlavah/metten. No exact owner instruction establishing the intended private reporting destination was found in the inspected authority chain. **Defer the destination edit.** Local origin is insufficient; no reporting form/account was opened and no private report was sent.

## docs/implementation-plan.md

- **F7 — confirmed:** issue-sequence URLs (lines 15–33) use the older owner. Align ordinary tracker destinations to the current project identity, preserving issue numbers/dependencies and historical status qualifications. No need to turn this index into another live tracker or infer current blockers.

## docs/agents/issue-tracker.md

- **F7 — confirmed:** line 3's explicit repository identifier conflicts with origin/current project links. Change only the ordinary target identifier to moultonlavah/metten. This audit made no tracker requests or mutations.

## AGENTS.md

No drift detected within current source/authority/verification guidance. The root README remains the project entry; no .github/README.md is present.

## GLOSSARY.md

No drift detected in inspected current terms, including input, measurement, evidence/coverage, purchase option and fixed format meaning. No glossary correction proposed.

## docs/agents/verification.md

No drift detected in inspected evidence navigation, staged-helper behavior and proof-review responsibilities. No correction proposed.

## docs/agents/domain.md

No drift detected within existing glossary/ADR ownership.

## docs/agents/triage-labels.md

No drift detected within documented label/authority semantics. Live label state was not queried.

## docs/strategies/README.md

No drift detected in local availability and guide/example ownership. Strategy validity is clearly separated from provider/trading readiness.

## docs/strategies/authoring.md

No drift detected in inspected learning thresholds/intervals, UI actions, immutable-copy flow, startup/composition and gap guidance against current source. Earlier independent authoring evidence remains historical.

## docs/strategies/agent-authoring.md

No drift detected in inspected public parse/preview/save/detail API guidance, source/unit naming and run-setting boundaries. The sample harness was read, not compiled or run.

## docs/strategies/reference.md

No drift detected in inspected format fields/variants, decimal and time semantics, limits, examples and version/error contract. The public-engine developer lifecycle and operation-bound explanation should be owned by the engine README correction under F3, not copied wholesale into an author guide. The diagnostic behavior gap F12 is against the operative design, not a reason to weaken this reference.

## examples/learning.json

No drift detected in the inspected shared format-2 specimen versus design item 7 and both guides: unique ask below 40 in [0,60), opposite at most 30 then same-side alternative in [60,120). No execution/profitability claim.

## docs/proposals/issue-4-egm/strategy-rules-design.md

- **F12 — confirmed substantive diagnostic contract gap, separate work:** line 612 promises rejected over-budget graphs identify named nodes and required/allowed byte counts. In strategy.rs:587–588, excess total bytes returns the root generic resource_limit; rule_error at lines 40–69 contains fixed text without node costs. The existing five-average public test (strategy tests:341–377) asserts rejection/code only. The rejection protects the memory limit, but does not give the promised repair details. Smallest next action is a separate bounded source/diagnostic correction, with an affected public-interface assertion, after authorization; **do not rewrite the requirement to match the generic error**. No unsafe allocation or trading defect follows from this finding.
- Other inspected implemented grammar/evaluation/storage/native/resource paths showed no additional confirmed contradiction. Source inspection and historical counters/proofs are not fresh acceptance or a newly independent mathematical proof of every resource claim.

## docs/verification/issue-4.md

No drift detected as a navigated historical evidence record. Latest corrections supersede earlier failures and resource table descriptions (notably the final raw-plus-series sweep). Current engine/test hashes match its final correction identities. Old open-issue/unfinished-phase statements retain their checkpoint-specific meaning, not present tracker status. Ignored harness logs, disposable databases, screenshots/tool transcripts and private run records were not reopened.

## docs/verification/issue-2.md

No drift detected within identity-reused historical foundation evidence. Existing later fixes/checkpoints remain authoritative over its earlier incomplete state; no current remote result inferred.

## docs/verification/issue-3.md

No drift detected within identity-reused dummy-only evidence and finished local section. No present process/account/security state inferred.

## docs/egm/2026-11-13_DRIFT_REPORT.md

No drift detected as historical observations. Earlier header/section-2 and ADR evidence fixes remain resolved; F1/F1b are residual statements, not reopening those exact resolved findings.

## Earlier sections of docs/egm/2026-11-14_DRIFT_REPORT.md

Preserved as history. Original F1 security/integration, F4 count, F5 inventory, F2 helper/map/count and F6/F7 contribution/destination findings are confirmed above. Original partial mixed-file disposition is superseded by the current inspection, not retroactively marked complete. F8's reporting destination remains unresolved. Council comments concerning clean-tree gates, generated-permission exclusion, implicit credential-source exclusion, unhandled beyond-list findings and a required extra council remain disproved/clarified as adjudicated; this pass followed those clarifications.

## assets/README.md

No drift detected within ownership/inventory coverage and verified first-pass identity. No export recertification.

## assets/brand/README.md

No drift detected within identity-reused usage/provenance/asset inventory coverage.

## assets/brand/specifications.json

No drift detected within identity-reused ownership/usage coverage. Deferred physical/product integration is not a completed-scope regression.

## assets/brand/licenses/font-source.json

No drift detected within identity-reused local provenance coverage; external availability/rights not newly certified.

## assets/brand/licenses/Clash-Grotesk-FFL.txt

No drift detected within identity-reused provenance/no-font-software-bundled scope.

## assets/brand/licenses/Inter-OFL.txt

No drift detected within identity-reused provenance/no-font-software-bundled scope.

## LICENSE

No drift detected within unchanged MIT license ownership.

## Minimal correction list and stopping boundary

This is a proposal only, not an executed correction pass:

1. **F1/F1b/F11:** qualify only stale implementation/evidence statements in the product specification and link the existing bounded #2/#3/#4 evidence/reference.
2. **F2/F2b:** update docs/egm/README.md's operative scope, ownership groups and repeated request together; add helper/test ownership and replace the agent-guide count.
3. **F3/F10:** update only current exclusions/missing boundary ownership in README.md, crates/README.md, crates/engine/{README.md,src/README.md}, src-tauri/{README.md,src/README.md}, src/README.md, tests/README.md and docs/README.md. Link shared reference/evidence; do not duplicate grammar or regenerate READMEs.
4. **F4/F5/F6:** replace fragile test totals with selection/groups, add the two omitted maintained-test boundaries and exact omitted CI checks to tests/README.md/CONTRIBUTING.md.
5. **F7:** align ordinary repository/tracker destinations in CONTRIBUTING.md, docs/implementation-plan.md and docs/agents/issue-tracker.md. Preserve historical URLs in reports/proposals/evidence.
6. **F8:** defer SECURITY.md's destination until exact owner intent is established. Do not assume ordinary link alignment settles private reporting.
7. **F12:** defer the named-cost diagnostic implementation gap to separate bounded source work; it is outside the documentation-only correction list.

No edits are proposed for scripts/README.md, AGENTS.md, accepted ADRs, current strategy guides, GLOSSARY.md, source/tests/configuration or historical records merely to make the map look complete.

## Remaining evidence limits

- This is a static drift audit. No tests/builds, release/resource measurements, browser/native/container launches, accounts/credential operations, trading, remote requests, council/design reviews, staging/commits/pushes or corrective documentation changes occurred.
- Historical proposal/review/source-capture files were inventoried and assigned historical ownership, not all substantively rereviewed. Their arbitrary old requirements are not audited as current implemented promises. The operative design/current guides and completed-scope evidence drive the comparison.
- The exact pinned standard-library implementation underlying the existing B-tree/allocator proof was not independently retrieved/rederived in this audit. Current source, toolchain identity, proof inventory and latest correction record were compared; no new resource certification is implied.
- Private/ignored execution artifacts were intentionally inaccessible to this report's proof chain. The committed records were read; their original tool runs were not independently replayed. Current remote CI/PR/issue state, private security-form destination/availability, source access, distribution, other Macs, production capacity and live behavior remain unestablished.
- F12 remains a confirmed implementation/contract diagnostic gap. F8 remains an unresolved destination decision. No other substantive behavior defect was confirmed by this inspection; this is not proof of absence of defects.


## Final reconciliation — 2026-11-14T10:27:02.421596-06:00

HEAD remains `96ad37ab547751f1f3b4b55d392bc42be3563e9c`. All 223 inventory paths remain present, with no additions/removals; all 222 non-report file fingerprints match audit start. Only the authorized report changed, and Git status remains the same single untracked report. The expanded pass's local relative file links resolve. The original 41,428-byte prefix hashes to `1da231767d657feecbc352ade2c75042179f8e527adfa004550e9e493a521734`, exactly matching the captured baseline.

Before appending this final verification note: **68593 bytes; SHA-256 `6f00e22abad3bcf4884074524f758dcb0c8e09a5bc0c877fff93a39027e1c872`**. The append operation verifies that entire pre-note prefix as well. No application/source identity changed or required reinspection. Corrections remain proposed; F8 and F12 remain deferred as stated.

---

## Owner-authorized documentation corrections — 2026-11-14T10:33:07.643739-06:00

Connor authorized the confirmed documentation corrections, with explicit concern that the README project page and existing documents be preserved. Applied Ponytail full: targeted replacements and small additions in existing documents only. Starting and final source HEAD remains `96ad37ab547751f1f3b4b55d392bc42be3563e9c`; no staging, commit, push or tracker operation occurred.

**Resolved documentation findings:** F1/F1b/F11 (bounded credential/integration and completed local strategy status); F2/F2b (current coverage/ownership, helper/test, shared-file scope and repeated audit instructions); F3/F10 (component contracts and documentation navigation); F4/F5/F6 (test selection, maintained-test ownership and omitted CI check instructions); F7 (ordinary repository/tracker destinations). The exact proposed scope from the expanded pass was used.

**Still deferred:** F8, the intended private security-reporting destination, and F12, the resource-limit diagnostic implementation gap. SECURITY.md and all application/source/test/configuration files are unchanged. No behavioral requirement was relaxed to match an implementation gap. Historical verification/proposals, council records and prior report text retain their original statements.

Preservation and static validation completed before this note:
- Inventory remains 223 paths with no additions, removals or renames. Only the 14 current documentation files listed below changed; all other files, including this report up to this append, matched their starting SHA-256 fingerprints.
- Root README content before “Implementation documentation and drift checks” is byte-identical to HEAD, preserving the logo, public introduction, navigation, setup instructions, checks and license section. All its headings and command blocks are unchanged; only three implementation-summary lines were replaced. No .github/README.md exists.
- Existing headings in the other corrected documents were preserved, except the deliberately replaced EGM scope title/exclusion subsection. No document was regenerated or truncated. Existing product requirements and historical evidence limits remain.
- Read back the changed wording/diffs against the audit's source/evidence, checked Markdown table continuity/fence balance and resolved 177 local file links/heading anchors. `git diff --check` passed. These are static documentation checks, not tests/builds or rendered GitHub/browser verification.
- Source, tests, permissions, configuration, dependencies, static assets, strategy guides, glossary, accepted ADRs, security policy and historical verification records retain their starting bytes. No fresh app acceptance, resource/security certification or operational result is claimed.

Before append: **69475 bytes; SHA-256 `4c25e4fdf26cd1a327344531530fabf330f75df36fa17eaeb53341302d3d4093`**. The append operation verifies that exact preexisting prefix unchanged.

| Corrected current document | Final SHA-256 before report append |
| --- | --- |
| CONTRIBUTING.md | `15280858519221bb65b9b1fb024581e8ac56215d9c4efe42429a84d4726e1e22` |
| METTEN_DESIGN_DRAFT_2026-11-12.md | `81a2bb08edde9007c61cbbc8eff8e698c3538a47a20b09f55e9c744411455c50` |
| README.md | `10f4a7f84e4f3968353e9c292441386c5367b288a5acf70a9ff5761ac09f9035` |
| crates/README.md | `dbbcf1b35be9bb9d9fb85f2e3a994ed71c29050e874b801c6cc615dd2f85e429` |
| crates/engine/README.md | `34cbf3fc09d84b109a4dd8869c64198e8c01ad23fc506de136c130822a5ca4b8` |
| crates/engine/src/README.md | `277ccad2d300319be82552bec68a0ae6fde50342ccfdc0194220f12cf98a5086` |
| docs/README.md | `cde9d0763e303529bda7ef5df43c3ff701739251b5c271c2faba7d2d96a3f4e6` |
| docs/agents/issue-tracker.md | `005b26dfdafa5535765be7159e1120bcf902b0a1fbd6fa7c2d918d51b7ee7964` |
| docs/egm/README.md | `627e3e855dd8fc3b6aac531fd93b14286a86ee2dd70ebda472618ed38ded3e81` |
| docs/implementation-plan.md | `25bb9d766ab08fc805ef9965ba4b6ab2dad61bc41370ab7321d12aed52078b13` |
| src-tauri/README.md | `8b5514756372ef41f852bc1126b92de5ec62696ace926392aa966235a69f2bb8` |
| src-tauri/src/README.md | `084c22506f5ac45bc2d41650727e1706098b32b5cc461b0c403161146b239f8d` |
| src/README.md | `2a2cee08e55eb4380c86fea5c67977c7d27da4abfa95c4017e54a89a372e894d` |
| tests/README.md | `0f002691ec7be46ba6326fa66763fbaecfaa44a5ad87df2a1bd38d1401174da8` |

No tests/builds, browser/apps/containers, credentials/accounts, private runtime access, remote checks, council/design reviews or trading were run. Existing acceptance evidence remains historical and version-bounded. The two deferred findings remain open for separate work.
