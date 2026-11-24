# Controlled paper learning example

The Paper workspace uses synthetic inputs and a 20× clock. It runs a saved format-2 strategy through the same Rust evaluator and financial engine used by the paper API. It does not connect Sources, submit orders or qualify live trading. Only one paper run may be active; unresolved holdings/outcomes block the next run.

## Run the example

1. Choose “Load the learning example to review and save.” Preview the unchanged learning JSON in Strategy versions, then save an immutable version.
2. Return to Paper, choose that saved version and set contracts, permitted price range in cents and starting paper dollars. The entire quantity including fees must be funded. Settings stay fixed for the run.
3. Press Go. It waits for the next full eligible synthetic market, rather than buying the partly elapsed market. The learning strategy remembers the early side, then can buy the opposite NO side at 25¢ in the fixture.
4. Select the participating market in the journal. Read confirmed quantity/cost/fee/debit and pending settlement. “View details” contains fixed settings, model assumptions and retained rule observations; older evidence is a bounded page, not a full tick trace.
5. Let the synthetic outcome arrive, or stop/quit and reopen. Reopening always stays stopped. “Resolve remaining synthetic outcome” explicitly advances only settlement from the persisted manifest; it never replays source data or strategy decisions.

With 10 contracts and $100 starting balance, the controlled NO purchase costs $2.50, its fee is $0.1313 and debit is $2.6313. Cash after purchase is $97.3687. The fixture's final NO outcome pays $10, producing $7.3687 settled net and $107.3687 cash. Those are synthetic receipt facts, not predicted returns.

## Execution and fee assumptions

The pinned `controlled_best_ask_finite_ioc_v2` simulator treats each attempt as an immediate-or-cancel order against one finite matching best ask. It fills whole contracts up to the available quantity and cancels the rest: a full fill, a partial fill, or a zero fill when less than one contract is available (no fee). Available depth is 100 contracts in the example, so 10 contracts fill in full. A book snapshot funds only one attempt; restating the same observation never grants its quantity again. After a partial or zero fill, a newer observation may fund a retry of exactly the unfilled quantity. A confirmed fill locks the side; retries never exceed the requested quantity or start a second purchase. Each retry repeats the rule, evidence, price and funding checks and stops when the purchase option's window or conditions end; confirmed contracts are kept through settlement. Runs recorded before this model keep their original `controlled_best_ask_full_or_zero_v1` label. Rules can qualify without any confirmed purchase. Execution requires the exact same ask identity/value that the evaluator used, a current source/book lease, permitted price and a fully funded durable intent. Price/funds refusal skips rather than resizing.

`controlled_general_taker_2026_07_07_m1` assumes 0.07×q×P×(1−P), where q is contracts and P is dollars per contract. Cost plus fee is rounded upward together to $0.0001; the recorded fee is that debit minus exact cost. Internal money uses 10^-8 USD checked integers, while receipts show the significant precision required to reconcile. Four decimals can therefore be meaningful; rounding each line to whole cents would hide part of this simulated charge. This assumption is pinned for repeatable paper tests, not validation of an actual series/account's current fees. Settlement has no extra fee in this model.

## Controls and failures

- Pause purchases now fences dispatch immediately; observations, checks and settlement continue. It is not a sale.
- Resume purchases requires newly accepted current execution evidence; a stale pre-pause book cannot purchase.
- Stop after this market completes the entered market and refuses the next. Before entry, it stops without counting a market.
- A lost Kalshi connection or unverified current market blocks purchases at once; Now shows paused, but it is not your Pause. After reconnect you do not need Resume: purchases in the same market continue only once its details are reverified, newer evidence from the new connection still qualifies, funds suffice and no purchase is being checked. Checkpoints and remembered sides seen earlier in the market are kept; nothing is assumed during the gap. A market whose opening was missed is not entered. Your own Pause, Stop and Quit always stay in force. A loss that arrives while another request is still being recorded stops the run. No new order is sent, but an order whose send was already committed is still sent once. The embedded playback never loses its synthetic feed, and the live feed is not connected to paper runs.
- Quit fences purchases before native waits, joins playback and asks the engine to save stopped/unresolved state. A failed save cancels quit and retains confirmed history.
- If a purchase reply is lost, Now shows “Trading paused — checking a purchase”. Metten keeps the reservation and sends nothing else while it looks up that exact order at the venue. It then reports the actual result, and later history keeps the interruption and result. A missing or unreadable venue record never counts as zero. Resume is refused while a check is outstanding; a Pause you made stays in force afterward.
- Quit checks in-progress purchases first. Reopen looks them up again but never resumes purchases or rebuilds a fill from saved request details. A purchase still unknown keeps purchases and new runs blocked. **Resolve remaining synthetic outcome** retries the check, then settles each final holding once from the saved manifest.

The main explanation prioritizes unresolved facts, confirmed full, partial and zero outcomes and specific purchase blocks/rule evidence. An unavailable input is not false, an attempt is not a fill, and pending settlement is not a loss. Observed values are labeled with their time; sparse retained observations prove no whole-market maximum or continuous condition history.

## Workspace and verification limits

This implementation migrates SQLite to schema 6, preserving session, strategy and earlier paper records; older binaries refuse the migrated workspace. Never point verification at real financial records. The headless check uses a new owned disposable workspace and no Keychain, provider, GUI or network setup:

```sh
npm run tauri build -- --bundles app
python3 scripts/verify-paper.py target/release/bundle/macos/Metten.app
```

See [issue-6 evidence](../verification/issue-6.md) for actual commands/results, source identity and remaining risk. The check also loses the first purchase reply, quits with that purchase unresolved and confirms it from the venue record after reopening. No paper check establishes live order acceptance, lookup or deduplication, live fees/fills, production source usability, capacity, physical power-loss guarantees, distribution signing or notarization.
