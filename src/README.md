# Controlled-session interface

## Overview

This interface displays the saved state of a controlled, non-trading session and requests actions from the native application. It does not determine recording success or own session state. The [product specification](../METTEN_DESIGN_DRAFT_2026-11-12.md) describes the broader intended product; [ADR 0001](../docs/adr/0001-rust-engine-and-tauri-boundaries.md) assigns authoritative state to Rust and presentation to the interface.

This contract covers the session, isolated dummy credentials and completed local strategy workflow. The [human guide](../docs/strategies/authoring.md) describes file selection, preview, save, inspection and edit-copy. Strategy rules do not run the controlled session.

## Interface

| Boundary | Contract |
| --- | --- |
| [contract.ts](./contract.ts): `Action`, `Record`, `Snapshot` | Actions are `start`, `pause`, and `advance`. Records contain ID, stopped/running/paused status, test tick, reason, and recorded timestamp. Snapshots carry running/pending state, a nullable recording error, history, and actual SQLite writer settings. |
| [main.ts](./main.ts): `element`, `render`, `refresh`, `control`, `poll` | Private entry-point functions bind required HTML IDs, render snapshots, invoke native `snapshot`/`control`, and refresh the display. Loading this module registers handlers and begins polling. |
| [view.ts](./view.ts): `presentation(snapshot)` | Exported pure function returns status/recording messages and the three control-disabled flags. TypeScript checks the implementation and infers its result for callers. |
| [main.ts](./main.ts): credential operation handlers | Invoke native `credential` with `save`, `replace`, `remove`, or `check`; include `value` only for save/replace. Display native status or a filtered failure message. |
| [index.html](../index.html): session, credential, history, and details sections | Supplies required IDs, labelled sections, buttons, password input, status regions, error alert, and expandable writer details. The page identifies dummy verification and no exchange connection. |
| [style.css](./style.css): `.controls`, `button:disabled`, `:focus-visible`, `#error` | Wraps controls, distinguishes disabled buttons, outlines focused buttons/summary, and preserves multiline errors. [vite-env.d.ts](./vite-env.d.ts) references Vite client declarations. |
| `contract.ts`: strategy request/response, detail, error and requirements types | Mirrors the native tagged protocol; version IDs/cursors stay strings. |
| `main.ts`: strategy handlers, `strategyControls`, `changedText`, `loadStrategies`, `inspectStrategy` | Read a user-selected bounded file or editor text, request preview/save/list/detail, inspect saved versions and edit a copy. No frontend rule evaluator or workspace path request. |
| `view.ts`: `strategyFailure` | Finite presentation of strategy failures, including uncertain save guidance; untrusted failures are narrowed from `unknown`. |
| `index.html`, `style.css`: strategy controls/status/preview/detail | Labelled file/editor controls and literal, wrapped explanation/detail text, with focus treatment. |

## Invariants

- `presentation` and `render` expect a nonempty `snapshot.history`; they read its first record. A recording error takes precedence over running/pending status, blocks start/advance, and never reports the requested change as saved. Pending work also blocks start/advance, with Pause available while saving. These are display rules, not a substitute for native validation.
- `refreshSequence` accepts only the newest requested refresh result, including failures. A failed current read says state is unavailable, disables start/advance, and permits Pause; it does not assert that the session stopped.
- `poll` waits for `refresh` to finish before scheduling its next 200 ms delay. A control-triggered refresh can overlap a poll; sequence checking prevents older results from replacing newer ones.
- `control` clears the prior request error, disables start/advance, requests the action, and refreshes afterward. `render` gives recording errors priority over request errors.
- Credential handlers clear the password input immediately, disable all four credential buttons until completion, and suppress unexpected framework errors. Only failures matching the explicit native-message prefix allowlist are displayed verbatim. This does not establish secret protection outside the interface.
- Dynamic session/history/status text uses `textContent`; history replacement uses created elements. Accessibility structure comes from the labelled HTML, status/alert roles, password label, and stylesheet focus treatment; this is source evidence, not an accessibility audit.

- Text/file/edit-copy changes invalidate the reviewed preview. Generation checks discard stale file, preview, list and detail replies. Save captures exact reviewed bytes, blocks duplicate pending saves and reports the confirmed identity separately from list refresh. Busy preserves a still-current preview; uncertain saves advise inspection before retrying and never retry automatically.
- Strategy JSON, names and explanations render as literal text. The native engine validates every save; supported status does not mean live-qualified.

## Side Effects

