# Focused independent recheck — A1/A2/A3

**Verdict: all three original Major contracts closed; design ready for the owner implementation-authorization gate.** One fresh-context read-only reviewer applied the implementation-readiness and codebase-details personas/standard. No consequential contradiction, new Major or weakened safety path was identified. This is design-contract readiness, not runtime proof or implementation/trading authority.

## A1 / original R1 — Closed

Section2.1 chooses latest-completed-only equality and explicit identical/conflicting/stale/gap/pending behavior. Bounded canonical content and completion revision are retained, pendingB0 commits before effects, and B1 atomically advances completed receipt/frontiers only after required durable outcomes. Stopped reopen preserves interruption evidence and refuses ordinary retries; it does not reconstruct source work or infer zero fill. Schema and failure tests carry these rules through.

The reviewer verified encoding feasibility in original source: public SourceEvent/Envelope/clock/binding/coverage/descriptor fields and Decimal::millionths permit normalized payload encoding. StreamToken fields remain private, but equality/SourceState::token are available; the draft checks them live and records a separate paper-owned context generation without unsupported introspection/serialization. No raw-input archive, new dependency or source-private API is required.

## A2 / original R2 — Closed

Section4.1 fixes stopped-only settlement behavior using an immutable bounded synthetic manifest saved in transactionA. Reopen restores the financial virtual frontier separately from evaluator frontier. Explicit check_settlement derives remaining work from confirmed holdings without settlement rows; E commits payout/net/cash/frontier together, without source/evaluator replay or automatic purchase admission. Duplicate checks cannot pay twice, absent final entries stay pending and ambiguous commits require identity inspection. Unresolved purchase intents still block settlement/new runs.

The pre-close example specifies cash/net/minimum frontier. Tests cover changed/removed binary fixtures, absent manifest entry, failedE and repeated reopen/check. The reviewer verified that the existing foundation restores only its separate legacy session tick (lib.rs260–268), so the correction supplies a real paper contract rather than assuming it exists.

## A3 / original B1 — Closed

Section5.1 separates numeric paper_market_id from exact source_market_id. Unique run/source mapping, run-constrained child foreign keys, proposal/dispatch uniqueness, exact series/contract checks and explicit request/response/cursor fields are consistent. Prepared rows do not count as participation. Nonnumeric source IDs round-trip via numeric paper IDs; cross-run refusal and exclusive paging tests are specified.

Original MarketDescriptor/BindingScope/RunContext/ObjectiveView retain string identity and existing source identity bounds (strategy.rs1525–1557,1714–1720,4691). Schema/API summaries match; no extra file or unsupported reuse seam is needed.

## Independent discovery and limits

Reviewer independently traversed immutable root AGENTS at c7b1b4c79d2ca603226f78bcddd143618c3a875e through verification/maintainability/code-map; consulted relevant product/ADR/contracts, original source/storage/evaluator/native interfaces, compatibility reference and check procedure. It used the lead’s fresh origin/main query2026-11-19T06:23:31Z within the pinned review round. Published/local instruction inventories were root-only; no applicable required guidance gap was reported. Selected older868609baffc25f06ecb43d8760e7cdd60f684a68 and unrelated files were preserved. Fresh conversation context used fork_turns none; host instruction-file exclusion is unverified.

The focused check reused unchanged full-review coverage and did not restart all EGM stages. No tests/builds/apps/accounts/provider/external URL calls or reviewer writes occurred. Runtime correctness, real fees/fills, physical storage behavior and actual GUI/WebView integration remain unverified. Implementation must produce the planned tests and evidence.
