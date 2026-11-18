# Confirmed scope for independent brainstorm review

Connor confirmed review of this scope: BRTI delivered through Kalshi as preferred BTC reference; distinct current/settlement averages; Kalshi market details, books and trades with quantities, direction and health evidence; explicitly named Coinbase/Kraken alternative prices without silent substitution; Rust-native integration with a suitable existing client if verified; controlled recovery, freshness, overload and delay checks. No code changes, account calls, paid services or trading. Exact D–H reproduction and new scoring/history semantics remain separate. Extra signed-spot-flow channels and macro/derivatives signals are researched candidates, not approved requirements. No superior BRTI uptime has been measured.

The following working material includes historical proposals and later findings. Challenge unresolved contradictions; assess a concept, not an implementation-ready design. The source code is authoritative for current implementation; this brief is proposed future behavior. User confirmed independent review on November 14, 2026.

---

# Issue #5: trustworthy BTC inputs without avoidable processing delay

* **Author:** Connor Moulton
* **Date:** 2026-11-14
* **Status:** Working hypothesis. Not owner-approved or independently reviewed. No implementation or account access authorized by this brief.

## Overview

Connect the BTC inputs already defined in issue #4: Kalshi contract prices and market references, BRTI, Coinbase BTC/USD trades and Kraken BTC/USD trades. Strategies choose explicit sources. Subscribe to what the selected strategy requires, including the engine's required contract prices, rather than collecting every available feed. The existing learning example needs only Kalshi contract asks; it is the first end-to-end case, not the complete connection scope.

Connor wants reliable delivery and a concise performance plan, with no additional source types. His concern about losing Kalshi's BRTI delivery has two meanings worth separating: recovering a temporary interruption, and having an independent route if that service becomes unavailable. The first belongs in this slice. An additional route to genuine BRTI needs feasibility and licensing evidence before commitment.

## Analysis

### Start from the existing input contract

The [strategy reference](../../../docs/strategies/reference.md) defines Kalshi buyable ask, bid and last trade for an exact contract and side; BRTI index value; Coinbase and Kraken last trade; and distinct opening and settlement references. An order's contract payout is not the underlying BTC settlement reference. Provider mapping for those reference fields remains unverified. Keep unavailable fields explicit until their exact meaning is established from market rules and provider evidence.

Issue #4 also requires genuine observation identity, preserved event order and separate evidence for current validity and complete history. A reconnect snapshot restores current state, not missed observations. An unchanged value and a live connection do not by themselves establish complete history. Heartbeats support health checks only to the extent their documented contents prove it. Missing evidence must leave affected calculations unavailable; never invent crossings or old checkpoints.

### Provider evidence consulted

