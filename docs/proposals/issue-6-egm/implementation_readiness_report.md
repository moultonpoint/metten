# Stage 9 — Implementation Readiness

Verdict: **revisions needed**, three distinct Major contracts unresolved. Original DetailedRange244–423, frozen input SHA256d1aa85359f66f80cb7ac55591feb09fd6291244b5efe268cefda9059a5c5ad53. Three fresh-context reviewers completed independently at c7b1b4c; locations below refer to that frozen input.

## Implementation Readiness reviewer

**R1 — Major: input replay horizon and retained comparison state** (262,311,319–323). The plan promises identical repeated ordinals are no-ops, conflicting content blocks and backward inputs block. It does not define latest-only versus arbitrary earlier retries, reopen behavior, durable comparison data or an atomic frontier update. `paper_runs` names only the last ordinal; pinned Recorder supplies transactions, not a paper replay policy. Smallest correction: select a bounded horizon, persist canonical comparison data and specify its commit boundary; assert latest identical/conflicting, older identical and stopped-reopen behavior. No raw-input archive is needed.

**R2 — Major: stopped-reopen settlement clock/cursor** (266,280,299,311,348). Playback stops on reopen, settlement requires virtual close, and check_settlement takes a run ID; the plan does not define restored virtual frontier/cursor or whether checking advances time, waits for wall time or permits only already-eligible outcomes. A pre-close quit may therefore strand holdings or give different visible settlement timing. Smallest correction: define a purchase-fenced settlement-only advance/reconstruction policy, exact retained/reconstructed state, repeat behavior and pre-close-reopen payout assertions. Do not replay source decisions or create purchase permission. Foundation lib.rs260–268 restores only its separate session tick, not this paper clock.

No demonstrated financial-arithmetic/dispatch-fence deficiency was found in this scope. Reviewer traced relevant product/ADR/contracts, engine queue/save, Recorder migration, evaluator selection, native quit and storage injection patterns. Independently loaded published guidance and queried current main/c7b1b4c; root-only instructions, no missing required guidance reported.

## Codebase Details reviewer

**B1 — Major: market identity across source/storage/IPC** (310–314,348–350). Requests name run/market IDs but all IDs are required to be positive decimal strings, while existing MarketDescriptor/BindingScope/ObjectiveView use exact string market identities (strategy.rs1525–1542,1714–1720; sources.rs478–488). Choose a numeric paper-market surrogate with unique run/source identity, or explicitly permit bounded exact source strings; specify request/response/FK/cursor mapping and add a nonnumeric market round-trip check. Otherwise an implementer must invent a consequential mapping or reject valid market identities.

**B2 — Nit: missing compatibility reference in inventory** (304,368–413). docs/strategies/reference.md159 documents workspace revision3/older-app compatibility; add it to the exact edit set for schema4 while preserving strategy format2/metten_rules_2.

No unavailable reuse seam was found. Original worker/Recorder, narrow Candidate/safeguard selection, native counting writer, Tauri manifest/capability and VM harness support the stated routes. Existing inventory paths were checked against the pinned tree; absent paths are explicit additions. Independent root guidance/current-main discovery and relevant source/contracts/configuration were consulted; no required gap reported.

## Implementation Style reviewer

No Blocker/Major. Three Nits: **T1**246/419/423 distinguish approved behavior from complete-for-review implementation wording; **T2**264 split combined book/coherence/capability paragraph without dropping rules; **T3**358 restore spaces in example prices. Read full active range, persona/setup and pinned root guidance/verification/maintainability/code-map/ADR. No required instruction gap reported.

## Limits

No tests, builds, apps, accounts, provider calls, external fee fetching or code mutations. These are planning deficiencies, not observed trading failures. Lead editorial corrections and compatibility-inventory addition are recorded in consolidated_review_report.md. R1/R2/B1 remain open. No second independent review of corrected editorial text, full EGM remediation or implementation approval is claimed.

## Subsequent focused clearance — 19 November 2026

The original R1/R2/B1 findings above are preserved as history. Their plan corrections were independently rechecked and closed: see [focused report](focused_recheck_report.md) and [resolution](focused_resolution.md). Runtime correctness and implementation authority remain separate.
