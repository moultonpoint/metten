# Issue #5 independent brainstorm review

Date: 2026-11-14. Mode: BRAINSTORM. The owner confirmed the scope before dispatch. Reviewers received no parent conversation (`fork_turns: none`), read their installed personas and a common frozen target, and returned findings through native agent results. Host instruction-file exclusion was not tested. This is a concept review, not implementation acceptance.

Reviewed original target: `.local-planning/issue-5/review-input.md`, SHA-256 `2737712aba906bdd26fdf74e3f54505c316f9c791aaecc95041c1a99ad55dc16`. The [archived copy](brainstorm-review-input.md) adjusts relative document links only for this destination; its SHA-256 is `9158c26fac1bfa25212ddb99a785201afa205d1b908521894325e244f8bc6ebb`. The original reviewed file remains unchanged in local planning. Seven independent lenses run in bounded parallel groups because only three child slots are available. Later groups receive the same target and no earlier findings. The lead performs synthesis directly, avoiding an extra orchestration agent that would consume a reviewer slot. No additional formal document standard applies to this upstream brainstorm.

## Reviewer status and grounding

| Persona / brainstorm lens | Status | Actual grounding reported |
| --- | --- | --- |
| egm-design / architecture, cost and quotas | Completed | Full persona and target; product specification, ADR 0001, code map; input, clock, coverage, limits and observation sections of strategy reference |
| egm-zoom-out / simplicity and reuse | Completed | Same core sources; engine/native contracts, manifests, focused evidence/validation code and caller searches, controlled continuity test |
| egm-codebase-details / operational visibility | Completed | Same core sources; engine/native/interface contracts; Snapshot, evaluator result and adapter evidence types |
| egm-comprehension / user experience | Completed | Full persona/target; product specification, ADR, code map; reference input, calculation, validity/history and error contracts |
| egm-implementation-readiness / migration and compatibility | Completed | Full persona/target; product specification, ADR, code map, native contract; reference format/storage/history/limits; bounded credential, storage revision and ingress code |
| egm-structural / maintenance | Completed | Full persona/target; product specification, ADR, code map; reference precision, binding, clock/history, limits and observation semantics |
| egm-inventive-catalyst / contradictions and alternatives | Completed | Full persona including pattern catalogue and target; product specification, ADR, code map; reference binding/history/observation sections |

All completed reviewers reported read-only investigation. They did not execute tests or contact providers. Their claims about provider behavior assess supplied evidence; they are not new access or continuity proofs.

## Findings and lead dispositions

### Blocker to promising full connected strategy capability: source continuity

Design and simplicity reviewers independently found that current BRTI/spot values do not prove the complete intervals required by held conditions or historical calculations. The operational reviewer identified the resulting user-visible confusion after reconnect.

Disposition: require a field-by-field feasibility matrix with current validity, genuine observations, complete-interval evidence, gap detection and recovery. Current display can be supported while a historical use remains unsupported. Preserve #4 semantics; no invented quiet coverage, backfill repair or checkpoint observations. An infeasible required capability is an explicit blocker, not an accepted substitute. This does not block completing the concept brief or fixture preparation.

### Major: separate retained provider information from strategy inputs

Design and simplicity reviewers found that quantities, trade direction and distinct average states extend beyond the current scalar format-2 bindings.

Disposition: retain useful provider metadata and expose bounded read-only details within #5. Populate an existing strategy binding only when its exact meaning is proven. New JSON fields/operators are excluded. A partial average never becomes the authoritative settlement reference. BRTI preference does not override an existing strategy's named sources.

### Major: production credentials are a new integration boundary

Simplicity reviewer traced the current credential adapter to a fixed dummy item in a disposable test Keychain. It cannot simply be reused as an implemented provider credential facility.

Disposition: reuse the proven native protection/admission patterns; design the minimum provider-specific Keychain storage, trusted signing access and packaged checks as explicit #5 work. Dummy-only evidence remains bounded. Real credentials/account calls still require separate exact authority.

### Major: shared outages and source identity

Design reviewer found that Kalshi-delivered BRTI, contract data and REST recovery share a provider dependency. Alternative spot prices cannot restore contract data or preserve BRTI semantics.

Disposition: show correlated Kalshi failure and per-source health truthfully. No automatic substitution. Any future change to a strategy's source occurs through an explicit reviewed strategy version, under existing immutable-run rules.

### Major: bounded processing and truthful progress

All three reviewers identified distinct raw-transport and normalized-ingress memory, recovery quotas, and receipt-versus-evaluation age concerns.

Disposition: reuse `EvidenceChange`, `CoverageFact`, `NormalizedTransaction`, `AdvanceBatch` and bounded `process_slice`. Keep queued normalized work within the existing combined 8 MiB/4096-transaction allowance per evaluator, and separately bound raw frames/books/subscriptions. Invalidity and overload must become effective even while evaluation is backlogged; Pending supplies no current candidate. Measure a named bounded workload and interruption/burst/overflow cases rather than create a benchmark platform. Coordinate reconnect/discovery traffic and follow provider limits.

