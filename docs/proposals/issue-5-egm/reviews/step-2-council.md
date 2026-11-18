# Step-2 plan Council

Status: three passes finished. Final pass has four usable reviews and one missing seat; full five-model 9.5 target NOT achieved. Revision 5 includes lead-verified final corrections after reviewed revision 4 and has no new independent score. Planning review only. No application code, builds or tests. Implementation remains paused for the owner's corrected-scope decision.

Round-1 target: [step-2 plan](../step-2-plan.md), revision 2, SHA-256 `65f4a2789a81ccf8c7dcd284003d448ecfedfe5bb1e244642cfa22dba4658f18`. Round-2 target: revision 3, SHA-256 `a9de03c19e010d380b96a60d05aa80d82bb1ecdefdd6882cffc9140989ac6eb3`. Every seat in each round receives the same target, original engine/design evidence and full published [maintainability standard](https://github.com/moultonlavah/metten/blob/f90316e283115f5c6ab9c0c54f47858b2a67703c/docs/agents/maintainability.md) from PR #44. Native Codex retrieves that exact standard; the free-model briefs contain its complete public text because their source reads are otherwise restricted to the selected worktree.

Round-3 target: revision 4, SHA-256 `493d50eea6c29099ce70fc7de5dc474eaa7be6b565f2ba80cd938e3cfef5c240`. No further rounds after this pass; missing fifth seat remains a reported limitation, never an implied score.

Final prepared plan: revision 5, SHA-256 `528b87824818515a4349ee99b658f9bc77ed2deb532e514b26f7d7ffa8b60a76`. This hash is not the target of the model scores below. The final changes implement the supported low/medium contract clarifications from round3; they were verified by the lead against original source and local document checks, without a fourth Council pass.

The prior revision-1 attempt was stopped before any complete round: the selected worktree predates PR #44 and its maintainability pointer. Codex was interrupted; cancellation was requested for Inkling and MiniMax. Partial results from that attempt cannot establish acceptance. GLM and DeepSeek had not been dispatched. Revision 2 explicitly applies the standard and the ordinary-change probes. The historical approved design remains unchanged.

## Scores and coverage

Target: each reviewer's overall assessment at least 9.5/10, without unresolved supported critical/high findings. Scores describe plan quality, not implemented behavior. All seven dimensions are reported; at most three full rounds. The lead verifies findings against original evidence and retains responsibility for acceptance.

| Seat | Round 1 | Round 2 | Round 3 |
| --- | --- | --- | --- |
| Codex / GPT-6.1 Sol | 9.5/10 | 9.5/10 | 9.6/10 |
| OpenRouter Free / Inkling 2 Small | 8.9/10 | no result | unavailable; Gemma qualification failed |
| OpenCode Free Trial / MiniMax M3.1 Flash Free | 8.3/10 | 9.2/10 | 9.3/10 |
| OpenCode Free Trial / GLM 5.3 Flash Free | 9.4/10 | 9.5/10 | 9.5/10 |
| OpenCode Free Trial / DeepSeek V4.1 Flash Free | 8.6/10 | 9.0/10 | 9.5/10 |

### Codex / GPT-6.1 Sol

Reviewer reported verified plan/design fingerprints and a complete read of the published standard at the pinned merge revision. Scores: reuse/locality 9.6; navigation/next-agent handoff 9.6; simplicity/code quality 9.5; performance/resources 9.5; state correctness 9.5; test/evidence quality 9.5; scope/authority 9.8. Overall 9.5.

Grounding included actual binding-uniqueness rejection and transaction application/evaluation, public same-time atomic tests, strategy observation/coverage semantics and later issue #4 confirmation/within-cell corrections. It explicitly checked the four maintainability recipe steps and ordinary-change probes. No confirmed finding requires a plan correction.

Implementation watchpoint: generation currency is separate from freshness. A matching result stamp cannot replace source validity deadlines or evaluator expiry. Lead disposition: confirmed existing requirement, already covered by separate validity/capability state and controlled expiry acceptance; retain as an implementation review check. No score-driven code or scope change.

## Evidence limits

### Inkling 2 Small

Overall 8.9. Dimensions in table order: reuse/locality 9; navigation 9; simplicity 8; performance/resources 9; state 9; tests/evidence 9; scope 9. Review reports full grounding read and confirmed source/proof pointers. No critical finding. Lead has read the actual completed thread and verified registration behavior in `strategy.rs::add_binding_to` and `RegistrationCursor::step`.

| Finding | Lead disposition before canonical revision |
| --- | --- |
| F1: exact forbidden-pair error unspecified | Confirmed contract omission; select and document the exact existing error rather than add needless error vocabulary. Current duplicate rejection is `unregistered_binding`. |
| F2: constructor borrow/retention ambiguous | Confirmed clarity gap; constructor should borrow validated strategy while deriving/validating bounded requirements and retain no compiled strategy copy. |
| F3: registration validation sharing unspecified | Confirmed maintenance risk: evaluator checks are currently private. Plan a minimal crate-private extraction of pure association/descriptor checks used by both owners, preserving evaluator resource and catalog checks at their owner. Verify affected callers and proofs; no parallel grammar validator. |
| F4: view overflow outcome unspecified | Confirmed missing failure result; name bounded `resource_limit` failure and complete required view fields. Review cap assumptions; reviewer has not demonstrated that an admitted bounded view necessarily exceeds 256 KiB. |
| F5: retired stamps not explicit | Confirmed clarity gap; stamps referencing missing/retired bindings fail closed and IDs cannot be rebound under an old stamp. |
| F6: glossary must gain implementation terms | Decline glossary expansion absent a resolved domain-language need. Root glossary remains domain vocabulary; generation/stamp/high-water mark are internal interface mechanics, defined in the owning source contract. The project domain instructions explicitly keep implementation details out of the glossary. Make this location explicit in the plan. |

Plan revision waits for all five first passes so every seat reviews the same target. These dispositions are preliminary until integrated with remaining reviews. Tracker facts were not independently network-checked by this reviewer; controlled atomic validity behavior and resource/caller contracts were source-checked, not executed.

No implementation, execution, actual provider guarantees, entitlement, rights, performance qualification or full issue #5 acceptance is established. U1–U7 remain open. Outstanding reviewer results and any material disagreement must be resolved or explicitly reported before claiming Council completion.

### MiniMax M3.1 Flash Free

Overall 8.3. Dimensions in table order: reuse 8.0; navigation 8.0; simplicity 8.5; performance 8.5; state 7.5; tests 8.5; scope 9.0. Lead read the completed thread metadata and owner-delivered full result, then traced gaps through the original evaluator and existing public tests. No tests executed.

| Finding | Lead disposition |
| --- | --- |
| F1: late gap silently ignored | Disproved as stated. `strategy.rs` gap scan at 3028–3030 explicitly accepts `frontier == initial_frontier && start_ms < frontier`, invalidating affected retained series before advancement. `phase1_late_contradiction_recollects_dependent_history_and_preserves_checkpoint` supplies a gap [500,600) at frontier 1001 and expects current ineligibility with latched checkpoint preservation; `phase1_old_contradiction_blocks_only_the_transitive_window_it_overlaps` covers older composed history; future-gap test proves effective-time handling. Proposed withholding of new coverage alone cannot replace revoking previously certified cells and is rejected. Add explicit late-contradiction timeline/interval contract for clarity, without restricting gaps to current frontier or changing the evaluator. |
| F2: duplicate registration truth | Confirmed, same as Inkling F3; specify shared pure descriptor/association checks, retain resource/catalog admission at its existing owner. |
| F3: counter/stamp relationship unspecified | Confirmed need for precision. Specify actual per-binding generation/incarnation alongside scope generations and metadata revision, and exactly how invalidation/retirement affect it. No observer can reuse a retired binding ID with an old stamp. |
| F4: mandatory merge/rebase to read standard | Disproved as necessity. Plan links are pinned remote GitHub URLs, not dead local links; all reviewers received full standard and inline plan requirements. Preserve selected worktree/uncommitted design rather than merge unrelated source changes as a prerequisite to planning. Add explicit in-checkout reference route and before-code baseline revalidation. Normal later branch integration remains separate; no claim local AGENTS contains the published pointer. |
| F5: evidence record not indexed | Confirmed navigation omission; code-map route must link directly to issue-5 evidence and source guide, avoiding edits to unrelated docs/README work. |
| F6: design inventory departure not pinpointed | Confirmed clarity gap; explicitly supersede design inventory's strategy.rs-only-extraction assertion with atomic admission/sharing correction and record exact owner authority before code. Preserve approved historical design/hash. |
| F7: pair error/duplicate wording | Confirmed, same as Inkling F1; name exact existing error and distinguish one normalized binding update from its allowed value/validity members. |
| F8: glossary needs implementation terms | Decline expansion for same reason as Inkling F6: domain.md directs implementation terminology to the owning module contract. Add definitions there and link from map/evidence, preserving root domain glossary. |

Reviewer source-checks support atomic pairing/resource reassessment and scope stops. Its gap bug diagnosis and mandatory-rebase remedy are not accepted merely to raise its score.

### GLM 5.3 Flash Free

Overall 9.4. Dimensions: reuse 9.0; navigation 9.0; simplicity 9.0; performance 9.5; state 9.5; tests 9.5; scope 9.5. Two enumerated low findings (closing prose says three, but only F1/F2 are supplied): no high/critical finding. Review independently read published standard text in brief, original code/contracts and the entire approved design; hash matches.

| Finding | Lead disposition |
| --- | --- |
| F1: local standard route and four earlier corrections not enumerated | Partly confirmed clarity gap. Pinned remote standard is a valid reference, so absence locally does not make it dead. Add inline named route plus four-item correction/disposition index to eliminate dependence on conversation or other reviewers' reports. |
| F2: invalidate's gap interval undefined | Confirmed interface omission. Distinguish explicit known historical contradiction interval from immediate current invalidity; do not invent intervals from heartbeat or last value. Validate interval against received frontier; unknown past coverage stays unproved. Shared loss revokes present state immediately and any substantiated affected interval is carried as an explicit gap, including late contradictions. |

### DeepSeek V4.1 Flash Free

Overall 8.6. Dimensions: reuse 9.0; navigation 8.5; simplicity 8.5; performance 8.5; state 8.0; tests 8.5; scope 9.0. Lead read the full completed review through its thread, then checked `Evaluator::apply` and rejection/continuation handling. No execution.

| Finding | Lead disposition and revision-3 remedy |
| --- | --- |
| F1: invalid pair's retained value can be exposed by later lease | Confirmed interface gap for the new pair. Current standalone validity intentionally retains raw values; this alone is not a proven existing engine defect. New pair finalization must clear a value suppressed by explicit invalidity/expired paired lease, irrespective of member order, preserving observation high-water marks. Add both-order → cell advance → lease-only → still-unavailable → fresh evidence recovery tests. Source invalidation requires new initialization. Account bounded finalization work. |
| F2: source commit versus evaluator ingress admission | Confirmed. One effect is submitted as one batch; source receipt is separate from evaluator completion. Actual metadata-inclusive admission remains `Evaluator::begin`. Any early/later rejection revokes touched source claims, discards obsolete result/effect, preserves counters and blocks ordinary input until revocations and fresh initialization. No rollback framework or production coordinator. Test early resource rejection, later validation rejection, and pending-result invalidation. |
| F3: historical issue4 pair rejection contradicts new route | Confirmed navigation risk. Cite issue4 line405 and expressly supersede the old single-change restriction in the new code-map/current-evidence route without editing historical records. |
| F4: grammar wording ambiguous | Confirmed. Say strategy-JSON grammar unchanged; evidence admission and pair-specific invalidity are the named correction. |
| F5: gap/duplicate wording ambiguous | Confirmed. Same-binding forbidden combinations only; gap A plus observation B remains allowed; pin allowed/forbidden cross-binding cases and existing error. |
| F6: compiled strategy retention ambiguous | Confirmed, shared with Inkling F2. Constructor borrows, retains bounded requirements only. |
| Optional glossary/return/error detail | Define module mechanics in owning contract per domain instructions. Explicit method results and existing pair rejection code are now specified. |

## Revision 3 changes and round-2 contract

All five first passes assessed unchanged revision 2. Revision 3 integrates the confirmed findings in the eight-file scope and records rejected recommendations with source reasons above. It explicitly shares pure registration/descriptor checks, borrows the strategy, states result/overflow errors, specifies binding/owner/incarnation stamps and retirement, defines known versus unknown gap intervals, fixes invalid-pair resurrection and source/evaluator rejection handling, indexes current evidence, and reconciles the historical admission rule. The code map links module mechanics through the owning contract, preserving root domain glossary and unrelated docs edits. Pinned PR44 guidance is explicitly reachable from the local plan; no mandatory merge/rebase is invented.

Each new round-2 review is a fresh delegate reviewing the exact revision-3 hash. Reviewers receive the same compact correction/disposition summary and full standard, assess changed paths plus regressions, and score all seven dimensions independently. Target remains every overall score at least9.5 with supported high findings resolved; stop after at most three rounds. If a reviewer disagrees with a disposition, it must identify original evidence/counterexample; the lead verifies it. No requests for score inflation. Actual code and runtime evidence remain future work.

Round-2 Inkling task completed without an assistant result or score. It supplies no acceptance evidence. Under the selection policy, try one unused free family (Gemma 4 31B) on the required synthetic qualification before assigning the missing review seat. No paid fallback or repeated quota retry. If qualification fails, disclose the missing seat and incomplete Council.

Gemma qualification failed before a usable result: upstream free model rate limit (`[Google AI Studio] google/gemma-4-31b-it:free is temporarily rate-limited upstream`). Lead read the failed thread metadata. No qualification pass, plan review or score exists. The policy's one eligible backup attempt has been used; do not invent a replacement, pay, add credentials or retry in a loop. Continue the four available round-2 reviews, but that round cannot claim complete five-model acceptance. Preserve this missing seat even if remaining scores improve.

### Round 2: Codex / GPT-6.1 Sol

Overall 9.5. Dimensions: reuse 9.6; navigation 9.6; simplicity 9.5; performance 9.5; state 9.5; tests 9.6; scope 9.7. Verified exact revision-3/design hashes and full pinned PR44 standard; original admission, application, registration, public pair/ingress/late-gap cases and proof contracts checked. No required correction or supported high/critical finding. The reviewer explicitly accepted plan completeness of one-batch rejection/revocation and pair finalization while leaving actual resource bounds and tests as implementation gates. No source change follows from this score.

### Round 2: MiniMax M3.1 Flash Free

Overall 9.2. Dimensions: reuse 9.3; navigation 9.0; simplicity 9.3; performance 9.2; state 9.0; tests 9.1; scope 9.3. No critical/high finding; two low precision gaps. The reviewer independently withdrew its round-1 late-gap bug claim after inspecting the actual initial-frontier branch and public tests.

| Finding | Lead disposition for next revision |
| --- | --- |
| F1: implicit ask structured meaning owner not named | Confirmed clarity gap. The existing string IDs/printed explanation imply fixed meaning; extraction belongs in strategy.rs, not a copied sources.rs whitelist. Specify one structured implicit-ask definition there reused by extraction, explanation and shared scope validation, preserving exact explanation bytes. |
| F2: cross-binding source event grouping not explicit | Confirmed. One coherent value update must produce one normalized transaction for all its bindings and validities, not one per binding. Separate genuine events at the same timestamp remain separate transactions. Add the public two-binding intermediate-checkpoint regression through sources.rs. Revocation/control transactions may precede the coherent value transaction, as already specified. |

Its domain.md citation uncertainty is resolved by the lead's actual read of that file: keep implementation details out of the root glossary. Optional separate commits and cosmetic ordering are declined as unnecessary for this planning task. Remaining reviews still use unchanged revision3; no mid-round target changes.

### Round 2: GLM 5.3 Flash Free

Overall 9.5. Dimensions: reuse 9.5; navigation 9.5; simplicity 9.5; performance 9.5; state 9.5; tests 9.3; scope 9.5. No high/critical finding. Supplied full standard accepted as grounding, no external/provider reads. Lead read completed thread metadata and full owner-delivered review.

F1 (old gaps only apply at an initial, rather than later-run frontier) is disproved to the extent it restricts normal later batches. `Evaluator::begin` sets `Pending.initial_frontier = self.frontier` for EVERY batch at strategy.rs2703. Thus a known old gap in a newly submitted batch is processed at that batch's initial frontier even after prior batches advanced the run. Public late-contradiction test submits [500,600) at already-completed frontier1001 in a new batch, invalidates the candidate and preserves the existing checkpoint. The plan already requires submitting a late contradiction at current accepted time, not injecting an event into a partially processed batch. Clarify "first scan of each submitted batch" in the next revision; reject a lifetime/first-batch restriction or a claim of general ignored late gaps.

F2 (name borrowed input projection type) is optional; shared structured implicit ask definition and minimal borrowed identity will address ownership without mandating an abstraction. Optional GapInterval field detail can be clarified as two u64 endpoints. Source domain instruction was read by the lead; no glossary expansion is required. The actual scores above are retained unchanged; disproving a finding does not fabricate an improved reviewer score.

### Round 2: DeepSeek V4.1 Flash Free

Overall 9.0. Dimensions: reuse 9.0; navigation 9.0; simplicity 9.0; performance 9.0; state 8.5; tests 8.5; scope 9.5. No high/critical finding, one medium test/contract gap and several low precision gaps. Lead read completed thread metadata/full owner-delivered review and checked actual standalone validity and after_checkpoint_update semantics.

| Finding | Revision-4 disposition |
| --- | --- |
| F1: standalone loss revival missing test | Confirmed missing PUBLIC SOURCE test. Preserve evaluator standalone validity's retained-value semantics (existing within-cell test depends on it); source owner refuses a lease after source invalidation with `invalid_request` and no effect until new verified initialization. Add that timeline plus unchanged evaluator regression and contract seam explanation. No uniform raw-value clear. |
| F2: registration type unnamed | Confirmed clarity gap; borrow existing `&[MarketDescriptor]` and `&[BindingDescriptor]`, not a new registration grammar. |
| F3: clock/accepted mapping implicit | Confirmed; explicit one controlled since-Go clock, monotonic receipt/acceptance/received watermark, increasing transaction numbers, batch target/frontier bounds and on-time boundary-before-advance timeline. No provider-clock ordering or backdated boundary. |
| F4: duplicate gaps not named | Confirmed minor omission; explicitly list two same-binding gaps as forbidden duplicate. |
| F5: after_checkpoint_update needs availability | Decline operator change. Existing source checks observed_transaction/number relative to checkpoint, not current price; preserve observation-identity meaning and independent current ask checks. Add an assertion to prevent accidental semantic drift. Unknown genuineness still cannot emit an observation. |
| F6: Busy fails closed unspecified | Confirmed; controlled protocol misuse terminally stops entire source owner and evaluator with SliceControl::Stop, discarding pending/refused results; explicit recovery uses new owner/evaluator. No recovery framework. |

## Revision 4 and final pass

Revision 4 also integrates MiniMax's two supported clarifications: one strategy-owned structured implicit-ask meaning reused by extraction/explanation/shared validation with exact explanation preservation, and one coherent source update → one normalized transaction across ALL bindings, with the public intermediate-equal-asks test through sources.rs. It clarifies per-batch initial frontier to reject the disproved lifetime restriction, gives GapInterval's endpoints, and incorporates the DeepSeek dispositions above. Eight implementation files remain unchanged. Local document links/fences/whitespace are checked; no application test or runtime result exists.

Four available model families review the SAME exact revision-4 hash freshly with the full standard and identical correction summary; no other review results are supplied. At most three passes are authorized by the Council skill. Failed Inkling and failed Gemma qualification remain disclosed, so even four final scores >=9.5 cannot establish five-model unanimity. The final owner recap must state actual scores, supported unresolved findings (if any), source evidence and that implementation stays paused pending the narrow corrected-scope decision.

### Round 3: Codex / GPT-6.1 Sol

Overall 9.6. Dimensions: reuse 9.7; navigation 9.7; simplicity 9.4; performance 9.5; state 9.6; tests 9.6; scope 9.7. The overall score meets the target; the simplicity dimension is still 9.4, and no dimension score is inflated or hidden by the overall. No supported critical/high/medium finding or requested expansion remains. Verified exact revision4 hash and complete published PR44 standard retrieval; traced actual atomicity, paired-versus-standalone validity, per-batch late-gap processing, pure registration seam and metadata-inclusive admission against source/public tests.

Recommends bounded plan acceptance subject to exact owner approval of the named evaluator departure. Actual heap/work derivation, simultaneous ownership, snapshot bound and final-candidate test evidence remain future implementation gates. The reviewer did not execute tests, write files, contact providers or read other Council reviews. No plan change follows from this pass.

### Round 3: MiniMax M3.1 Flash Free

Overall 9.3 (target not met). Dimensions: reuse 9.5; navigation 9.0; simplicity 9.3; performance 9.3; state 9.4; tests 9.2; scope 9.6. No critical/high finding. Reviewer discloses repo-wide grep incidentally surfaced two Council-record lines; it did not open the file and says no finding derives from them. This is a limited first-pass-isolation caveat, not hidden or treated as runtime proof. Lead read completed thread metadata and full owner-delivered result, then actual raw_value, implicit explanation output and documentation map.

| Finding | Lead disposition for final bounded correction after remaining reads |
| --- | --- |
| F1: standalone lease seam wording | Partly confirmed wording risk. Raw value retention is distinct from usable historical cells. Existing raw_value checks current validity for point reads, with coverage/bad-cell for cell reads; the named history test asserts invalid cell persistence after renewal, not a universal revival result. Clarify precise unchanged test outcome, avoid claiming that cell test proves current-only revival. Keep source lease refusal policy and test. |
| F2: documentation map update absent | Known bounded deferral, not adding a ninth implementation file. Actual docs/README currently links issue2/3/4 evidence and inherited issue5 design only. State guide/evidence documentation-map link is deferred to approved sequence step7; code map must link both now, providing an actual current route and satisfying indexed evidence. Preserve unrelated edit. |
| F3: exact implicit explanation bytes need test | Confirmed specificity improvement. Require exact literal assertion per implicit ask line, including punctuation/newline; test zero/one/two implicit asks and declaration ordering without broad golden snapshots. Current source string is verified. |
| F4: precise red regression vehicle | Improvement, not blocker (reviewer acknowledges current plan permits demonstrating existing rejection). Specify public split same-time value/validity timeline plus pair rejection first; corrected coherent pair must avoid intermediate checkpoint/count behavior. |
| F5: deadline authority | Optional useful clarification. Source owner emits validity in the controlled source pipeline; evaluator keeps a derived copy and never supplies validity back to the source owner. Other direct evaluator controlled tests retain their existing interface. Rejection divergence follows the explicit invalidation protocol; do not claim disagreement is impossible. |

Any final wording changes after the remaining round3 reviews are lead-verified and explicitly not assigned new model scores. Three-pass budget is exhausted; preserve actual 9.3 and missing fifth seat rather than infer target acceptance.

### Round 3: GLM 5.3 Flash Free

Overall 9.5; every dimension 9.5. Verified exact revision4/design hashes, original state/admission/registration/proof/test routes and full published standard supplied in brief. Lead read completed thread metadata and owner-delivered full report. No critical/high finding; two supported wording points plus an optional caution. Source-guidance facts were checked; session authority/live tracker claims were not independently network-verified.

| Finding | Lead disposition for final bounded correction |
| --- | --- |
| F1: selected-side resolution ownership | Confirmed clarity gap. Requirement records retain selected_side; concrete YES/NO resolution happens only in registered BindingDescriptors. Source retains one requirement record, tracks per-concrete-binding readiness, and cannot advertise selected-side requirement readiness without its required sides. Do not expand the <=34 requirement inventory. |
| F2: Stop described as discarding memory/results | Confirmed precise wording needed. Mark pending/refused results unusable; evaluator retains cancelled pending ownership until destruction per existing Stop contract. No new synchronous teardown is implied. |
| F3: implicit startup map caution | Optional explicit note: startup_ms contains declared inputs only; derive implicit ask zero directly, not by indexing missing IDs or changing requirements map. |

Reviewer recommends owner disposition with low/nit points, no additional framework or operator change. Keep actual 9.5 score on reviewed revision4; any lead wording correction remains unscored. DeepSeek's final pass is the last outstanding result.

### Round 3: DeepSeek V4.1 Flash Free

Overall 9.5. Dimensions: reuse 9.6; navigation 9.6; simplicity 9.5; performance 9.5; state 9.3; tests 9.4; scope 9.7. Scores assess reviewed revision4 only; the state/test dimensions below9.5 are retained explicitly. No critical/high finding; one medium interface-input gap and two low state/test gaps. Lead read completed thread metadata/full owner-delivered result and checked actual frontier, snapshot age assumptions and validity semantics.

| Finding | Final revision-5 correction |
| --- | --- |
| F1: completed evaluator frontier has no explicit input | Confirmed. Envelope carries completed_frontier_ms from last completed Decision.evaluated_through_ms, supplied by the controlled one-batch consumer; reject at_ms below it with invalid_order BEFORE mutation. No evaluator-private-state coupling. Add exact public assertion. |
| F2: snapshot clock reversal | Confirmed. snapshot(now) earlier than last accepted time returns invalid_request before subtracting age; test no underflow/wrapped age. |
| F3: healthy lease renewal lacks positive test | Confirmed. Initialized usable binding without loss accepts substantiated future-deadline renewal, emits Validity without observation/reinitialization; paired negative loss case still refuses until init. |
| Optional mask lifetime | Concrete current need, now specified: validation drains as before; one separate bounded current-transaction application mask built incrementally, then finalized/drained in slices before conditions. Charge simultaneous maps/work; no whole-batch mask retention or prior-event scans. |
| Optional explanation capitalization | Covered by exact literal line assertions, including lowercase kalshi/punctuation/newline; no rendering from a display-capitalized label. |
| Optional workspace checks | State focused check limits and later integrated gate; expand checks if actual existing cross-crate shapes/callers change. No app build solely for planning. |

## Final disposition and verification

Lead addressed all confirmed round3 contract gaps in revision5: explicit completed frontier; backward snapshot rejection; healthy renewal test; selected-side selector retained until concrete registration; Stop retains cancelled memory while results become unusable; exact implicit explanation-line checks; precise baseline reproduction; accurate raw-value versus historical-cell semantics; source-only validity authority; explicit step7 docs-map deferral with step2 code-map indexing; and current-transaction mask lifetime/accounting. No provider capability, operator, dependency, transport coordinator or new implementation file was added. The code/table scope remains eight implementation files. Declined findings retain original evidence-based reasons above.

No known supported plan finding remains unaddressed after these lead corrections, but their sufficiency was not independently re-scored. Actual final-round scores are Codex9.6, MiniMax9.3, GLM9.5, DeepSeek9.5; Inkling no result, Gemma qualification rate-limited. MiniMax's lower score and the missing fifth seat mean the owner's requested complete five-model>=9.5 outcome was not achieved. No average, inferred post-fix score or unanimity claim substitutes for that outcome. The Council skill's maximum three rounds has been reached; no further delegates/retry loop are running for this review.

Final local verification: checked the two preparation documents' relative file targets, referenced local anchors, Markdown fences and whitespace; approved design still has its 599-line recorded hash; Git diff/status shows no application-source changes or new source files. Existing unrelated docs/README edit and inherited proposal records remain. Only this Council record and step-2-plan.md were authored in this planning task. No builds, executable engine tests, app launches, provider/accounts/credentials, git stage/commit/push, issue mutation or publication occurred.

Recommendation: accept the corrected bounded plan on its evidence and limitations, and obtain the owner's exact implementation decision for the named evaluator admission/paired-invalidity correction before code. It preserves the source-capability stops and includes concrete final-candidate behavior/resource checks. It is prepared locally, not published or implemented. U1–U7, runtime correctness, actual resource compliance and provider/access/rights qualification remain unresolved.
