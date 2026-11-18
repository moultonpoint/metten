# Feature critique — 2026-11-14

Target: [design](../market-data-design.md), lines 1–456. Later implementation section checked for mitigations. Frozen identity: [inputs](inputs.md). Three fresh-context reviewers completed; target unchanged.

## Design critic

**D1 — Major — lines 108–115, 294–306, 331–337: recovery must reach both books sharing the subscription.** The design says “affected book” and stream-specific invalidation but does not define the boundary when current and next markets share a Kalshi book subscription. Official documentation states sequence counts belong to the subscription. If current A emits 41, successor B's 42 is missed, then A emits 43, repairing only A leaves B incorrectly ready. Define subscription-wide invalidation for an unexplained sequence gap, revoke successor readiness, and require replacement snapshots for every retained subscribed market. Test a gap whose missing update belongs to the other market. Check lifecycle sequence before discarding unrelated payloads as applicable. Evidence: [official order-book contract](https://docs.kalshi.com/websockets/orderbook-updates), cached source 26.

**D2 — Major — lines 360–364, 479–484: deterministic saved-credential selection is missing.** The design keys protected items by environment and key ID, but connections are session-local and Connect has no key selector. Saving A, replacing with B under another ID, then restarting can leave two items with no declared active selection. Specify exactly one configured identity per provider/environment and its protected restart lookup. A fixed provider/environment item containing key ID and key material is a small option; an active-identity record is another, with more failure behavior to define. Add controlled save-A → replace-B → reopen → selection verification, including failures. No SQLite change is inherently needed.

Read: complete persona, feature standard, target and inputs; product specification, ADR, concepts brief, code map and verification guidance; relevant strategy/native contracts and engine evidence/evaluator code; cached issue and official book/lifecycle/list/authentication sources; excerpts of supporting research. Missing entitlement, rights, leases, calibration and performance were recognized as explicit gates, not false claims. No implementation or execution checks.

## Structural review

No findings. All required feature subjects are present; the standard does not mandate literal headers. One H1 with consistent H2/H3 nesting; six consistent tables; paired Mermaid fence; eleven local link occurrences resolve, including the alternatives fragment. Alternatives give benefits, costs and decisions without attributing rejection to an upstream maintainer.

Read: complete persona, standard, inputs and active target; bounded rereads recovered truncated output. Checked local file/fragment existence. No external availability, browser or implementation verification.

## Style review

**F1 — Major as reported — line 206 versus 494 — CALIBRATE:** diagnostic rotation appears mandatory despite the selected in-memory approach. Use the planned bounded diagnostic activity. Same underlying issue as Z1/C1; lead synthesis will assign one priority.

**F2 — Nit — lines 35, 81–91, 249 — CALIBRATE/RELOCATE:** remove stale drafting statements about choosing libraries/interfaces in later sections; point to decisions already present and retain only actual unresolved proof/calibration gates.

**F3 — Nit — line 101 — DROP proposed:** broad statements that no design guarantees no timeouts, maximum performance or a trading edge repeat later actionable limits. Reviewer recommends retaining concrete bounded-delay/failure requirements instead. Lead must assess whether this contextual caveat answers the owner's expressed concern before accepting deletion.

**F4 — Nit — line 401 — CALIBRATE:** replace the assertion that learning-example success “would then be mistaken” for completion with the concrete statement that it does not satisfy the other named-source requirements.

Read: complete persona, inputs, active range and later section for contradictions; bounded rereads recovered truncation. No independent source, provider, approval or execution verification.

All reviewers were read-only. No builds, browser actions, account access, implementation or target edits occurred.
