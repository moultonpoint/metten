# Issue 5 step 4 implementation plan

Status: prepared for owner review, revision 2, 16 November 2026. This request authorizes planning only. Implementation, external connections, credential access, publication and later stages have not been authorized by this plan.

## Outcome and scope

Connect the existing native transport foundation to the existing synchronous source owner through concrete provider adapters. Implement Kalshi first, including exact contract books/trades and current/next discovery, then BRTI, Coinbase Exchange and Kraken. Prove the resulting values, identity, ordering, loss handling and handoff using synthetic messages, loopback peers and controlled time. Keep unproved freshness, observations, history and references unavailable. No connected financial run is created.

The expected change path is fixed transport receipt → bounded provider decoding/reconstruction → existing owned worker and SourceState → bounded derived snapshot; controlled tests additionally feed SourceEffect batches into the real Evaluator. Native modules own provider formats and reconstruction. SourceState owns normalized source facts, authoritative normalized market descriptors, capabilities, market readiness and evidence generations. Evaluator retains all strategy/history/arithmetic ownership. The learning strategy is a fixture consumer, never a strategy-name branch.

Reuse the [approved architecture and ordered sequence](market-data-design.md#ordered-work-and-stopping-conditions), [step-2 contract](step-2-plan.md#small-public-interface), [native contract](../../../src-tauri/src/README.md#step-3-provider-foundation), [engine source contract](../../../crates/engine/src/README.md#synchronous-source-evidence), [source profiles](../../sources/README.md), [product specification](../../../METTEN_DESIGN_DRAFT_2026-11-12.md) and [ADR 0001](../../adr/0001-rust-engine-and-tauri-boundaries.md). This plan refines step 4; it does not supersede those behavioral requirements. Revision 2 resolves the independent review finding about changed schedules: it explicitly adds a bounded SourceState descriptor-replacement transition and its tests, and defines the limit for an already-entered market or an attached controlled evaluator.

Excluded: Sources UI/IPC/permissions and packaged source-frame bootstrap (step 5); calibration and capacity qualification (step 6); a general feed platform; financial integration; order/account/history endpoints; ordinary Keychain operations; real feeds, credentials or accounts; paid services, legal outreach, publication and deployment. Normal application startup remains disconnected. No source-selection persistence, SQLite migration, strategy change, compiler upgrade or engine network dependency is needed.

## Baseline and instruction discovery

Selected worktree: `/Users/connormoulton/.t3/worktrees/metten/t3-5ff0c48e-8b502ba4b606ca11`, branch `t3/brainstorm-issue-5`, code `cc22fa9d2e4e749df0ef624aac89fa7cc745700e` (the reviewed socket-retry correction atop step 3). Preserve the unrelated tracked `docs/README.md` edit and every untracked proposal/review artifact. No staged changes existed at planning entry.

The authoritative remote is `https://github.com/moultonlavah/metten.git`; fresh `git ls-remote --symref` resolves default branch main to `354f731bc75224d9724ec460b2bb0037a9fa12ea` on 16 November 2026. Root AGENTS and applicable published discovery/verification, maintainability, CONTRIBUTING, domain, triage and issue-workflow instructions were traversed. Published/local nested discovery finds root AGENTS only. The local root lacks newer process pointers; apply the published standard without altering selected code. No consequential instruction conflict or missing required repository guidance was identified. The existing issue-5 evidence record holds this task's discovery/check receipt; refresh published guidance at implementation handoff and delivery.

Approved design SHA-256 is `21e2926329f67fef05b264dcb8a0dfca0455aaa697c753bfc4f2d95b19fe4bc7`; preserved step-2 revision-5 plan SHA-256 is `528b87824818515a4349ee99b658f9bc77ed2deb532e514b26f7d7ffa8b60a76`. Both matched during planning. The step-2 plan explains the existing seam, not permission to change it. Issue #5 remains open, product/high/ready-for-agent, all four acceptance criteria unchecked, no comments/assignee; native/body blockers #3 and #4 are closed, no children were returned. This local plan makes no tracker changes or issue-completion claim.

## What must remain unavailable

Production-profile capabilities are separate from parser success. A synthetic fixture can supply explicit capabilities and leases through the trusted Rust test interface; it cannot upgrade the production profile. Do not expose a runtime or renderer switch that enables those fixture facts.

| Gap | Step-4 treatment | Evidence needed before enabling the capability |
| --- | --- | --- |
| U1 freshness | Decode/display received values and ages; leave current strategy usability and real-profile successor readiness blocked. Never derive a price lease from the 10-second transport deadline. | Field-specific justified validity policy and the approved measurement evidence. |
| U2 book observations | Reconstruct every admitted book transaction. Treat mapped book values as initialization/resynchronization while observation semantics remain unsupported. | Explicit mapping of same-price, quantity-only and unrelated-level changes to the existing observation meaning. |
| U3 complete history | Preserve known loss and refuse unproved CoverageFact emission. Reconnect initializes the present, never repairs old cells. | Source-specific completeness, watermark and interval-alignment evidence. |
| U4 precision and identity | Preserve lexical numbers; reject incompatible values without rounding. Apply the conservative identity rules below without claiming universal provider precision support. | Provider precision envelopes and substantiated conflict/replay behavior. |
| U5 references | Opening and settlement bindings remain unavailable. Provider averages are labeled details, never substitute references or engine averages. | Exact rule/field/sample/finality/rounding correspondence. |
| U6 access | Exercise synthetic missing/absent/denied cases; no actual entitlement claim. | Separately authorized account/channel verification. |
| U7 rights | Use generated fixtures; no actual feed collection, retention/distribution claim or legal clearance. | Applicable rights evidence and any exact owner authorization needed. |

Completion therefore has two explicit results: controlled integration works with stated fixture assumptions; real-profile capabilities remain blocked where their evidence is missing. If Connor requires proven real-feed usability as this stage's result, that is a scope decision before implementation, not something tests can silently establish.

## Ownership and minimal interfaces

Use the existing three provider-task slots, one native worker, one-slot ordinary intake queue, Transport, Memory, Budget/Retry and SourceState. Concrete modules are enough. No new crate, generic adapter registry, middleware, event bus, scheduler or per-message task is justified.

| Owner | Authoritative state and interface | Required boundary |
| --- | --- | --- |
| Transport | Fixed hosts/routes, TLS, charged raw bytes, actual-attempt ledger and one provider Retry owner. | Never decodes market meaning or owns SourceState. |
| One task per provider | Socket/session lifecycle, pending finite controls, at most the existing two shared metadata requests, operation deadlines and cancellation. | No CPU-heavy book reconstruction, no shared application lock across await, no hidden retry loop. |
| Provider adapters on the worker | Bounded typed decoding, exact unit conversion, per-provider sequence/identity ledgers and two Kalshi book states. | Library/wire types remain native; no strategy names or financial calculations. |
| Existing native worker | Sole SourceState writer, stream-token mapping, local accepted order, coalesced desired subscriptions/recovery, bounded latest source view. | Invalidity has priority over ordinary intake, including an idle or full queue. |
| Controlled test consumer | One outstanding SourceEffect, actual Evaluator admission/slices, full dependency stamps and completed frontier. | No automatic production evaluator/run creation or Start/Advance wiring. |

Expose only the internal native operations actually needed: prepare an admitted validated-strategy/source context, start/stop a fixed provider, consume a bounded receipt, and read a derived latest snapshot. No Tauri command is added. A test constructor supplies controlled peers/time/capabilities; production construction has fixed destinations and unsupported capabilities. Keep implementation details private and reuse existing errors where their meaning fits; add finite source-specific reasons only when a tested distinction cannot be represented.

The read-only context borrows validated strategy requirements for construction, using SourceState::new rather than retaining another strategy graph. Its monotonic epoch is the controlled source-context start (the existing since-Go coordinate), not a financial Go. Fixtures provide the matching RunContext. Signed market offsets permit a current market opened before that epoch. Production adapter integration does not start a financial run or manufacture post-Go history.

### Socket service without accidental cancellation

The current public send/receive methods borrow the whole Connection and intentionally close/revoke if dropped. An actor that selects receive against each control/timer and discards the losing receive future would disconnect itself on normal housekeeping. Step 4 must handle this concrete integration boundary in Transport, preserving the step-3 public cancellation contract.

Use the installed futures/Tokio facilities to service read and write halves within one owned provider future, concurrently polled child futures rather than extra spawned tasks. Keep receive progress alive across control scheduling and metadata completion. One lifetime owner revokes the connection when either side fails or is canceled; it cancels the sibling and observes both before the provider task returns. Necessary pong flushing cannot depend on the subscription budget. Test the library's actual split/flush behavior before relying on it; if the selected implementation cannot preserve those semantics, stop and revise this bounded transport design rather than add detached loops. No caller-facing arbitrary socket interface is needed. Keep these implementation choices behind Transport and test the actual installed tokio-tungstenite 0.30.0/futures-util 0.3.32 behavior, not a mocked socket service.

Keep provider-task permission separate from one socket session. Currently Connection::fail revokes the supplied Fence permanently, while Coordinator::recover_provider refuses recovery until the owned provider task is joined. Do not bypass that guard to reconnect from inside a running task. For the new service loop, derive a bounded session fence from the task's existing permission fence; its current check requires both its own session and the parent task permission. A failed session is terminal, revokes its queued evidence and updates SourceState generations; the still-authorized provider task may create a new session only after the old read/write futures and socket are finished. Parent Stop/revocation or worker failure invalidates every child session immediately. Provider task return/panic still uses the existing lifetime guard. Retain no unbounded session registry or detached reconnect handle. Tests must race parent Stop with old-session completion and attempted new-session creation.

The task remains the single reconnect owner around Transport's existing admission/backoff; socket errors must be recorded exactly once, never by both layers. Retain exponential retry delay starting at 5 seconds, capped at 60 seconds with bounded positive jitter; after five failed attempts, admit at most one attempt per minute. Reset only after 30 seconds of sustained substantiated usable data. A fresh socket or handshake does not reset retry failures. Reset the usable interval at every disconnect/replacement, including explicit close, and call retry_usable only for substantiated usable data. Unsupported U1 profiles cannot claim sustained usability merely from receipt. Classify permit-expired work separately from actual I/O timeout; do not retry Stop, denial, invalid configuration, protocol or capacity failure. Document any newly classified actual-I/O timeout and prove it does not count unpolled work.

### Nonqueued invalidity and publication

Extend the existing provider fence/control path with fixed stream scope and reason facts needed for shared-book versus whole-provider loss. Keep the stop signal outside ordinary intake. The worker checks these facts before decoding/applying ordinary work, between bounded operations and immediately before publication; it also checks while idle. On invalidity, SourceState::invalidate advances the relevant engine generations before fresh initialization. A derived view is unusable immediately when its associated transport/stream fence is revoked, even before the worker publishes the replacement view.

Bind subscription identity to the SourceState StreamToken captured when that subscription/recovery work starts. Queued metadata or data retains that captured identity. Never acquire a new token when a late response arrives. Carry monotonically checked subscription incarnations so reusing a server sid does not legitimize old work. Fence each published snapshot with the generations it actually used; an atomic permission recheck is required at the consumer boundary, not only before constructing the snapshot.

Use one bounded latest desired-subscription/recovery record per provider, not an unbounded command queue. Replaceable desired sets can use watch; every required data observation stays on ordered intake. Track acknowledgments separately from desired state. Coalesce only redundant discovery/recovery requests, never price events or mandatory command outcomes. Maintain fixed outstanding-operation records with the existing two-request metadata admission. Stop takes precedence even when data or control work is permanently ready.

## Ordered implementation and completion checks

Implement these in order within issue #5. Record each result in the existing verification record. Completing an intermediate item does not complete step 4.

### 1 Establish bounded parsing and the controlled integration seam

Add the three concrete provider modules, initially exercising Kalshi only. Decode typed bounded fields with Serde; validate nesting/text/collection limits before retaining allocations. Unknown bounded provider extension fields may be skipped; reject duplicate known fields and malformed required structure. Do not deserialize an unbounded serde_json::Value tree and validate afterward. Preserve Kraken numeric lexemes, including exponent cases; use borrowed raw tokens/custom visitors or a narrowly justified existing-serde feature, never f64 conversion. A small shared parser helper is justified only for identical lexical/depth/bound enforcement used by the real adapters.

Wire raw socket/metadata receipts to the existing worker and SourceState. Metadata responses carry their route, generation and charged bytes; only the fixed route enum can originate them. The worker alone runs decoder/book state. Retain raw payload reservations until decoding/handoff finishes and reserve decoded/output growth before allocation. An input failure invalidates its affected stream rather than leaving its previous value healthy.

Done when real decoder → SourceState fixtures exercise exact units, wrong identity, malformed and oversized messages, acknowledgment without data, unsupported capabilities and prompt nonqueued invalidity. The socket-service test must show that sending an ordinary control or finishing metadata while receiving a partial frame neither loses bytes nor spuriously revokes a healthy connection.

### 2 Reconstruct Kalshi contract books and trades

Subscribe to real-time orderbook_delta for exact admitted tickers. A snapshot replaces the full side maps, including empty arrays. Apply each delta once using exact fixed-point quantities; zero removes a level, a negative resulting quantity or malformed side/price invalidates the affected stream. Enforce uniqueness/ordering rules for snapshot levels. The best positive YES/NO bids determine their own bids and opposite-side buyable asks through exact 100-minus-bid conversion. Missing opposite liquidity means unavailable, not zero. Preserve buyable quantity as bounded detail, not a strategy operator.

One wire transaction produces one coherent normalized update across all affected YES/NO bindings, including validity when substantiated. Validate the full transaction before exposing any changed side. Maintain sid/seq at subscription scope across both retained markets, checking envelopes before payload filtering. A:41 → missing B:42 → A:43 revokes both books and successor readiness. One coalesced recovery obtains verified replacement state for every retained ticker; restoring one book cannot restore the other. A second gap invalidates both again. Replacement responses require the current recovery/subscription identity; repeat subscribe/ack alone never grants initialization.

Trade decoding uses the exact YES/NO dollar fields, quantity, trade ID, source time and market identity. Initialization/replay never adds observations. New same-price trade identities remain distinct accepted events. For an empty or newly missing side, do not leave a prior ask usable. Use the existing invalidation/reinitialization contract for the affected book stream (both retained books if the shared scope requires it); preserve a last display value only with an explicit unavailable reason. No new engine deletion/value semantics are implied.

Kalshi's opaque trade IDs require a charged duplicate/conflict ledger for each of the two retained contracts; retain it across that contract's reconnects, release it at retirement, and fail capacity rather than silently evicting IDs and recounting replays. Do not turn an opaque ID into a guessed monotonic sequence.

Done when controlled fixtures reconstruct exact learning-example ask inputs (e.g. YES/NO bid 36/64 cents gives asks 36/64), exercise empty/replacement books, both-side atomicity, duplicate/conflicting trades and cross-market sequence gaps. Under explicitly controlled capabilities, feed the real learning JSON through SourceState and Evaluator to verify its unchanged checkpoint/relative-side behavior. With ordinary unsupported profiles the same messages produce display facts and capability blocks, not a usable decision.

### 3 Discover current and successor markets and prove handoff

Use only exchange status, KXBTC15M series, bounded filtered listing and exact-market detail routes. Select by verified series, IDs and authoritative open/close times, not title/list order/ticker guesses. Validate relevant paused/status/schedule facts. Reject ambiguous pairs, repeated cursors and incomplete discovery; allow at most two listing pages. Use compatible filters, without combining status=open and close-time filters. Do not assume cursors share ticker grammar: confirm bounded opaque cursor encoding against official API evidence and keep URL construction safe within the fixed route.

Reuse validated metadata. Normal startup permits at most six planned GETs (status, series, two pages, two details); each successor discovery at most four (two pages, two details), counting retries separately. Lifecycle changes request only needed coalesced detail. Validate global lifecycle envelopes before ignoring unrelated markets; there is no invented server ticker filter. No quote polling or refresh-driven provider requests. Subscribe only to admitted requirements and approved source details: the learning ask-only context does not implicitly open BRTI or spot feeds. The all-source controlled fixture separately checks three initial handshakes and seven initial subscription/discovery controls, excluding pongs, errors and counted retries.

Prepare the immediate successor on existing Kalshi subscriptions; require metadata, initialized current-generation book and all applicable source capabilities/history. At opening, process the source boundary before clock advance, once only. An unready or late-verified successor may become the displayed current market but cannot receive a backdated entry. Late old events stay with the old identity. Retire the old subscription/book/identity ledger and source descriptors before admitting a third market; preserve underlying connections and durable/evaluator history.

Fallback checks at 60, 30 and 10 seconds before expected opening perform only missing work and skip missed points, in-flight equivalents, Stop or budget/backoff refusal. After opening, missing-successor discovery is coalesced to at most one attempt/minute, including lifecycle-triggered checks. Map UTC schedules to monotonic deadlines with checked signed offsets; sleep/wake or detected wall-clock discontinuity revokes schedule/readiness assumptions and triggers bounded revalidation, never replayed timer bursts. Do not invent a clock-drift tolerance as a source-validity guarantee.

#### Changed descriptor recovery

The existing Metadata event changes only verified; RegisterMarket cannot replace a retained ticker. Add one bounded SourceEvent::ReplaceMarket transition in sources.rs for a retained, not-yet-entered market. Its inputs are the captured Metadata StreamToken, the expected old normalized MarketDescriptor and a replacement descriptor. Keep series_id, market_id and contract_id identical; a changed identity is not a schedule correction. Match the expected descriptor and token before replacement, using existing clock, identity-length, schedule-duration and nonoverlap validation against the other retained market. Do not store an independently mutable native normalized schedule or add an upsert/catalog framework.

On a genuine schedule change, revoke Scope::Market before any new descriptor can be used. Reuse its metadata revision, book/trade generations and invalidation effect so old snapshots, pending results and metadata completions fail their existing checks. Validate bounded inputs before copying; a candidate that cannot be installed leaves affected preparation unavailable and returns a finite error, with the existing revocation-delivery obligation if an error supplies no effect. There is no rollback that restores readiness. Atomically replace only the matching descriptor, retaining at most two markets and the existing binding IDs/scopes; reset its preparation and affected initialization/validity/coverage facts. Preserve observation high-water marks and the adapter's identity ledgers. Advance affected binding incarnations or the existing equivalent stamp components so no pre-change stamp survives. Do not clear unrelated underlying data or reopen BRTI/spot sockets.

The replacement is initially unverified. Obtain fresh metadata under the new Metadata token before setting verified, then rebuild required book/trade initialization under current tokens. Do not re-label the response that triggered revocation as a fresh verification or acquire new tokens to admit an old queued response. Recompute the displayed current identity from valid descriptors; never leave a market displayed as current solely because the former schedule placed it there. An identical descriptor is an idempotent no-op, not a reset of entry/readiness. A newly accepted schedule with an opening at or before the accepted time may update display after verification but cannot create a backdated first entry. Reject overlapping/invalid schedules and foreign identity; remain unavailable until a valid replacement or explicit new-context recovery.

An already-entered market's schedule is immutable within that source context. A correction affecting it revokes affected evidence and requires an explicit stopped/new-context recovery; it cannot rewrite a past boundary, checkpoint or history. Retain the old context/history as stopped evidence. Source-only recovery of an unentered successor preserves the existing epoch and once-only-entry bookkeeping; it is not a new financial Go.

The controlled Evaluator also retains immutable registered descriptors. Do not send a duplicate RegisterMarket, deliver new-schedule updates into that old evaluator, or add evaluator catalog mutation in this step. On replacement of any descriptor registered in an attached controlled evaluator, invalidate its result stamps immediately, deliver applicable revocations under the existing one-batch protocol when possible, and terminally stop that evaluator with SliceControl::Stop. Preserve its retained history and checkpoints; do not silently recreate it or claim uninterrupted evaluation. A later explicit controlled recovery constructs a fresh source/evaluator pair with matching descriptors and a new epoch, starting new history under the existing next-full-window rule. This restriction affects the controlled evaluation fixture only: the native read-only source owner can recover the unentered successor and publish its actual data; no production financial run exists. Future connected trading must separately define schedule correction for active runs before it can consume this path.

Required public-source regression: register M and successor N (initially opening at 12:15), prepare N, then supply a verified same-ticker schedule correction. Assert immediate readiness/stamp loss; old metadata/book completions rejected; the same retained slot holds only the new descriptor; fresh metadata alone cannot restore readiness; fresh current-generation data can restore controlled readiness; and an on-time new boundary enters once. Add late-open, invalid/overlapping schedule, identical retry, expected-old mismatch and Stop/Pending races. For a changed descriptor already in a controlled Evaluator, prove the old result is unusable and the evaluator stops with its history retained; explicit new-context recovery never inherits a candidate/checkpoint or backdates entry. An entered-market correction must take the stopped/new-context branch. First demonstrate the current interface cannot install the changed descriptor, then make the positive replacement regression fail against that baseline.

Done when repeated controlled rollovers and changed-descriptor recovery prove zero boundary-search GETs for ordinary prepared handoff, no underlying socket reopening, correct request counts, once-only entry, no old-event contamination and bounded retained books/tasks/IDs. Changed-schedule recovery incurs only the existing bounded metadata/rebuild work, not a zero-request promise. Real-profile U1 remains a readiness block; the positive readiness path uses explicitly controlled evidence. Endless evaluator catalog growth and uninterrupted evaluator history across descriptor replacement are not claimed by this source-only rollover proof.

### 4 Add BRTI and named spot trades

Kalshi BRTI uses the same socket/credential and its own logical stream. Discover BRTI with the finite index-list command once per generation, then require matching outer index_id and nested data.id. Preserve the raw tick separately from averages/counts/window metadata. Initialize after reconnect; subsequent strictly newer accepted tick times can carry distinct event identities, including equal prices. Equal identity/equal value is replay; equal identity/conflicting value invalidates; older work cannot replace the present. Keep U4's broader correspondence/precision limits visible. No 5-Hz, history passthrough or independent BRTI connection is added.

Coinbase Exchange uses matches plus heartbeat for BTC-USD. last_match initializes; unique match events update accepted state, including same-price trades. Preserve product/trade ID/time/sequence and maker-side meaning. A heartbeat may expose an omitted received trade but is neither a price observation nor complete-history proof. Do not infer trade loss from every global/product-filtered sequence jump, and add no REST repair.

Kraken v2 uses trade for BTC/USD with snapshot:true. The snapshot initializes only; iterate update members as distinct events in accepted order, even at equal timestamp/price. Preserve lexical price and exact integer trade IDs. Retain numeric-ID high-water/conflict facts across reconnects; reject obsolete replay without inventing observations. Heartbeats prove transport activity only. Sequential identifiers alone do not clear U3's continuity gate.

Done when each real decoder feeds the same source-owner seam, with exact USD/BTC units and tests for initialization versus genuine events, same-price trades, replay/conflicts, wrong product/index, absent versus denied access, batch order and unproved history. Optional failures invalidate only their dependencies; whole Kalshi loss invalidates both contract data and BRTI. No source substitution exists.

### 5 Close cancellation, memory and recovery boundaries

Exercise the integrated read/control service during partial I/O, subscription acknowledgment wait, metadata body read, permit wait, reconnect and worker admission. Handshake deadlines remain 10 seconds; acknowledgment 5 seconds; first verified data or replacement snapshot 10 seconds. Keep total absolute deadlines across fragments/iterations. An acknowledgment does not satisfy the first-data deadline. No-data/timeout may explain transport recovery but cannot supply a price lease or missing history.

Retain the step-3 total 5-second shutdown phase, handle retention, actual completion observation and engine-save guard. No source work detaches on task error/panic/cancellation or worker return. Explicit recovery must first observe retained completion and create fresh source/subscription identities. On failed ordinary admission or SourceState errors that require revocation, deliver applicable invalidation before later initialization. If safe progress cannot be established, remain stopped with finite failure.

The worker checks revocation even while a controlled evaluator batch is Pending. A completed result is usable only after checking stamps for its full dependency set plus external transport fences. A rejected batch invalidates touched accepted source claims and delivers revocations before recovery; unexpected Busy terminally stops the controlled source/evaluator pair. Production read-only code publishes SourceSnapshot, not evaluator purchase candidates. Keep the existing evaluator one-outstanding-batch protocol and reuse its sliced processing.

Done when full-queue invalidity/Stop, stale-generation completion, provider/worker failure, memory refusal, partial protocol cancellation, reconnect and nonresponsive-owned-work shutdown all have meaningful covering checks on the integrated path. No tests may clear controls, access ordinary credential items or silently fall back to real hosts.

## Resource and delivery contract

Keep the approved ceilings; these are refusal boundaries, not qualified performance claims:

| Resource | Bound and required accounting |
| --- | --- |
| Providers and ownership | Three fixed connections/tasks, one worker, one ordinary intake slot. No per-message or detached recovery tasks. |
| Raw input | 1 MiB per message/frame/REST body; one charged raw message per active task plus explicitly charged queued/worker ownership. |
| Parsing | Depth 48, text fields 16 KiB, retained machine/provenance IDs 128 bytes; bounded incremental typed collections. Reject incompatible numbers before normalized admission. |
| Books | At most two active books, 8192 levels per side, combined 8 MiB within adapter allowance. Include capacities, maps and old/new replacement overlap before mutation. |
| Adapter allocation | Shared 32 MiB, including socket reservations, raw/decoded bodies, partial sends, identity ledgers, pending controls and derived native views. Fixed structs/stack storage are separately inventoried. No separate optimistic per-provider memory owners. |
| Engine handoff | Existing SourceState limits (34 requirements, 128 bindings, two markets) and Evaluator 8 MiB/4096 admission including its private metadata. No enlargement. |
| Waiting and publication | Existing two-second raw queue-age refusal, independent of source validity. Latest snapshot/details at most 256 KiB; at most 32 compact transitions, aggregate repeated reasons. No raw feed archive or new durable tables. |
| Requests | At most two metadata requests, existing 50-token burst/10-token-per-second local refill and verified endpoint costs; one handshake/provider, one subscription control/second/provider, pongs separate. |

Reserve before collection growth and preserve the reservation with ownership across task/queue/worker handoff. A valid required snapshot that cannot fit is explicit unsupported capacity, never a truncated book. Fixed data arrays/allocations cannot bypass accounting just because a helper does not use Memory. Build a simultaneous-ownership inventory including replacement overlap; test exact cap and cap-plus-one and release after rejection/cancellation. Step 6 still owns measured workload qualification and any proposed cap revision.

## File inventory and maintainability checks

Apply the [published maintainability standard](https://github.com/moultonlavah/metten/blob/354f731bc75224d9724ec460b2bb0037a9fa12ea/docs/agents/maintainability.md) and [published verification procedure](https://github.com/moultonlavah/metten/blob/354f731bc75224d9724ec460b2bb0037a9fa12ea/docs/agents/verification.md) at the pinned planning baseline, then refresh them before implementation.

Paths are relative to the repository root. Add only files required by a current owner; update this inventory before accepting a necessary departure.

| Path | Planned change and reason |
| --- | --- |
| crates/engine/src/sources.rs | Add only the bounded same-identity, unentered-market ReplaceMarket transition, revocation/stamp handling and fail-stopped entered-market policy described above. Reuse existing schedule/scope checks. |
| crates/engine/tests/sources.rs | Public changed-schedule replacement, obsolete completion, fresh recovery, invalid/no-op/entered cases and attached controlled-evaluator stop/new-context regressions. |
| crates/engine/src/README.md | Document the new source transition, failure/ownership/resource obligations and the controlled-evaluator continuity limit. |
| src-tauri/src/providers/kalshi.rs | New concrete Kalshi decoder, reconstruction, discovery validation and protocol state. |
| src-tauri/src/providers/coinbase.rs | New Exchange match/heartbeat decoder and identity handling. |
| src-tauri/src/providers/kraken.rs | New v2 trade snapshot/update decoder and identity handling. |
| src-tauri/src/providers/mod.rs | Extend existing coordinator/worker, typed receipt/control ownership, stream fencing and bounded latest source view. Keep one accepted-state writer. |
| src-tauri/src/providers/transport.rs | Internal concurrent read/control service, charged metadata receipt and needed fixed-route/cursor handling; preserve public cancellation, TLS and request restrictions. |
| src-tauri/src/providers/memory.rs | Extend existing reservations only for actual decoded/native-view ownership needs. No parallel accounting system. |
| src-tauri/src/providers/budget.rs | Change only if actual integration reveals a necessary retry-continuity or admission correction; retain shared policy and test it. |
| src-tauri/src/providers/tests.rs | Adapter→SourceState→controlled Evaluator fixtures, exact request ledgers, rollover, parser/memory, cancellation/recovery/shutdown regressions. Keep fixture strings local initially; add a fixture file only if genuinely reused or clearer. |
| src-tauri/Cargo.toml and Cargo.lock | Conditional: smallest existing Serde feature change needed for exact numeric lexemes. No new library/version upgrade without demonstrated need and affected evidence. |
| src-tauri/src/README.md | Actual implemented ownership, contracts, capability limits and focused checks. |
| docs/agents/code-map.md | New decoder/integration routes linked to owners and tests. |
| docs/sources/README.md | Update specific mapping/evidence dispositions only as established; preserve U1–U7 rather than declaring them resolved by fixtures. |
| docs/verification/issue-5.md | One implementation/evidence ledger, source identities, executed results, reused checks, review findings and remaining gates. |

The only planned engine production change is SourceState descriptor replacement described above; use existing strategy types and the unchanged Evaluator. Its bounded validation, revision/incarnation headroom, emitted invalidation effect, fixed-market storage and simultaneous replacement allocation must be included in the source work/storage argument. The old descriptor and replacement input may coexist only within the existing finite limits; refuse before committing a replacement if admission cannot fit. No evaluator catalog mutation, strategy.rs production edit or new event-history model is included. If another concrete missing capability prevents the approved mapping, reproduce it, describe the smallest correction and affected resource proofs, and revise this plan before implementation reaches it. No schema, arithmetic, financial or timing semantics may change implicitly. Do not edit main.rs, credential adapters, runner, UI, permissions or generated manifests for step-5 work. The existing worker/coordinator can be exercised internally until explicit step-5 command admission is authorized.

The small shared parsing helper, if required, lives beside the native provider modules and must have multiple actual adapter callers; it hides lexical/cap enforcement only. No speculative provider abstraction. An ordinary wire-field rename should change its decoder and fixture only; a normalized source-failure reason follows its existing owner and downstream response only when that response exists. A supported threshold change stays JSON. These are the maintainability review's concrete change-path checks.

## Verification and stopping conditions

During planning, check links, numerical policy, identities, source ownership and scope; do not build the app or run credential walkthroughs just to validate prose. During implementation, use the real decoders/source API and controlled peer/clock input, with finite fixture timeouts and exact owned-resource cleanup. Tests must fail on the named wrong result, not merely report no crash.

Minimum implementation evidence:

1. Exact learning JSON contract-price/checkpoint/relative-side timeline through actual decoder, SourceState and Evaluator; initialization/replay add no observations and coherent YES/NO updates never expose intermediate decisions.
2. Healthy/absent/denied/wrong-identity/malformed/over-precision/over-cap cases for all three provider modules and four logical sources; ordinary profiles remain blocked by applicable U1–U7.
3. Shared-subscription gap/replacement matrix for both books, duplicate/conflict identity tests, old subscription/connection results, no historical repair and isolated optional-source loss.
4. Repeated rollover with exact discovery/control ledger, zero ordinary prepared-boundary GETs, no underlying reconnection, readiness lost before open, late display without late entry, old-event isolation, bounded missing-successor recovery and sleep/wake revalidation. Same-ticker changed-schedule replacement must pass the complete public-source regression above, including stopped/new-context handling where an entered market or attached evaluator prevents in-context continuation.
5. Resource exact-cap/refusal/release and replacement-overlap evidence; busy/full/over-age queue behavior; partial I/O and control/receive coexistence; stop/caller-drop/worker/provider return/panic; single-deadline shutdown and retained unfinished work.
6. Ordinary native and engine suites, formatting and workspace Clippy on the final behavior source. Existing meaningful tests are reused when unchanged; extend only for uncovered transitions. A concrete bug found during implementation gets a failing-before regression where feasible.

Use the pinned toolchain (`/Users/connormoulton/.cargo/bin/cargo` if absent from PATH):

```sh
cargo test -p metten --locked
cargo test -p metten-engine --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Because this plan now changes SourceState production behavior, rerun its public tests and the ordinary engine suite and update the affected source work/storage proof; do not reuse the old source proof unchanged. Both release-hour fixtures and evaluator arithmetic/resource proofs remain reusable only while strategy.rs, the evaluator/Decimal implementation, toolchain and relevant dependencies remain unchanged. If implementation reaches those assumptions, rerun the affected release fixtures and proof review before acceptance. Reuse UI/Python/Keychain evidence only for unchanged boundaries. Build the Mac bundle if native packaging/dependencies are affected; it is build evidence, not provider or GUI acceptance. Any shared credential/bootstrap/IPC change expands verification and needs an explicit scope reconciliation rather than bypassing its packaged requirements. Remote CI remains unverified until separately authorized publication.

At implementation delivery, inspect the complete diff against its selected baseline, use the published staged-snapshot procedure if a mixed-work commit is authorized, and record exactly what ran against which source. A reviewer follows published AGENTS→map→contract→actual code/tests, checks ordinary change locality, single state ownership and the transition/resource claims. No Council, fixed multi-reviewer roster, scores or independent deployment process is required by maintainability. Delegate only when separately requested or required by applicable instructions.

Step 4 is complete only when all five implementation items and their required controlled evidence are accounted for, unresolved capabilities are explicit and no later-stage work leaked into scope. Failed required checks or an unresolved concrete review objection prevent completion. Stop before Sources commands/UI, packaged source-frame walkthrough and calibration. Issue #5 remains open until its broader acceptance is actually established; this plan cannot close it.

## Public evidence used for planning

The source-profile guide remains the evidence inventory. A bounded public-document recheck on 16 November 2026 confirmed the selected book, BRTI and named spot-feed shapes; it establishes documentation, not observed service delivery or rights:

- [Kalshi orderbook updates](https://docs.kalshi.com/websockets/orderbook-updates)
- [Kalshi CF Benchmarks value feed](https://docs.kalshi.com/websockets/cfbenchmarks-value)
- [Coinbase Exchange channels](https://docs.cdp.coinbase.com/exchange/websocket-feed/channels)
- [Kraken v2 trades](https://docs.kraken.com/exchange/api-reference/spot-websocket-v2/trade)

Before implementing a route/channel, recheck its exact official schema, recovery commands, timestamp/identity units and installed-library behavior. In particular confirm cursor encoding, Kalshi real-time replacement semantics and Kraken lexical-number preservation; do not treat a documentary example as a universal bound. An unresolved schema detail blocks that behavior, not permission to guess or silently substitute. No new legal conclusion or access guarantee follows from this limited recheck.
