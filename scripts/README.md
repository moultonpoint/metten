# Verification tools

## Staged checkpoint checks

Use [check-staged.py](check-staged.py) to check a mixed-work checkpoint independently of unstaged files. Usage, failure handling and archival-format exceptions are in [the agent verification guide](../docs/agents/verification.md#verify-a-mixed-work-checkpoint). Its disposable-Git regressions run with `python3 tests/check-staged.test.py` and in CI. The helper runs only the supplied command; it does not start native or credential checks itself.

The current non-trading package checklist, prerequisites, artifact identity, controlled storage failure and cleanup procedure are in [issue-31.md](../docs/verification/issue-31.md). It reuses this runner and the existing native app; it does not replace later release acceptance.

## Overview

[verify-credential.swift](verify-credential.swift) owns the default dummy packaged-app walkthrough for issue #3 and the explicit native provider check for step 3 of issue #5. It is a verification tool, not part of the trading engine. Its completed evidence and limitations are in [issue-3.md](../docs/verification/issue-3.md); process-only regression checks are described in [tests](../tests/README.md).

## Interface

- Normal invocation accepts exactly one finished `.app` path. `run()` requires an existing Accessibility grant, obtains `CFBundleExecutable` from `Contents/Info.plist`, and launches that executable directly through `launch()`.
- `launch()` selects a fresh `METTEN_TEST_WORKSPACE`. With isolated setup enabled it passes descriptor 3 as `METTEN_VERIFY_FD`, sending owned Keychain identity/password/session over a private socket, followed by sequenced `expectation()` frames. Dummy values are generated in memory, not passed in command arguments or environment variables.
- `--self-test` runs `recoverySelfTest()` using child processes and sockets without opening Metten or calling Keychain/Accessibility APIs. `--output-child` is its internal large-output fixture.
- `press()`, `fill()`, `waitText()` and `clearCheck()` operate the owned app through Accessibility. `denyOwnedPrompt()` touches only one uniquely identified Metten/dummy SecurityAgent prompt's Deny control.

## Invariants

- `waitForChildExit()` must confirm the exact spawned child's exit before resources are released. `requestQuit()` requests guarded window-close once; `awaitRetainedChildExit()` retains ownership if exit is pending. No force-kill or success inference from a timeout.
- `send()` catches a closed socket without terminating through SIGPIPE. `startChildOutputCapture()` drains output concurrently; `collectChildOutput()` joins the reader before scanning or closing it.
- `browserStoragePaths()` validates the tested bundle identifier and selects its own WebKit/cache/HTTPStorage paths; renamed test bundles do not scan the foundation app’s directories. Its path and rejection assertions run in the process-only self-test.
- `inspectAccess()` checks the actual item location, creator identity and protected sensitive authorizations. `scan()` rejects either dummy marker in captured output or regular files. This is bounded marker evidence, not an exhaustive secret-leak audit.
- `metadata()` snapshots default/search-list paths and requires equality after creation, execution and cleanup. The runner does not change or restore global Keychain lists.
- Cleanup targets only recorded owned Keychains, directory and failed-write sidecars. Sidecar removal checks inode identity; unconfirmed child exit keeps resources owned, and failures remain failures after late cleanup.

## Side Effects

Normal execution launches Metten repeatedly, creates two disposable Keychains and SQLite workspaces, enters dummy values, inspects access metadata, locks/unlocks its own Keychain and temporarily sets `UF_IMMUTABLE` on its own file to provoke a failed replacement. It denies its own access prompt, scans app-specific WebKit/cache/HTTPStorages files if present, and deletes owned test resources only after child exit. A stalled native prompt may keep the runner waiting for actual app exit. It does not grant Accessibility permission or access real accounts.

## Verification

`python3 tests/credential-runner.test.py` compiles this runner and exercises the process-only self-test. The separate packaged invocation is documented in [issue-3.md](../docs/verification/issue-3.md). Do not rerun packaged credential verification for ordinary documentation-only changes. An explicitly requested checklist execution, such as issue #31, records fresh artifact/environment evidence and distinguishes it from earlier bounded results.

## Packaged provider credential check (step 3)

`swift scripts/verify-credential.swift --provider-credentials target/release/bundle/macos/Metten.app`
uses the same owned-process/private-descriptor/disposable-Keychain/cleanup machinery.
It needs no renderer/Accessibility interaction: the finished executable validates
a strict sources-mode bootstrap, performs finite controlled provider credential
operations and verifies a local signature, acknowledges completion, and exits.
The runner separately confirms exit, scans outputs/files for private-key PEM
markers and dummy values, and verifies unchanged global Keychain metadata.
Invalid bootstrap fails closed before ordinary startup. It never connects to a
provider or reads ordinary credential items. This is not the step-5 source-frame
walkthrough. Default dummy UI mode remains separate, including its prompt checks.

## Isolated sources walkthrough (step 5)

`swift scripts/verify-credential.swift --sources <finished.app>` selects a strict
sources bootstrap and an owned disposable workspace/Keychain, opens Sources and
uses ordinary provider credential/source commands through Accessibility. Controlled
RSA keys are generated in memory by the installed OpenSSL tool; they are never
arguments, files or output. The walkthrough checks save/duplicate/replace/reopen/
remove, four logical connections over the controlled native connections and
synthetic provider frames. Payload strings retain lexical numbers. Every frame
waits for its native publication acknowledgment before the next is sent.

The native source mode is permanently isolated, including after channel failure;
it cannot read ordinary credential items or construct production requests. No
account/provider/paid access is authorized by this tool. The early
`--provider-credentials` native signature/fixed-item check remains a distinct mode.
Default dummy behavior remains unchanged; the three walkthrough modes are not
interchangeable acceptance evidence.

Source framing uses four-byte big-endian length, 256-KiB envelope/128-KiB payload,
exact session and positive canonical decimal-string u64 sequence. Native reads
and processing/ack phases each have a two-second total deadline; the runner's
ack wait is at most three seconds. Ack1 follows accepted or explicitly invalid
native publication, not GUI painting. Failed/late ack, EOF or Quit fails and
revokes fixture evidence. The runner preserves exact-child exit observation,
marker scans, unchanged global Keychain metadata and owned-only cleanup. If
WebKit controls are inaccessible, record that concrete GUI blocker; never claim
native-only or process-only checks establish the walkthrough. Do not rerun an
unchanged known-blocked dummy GUI route merely to repeat it.

The Sources adapter expands a uniquely titled owned disclosure using its supported
focus relationship and an exact-child Space event, then observes AXExpanded.
WebKit on the tested host advertises no Press action for these disclosures and
non-settable AXValue/AXExpanded; calls can return success without a state change.
Inputs use focused exact-child Unicode key events, never clipboard or coordinates,
and require exact in-memory readback without printing the material. Source rows
are selected by their own DOM identifier and checked for distinct values, units
and identities. A fresh BRTI tick after scoped Kalshi disconnect checks retained
shared ownership. Save/duplicate/replace require an empty actual secret field
before waiting for credential completion; this is prompt packaged observation,
while the frontend test separately establishes synchronous clearing semantics.
Process-only assertion checks do not establish real WebKit access.

A directly spawned app is activated once before disclosure discovery, with a
bounded 20-second readiness wait; its window may otherwise remain absent. The
tested WebKit exposes visual source-row line breaks as spaces in AXStaticText.
Row assertions therefore normalize whitespace and match complete token sequences,
retaining exact price/unit/identity boundaries rather than relying on visual
lines or unrestricted substrings. The process regression includes the captured
controlled-row shape and rejects price, precision, unit and identity collisions.

## Headless controlled paper walkthrough

`python3 scripts/verify-paper.py <built-Metten.app>` resolves CFBundleExecutable from bundle metadata and requires the `metten-paper-check-v1` marker before launching. It creates only its own 0700 workspace, invokes the exact executable with `--paper-check`, bounds output to 64 KiB and validates protocol plus schema/cash/fees/settlement/finite-zero SQLite receipts after confirmed process exit. A 30-second deadline sends private STOP, then waits five seconds; an unconfirmed exit preserves the owned workspace and process identity/handle. It never kills an application or deletes a live owner's resources. Successful/confirmed-failed children are reaped, streams closed and only the original owned directory removed. No credentials, GUI automation or network is involved. Process tests: `python3 tests/paper-runner.test.py`.