- `main.ts` mutates page text, history nodes, input values, and button state; registers click handlers; and schedules recurring browser timers.
- Native `invoke` calls cross the desktop boundary. This interface requests session changes, dummy-credential operations and local strategy operations; actual persistence and protected storage belong to native code. `presentation` performs no I/O.

## Verification

From the repository root, `npm run check` performs TypeScript checking and runs `tests/*.test.mjs`. `npm run build` checks types and builds the Vite interface; it does not establish native packaging or Keychain behavior. See [test boundaries](../tests/README.md). No tests, build, browser, native app, or credential operations were run for this documentation change.

## Sources interface (step 5)

The explicit `sources-panel` details section activates one local source read per
second while open and visible. Snapshot, details and saved-requirement inspection
share one read slot; only the newest desired display request is retained. Action,
selection and visibility generations suppress obsolete successes and failures.
Owner/revision comparisons use exact string/BigInt semantics; the foundation
200-ms poll is unchanged. No screen read connects or polls a provider.

Four fixed rows expose named source actions and native-derived values/units/
identity/receipt age, current/next preparation and unsupported history/reference
facts. Expired verified values show “Stale data”; closed-market values show “Market closed” with the last value retained. These are distinct from awaiting data and unverified freshness. Explicit refusal differs from silence/awaiting data. Kalshi/BRTI share
credential identity; no spot substitution exists. `sourcePresentation` and
`sourceFailure` are pure finite presentation helpers. Dynamic text is literal.
Failure reasons render even when no price has arrived; an explicit invalid-data
reason is not presented as ordinary waiting. Provider PEM input is obscured and
clears synchronously before submission,
including Busy/error paths. It is never repopulated or persisted by the frontend.
Actual OS protection/native IPC and packaged GUI acceptance require separate
proof; VM checks and headless rendering do not supply it.

The owner-requested cleanup/brand pass makes Sources the initial active section, with four flat feed rows and progressive credential/requirement details. It uses the unchanged approved 240px-visible SVG lockup, cobalt/ink/chalk palette and locally bundled licensed Inter at weights400/500/600. Runtime asset provenance is in [assets/fonts](../assets/fonts/README.md). Existing session/strategy/dummy IDs and commands remain available below; only the source screen's visual hierarchy and wording changed.


Native source snapshots identify `demo` or `production`; the header and credential help show that environment. Ordinary setup now uses production market data, while the isolated walkthrough remains Demo. Credential requests contain no environment selector: native transport ownership chooses the matching fixed Keychain item. No source connects automatically and trading remains unavailable. The Sources VM regression covers disclosure and the selector-free request.

## Controlled paper interface

`contract.ts:PaperRequest/PaperResponse/PaperRun/PaperReceipt/PaperWitness` mirrors the finite native command. `main.ts:paperAction/readPaper/renderPaper/showPaperReceipt` owns the controls and visible-only serialized 200 ms reads. Action/visibility/selection generations and completion epochs discard stale successes and failures; unconfirmed Go preserves its exact UUID/settings for explicit retry. Fixed active settings and pending-holding control availability come from the committed run, independent of the selected history page. Objective and side-ambiguity blockers remain the main reason even when supporting rule witnesses are true. Current read failure retains history and Pause. A history-only failure after an accepted snapshot keeps that snapshot's controls, including Check settlement, retains the last page and retries history on the next summary. Keyed journal buttons preserve keyboard focus; opened details refresh on settlement, and history/evidence paging replaces bounded pages.

`view.ts:paperMoney/paperQuantity/paperReceipt/paperWitness` formats exact decimal strings with BigInt and displays engine-supplied facts. It does not calculate fees, holdings, net or strategy truth. Unconfirmed outcomes do not assert zero cost/fill; pending settlement is not loss. `paperChecking/paperRecovery` render the engine's `recovery` record: while it is unconfirmed, status reads exactly “Trading paused — checking a purchase” and Resume stays disabled; afterward `paper-recovery` and the receipt's Purchase check fact report the actual result. An unresolved stopped run enables only the explicit check/settle action. Named retained checks include observed/required values, unit/side/time and compound-root context. All dynamic content uses literal text. JSON details include historical fixed settings and retained evidence with explicit sparse-record limits.

`index.html`/`style.css` extend the shipped Metten lockup, Inter, cobalt and chalk palette with fixed settings, distinct controls, receipt/journal and details. Synthetic input/20× clock labeling is always visible. Labels, status/alert regions, visible focus, 44 px controls and narrow wrapping are source contracts; rendered Chromium evidence and its WKWebView/assistive-technology limitations are recorded in [issue 6](../docs/verification/issue-6.md). Existing Sources/strategy/dummy flows remain separate. No charts, statements or live actions are provided.
