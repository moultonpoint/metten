# Native desktop bridge and dummy credentials

## Overview

[main.rs](./main.rs) owns Tauri startup, the native-selected workspace, screen
commands, and guarded exit. [credential.rs](./credential.rs) is the Mac-only,
dummy Security adapter and validated disposable-bootstrap seam. The shared engine remains the source of session and
recording truth; this bridge is not a trading engine or provider integration.
See the [desktop package contract](../README.md) for build, permissions, icons,
configuration, and evidence limits. Completed issue #4 adds local strategy
operations; [its evidence index](../../docs/verification/issue-4.md#current-evidence-index)
records their bounded checks. Session Start/Pause remains non-trading.

## Interface

| Symbol | In-scope contract |
| --- | --- |
| `main.rs:Workspace` | Holds `Result<Engine, String>`, Mac `Credentials`, shared `StrategyAdmission`, `quitting`, `exit_ready`, and a mutex-protected `quit_error`. A workspace-open failure is retained for command errors rather than inventing engine state. |
| `main.rs:snapshot` | Returns engine `Snapshot` with effective `running`, `pending`, `recording_error`, committed `history`, and `writer_settings`; overlays a saved quit error when present. Propagates workspace-open errors. |
| `main.rs:control` | Accepts the engine `Action` values `start`, `pause`, or `advance`; refuses while quitting and otherwise calls `Engine::request`. Acceptance means queued, not saved; callers must observe the returned snapshot. |
| `main.rs:decode_credential` | Requires a JSON object with only `operation` and optional `value`. `save`/`replace` require a string value; `remove`/`check` forbid the value key. Other shapes, keys, operations, and raw bodies return `Invalid credential request.` |
| `main.rs:credential` | Uses raw request decoding and a blocking worker. `check` consumes a runner expectation, then all operations acquire `Credentials::begin_operation` before `execute`. Returns fixed status/error strings, never the saved bytes. Non-Mac builds refuse with the isolated-Mac-runner requirement. |
| `main.rs:decode_strategies`, `strategies` | Strict bounded tagged preview/save/list/detail requests; one admitted blocking task invokes the engine, with sanitized failures and no credential operation. |
| `main.rs:StrategyAdmission`, `StrategyGuard`, `admit_strategy`, `wait_for_strategy` | Nonblocking Busy/quit admission; owned guard releases and notifies the quit waiter, including error/unwind or canceled-before-execution paths. |
| `main.rs:begin_quit` | Marks quitting once, waits for admitted strategy work and then `Credentials::operation`, calls `Engine::stop_and_save`, then sets `exit_ready` and exits on success. Failure retains its error, clears quitting for retry, and shows/focuses `main`. If no engine opened, there is no engine state to save. |
| `main.rs:main`, `context` | Build the Tauri context, replace the Mac predefined Quit item with guarded `quit-session`, open native-selected `session.sqlite`, register the in-scope commands, and intercept close/exit requests until shutdown is ready. |
| `credential.rs:Credentials::setup` | Accepts `METTEN_VERIFY_FD=3` and a valid inherited Unix socket bootstrap; otherwise stores no usable target. Opens/unlocks the validated disposable Keychain, clears its retained password string, and starts the expectation observer. |
| `credential.rs:Credentials::begin_operation`, `operation` | Serialize credential operations and shutdown. A command checks the quitting flag after obtaining the lock and returns `Quitting — credential action refused.` when set. |
| `credential.rs:Credentials::expectation` | Takes exactly one pending expected dummy value. A failed channel or missing expectation returns the fixed verification-unavailable error without waiting for input. |
| `credential.rs:Credentials::execute` | `save` uses `SecItemAdd`; `replace` uses `SecItemUpdate`; `remove` uses `SecItemDelete`; `check` uses `SecItemCopyMatching` and compares the actual native-read bytes with the private expectation. |

Credential success categories are **Saved**, **Replaced**, **Removed**, and
**Readable**. `credential.rs:map_status` maps duplicate, missing, canceled,
authentication-failed, and denied OS statuses to fixed text; other failures use
**Unavailable**. `unlocked` supplies **Locked**; `execute` also reports invalid
dummy input, unsupported action, or **Mismatch**. Saved and readable are separate
states, and every success explicitly says no exchange is connected. A blocking
worker failure in `main.rs:credential` has a fixed reopen-app error.

## Invariants

- `main.rs:main` chooses the directory from native `METTEN_TEST_WORKSPACE` or
  `app.path().app_data_dir()`. `Engine::open` records a stopped boundary on open;
  neither a controlled session nor a reopened session establishes a financial run.
  History and writer settings are supplied by the engine, not reconstructed by
  the bridge.
- `main.rs:decode_credential` prevents framework decoding errors from quoting
  submitted values. `credential.rs:dummy` requires the `metten-dummy-` prefix,
  20–128 bytes, and only ASCII letters, digits, or hyphens. No generic secret-read
  operation, caller-selected service/account, or screen-supplied Keychain path
  exists in this boundary.
- `credential.rs:Bootstrap::validate` requires a canonical `dummy.keychain-db`
  inside an owner-owned mode-0700 `metten-credential-*` directory, with a regular,
  owner-matching, single-link file and the recorded device/inode. It rejects
  System/login paths and aliases. `Bootstrap::open` checks actual Keychain path
  and revalidates identity. `record_native_write` updates the accepted inode only
  immediately after this adapter's successful write, then validates again.
- `credential.rs:Credentials::execute` fixes generic-password class, service
  `com.metten.dummy-connection`, account `dummy`,
  `kSecAttrSynchronizable=false`, and `kSecUseDataProtectionKeychain=false`.
  Add targets `kSecUseKeychain`; other operations restrict `kSecMatchSearchList`
  to that one reference. There is no default/login fallback or search-list change.
  Replace is neither delete-first nor upsert; missing and duplicate items remain
  distinct failures. `unlocked` refuses a locked target without automatic unlock
  during an operation.
- `credential.rs:Bootstrap` and `Expectation` deny unknown JSON fields. Setup
  bounds password length to 32–128 bytes and session length to 32 bytes.
  `frame`/`receive` require a 1–4096-byte frame within one two-second deadline.
  `observe` permits one pending expectation, requires matching session, next
  sequence, `local-use` phase, and bounded dummy data, and acknowledges receipt.
  Protocol failure clears pending data and makes the channel unusable;
  `expectation` consumes a valid value once.
- Strategy work holds no credential lock. Quit waits on strategy completion before taking the credential lock, and engine save rechecks quitting under its recorder lock before mutation.
- `main.rs:begin_quit` waits for an ongoing native credential operation. Queued
  operations refuse after acquiring the lock when quitting is set. There is no
  arbitrary Security-call cancellation: an actual OS prompt must finish before
  the saved-stop/exit path can complete. A quit-save failure remains visible
  through `snapshot` and permits a later Quit retry.

## Side Effects

- `main.rs:main` creates the selected directory and calls `Engine::open`;
  `control` queues engine actions and `begin_quit` requests a durable stopped
  record. SQLite, locking, and committed history belong to the engine.
- `credential.rs:Credentials::setup` consumes the inherited socket, opens and
  explicitly unlocks the runner's disposable Keychain, and spawns `observe`.
  `observe` reads framed private expectations and writes one-byte acknowledgments.
  No screen command creates a Keychain or changes global Keychain configuration.
- `credential.rs:Credentials::execute` can mutate the fixed dummy item or read it
  for native comparison; native APIs may prompt. Reads return only fixed results
  to the screen. Temporary secret bytes and bootstrap data exist in process
  memory; this is not a claim of memory zeroization or compromise protection.
- `main.rs:begin_quit` mutates shutdown flags/error state, may focus the window,
  and exits after successful saving. `CloseRequested` and `ExitRequested` are
  prevented while the guard owns shutdown.

## Verification

From the repository root, `cargo test -p metten --locked` exercises native unit
and command-authority checks. `main.rs:permissions_tests` covers fixed decoding
errors, strategy admission/release/quit waiting and local-main-only authority. `credential.rs:tests` covers queued refusal
and quit waiting, file identity/alias rejection, bounded dummy input and status
mapping, bounded/closed frames, and one-use/duplicate expectations. These
controlled checks do not themselves prove real prompt refusal or packaged UI
behavior.

The latest finished section of [issue #3's verification record](../../docs/verification/issue-3.md)
records the separate dummy-only packaged result, including actual access denial
and pending-prompt Quit. It distinguishes those observations from mapped error
tests and from the controlled queued-command test. Reuse its recorded evidence
only for unchanged behavior; no test, build, launch, or credential operation was
performed for these README additions. See [issue #2](../../docs/verification/issue-2.md)
for the retained non-trading session and failed-save/retry evidence.

## Step-3 provider foundation

[provider_credential.rs](provider_credential.rs) owns strict version-1 fixed-item
Kalshi records, supported key validation, nonsecret status and fixed-item readback.
The ordinary store uses the data-protection Keychain with synchronizing disabled
and WhenUnlockedThisDeviceOnly protection. The separate dummy adapter is unchanged.
Step5 now registers the finite provider credential command described below. Native callers must serialize
credential/connection admission and move exclusive ownership to the native blocking
worker. `Credentials::change` validates before revocation, stops/joins the shared
Kalshi/BRTI task through the existing Tauri runtime, and only then mutates the fixed
item. Other provider fences stay current. A timeout retains the task and refuses
mutation. Do not invoke this blocking interface under a shared lock or on an async
runtime thread. Step-5 command admission is described below.

[providers/mod.rs](providers/mod.rs) owns three fixed provider task slots, one
optional worker, a eight-slot raw intake queue, independent generation fences
and a stop signal outside the queue. Consumers must recheck the relevant fence
before publication. Step 4 adds the read-only source owner described below; no production financial run is connected.
The worker consumer must perform bounded work; a nonresponsive worker retains
its handle and blocks successful shutdown. A fresh owner cannot revive an old fence.

Unsubscribe tracking holds at most four commands with up to four subscription IDs
per command. ID-less completion releases the matching subscription; the slot is
reusable after all its subscriptions complete. The last four completed command IDs
remain recognizable for racing code-7 replies; unknown command IDs still fail.

[providers/transport.rs](providers/transport.rs) owns a pooled fixed-environment
HTTPS client and fixed TLS WebSocket destinations. GET routes and control messages
are finite Rust enums; account reads, purchase routes and arbitrary URLs/methods
are absent. Redirects, proxies and library HTTP retries are disabled. The local
loopback connector exists only under cfg(test). Interrupted send/receive drops
the socket and revokes its fence; it cannot resume partial protocol work.
[signing.rs](providers/signing.rs) parses Ed25519 PKCS#8 and 2048-bit RSA
PKCS#1/PKCS#8, accepting surrounding pasted whitespace within the original16KiB input limit, with canonical GET/path/timestamp signatures. No secret types
implement serialization or Debug for transport/status output.

[budget.rs](providers/budget.rs) limits each provider's local traffic: REST
capacity 50 tokens, refill 1/100 ms, cost 10; at most two metadata operations in flight;
one handshake at a time per provider; control capacity 1, refill 1/s. Necessary
pongs use their own counter and consume no subscription-command capacity.
Data-message receipt has a separate bounded counter. These conservative application limits do not
prove an account tier or prevent external-consumer throttling. Reservations are refunded if canceled before first transport polling. Counters
include failed transport attempts and retries, but never claim server receipt. HTTP, handshake and established-socket
transient read/control-write failures use one persistent provider retry owner: 5-second exponential
start, capped at 60 seconds with bounded positive jitter; five failed attempts
enter a queryable slow-retry state with at most one attempt/minute. Thirty seconds
of verified usable data resets it; a handshake never does. Socket loss resets
the usable-data interval before later recovery. Intentional cancellation resets
that interval without incrementing transient failures; malformed protocol remains
terminal. Each failed socket operation is recorded once, then closed/revoked. Denial/protocol/capacity
failures are terminal for that operation. Metadata and handshakes have a 10-second
total cap including local waits; a later explicit attempt retains pending backoff. The sole bounded retry owner services each admitted operation. The explicitly
started step-4 provider task owns reconnect and metadata discovery/page coalescing;
there is no detached reconnect task.

[memory.rs](providers/memory.rs) reserves adapter payload capacity before allocation
and retains permits with returned bytes across queue/worker ownership. Its shared
32 MiB ceiling refuses capacity rather than waiting for other buffers to be freed.
Each socket reserves 4 MiB for bounded protocol framing/write/read storage; each
returned/queued body reserves its payload allowance. Step-4 book/identity growth
uses this same owner before allocation. TLS/kernel
state is outside this application-payload accounting. Intake older than two seconds
is revoked without consumption; that buffer-age ceiling grants no price validity.
Worker messages retain their own fence, so explicit provider recovery cannot leave
the worker consulting an obsolete fence.

Messages, frames and REST bodies are capped at 1 MiB. The three task slots,
eight-slot queue, one worker and finite socket buffers bound transport ownership;
this is not the step-6 simultaneous-allocation or throughput qualification.
Transport calls require an absolute deadline that includes permit wait and I/O.
Those deadlines confer no price-validity/history capability. Native Quit takes
source ownership out of the mutex, revokes before waiting, and uses one total
5-second deadline without a lock held across joins. Unfinished handles are
restored to Workspace, cancellation is requested, and engine save is refused.
A later Quit checks those same handles. Explicit recovery requires observed
completion and an inactive general Quit; nothing connects on startup or completion.

The [step-3 evidence](../../docs/verification/issue-5.md#step-3-implementation-baseline)
records tests and outstanding gates. `cargo test -p metten --locked` runs controlled
signature/credential/budget/local-peer/ownership tests; the ignored
`isolated_native_fixed_items` test separately creates/deletes an owned disposable
file Keychain. It does not establish the packaged data-protection-store path,
OS-prompt behavior, provider support, real credentials or external acceptance.

The packaged check `swift scripts/verify-credential.swift --provider-credentials
target/release/bundle/macos/Metten.app` selects an early native-only check through
`METTEN_PROVIDER_CHECK=1` plus descriptor 3 and a strict `mode: sources` bootstrap.
The same Bootstrap validates canonical path/device/inode/ownership, opens/unlocks
the disposable Keychain and clears its password before use. Failure exits; it
never falls back to normal startup, normal Keychain or fixed provider hosts. One
private receiver runs the finite check and acknowledges only after all credential
operations/signature checks complete; process exit is separately observed by the
runner. This is a step-3 check, not the future step-5 provider-frame protocol or
Sources command. Normal dummy framing and absent-mode behavior are preserved.

After a provider-stop timeout, Connect must re-run `stop_provider` (or the full
shutdown check) to observe retained-handle completion before `recover_provider`.
Recovery never drops a handle just because `is_finished` became true. Any provider
task exit revokes its fence; unexpected worker exit revokes the shared intake gate
and requires joined global recovery. Public borrowed socket operations use drop
guards: an outer timeout/select that drops them also closes/revokes, including
cancellation while waiting for a permit. Obsolete/stopped intake has saturating
reason counters rather than an unbounded event log.


## Step-4 read-only source owner

[providers/source.rs](providers/source.rs) owns `SourceWorker`, the sole native
writer of `SourceState`, and its derived `SourceView`. `prepare` borrows a validated
strategy, retains requirements and at most two normalized market descriptors, and
uses ordinary unsupported capabilities. `Coordinator::start_sources` uses the
existing single worker; `start_provider` explicitly starts one of the three fixed
provider slots. Normal startup, screen refresh and financial Start/Advance do not
call these APIs. No command, permission, SQLite revision or credential adapter is
added. Trusted native setup supplies already-owned signing material; it performs
no ordinary credential-item access.

[Kalshi](providers/kalshi.rs) owns two bounded reconstructed books and opaque trade
identity ledgers; [Coinbase](providers/coinbase.rs) and [Kraken](providers/kraken.rs)
own their fixed BTC/USD trade formats/high-water facts. [wire.rs](providers/wire.rs)
checks one-MiB input, depth48, 16-KiB lexical strings and bounded typed lists before
retention, preserving numeric tokens with Serde raw values. Dollar contract prices
map exactly to six-place cents (up to eight dollar fractional places); USD/BTC
prices must fit the engine's existing six-place contract. No floating-point
conversion or rounding is used. Incompatible precision is refused. Quantity,
maker/taker direction, book clock/sequence and BRTI averages remain bounded labeled
details, never additional strategy values or opening/settlement references.

The service concurrently polls the installed WebSocket read/write halves inside
one provider future. A normal control or metadata wake never drops a partial
receive. After a handshake, the task rechecks source identities and the latest
retained markets before sending its first subscriptions; retired markets from
connection setup cannot become extra subscriptions. Session permission requires the parent provider permission and worker
permission; dropping even an unpolled owned service closes/revokes its connection.
Task return/panic/cancellation returns its control receiver through a drop guard;
explicit provider recovery first joins the retained task, then uses fresh parent,
source and session identities. One total five-second shutdown and the existing
engine-save guard remain authoritative. No worker or recovery task detaches.

Raw receipts retain charged bytes, local receipt time and the tokens captured
before their receive. The worker validates sequence before filtering irrelevant
Kalshi tickers. A book subscription gap revokes both books and native view epochs,
while BRTI/spot streams retain their independent identities. One replaceable watch
record holds at most book/trade/index recovery work; the finite seven-command
mailbox retains initial and mandatory membership controls. Recovery is checked
again at actual permit start; superseded work spends no attempt. A frame returned
by an old pending receive is rejected. If that was the first response to a recovery,
one coalesced counted snapshot retry obtains state under the new receive identity;
it never relabels old bytes. A replacement restores each book separately. Empty
liquidity expires only the absent initialized binding through the existing lease/
reinitialization contract; no old ask remains usable and later deltas can recover.

Acknowledgment and first-data waits are independent: five and ten seconds from
actual control attempt, with both retained books required for replacement completion.
Matching real data can also prove channel presence. Index-list absence remains
awaiting data, distinct from explicit transport access refusal. An established
read I/O timeout records one transient retry; expiration while waiting only for a
control permit does not. Unsupported freshness never resets slow retry merely
because frames arrived. Explicit close and unpolled owned-service Drop reset the
usable interval without counting a transient failure. Ping wakeups coalesce while
the library owns the pending automatic pong, independently of subscription permits.
The keepalive counter counts explicit flush attempts, not every automatic wire pong.
Network deadlines grant neither price validity nor history.
The socket reader holds at most one charged received frame while waiting for an existing raw slot, bounded by the original receipt plus the unchanged two-second intake-age limit. Stop/revocation cancels that wait and the original captured tokens/receipt are retained; expiry closes/revokes rather than dropping a required frame. The queue remains eight slots and the shared32MiB allowance is unchanged. Direct nonwaiting admission retains its full-queue refusal behavior.


[discovery.rs](providers/discovery.rs) orchestrates only the fixed public metadata
routes; charged route/token/epoch receipts are parsed on the source worker.
Initial-attempt scheduling is separate from completed exchange/series validation:
recoverable status, budget or deadline failures retain those startup checks for
the next paced attempt. Only successful discovery establishes validation.
Startup uses at most six planned GETs, successor discovery four, and relevant
lifecycle revalidation only the retained exact details. Retries have separate
actual-attempt accounting. At most two pages/400 candidates and two confirmed
market descriptors are retained. Cursor encoding preserves bounded opaque values.
Schedules are exact UTC metadata mapped to signed monotonic offsets. The
60/30/10 checks perform missing work, skip missed opportunities, and pace missing
post-opening work to one opportunity/minute. A scheduling interruption or wall-
clock discrepancy beyond the existing two-second intake bound revokes schedule
assumptions and revalidates; that detection bound is no timeliness guarantee.
Prepared handoff performs no boundary-search request or socket reopening. Due
prepared boundaries are dispatched in opening-time order at their exact scheduled
clock before the worker advances to its sampled time, including a tick that crosses
opening. Known invalidity is applied first. Metadata/data accepted after opening
cannot be moved to an earlier clock to create an entry; late readiness remains
display-only. Controlled callers advance scheduled boundaries before ordinary
current-time source events, as the owned worker does. Existing status/series
validation can be reused across reconnect; metadata is reverified in the new
source generation. One nonqueued scheduling-loss notice blocks market views
immediately until the worker applies invalidation. Terminal notices cannot be
overwritten by a weaker notice. The worker owns the reanchored epoch used by later
connections; ordinary metadata-request duration alone is not a sleep event.

An unentered same-identity schedule correction uses engine `ReplaceMarket` and
then a fresh metadata request before rebuilding. Entered corrections stop the
source context. The controlled evaluator's registered catalog is immutable:
stop it on any registered descriptor replacement and require an explicit fresh
pair/epoch. No connected financial evaluator exists here. Controlled `process`
callers supply the actual completed frontier, drain one effect before another,
check the full dependency stamp and transport permissions immediately before use,
and follow the engine's rejection/Busy/Stop protocol. Worker/view invalidity has
priority even with a full ordinary queue. `snapshot()` and scoped `details()`
recheck retained permissions; neither performs I/O. Wire projection also rechecks the supplied current clock against each binding's validity deadline and market close. Expired verified data reports `stale_data`; a closed market reports `market_closed` and expired preparation. Both retain last values without granting usability; closed-market history readiness is false and it is no longer advertised as current. Missing data and unknown freshness remain distinct. Details for book/trades require
the exact market identity and retire with its slot. A retained snapshot is a point-
in-time fact; current permission alone grants no ongoing price freshness.

The common 32-MiB owner covers socket/raw reservations, decoder scratch, book and
identity growth, controls and retained derived views. Each book side permits8192
levels; old/new overlap is reserved before mutation. A required over-cap book or
identity ledger fails capacity instead of truncating/evicting. Each view retains a
256-KiB reservation; native fixed requirements/details/control storage retains a
64-KiB base charge shared by the worker, retained source handle and provider
futures until the final owner drops it. Parsed metadata retains its scratch
reservation through the command/reply handoff and caller cancellation.
SourceState/evaluator allocations keep their existing separate
proofs. [Step-4 evidence](../../docs/verification/issue-5.md#step-4-implementation-and-verification)
contains the simultaneous-ownership argument and executed check limits. These are
refusal bounds, not measured production capacity or allocator/RSS qualification.

Focused evidence includes the unchanged learning JSON through actual Kalshi books,
SourceState and Evaluator; two-book shared-gap/replacement on one socket; empty
books; three tasks/seven controls; six/four GETs; repeated bounded rollover;
identity/precision/capacity rejection; cancellation, explicit recovery and the
inherited shutdown tests. U1–U7 in the [source profiles](../../docs/sources/README.md)
remain blocked. No provider delivery, real entitlement/rights, complete history,
financial outcome or production performance is established.

## Step-5 source commands and isolated receiver

`main.rs:sources` admits exact `snapshot`, `details`, `inspect`, `connect` and
`disconnect` objects. `provider_credential` admits only Kalshi status/save/replace/
remove in the native-selected environment: production for ordinary source setup and Demo for the isolated walkthrough. The renderer cannot select an environment or credential item. Unknown/extra fields, raw bodies,
invalid decimal-string IDs and oversized material return finite sanitized errors.
The bounded counting writer enforces 256-KiB request/output ceilings without an
oversized serialized clone. `SourceResponse` retains its response reservation
through serialization. IDs, revisions, sequences and large counters stay strings.
Inspection reads the real saved immutable version and derives validated source
requirements; it never changes subscriptions or owns another rule representation.

Workspace has one admitted mutation and one admitted read. `ProviderOwner` moves
Coordinator and fixed credential ownership into blocking work and restores taken
ownership on errors/unwind. Published source handles remain readable during
native operations. A clone of the existing revocation signals lets Quit stop
intake immediately while an OS operation owns the coordinator. Quit waits only
within one five-second source deadline for mutation ownership, receiver and source
completion; unfinished work stays owned and no engine save/exit is claimed.
Native prompt completion remains noncancelable. Explicit recovery first joins old
handles. No completion, saved credential or window opening connects automatically.

`SourceWorker::prepare_display` uses the fixed engine display catalog. A fixed
logical desired set shares the existing worker and three task slots. Membership
changes revoke stream permissions before publication, preserve the shared
Kalshi/BRTI socket while either consumer remains, and retire bounded decoder SIDs
without resetting the other source. The Kalshi decoder retains at most four retired
subscription IDs until unsubscribe completion, accepts their control completion,
and discards their late prices before recovery logic even after that logical source
is enabled again. Book/trade control IDs clear on retirement. A same-channel
subscribe waits for completion and runs only while that source remains desired;
market-discovery updates also respect this wait. Completions release this bounded
identity state; reconnect clears it. Code9 command refusals map to access_refused
for the matched stream. Known-SID provider endings (codes10/25 and unsubscribed,
with no command ID required) invalidate only that stream and disable its wait
and recovery state, preserving the other logical source. Completion of an already
blocked stream preserves its original invalid/overflow/access reason. Malformed
lifecycle data blocks all affected market streams and cancels each stream's
transport wait/recovery, leaving BRTI's independent stream intact. Transport
progress retains at most four outstanding unsubscribe command IDs; a matching
code7 is cleanup, while an unknown command remains a protocol failure. A command
acknowledgment releases its tracking slot; capacity exhaustion refuses a new
write rather than losing command identity. A local two-second membership wait confirms
the worker observed the desired set before connection start; it grants no price
lease. `SourceView::wire_snapshot/wire_details` recheck permissions, preserve exact
units/identity/receipt age and distinguish received values from usable inputs.
Snapshots/details/inspection make zero provider requests. Failure diagnostics
retain at most one failed connection fence per provider, sharing its existing
cause after active-link cleanup. Views copy only the finite failure category;
current stream epochs, worker/schedule fences and logical membership still fence
that diagnosis independently of usable-data permission. Explicit disconnect
supersedes the diagnosis and a fresh connection clears the failed fence. This
preserves invalid-first and good-then-invalid explanations across later worker
publications without restoring strategy usability. Diagnostics remain bounded in
memory; no source preferences or feed history are stored durably.

`credential.rs:read_bootstrap` is the shared descriptor-3 bootstrap reader for
mutually exclusive dummy, early provider-check and source-walkthrough launches.
`METTEN_SOURCES_CHECK=1` alone grants nothing: strict sources-mode bootstrap and
owned disposable-store identity must validate or startup exits. The mode remains
isolated after channel failure. The ordinary store and production transport are
unavailable in this launch. `Transport::isolated` rejects real GETs/handshakes;
controlled connections feed the actual decoders and worker. Synthetic current/
next metadata is installed only after explicit Kalshi Connect in this mode.

`SourceVerifier` owns one receiver thread; `observe_sources` owns exact framing,
sequence and acknowledgment ordering. Envelopes are at most 256 KiB with UTF-8
payloads at most 128 KiB, exact session, canonical string-u64 sequence starting1,
kind provider_frame and one of three provider identities. Read deadline is two
seconds total; dispatch plus acknowledgment has another two-second total deadline.
The runner waits at most three seconds. Completion replies are attached only to
fixture intake and complete after accepted or explicitly invalid publication.
Ack1 proves that native publication, never healthy data or renderer painting.
Malformed framing/dispatch/ack/EOF permanently fails the channel, revokes all
fixture generations and blocks reconnect. Drop/panic also revokes through the
receiver lifetime guard. Quit cancels pending dispatch and observes receiver exit
under the same source deadline. No second descriptor reader or raw fixture IPC
command exists. Dummy framing/receipt-only ack semantics remain separate.

Native/public tests and `tests/sources.test.mjs` cover their real owners. The
[issue-5 record](../../docs/verification/issue-5.md) distinguishes final executed
checks, isolated packaged evidence, GUI/ordinary Keychain limits, and still-open
U1–U7. Controlled data does not establish real access, rights or production capacity.

Normal dummy verification (including invalid inherited setup) refuses the new provider/source mutation commands. Only a validated sources walkthrough enables its controlled source path; invalid source-mode startup exits. This prevents the newly added interface from reaching ordinary provider credentials or public feeds during an old dummy walkthrough.

Disconnect revokes the logical gates before mutation admission, so it can cancel a Connect blocked in native credential access even if cleanup reports Busy. Connect retains the request's intent revision and cannot revive a superseded request. The fixed desired watch record carries checked revocation revisions as well as final membership; the worker applies every new revocation even when an off/on pair coalesces to the same final Boolean. This prevents watch coalescing from restoring old source permission. The UI allows Disconnect during pending Connect and suppresses older action feedback.

<a id="offline-source-calibration-step-6"></a>

## Prototype verification and retired capacity tooling

Use the ordinary native provider tests and public engine source/evaluator tests for real product behavior: source identity/units, observations versus replay, snapshot/delta state, invalidation, scoped membership, read-only routes, cancellation, publication and joined shutdown. Controlled timestamps and the existing lease/receipt fixtures remain test inputs; they establish no upstream freshness or real-provider guarantee.

Capacity campaigns, collector diagnostics, histogram/reporting infrastructure, process probes, counting allocator and worker probe callbacks have been deleted by owner direction. No optional performance entry remains to run while completing issues 5 or 6. Issue 16 owns any later measurements based on the integrated application's actual requirements. Historical failures are preserved in [issue-5 evidence](../../docs/verification/issue-5.md#current-evidence-index); they are not prototype prerequisites or a qualified-capacity claim.


### Kalshi provenance in source details

Book deltas retain `msg.ts_ms` as provider event time; book snapshots do not invent an event time when it is absent. Book, trade and BRTI details retain `sending_ts_ms` separately as the provider's network-queue time. All three preserve `sid` and `sequence` in details, encoded as decimal strings. Trade/BRTI event times, BRTI `received_at`, local receipt age and connection generations remain separate facts. Coinbase/Kraken have no Kalshi subscription ID or network-queue timestamp; those detail fields are null.

`kalshi_provenance_survives_real_decode_and_details_projection` uses the real decoder, SourceState and wire-details projection, including a sequence above JavaScript's exact integer range. This preserves provenance without granting strategy freshness or history. The added optional scalars introduce no collection, task or raw-history retention; the existing bounded detail/view reservations and serialized-output limit still apply.


### Production source setup

Ordinary startup uses the existing production read-only transport and fixed `kalshi:production` Keychain item; isolated source verification remains Demo and cannot reach production. The transport determines the environment in source snapshots and credential operations. The native credential command accepts only provider/operation/material, rejecting renderer environment selectors. The Sources header and credential help display the native environment. Reopening remains disconnected and saving a key never connects. Production means real market data, not enabled trading: account/portfolio/order routes remain absent.

The Kalshi adapter consumes documented `event_lifecycle` and `event_fee_update` messages on the global lifecycle subscription, preserving the shared sid/sequence and bounded typed validation before ignoring creation/fee details unused by this read-only slice. Event creation does not change retained exact-market metadata; bounded discovery still prepares successors. Snapshots with an omitted/empty side replace that side with no liquidity, never zero price or a retained old ask. Actual production receipt and the focused regressions are recorded in [issue5 evidence](../../docs/verification/issue-5.md#authorized-production-market-data-intake).
