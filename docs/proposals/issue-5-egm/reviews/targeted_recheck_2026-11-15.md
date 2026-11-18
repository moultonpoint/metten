# Targeted independent design recheck — 2026-11-15

**Outcome: A1–A4 PASS.** One fresh-context, read-only reviewer found no remaining design gap or directly introduced regression within the four corrected contracts. This closes those findings for the design handoff; it does not establish implemented behavior or authorize implementation.

Target: [market-data design](../market-data-design.md), all 599 lines. SHA-256 `21e2926329f67fef05b264dcb8a0dfca0455aaa697c753bfc4f2d95b19fe4bc7`. Measured identity matched the expected corrected version. Baseline findings: [consolidated review](consolidated_review_report.md). Applied corrections: [resolution record](resolution_report.md). Those historical records and the target remain unchanged by this recheck.

## Independent reviewer findings

### A1 — PASS: shared-subscription recovery and successor readiness

Lines 113, 298, 308 and 335 consistently make the shared order-book subscription the continuity boundary. An unexplained gap revokes both retained books and successor readiness before further publication. Recovery requests replacement snapshots for every retained subscribed ticker through one coalesced, budgeted operation. Each book returns independently only after verified replacement in the current recovery generation and intact subsequent sequence; another gap revokes both again. Old work cannot restore validity.

The required case at line 579, `A:41 → missing B:42 → A:43`, directly covers the original failure: the market exposing the gap cannot leave the other market falsely ready. Cached official book evidence confirms real-time sequence scope and that requested snapshots replace the relevant local book. Line 298 identifies all-retained-market invalidation as a conservative inference and separates it from the conflation-specific recovery instruction. No additional socket or recovery subsystem is introduced. **Remedy: none.**

### A2 — PASS: deterministic saved credential across replacement, restart and failure

Lines 362–368 replace key-ID-based selection with one fixed generic-password item per environment: service `com.metten.provider-credential`, account `kalshi:demo` or `kalshi:production`. Key ID and private key are one protected versioned record. Status, Remove and explicit post-restart Connect target that same item; inaccessible, missing or invalid records prevent connection rather than selecting an alternative.

Line 364 supplies the missing mutation outcomes: Save refuses an existing item, Replace requires one and atomically updates its entire record, failed replacement preserves the previous record, failed removal cannot report success, and ambiguous native outcomes require readback before success or Connect. Replacement/removal revoke the shared Kalshi/BRTI generation first and leave connections stopped on failure. Lines 486, 490 and 522 connect this policy to command validation, bounded material and its implementation owner. Line 579 requires changed-ID replacement followed by reopening and selecting B, plus failed mutation and refused/missing lookup cases.

Actual native code supports the proposed extension: `credential.rs:326–394` uses fixed generic-password selectors and `SecItemAdd`, `SecItemUpdate` and `SecItemDelete`, with explicit disposable-store targeting and post-write identity validation. The proposed provider module is separate from the preserved dummy identity. **Remedy: none.**

### A3 — PASS: bounded shutdown with confirmed completion and no automatic restart

Lines 356 and 370–372 specify one five-second monotonic deadline covering the entire provider/source shutdown phase from initial revocation. They define cancellation of pending work, close requests, bounded worker exit, transport dropping and asynchronous cancellation at expiration. Requesting cancellation does not count as completion: unfinished handles remain owned, and engine save/exit cannot proceed until actual completion is observed.

The failure text describes stopped intake without falsely claiming closed connections. A later explicit Quit checks the same handles with a new bounded wait. Resetting the general Quit-attempt guard cannot clear the independent source-stop latch. Explicit Connect can clear that latch only after all old source work has finished and no Quit attempt is active; otherwise it returns Still stopping. Completion notifications never restart intake.

This addresses the integration risk in `main.rs:176–212`, where existing failed-save handling clears `quitting`. It preserves the native credential-operation wait at lines 185–190 and the documented Keychain-prompt exception. The nonresponsive-peer and actual-completion cases appear in target lines 521 and 579. **Remedy: none.**

### A4 — PASS: private fixture protocol, dispatch and isolation

Lines 523–524 and 570–577 identify one receiving owner in `credential.rs`, validated attachment through `main.rs`, and mode selection only after existing bootstrap/store/peer checks. Descriptor ownership is exclusive. Missing mode preserves dummy behavior; source verification requires validated `sources` mode.

The source envelope, allowed providers, string-preserving payload delivery, frame/payload caps, canonical string sequence, session matching, deadlines, single outstanding message and acknowledgment meaning are specified. Frames reach the controlled connection's actual decoder and bounded admission path under its current generation. Acknowledgment follows publication of accepted or explicitly invalid state; it does not imply healthy data. No connected controlled source means rejection.

EOF, malformed input, dispatch timeout and failed acknowledgment revoke fixture evidence and permanently fail the channel. Production host construction and ordinary Keychain lookup remain disabled for the entire source-mode process, including failure paths. Stop/Quit owns receiver cancellation and joining. Default dummy framing, numeric sequence, pending-value limit and acknowledgment meaning remain unchanged.

These decisions address the existing limitations: `credential.rs:38–54`, `:86–149`, `:260–305` has a strict dummy bootstrap/observer and immediately spawns its receiver; `verify-credential.swift:51–73`, `:266–295` owns framing, acknowledgment and descriptor-3 bootstrap. Revised inventory and required protocol cases cover those changes and affected dummy evidence. **Remedy: none.**

## Execution and limits

One native reviewer was dispatched with `fork_turns=none`, without parent conversation history. It read the complete implementation-readiness persona and standard, entire target, complete consolidated and resolution reports, and code map. Supporting reads covered credential bootstrap/framing/observer/fixed-item operations and tests; native setup, command admission and Quit; runner sending, acknowledgment, bootstrap, replacement/reopen, retained-child and cleanup behavior; native contract and runner regression test. Cached official book-description evidence included the real-time/conflation distinction. No required read was incomplete. The actual final reviewer report was received before this record was written. Exclusion of host/project instructions was not independently tested.

This is the requested focused recheck, not a new full five-stage review, a repeated convergence loop or qualification of the entire system. A5–A8 wording corrections were not separately rerun through every original lens. The lead received and checked the four dispositions against the corrected contracts. No additional remedy was identified.

The target and previous review records were fingerprinted for this check. Final verification confirmed their contents stayed unchanged, as did the engine/native/frontend/test/tool source trees. The only maintained artifact added by this recheck is this report. Local report links and the target identity were checked.

Skipped: fresh provider research, implementation, builds, runtime tests, app/browser launches, accounts, network calls and secrets. Provider capability, rights, dependency compatibility and performance calibration/qualification remain explicit unproved gates. This pass supplies document-level closure only. Implementation remains prohibited until separately authorized by the owner.
