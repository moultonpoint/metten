# Issue #5: connect trustworthy BTC market data

* **Author:** Connor Moulton
* **Date:** 2026-11-14
* **Status:** Owner approved this revised brief as the basis for formal design on 2026-11-14, after all seven independent brainstorm lenses completed and the lead incorporated their findings. This document grants no implementation, account or trading authority.

## Overview

Metten should show the supported Kalshi BTC 15-minute market and deliver the exact named information a strategy requests, with an honest explanation when it is unavailable. BRTI delivered through Kalshi is the preferred reference for strategies that choose it. Coinbase and Kraken remain distinct BTC/USD prices that a strategy may explicitly name. They cannot silently become BRTI or restore a missing Kalshi contract book.

Issue #5 is a read-only connection slice. It includes guided source setup, secure provider credential integration, current market information, bounded source details and controlled proof of recovery and processing behavior. The learning example's Kalshi asks remain the first end-to-end case. Existing format-2 strategies keep their meaning. Exact D–H reproduction, new calculations and trading execution remain separate work.

The [product specification](../../../METTEN_DESIGN_DRAFT_2026-11-12.md), [ADR 0001](../../../docs/adr/0001-rust-engine-and-tauri-boundaries.md) and [strategy reference](../../../docs/strategies/reference.md) remain authoritative. The [frozen review input](brainstorm-review-input.md) preserves earlier proposals; this brief replaces their conflicting wording. The [review report](goldfish-review.md) records findings and limitations.

## Analysis

### Establish what each source can prove

A connected socket, subscription acknowledgment, current price and complete history are different facts. The initial feasibility gate must produce a compact matrix for each existing binding: exact provider field and units; instrument/market/side; genuine observation identity; evidence of current validity; evidence of complete intervals including quiet periods; gap detection; and what recovery restores. Document numerical precision and admitted delivery profile too.

A source may support current display but not a history-dependent rule. If complete intervals cannot be substantiated, report that limitation explicitly. Do not invent quiet coverage, fill past gaps from snapshots, repair finalized missing history or reconstruct checkpoints that were never observed. Missing evidence must not turn into an endless warmup. An infeasible required binding blocks that capability; it does not justify silently changing source or rule meaning.

BRTI preference does not add a BRTI dependency to a Kalshi-only strategy. The selected strategy's existing requirements determine readiness. A source failure unrelated to its requirements must not block it; the existing rules continue to govern alternatives and startup history.

### Separate connection information from strategy capabilities

| Information received | Use within issue #5 | Strategy meaning |
| --- | --- | --- |
| Kalshi side-specific ask, bid and last trade | Decode, validate and show the exact contract values | Populate existing bindings only with proven exact mappings |
| BRTI index tick | Decode, validate and show source/profile/age | Existing explicitly named BRTI input |
| Coinbase/Kraken BTC/USD last trade | Connect and show each source separately | Existing explicitly named inputs; no mutable best-price alias |
| Market identity, schedule, rules and reference values | Bind the supported market; distinguish provisional and established facts | Opening/settlement bindings remain unavailable until their exact market-rule meaning is established |
| BRTI trailing average and accumulated final-minute average, window/count | Bounded source details and mapping evidence | Informational in #5; neither is automatically the final settlement reference or a new JSON input |
| Kalshi book quantities and trade direction/size | Maintain faithful books/trades and offer bounded read-only detail | No new author-selectable quantity, flow or imbalance field in #5 |
| Source time, receipt time, connection/subscription identity and sequence | Prove identity, health, continuity and processing progress | Trusted evidence; never supplied by strategy JSON |

Current BRTI ticks, a trailing average, a partially accumulated closing average and the final market-defined settlement value must have distinct labels. Exact sampling boundaries and rounding require correspondence checks; a plausible average is insufficient. Additional received fields do not authorize a strategy-format extension.

Coinbase/Kraken bid/ask midpoint selection, signed spot-flow channels and long historical backfill remain deferred compatibility/research candidates. If a future strategy needs them, define them explicitly rather than changing a saved last-trade binding.

### Keep the connection path inside Rust

Use trusted Rust provider adapters under the accepted engine/desktop boundary. The screen requests actions and displays results; it does not receive secrets or decide data validity. Subscribe from admitted named requirements, never strategy names such as D or F.

