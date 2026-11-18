What this proposes

The document proposes a read-only connection between Metten and four separately identified sources: Kalshi contract data, BRTI delivered through Kalshi, Coinbase last trades and Kraken last trades. Rust will own connections, protected credentials, validation, request limits, bounded processing and recovery, while the screen explains which values are usable and which historical calculations lack sufficient evidence. It will prepare the next BTC 15-minute market before opening and perform one identity-safe handoff when the required evidence is ready. Provider guarantees, access rights and processing capacity remain evidence gates; the design does not authorize implementation, account access or trading.

How the existing system works, per this document

Metten currently validates and explains supported strategies using controlled inputs rather than production provider connections. Its engine already represents observations, validity, coverage, gaps and bounded evaluation, and the proposal reuses those interfaces without changing strategy meanings. Existing credential verification uses an isolated dummy item rather than a production Kalshi key. The foundation session and its commands remain separate from the proposed source screen; production strategy evaluation from connected feeds belongs to later integration.

**C1 — Nit — Conflicting description of diagnostic workload**

**Affected scope:** “Benchmark contract,” line 206; “Initial byte profile and evidence gates,” line 494.

The packaged-app benchmark calls for “diagnostic rotation active,” which reads as a required background operation. The detailed plan instead explicitly chooses bounded in-memory diagnostics and excludes rotating log files. A reader can understand the chosen architecture, but cannot tell which diagnostic activity the benchmark must actually exercise.

**Smallest clarification:** Replace “diagnostic rotation” in the benchmark row with the intended bounded in-memory diagnostic update/publication activity. If a different rotation mechanism is intended, name it and distinguish it from file rotation.

**C2 — Nit — Earlier instructions still describe completed sections as future work**

**Affected scope:** “Checks performed for this revision,” line 249; “Modules and interfaces,” line 265; “User journeys and acceptance of this section,” line 375; completed Detailed Implementation Plan, lines 457–584.

Several passages say that remaining formal sections must settle interfaces and bounds, that a later section “will enumerate” files, or that Detailed Implementation “must turn” locations into a complete list. Those sections and the complete inventory now exist. The document clearly identifies genuinely unresolved evidence elsewhere, but these older instructions make it harder for a new reader to distinguish unfinished drafting from intentionally deferred calibration and provider proof.

**Smallest clarification:** Change these passages to present-tense references to the completed Detailed Implementation Plan, retaining only its explicitly outstanding feasibility and calibration gates.

No Blocker or Major comprehension findings. The proposal’s main flow, separation from connected trading and distinction between current values and usable history are understandable. General software terms were not treated as missing definitions; project interfaces and strategy authorities have links, and their relevant roles are summarized in the document.

Actual reads: the complete comprehension-review persona and the complete target document, lines 1–586, including Feature scope 1–456 and Detailed Implementation scope 457–586. No other documents, linked sources, code, reports or external material were read; no files were changed and no checks or account actions were run.

Skipped: verification of linked authorities and implementation behavior, as required by this document-only review. Risk: this report establishes comprehensibility, not provider feasibility, correctness or acceptance.
