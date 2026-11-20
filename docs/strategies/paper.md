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

The pinned `controlled_best_ask_full_or_zero_v1` simulator uses one finite matching best ask. Available depth is 100 contracts in the example. Sufficient quantity fills the whole objective at that ask; insufficient quantity records zero, with no fee and no partial-fill retries. Rules can qualify without any confirmed purchase. Execution requires the exact same ask identity/value that the evaluator used, a current source/book lease, permitted price and a fully funded durable intent. Price/funds refusal skips rather than resizing.

`controlled_general_taker_2026_07_07_m1` assumes 0.07×q×P×(1−P), where q is contracts and P is dollars per contract. Cost plus fee is rounded upward together to $0.0001; the recorded fee is that debit minus exact cost. Internal money uses 10^-8 USD checked integers, while receipts show the significant precision required to reconcile. Four decimals can therefore be meaningful; rounding each line to whole cents would hide part of this simulated charge. This assumption is pinned for repeatable paper tests, not validation of an actual series/account's current fees. Settlement has no extra fee in this model.

## Controls and failures

- Pause purchases now fences dispatch immediately; observations, checks and settlement continue. It is not a sale.
- Resume purchases requires newly accepted current execution evidence; a stale pre-pause book cannot purchase.
- Stop after this market completes the entered market and refuses the next. Before entry, it stops without counting a market.
- Quit fences purchases before native waits, joins playback and asks the engine to save stopped/unresolved state. A failed save cancels quit and retains confirmed history.
- Reopen never resumes purchases or reconstructs an interrupted intent as a fill. Unconfirmed purchase state remains blocked; a confirmed holding may use the explicit saved-manifest settlement action.

The main explanation prioritizes unresolved facts, confirmed full/zero outcomes and specific purchase blocks/rule evidence. An unavailable input is not false, an attempt is not a fill, and pending settlement is not a loss. Observed values are labeled with their time; sparse retained observations prove no whole-market maximum or continuous condition history.

## Workspace and verification limits

This implementation migrates SQLite to schema 4, preserving session/strategy records; older schema-3 binaries refuse the migrated workspace. Never point verification at real financial records. The headless check uses a new owned disposable workspace and no Keychain, provider, GUI or network setup:

```sh
npm run tauri build -- --bundles app
python3 scripts/verify-paper.py target/release/bundle/macos/Metten.app
```

See [issue-6 evidence](../verification/issue-6.md) for actual commands/results, source identity and remaining risk. No paper check establishes live fees/fills, production source usability, capacity, physical power-loss guarantees, distribution signing or notarization.