Check one suitable Rust client against the required channels, current authentication, exact decimal handling, raw/unknown-message visibility, sequence handling, reconnect generation, buffering and secret redaction. Reuse it if it reduces the work while passing those checks. Otherwise use established networking and signing libraries behind a narrow adapter. Do not create a general Kalshi SDK or implement transport/cryptography from scratch. No client has yet passed this evaluation. [Kalshi's official SDK guidance](https://docs.kalshi.com/sdks/overview) treats its specifications as authoritative and allows direct/generated integration.

The existing credential code proves only an isolated dummy item in a disposable Keychain. Issue #5 needs a provider-specific native storage/signing path and guarded save/replace/remove behavior. Reuse the demonstrated protection and command patterns, preserve the dummy runner, and test the new path with controlled secrets. Never migrate dummy credentials into provider identities. Actual credential/account validation needs separate exact owner authority.

The read-only boundary must prevent purchase calls even if a selected library offers trading methods. Test the allowed request surface, not just the absence of a purchase button. Preserve finite sanitized errors; do not expose upstream bodies, keys, signed headers or private payloads through screen errors or diagnostics.

### Preserve correctness while measuring delay

Reuse the engine's existing observation, resynchronization, validity, gap, coverage and sliced-evaluation interfaces. A reconnect snapshot establishes present state; it does not count as a new observation. Keep genuine transactions ordered, including brief excursions inside a burst. Coalesce screen refreshes rather than strategy-relevant observations.

Keep normalized pending/queued work within the existing combined admission allowance, currently 8 MiB and 4096 transactions per evaluator. Separately bound transport frames, decoded books and shared subscriptions. Avoid allocating an independent full allowance for every socket or waiting batch. Coordinate recovery/discovery requests under provider limits. Overflow and invalidity must become effective even while evaluation is backlogged; an unfinished evaluation supplies no current candidate.

Begin planning with the per-second BRTI delivery profile and real-time Kalshi books. Identify the profile in evidence. A switch to faster BRTI or combined book updates can change observation-based rules and is not a transparent speed improvement. Freeze the chosen initial profile in formal design after confirming its field coverage; changes need explicit compatibility evidence.

Measure source-to-receipt delay with clock uncertainty, receipt-to-engine acceptance, completed evaluation delay, oldest queued work, memory/CPU and gap-detection/rebuild time. Use one stated admitted workload plus burst, overload and recovery cases. Set operational limits from evidence during detailed design/implementation. No speed or uptime promise exists yet.

### Make connection problems understandable

The Sources view shows the source, affected input, established reason and useful next action. Distinguish not configured, confirmed access refusal, waiting for first verified data, stale data, closed connection, rebuilding and local overload. Silence is unexplained absence; it is not proof of entitlement refusal. Socket activity is not a valid-price receipt.

Show current-price validity separately from history readiness: for example, “BRTI price current; six-minute calculation rebuilding after a gap.” If continuity is unsupported, say so instead of promising that waiting will fix it. Do not imply connected strategy execution exists before its later integration.

Details identify the exact value/profile/market, last valid receipt, continuity status and completed processing age. Keep local transition summaries, aggregated rejection counts and bounded event-correlation evidence linking provider identity to accepted engine observations. No hosted tracing, continuous raw-feed archive or new diagnostic console is needed. Diagnostic failures must not stall intake or masquerade as provider failures.

Connections reopen unavailable until freshly established; the application retains its stopped-reopen behavior. Persist only needed nonsecret settings. Introduce a storage revision only for an actual durable structure change, documenting older-version refusal and preserving immutable strategies and history. Do not persist a green health state as restart truth.

### Evidence and access boundaries

The [external research report](external-inputs-research.md) supports interface availability and distinctions, not operational access or profit. Connor's existing bots did not use BRTI; it is a proposed new primary, not a proven source from prior operation. Kalshi's BRTI and contract feeds share a failure domain. Alternative BTC prices cannot replace the unavailable contract data needed for future purchases.

Source availability, retention/licensing constraints and any paid entitlement need explicit evidence. Where rights or provider semantics remain unresolved, record the exact blocker before committing to that capability. No account login, purchase, subscription payment, redistribution or real-money validation is authorized here.

## Goals vs. Non-Goals

* **In scope:** four existing named sources; exact supported-field feasibility; guided connection setup/status; provider-specific credential integration; trustworthy current and continuity evidence; bounded read-only source details; recovery, overload and delay evidence; access/rights blockers.
* **Out of scope:** purchase execution and paper fills; exact D–H rules or code imports; new JSON inputs/operators; week-long backfill or changed history semantics; automatic source substitution; direct CF redundancy; signed spot-flow subscriptions; derivatives, other venues/assets and macro feeds; raw-feed archival; paid or account operations without exact separate authority.

## Alternatives Weighed

* **Kalshi-only completion:** smaller but leaves confirmed named alternatives unfinished. Use it as the first implementation stage, not the whole issue.
* **Coinbase/Kraken as automatic BRTI replacements:** changes source meaning and cannot restore missing Kalshi contract data. Keep explicit named strategies and reviewed versions.
* **Python/PyKalshi runtime:** useful for a Python bot, but adds packaging and a language/process bridge here. Prefer Rust; no measured performance superiority is claimed.
* **Unconditionally adopt a Rust SDK:** saves work only when required behavior remains observable and testable. Verify a bounded candidate, otherwise use standard transport libraries directly.
* **Collect every available feed or build new history/recovery machinery:** adds work while duplicating existing engine responsibilities. Subscribe to admitted needs and reuse the evidence contract.
* **Add a second BRTI route or redundant sockets now:** unproven benefit and additional access, identity and maintenance work. Revisit only for a measured requirement and separately authorized rights.
* **Treat faster/combined feeds as a free optimization:** can alter observations and rule outcomes. Preserve an explicit delivery profile and require compatibility evidence.

These are the lead's synthesized recommendations, not claims that an external maintainer or Connor rejected each alternative.

## Key Decisions & Staging for Next Steps

1. Formal design starts with the field/continuity/access matrix and the distinction between displayed provider information and existing strategy inputs. Full connected historical capability is not promised until supported by evidence.
2. Define the smallest production credential boundary and read-only request surface; evaluate the Rust client and fixed delivery profile against those requirements.
3. Establish Kalshi learning-example data, then the remaining named sources through the same evidence contract. Do not turn #5 into connected trading or a second strategy engine.
4. Finish controlled decoding, acknowledgment-only, wrong-source/contract/unit, denial, duplicate/replay, gap/reconnect, snapshot replacement, overload and brief-excursion checks. Add focused packaged credential/IPC/status checks and one bounded performance workload. Reuse passing arithmetic and resource evidence where unchanged.
5. Report account access, rights and real-feed evidence separately from controlled acceptance. Any missing required evidence remains visible. Provider impossibility, required paid access or changed strategy meaning returns to Connor as a concrete decision; routine library, retry, queue and UI choices remain engineering-owned.

The intended result is a reviewed concepts brief suitable for the formal EGM design step, not permission to implement or a claim of trading readiness.