### Major: useful local explanations without raw-feed storage

Operational reviewer requires separate current-price validity, history readiness, last valid receipt, last socket activity, queued-event age and completed-evaluation age. Confirmed denial needs actual denial evidence; silence is unexplained absence.

Disposition: use concise status and next action with bounded details. Keep local transition summaries, aggregated reject counts and bounded correlation from provider identity/connection generation to normalized observation and acceptance outcome. No hosted tracing or continuously stored raw feed. Diagnostic failures must not stall intake or masquerade as provider failures.

### Major: deferred work must remain deferred

Design and simplicity reviewers found that appended D–H findings could reintroduce midpoint/history requirements and optional research channels into #5.

Disposition: retain their research links as context, explicitly exclude exact D–H reproduction, week-long warmup, scoring, signed spot-flow channels and macro/derivatives signals from #5 acceptance. Revisit only through separately approved capability work. A faster BRTI delivery profile is not a transparent optimization because it changes observation frequency.

### Major: restart and storage compatibility

Compatibility reviewer requires fresh establishment of health after restart and preserving the stopped-reopen behavior. New source settings must not trigger unnecessary storage changes; newer-revision refusal and preservation of old records matter if a revision is required. No migration of dummy credentials is appropriate.

Disposition: persist only necessary nonsecret configuration; keep transient health unavailable after reopening. Preserve immutable format-2 strategies and their binding meanings. Formal design must enumerate any actual new durable records and explain migration/older-app behavior only where needed.

### Major: dependency behavior and semantics must remain observable

Maintenance and inventive reviewers found that SDK reuse can hide queue loss, precision changes, reconnect generations and unknown-message handling. A corrective wrapper that duplicates the client's state may cost more than direct mapping.

Disposition: verify one suitable Rust candidate against the required interfaces and failure fixtures; keep its types and secrets inside trusted provider code. Use established transport/signing libraries directly if reuse requires extensive bypasses. Neither Python nor Rust has a demonstrated speed advantage from this review. A profile/library upgrade must not quietly change observations.

### Major: bounded delay and loss cannot be solved by buffering alone

Inventive reviewer identifies a real tension: a larger queue can preserve messages while making all decisions older. A smaller one can invalidate recoverable bursts sooner. Proposed transforms are segmentation of receipt/evaluation from screen/diagnostics, and beforehand cushioning through bounded admitted buffers and reliable invalidity signaling.

Disposition: bound retained work and oldest-event age; reuse existing evidence admission and maintain a control path for overload invalidation when ordinary intake is full. Do not replay old backlog as fresh evidence. Secondary costs are memory, coordination and temporary unavailability. No durable broker, new event bus or archive is justified.

### Major and minor user-facing clarifications

Comprehension reviewer independently corroborates source-versus-history readiness, explicit alternatives and field-surface distinctions. It also requests an affected input, established reason and useful next action for blocking states, plus proof of flowing data behind details. Superseded approval notes obscure the actual gate.

Disposition: rewrite the brief as one current proposal, retaining the frozen target as history. Use distinct labels for BRTI tick, provisional closing average and established reference; primary does not mean automatic fallback. Provide bounded details without a diagnostic console. Owner confirmation and this review grant no implementation/account authority.

## Synthesized alternatives and achievable result

The inventive lens found no foundational false compromise in the existing direction. Established transforms help only within real external limits: separation prevents screen work delaying receipt; fixed profiles separate delivery frequency from local processing delay; the trusted adapter boundary allows client reuse without surrendering data truth.

Each choice has costs: independent receipt/evaluation needs ownership/order; bounded buffers use memory and can require temporary unavailability; SDKs add upstream maintenance while direct mapping adds local maintenance. Architecture cannot create entitlement, use rights, missing historical evidence or provider uptime. Coinbase/Kraken are not BRTI. More Kalshi connections do not eliminate a shared provider failure.

The achievable result is one local Rust application that admits supported observations once under truthful identity/timing, explicitly marks unsupported or missing evidence, keeps the screen responsive, and cannot submit purchases in this slice.

## Synthesis status and limitations

All seven requested lenses completed; none failed, timed out or was counted as clean without a report. No extra reviewer round was run. The lead deduplicated findings and incorporated the dispositions into `concepts_brief.md`, including explicit production credential work, a read-only information matrix, source-feasibility gate, existing-evidence reuse, profile/queue ownership, restart behavior and bounded source details. The final edited brief was checked by the lead, not independently re-reviewed; the seven reviews apply to the frozen target above.

The credential boundary and unsupported source continuity are blockers to promising production connected acceptance, not blockers to concluding this brainstorm or preparing a formal design with those gates. No additional product decision was identified within the confirmed scope. Actual access, retention/use rights, field correspondence, client suitability, continuity proof and performance remain outstanding evidence. Required paid access, an infeasible promised capability or changed strategy meaning would require a concrete owner decision.