* [Kalshi order books](https://docs.kalshi.com/websockets/orderbook-updates): use real-time mode for the initial plan. Build from a snapshot, apply ordered deltas, and replace the book on a replacement snapshot. Detect subscription sequence gaps and rebuild. The documented combined-update mode can omit intermediate changes and therefore needs separate compatibility proof for #4's observation-sensitive rules. An acknowledgment can arrive without actual market data when selection is wrong.
* [Kalshi BRTI](https://docs.kalshi.com/websockets/cfbenchmarks-value): discover the index and verify actual BRTI frames. The documented stream is roughly once per second. [The 5 Hz channel](https://docs.kalshi.com/websockets/cfbenchmarks-value-5hz) offers up to five updates per second and different payload contents. Treat channel choice as a documented input-delivery profile: changing observation frequency can change rule outcomes. Do not silently switch profiles. Start with the existing per-second baseline for planning; compare the faster profile before approval. Neither channel's existence proves Connor's access.
* [Coinbase channels](https://docs.cdp.coinbase.com/exchange/websocket-feed/channels): trade messages plus product heartbeats are a candidate route. The documentation warns that matches can be dropped; heartbeat trade IDs help detect omissions. The first last-match message is initialization, not a newly observed trade. The ticker combines some updates, so it cannot automatically replace a complete trade stream.
* [Kraken trades](https://docs.kraken.com/exchange/api-reference/spot-websocket-v2/trade): use BTC/USD, preserve each trade in a batch, and deduplicate by documented trade identity. Its optional snapshot contains only the latest 50 trades, not guaranteed outage coverage. [Kraken heartbeats](https://docs.kraken.com/exchange/api-reference/spot-websocket-v2/heartbeat) establish connection activity but carry no trade watermark. Proving history completeness requires more than these heartbeats; do not assume trade identifiers are gap-free without evidence.
* [Kalshi CF REST](https://docs.kalshi.com/cfbenchmarks/rest-passthrough.md): documented historical access requires entitlement and costs more read capacity than ordinary calls. It shares the Kalshi delivery dependency, so it is not an independent replacement for an unavailable Kalshi service. Historical retrieval cannot retroactively authorize a missed action under #4.
* [CF's BRTI page](https://www.cfbenchmarks.com/data/indices/BRTI) directs product/service users to contact CF for real-time or historic access. Direct CF delivery is a candidate for future authorized investigation, not a promised free backup. No evidence consulted establishes local retention, redistribution or derived-data rights for Metten.

The CF documentation root returned 404 in this session; its API guarantees were not verified. No authenticated messages, contracts or account entitlements were inspected. Public documentation establishes possible interfaces, not operational access or measured latency.

### Smallest complete connection plan

1. Map every supported #4 binding to a documented provider field or an explicit unsupported/unavailable reason. Resolve exact BTC15m market identity, YES/NO interpretation, price units and market-reference meanings. Record access and retention uncertainties before implementation commits to them.
2. Keep provider decoding in trusted Rust adapters. Preserve source, instrument, contract/side, units, provider time, local receipt time, connection generation, subscription and available sequence/trade identity. Do not manufacture a provider sequence where none exists. Keep secrets inside trusted connection code and existing protected credential storage.
3. Run required connections concurrently and deliver validated observations to the existing engine in a defined acceptance order. Keep screen updates, slow network recovery and diagnostic writes from delaying receipt and evaluation. Preserve all strategy-relevant observations; combine screen refreshes only. Use bounded queues and expose overload as invalid data rather than silently dropping messages.
4. Give each required binding its own state: not configured, access denied, awaiting first data, current, stale, disconnected or rebuilding. Distinguish confirmed denial from unexplained absence. Display a concise reason and the affected strategy inputs. A failed optional source must not block an independent strategy that does not use it.
5. On interruption, invalidate affected evidence, reconnect with bounded retry pacing, resubscribe and rebuild current state. Preserve already observed checkpoints under #4. Recover historical readiness only under its existing rules. A Coinbase or Kraken connection remains its own input; it never becomes BRTI. Failure handling must prevent future unsafe purchase eligibility; this slice itself exposes no purchase endpoint.
6. Prove the adapters with controlled messages, then add focused packaged checks for credentials, connection status and the screen boundary. Any real credential/account check remains a separately authorized step. Future order integration reuses this data path; order submission and fill recovery are outside #5.

### Performance evidence to require

Measure provider timestamp to local receipt with clock uncertainty recorded; local receipt to validated engine acceptance using a monotonic clock; acceptance to completed evaluation; queue depth/oldest-event age; and time to detect a gap and restore trustworthy state. Report typical and tail delays plus maximum observed backlog, CPU and memory for a stated workload. Separate provider cadence from network delay and application delay. No millisecond promise is established yet.

Use bounded generated provider-message fixtures with bursts and interruptions, and compare the engine's output with its controlled-input expectations. Include a brief true/false excursion within a burst so an optimization that drops intermediate observations fails visibly. Include duplicate/reordered messages, wrong source, wrong contract, wrong units, acknowledgment without data, denied access, queue overflow, a replacement snapshot, reconnect replay and a gap during a rolling window. Verify that the read-only boundary cannot submit an order.

Select source-specific freshness, queue and workload limits from documented guarantees and measurements; none are approved constants yet. When completeness cannot be substantiated, report the affected history-dependent input unsupported rather than waiting indefinitely or claiming healthy coverage. Controlled checks do not establish real feed performance or live fills.

## Goals vs. Non-Goals

* **In scope:** the four existing named sources; exact supported-field mapping; guided source setup/status; recoverable connections; data identity and completeness; bounded processing; provider fixtures; performance evidence and explicit access/licensing blockers.
* **Out of scope:** extra providers/assets, new strategy operators, D/F definitions, order submission, paper fills, account access without exact authorization, paid subscriptions, raw-feed archival, automatic substitution and a second BRTI delivery contract.

## Alternatives Weighed

* Kalshi-only completion: smaller, but leaves the other requested #4 sources without connections. Keep Kalshi first in implementation sequence rather than make it the whole outcome.
* Substitute Coinbase/Kraken when BRTI disappears: rejected in this proposal because they measure different source prices and would change explicit strategy meaning.
* Fetch all available data: unnecessary bandwidth and processing; subscribe to exact admitted needs and share required subscriptions.
* Poll repeatedly for live prices: avoidable request load and missed intermediate observations. Use REST selectively for discovery and documented recovery.
* Add redundant connections or direct CF immediately: no evidence yet that their benefit justifies complexity or access cost. Reconsider only for a measured reliability requirement and documented access.
* Promise the fastest possible app or silently choose the highest feed rate: untestable promise and a possible rule-behavior change. Require measured delays and an explicit delivery profile instead.

These are proposed tradeoffs, not additional owner decisions.

## Key Decisions & Staging for Next Steps

### Connection-library recommendation — not yet an adopted dependency

Use a Rust-native Kalshi adapter within the accepted Rust/Tauri boundary. PyKalshi is an unofficial Python client with useful streaming, retry and book helpers, but adopting it would require integrating and packaging a Python runtime or service with the Rust application. No measured speed or reliability comparison establishes a winner; avoiding the extra runtime is the architectural reason to prefer Rust here.

Kalshi's [official SDK guidance](https://docs.kalshi.com/sdks/overview), checked November 14, lists Python and TypeScript SDKs and warns they may lag its API. It recommends its OpenAPI/AsyncAPI specifications as authoritative and generated or direct integration for production. Prefer established Rust networking and cryptographic libraries, with a small Kalshi-specific boundary limited to required endpoints/channels. Do not build a general SDK or implement transport/cryptography from scratch.

Community Rust clients exist. [kalshi-pmx](https://docs.rs/kalshi-pmx/latest/kalshi_pmx/) is a plausible reuse candidate because its published documentation includes CF channels, raw-message preservation and reconnect tests. This pass reviewed documentation, not a full source/security/API-compatibility audit; it is not selected or proven. Before choosing a dependency, check current authentication schemes, exact BRTI fields, precision, snapshot/delta sequencing, unknown messages, queue behavior, secret handling and recovery against the current official specs. Reuse a suitable client if that is smaller than maintaining the required adapter; otherwise use standard transport libraries directly.

The meaningful acceptance checks are controlled disconnects, missed/duplicate messages, snapshot replacement, stale BRTI and overload, plus measured receipt-to-engine delays. Automatic reconnect alone does not prove restored data validity. No dependency was installed, no benchmark was run and no account connection was attempted for this recommendation.

Confirmed direction: reliable existing BTC inputs, Kalshi connectivity and independent Coinbase/Kraken prices, no additional input categories, and a concise performance plan. Existing source identity and gap semantics remain binding.

Proposed sequence: prove field/access feasibility; establish shared adapter admission and health behavior; complete Kalshi learning-example integration; connect BRTI, Coinbase and Kraken through the same contract; finish outage, overload and performance evidence. Any source whose required semantics or rights cannot be established becomes an explicit blocker to the full scope, not a silently completed checkbox.

Next gate: Connor confirms the anchored hypothesis before independent Goldfish review. This file has not received that review. The brief remains in local planning storage until approved for a maintained proposal.

## D–H compatibility discovery — 14 November

### Subsequent owner preference and external research

Connor prefers BRTI delivered through Kalshi as Metten's primary reference, with Coinbase/Kraken available as explicitly identified alternatives. He has not used BRTI in his existing bots; superior reliability remains unmeasured. Exact source switching must preserve the named-source contract and cannot replace missing Kalshi contract data.

The completed [external research report](external-inputs-research.md) recommends retaining the distinct BRTI settlement-average states, Kalshi executed-trade side/size and book changes. It identifies signed BTC spot trades from one existing provider as the first genuinely additional channel to evaluate. These are researched candidates, not approved schema extensions or demonstrated trading improvements. Other venues, derivatives/options signals and extra assets remain lower priority. The lead verified key official schemas, one actual listing's settlement rules and selected pinned public implementation claims. No account or performance test was run.

Connor subsequently authorized read-only inspection of the accessible Kalshi Bot repository and subagent assistance. This authorizes comparison, not importing its implementation, choosing its thresholds for Metten, operating it or accessing accounts. This compatibility scout is separate from the still-pending Goldfish brainstorm review.

Source baseline: clean `/Users/connormoulton/kalshi-live/kalshi_bot` checkout at `da76992c`. This is source evidence, not proof of deployed runtime behavior. The development checkout shares that commit but has unrelated local work; final findings must be checked against the clean baseline. GitHub main reported `43f40adf` during the inspection; do not describe the local baseline as identical to remote main.

The original four-source proposal is insufficient to reproduce the existing D–H bots exactly:

* All five tracked examples select a shared historical scoring model. It uses changes over six and 24 hours, recent range position and realized volatility against a roughly week-long baseline, plus piecewise component scores and a weighted total. These exceed Metten's present calculations, individual one-hour lookbacks, four-hour cumulative warmup ceiling and post-Go-only history contract.
* The existing selected BTC price prefers the midpoint of Coinbase/Kraken bid and ask, falling back to last trade. Metten presently exposes only their named last-trade values. The old source-selection policy can also switch between providers; Metten requires explicit source identity. Exact reproduction requires a deliberate supported input/policy decision, not relabeling a last trade or silently substituting a source.
* Kalshi executable book quantities and exact market threshold/close metadata are required alongside scalar prices. F/G/H additionally depend on ordered, distinct, fresh book observations and connection identity. H uses a specific pre-boundary book selection, which is not automatically equivalent to current-price evaluation or a remembered checkpoint.
* E/F/G/H are variants that buy and hold to settlement, making their broad lifecycle closer to Gen 1. Their exact attempt, zero-fill, partial-fill and timing policies still need comparison with Metten's shared execution contract. D's opposite-side hedge/lock lifecycle falls outside Gen 1's no-hedging boundary. Separately selectable resting F policies also fall outside IOC-only scope; their presence does not make them the default tracked F method.
* The inspected shared signal and tracked source path use Coinbase/Kraken spot and Kalshi market/book data, not a BRTI signal input. BRTI remains a legitimate separately named Metten input, but adding it does not fix the historical/calculation or execution gaps above.

Recommendation: keep issue #5 focused on truthful source connections and exact field feasibility. Include Coinbase/Kraken bid/ask/midpoint availability and historical access in the feasibility inventory, without silently extending the accepted strategy schema or history semantics. Treat exact D–H reproduction as a separately scoped compatibility/design decision before any implementation commitment. A simplified future D/F strategy can use existing capabilities only if its differences are explicitly accepted.

Verification performed: source/template/registry comparison and direct code reads by the lead and two native Codex scouts. No strategies were executed, no tests importing the bot runtime were run, and no real feeds, credentials, accounts or operational configs were inspected.

Source landmarks for this comparison (all Kalshi Bot paths use the clean baseline above): `bot/market.py:280` (quote fields), `bot/market.py:323` (midpoint preference), `bot/spot_selector.py:43` (source selection), `bot/strategy.py:766` (historical scoring inputs), `bot/spot_history_aggregate.py:9` (history profile), `bot/spot_history_aggregate.py:127` (volatility), `bot/conviction.py:8` (scoring), and `bot/experiment_registry.py:66` (D–H method definitions). Metten's comparison points are `docs/strategies/reference.md:24`, `:38`, `:117`, `:134` and `METTEN_DESIGN_DRAFT_2026-11-12.md:173`.
