# Issue #30: CodeQL coverage and merge checks

## Current evidence index

Issue #30 remains open. The initial default scan's Mac credential coverage gap
is recorded below. The dedicated macOS workflow's hosted diagnostics now
include the native credential module without the original platform-exclusion
warning; see [verified hosted evidence](#verified-hosted-evidence--14-november-2026-utc).
This establishes the specific platform correction, not complete semantic coverage
or absence of vulnerabilities. Independent review found no workflow blocker.
Scanning remains nonblocking; the existing `mac-foundation` merge requirement
remains in force. The [enforcement proposal](#enforcement-proposal--15-november-2026-utc)
is ready for owner decision under child #40. This record is not security or release acceptance.

## Enforcement proposal — 15 November 2026 UTC

Status: prepared for owner decision; **not applied or tested**. The remaining
work is tracked by child [#40](https://github.com/moultonlavah/metten/issues/40).
Its acceptance explicitly requires exact owner authority before protection or
security-setting changes. The request to implement #30 starts preparation; no
specific settings payload has yet been approved. Both issues remain open.

Expected change path: GitHub owns merge eligibility; the existing CodeQL workflow
owns analysis and uploads. Add one repository ruleset, retain the existing branch
protection, verify the GitHub PR/check/analysis boundary, and update this record
and the contributor contract. No application code, new action, scanner framework,
compiler, dependency or source exclusion is needed for this proposal. The code
map's existing CI route stays valid.

### Published instruction baseline

This existing lead session was explicitly refreshed after the owner supplied the
published-instruction rule. A fresh `git ls-remote --symref origin HEAD` identified
`https://github.com/moultonlavah/metten.git`, default branch `main`, published commit
`a22a9a9188b683e07be141702e93295d83a4ade1`. Read root `AGENTS.md` and its applicable
maintainability, verification, code-map, issue-tracker, triage and domain procedures
from that immutable commit. The published tree has no nested AGENTS/CLAUDE file or
additional instruction-discovery procedure. Selected local instructions and task
acceptance were also read. No required guidance was unavailable or in conflict.
The published files match the selected checkout's guidance byte-for-byte; no code,
index or tracking ref was changed by discovery.

The completed reviews predate the new explicit refresh rule; their independent
immutable-entry discovery was not attested and is not claimed retroactively.
Their findings remain useful evidence against identical guidance. Any further
review/session must independently load the published root entry, pin this shared
baseline, and refresh it at handoff or delivery. The settings proposal remains
approval-pending; this refresh grants no additional authority.

### Refreshed baseline

Read-only inspection on 15 November found main at
`a22a9a9188b683e07be141702e93295d83a4ade1`:

- [Foundation 43743468399](https://github.com/moultonlavah/metten/actions/runs/43743468399)
  and [CodeQL 43743468350](https://github.com/moultonlavah/metten/actions/runs/43743468350)
  passed. All four language jobs passed separately. Analyses `2171758449` (Rust),
  `2171740821` (JavaScript/TypeScript), `2171740016` (Actions) and `2171741043`
  (Python) each report zero results and empty errors, with their respective
  `/language:<language>` categories. CodeQL is 2.27.4.
- Parsed the current main Rust job's emitted diagnostic JSON and compared its
  explicit extraction paths with `git ls-files '*.rs'`: all nine paths match,
  including the engine and native credential module. No emitted warning/error
  diagnostic or original semantic-analyzer/macro-expansion warning occurs.
  The report has 68 note-level diagnostics; internal consistency notes remain.
  This preserves the bounded coverage conclusion above, not perfect analysis.
- [Representative PR run 43742565783](https://github.com/moultonlavah/metten/actions/runs/43742565783)
  passed all four language jobs on head
  `e84a0e8d9f99d61475bd53ccf8d1913e6a02f8d8`; its
  [Foundation run](https://github.com/moultonlavah/metten/actions/runs/43742565772)
  also passed. The earlier reviewed PR/main extraction evidence remains applicable
  to the unchanged scanner configuration. This is prior-source evidence, not CI
  or merge eligibility for this documentation commit.
- Standard queries, macOS Rust/build mode `none`, and Linux JavaScript/TypeScript,
  Actions and Python remain configured without custom packs or exclusions.
  Apple Silicon beta support and unscanned standalone Swift remain limitations.
  The current checkout action emits a Node 20 deprecation warning; this is separate
  from extraction diagnostics and does not justify an unrelated pin update here.
- Paginated code-scanning alerts returned `[]`; no fix or dismissal is needed.
  This does not resolve the separate glib dependency advisory tracked by #39.
- Default setup is `not-configured`; advanced setup is active. Rulesets returned
  `[]`. Classic main protection requires only `mac-foundation`, strict, bound to
  GitHub Actions app `15368`; administrators are enforced. Stale approvals are
  dismissed, conversation resolution is required, force pushes/deletion disabled.
  Required approval count remains zero. Auto-merge is false; workflow defaults
  are read-only and cannot approve PRs. Ordinary workflow permissions remain
  `contents: read`.

For exact baseline comparison, the full branch-protection GET response has
SHA-256 `dcf671bcf2abb12a1b8699891f7dc768f9b2c3bd5769e607e4a146b61a07612e`
after parsing JSON and serializing with sorted keys, separators `(',', ':')`,
UTF-8 and no trailing newline. The full response is saved outside source at
`/tmp/metten-issue-30-protection-before.json`; it contains settings only. The
ruleset baseline is exactly `[]`. Capture fresh baseline responses before any
approved mutation; a changed digest requires inspecting the difference first.
Rollback deletes only the added ruleset, so it never rewrites classic protection.

### Exact proposed settings

Create **one** active branch ruleset named `Verified CodeQL`, targeting only
`refs/heads/main`, with no exclusions or bypass actors. Leave classic protection
and all other repository settings untouched. Proposed POST body for
`repos/moultonlavah/metten/rulesets`:

```json
{
  "name": "Verified CodeQL",
  "target": "branch",
  "enforcement": "active",
  "bypass_actors": [],
  "conditions": {"ref_name": {"include": ["refs/heads/main"], "exclude": []}},
  "rules": [
    {
      "type": "required_status_checks",
      "parameters": {
        "strict_required_status_checks_policy": true,
        "required_status_checks": [
          {"context": "Analyze (rust)", "integration_id": 15368},
          {"context": "Analyze (javascript-typescript)", "integration_id": 15368},
          {"context": "Analyze (actions)", "integration_id": 15368},
          {"context": "Analyze (python)", "integration_id": 15368}
        ]
      }
    },
    {
      "type": "code_scanning",
      "parameters": {
        "code_scanning_tools": [
          {"tool": "CodeQL", "alerts_threshold": "none", "security_alerts_threshold": "none"}
        ]
      }
    }
  ]
}
```

The individual job names and Actions app ID were observed on current main.
They require each language independently rather than an aggregate workflow
conclusion. The native CodeQL rule requires tool results without introducing a
new alert-severity policy: both alert thresholds are `none`. All alerts still
require triage under #30; any later severity-based merge policy is a separate
owner decision. This proposal does not claim to block newly discovered findings
by severity.

GitHub [documents the rules API](https://docs.github.com/en/rest/repos/rules#create-a-repository-ruleset)
and [native scanning merge protection](https://docs.github.com/en/code-security/concepts/code-scanning/merge-protection).
Native scanning rules have documented exceptions, and status checks can accept
skipped/neutral jobs. Neither mechanism alone is assumed to prove complete
per-language enforcement. The combined behavior needs the controlled tests below;
no eligibility or missing/failed-analysis guarantee is claimed yet.

### Controlled acceptance and rollback

After approval, re-read main, alerts and all settings. Save the complete baseline
responses outside source; stop if unrelated settings changed. Create the ruleset
and save its returned numeric ID and read-back response. Confirm effective rules
on main and unchanged classic protection, workflow permissions and manual merges.

Use one explicitly identified disposable PR to main, with temporary changes only
to its copy of the existing scanner workflow. Keep application sources, pins,
permissions and Foundation checks unchanged; never merge it or call the merge API.
Record head and test-merge SHA, run/job and analysis IDs, required-check conclusions,
rule evaluations where exposed, and GitHub merge eligibility for each candidate:

1. Missing analysis: remove the Rust matrix entry, let the other three languages
   and Foundation succeed, then confirm missing `Analyze (rust)` and blocked
   eligibility. A green aggregate result must not satisfy Rust.
2. Failed analysis: restore Rust and let initialization complete successfully.
   In a Rust-only step, move the initialized database from the run-log-confirmed
   `$RUNNER_TEMP/codeql_databases/rust` to an unused path in that same owned runner
   temporary directory. Confirm the source exists and destination is absent before
   moving; fail the experiment if the observed database path has changed. Confirm the
   analyzer actually runs and fails because no database exists, with the other
   languages and Foundation passing and merge eligibility blocked. Record the
   action failure and any uploaded failure diagnostic/API analysis error separately;
   an absent API analysis is not an uploaded errored analysis. Do not substitute a
   shell failure before initialization for this analyzer-boundary experiment.
3. Incomplete analysis: skip Rust initialization/upload **and its diagnostic-report
   step** while allowing its job to succeed (the report otherwise fails on missing
   SARIF); separately skip the Rust job. Confirm that neither absent current Rust
   analysis can pass the combined rules. Inspect exact-commit categories, not an
   older head's results. If GitHub treats either as eligible, the proposal fails;
   leave both issues open and prepare the smallest supported correction rather
   than claiming a reliable gate.
4. Neutral conclusion: GitHub also accepts neutral required-check conclusions.
   Check whether the existing Actions/analyzer path can produce one without forging
   an Actions check or using additional credentials. If supported, require blocked
   eligibility without fresh Rust analysis. If not, record this as untested; do not
   claim neutral-case enforcement or treat another app's neutral check as equivalent.
5. Restricted PRs: inspect a Dependabot and external-fork PR, when safely available,
   for upload permissions, all four current categories and blocked/eligible states.
   Do not broaden token permissions, use `pull_request_target`, enable access to
   fork secrets or execute untrusted code with a privileged token. GitHub documents
   [special CodeQL handling for Dependabot PRs](https://docs.github.com/en/code-security/how-tos/find-and-fix-code-vulnerabilities/troubleshoot-code-scanning/resource-not-accessible).
   Verify actual behavior; where a safe representative candidate is unavailable,
   record the test as missing and leave #40 incomplete rather than promise merges
   will remain available to that contributor class.
6. Restore the workflow byte-for-byte from main and obtain all four fresh analysis
   categories and full Foundation success. Confirm up-to-date, conflict-free,
   non-draft eligibility under the actual rules. A draft or unrelated failing
   check cannot be evidence of scanner enforcement. Close the disposable PR and
   delete only its test branch; retain immutable run/candidate links in this record.

No deliberately failed/unscanned candidate reaches main. Do not inject a real
vulnerability or use real credentials. If evaluation details cannot distinguish
scanner blocking from an unrelated blocker, record the evidence gap and stop.
A successful scan does not prove vulnerability absence or native coverage beyond
the inspected diagnostics.

If the API rejects the payload, make no substitute settings change without
reviewing it. If tests fail, delete only the newly created ruleset by its saved
ID, then verify rulesets return to the captured baseline and classic protection
is byte-for-byte equivalent after normalizing API ordering. Do not overwrite
concurrent settings or disable existing protection. Rollback authority is limited
to removing this session's new ruleset. Report the concrete failure and smallest
supported next proposal. Completion requires successful observed negative and
positive cases, final settings read-back, independent review, and owner acceptance
of #40 and the remaining parent criteria.

### Preparation review and checks

Independent Standards/security review (Codex / GPT-6.1 Sol) found one test-plan
issue: missing SARIF would make the diagnostic step fail the intended successful
but incomplete job. The plan now skips that step for that controlled candidate.
Independent Spec review (OpenCode Free Trial / DeepSeek V4.1 Flash Free) verified
the baseline and payload and identified six partials. Corrections above cover the
analyzer failure boundary, diagnostic step, neutral-case evidence limit, exact
rollback baseline hash, removal of the new severity policy, and restricted-PR
verification. The lead checked the complete missing/failed/incomplete-analysis
flow; these are proposal corrections, not completed hosted acceptance tests.
No additional review round was needed for these bounded documentation fixes.

Local preparation checks validate JSON parsing, exact four check names and Actions
app ID against observed main checks, local document links, whitespace, and unchanged
GitHub protections/rulesets. No application, workflow or dependency source changed;
existing Foundation/coverage evidence is reused only for those unchanged sources.
No hosted CI is claimed for this local documentation commit. There is no wired
standalone actionlint or JavaScript lint gate; TypeScript checking remains in
Foundation. Applying settings and all controlled cases above remain pending.


## Verified hosted evidence — 14 November 2026 UTC

Source: `2c1d266afa83c153f6bc9726529797ef8fc75b50`.
[CodeQL run 43566532454](https://github.com/moultonlavah/metten/actions/runs/43566532454)
passed all four language jobs. The
[Rust job](https://github.com/moultonlavah/metten/actions/runs/43566532454/job/132361501855)
used Rust 1.100.0 on `aarch64-apple-darwin`. Its diagnostic-report step explicitly
names all nine expected Rust files, including `src-tauri/src/credential.rs`, as
successfully extracted; extraction reports zero files with errors. The original
semantic-analyzer-unavailable/macro-expansion-skipped warning is absent, and the
emitted diagnostics contain no unresolved-macro, extraction-warning/error,
unextracted-element or low-database-quality entry. The module is included from
`main.rs` under `target_os = "macos"`; macOS extraction addresses that platform
boundary without launching the app or using credentials.

The report also emits note-level internal consistency diagnostics: control flow
(14 entries), data flow (29), syntax tree (9), type inference (5), and SSA (10).
These entries supply neither source locations nor numerical inconsistency counts;
they do not establish zero internal inconsistencies. Apple Silicon's beta support
and the unscanned standalone Swift harness remain limitations. File counts and
green jobs alone would not establish the platform correction.

[Foundation run 43566528865](https://github.com/moultonlavah/metten/actions/runs/43566528865)
passed on the same exact source, including both Python suites, interface and
TypeScript checks, web build, Rust formatting, workspace tests, release full-hour
tests, clippy, and packaged Mac build. The PR-triggered Foundation run also
passed. Automated Codex review reported no major issues on that source; there
are no submitted formal reviews or unresolved review threads.

Cleanup incorporates current main `47d0f38` through a normal merge and updates
only this evidence and the contributor coverage wording. The paired-action update
guidance is frequency-neutral to remain accurate alongside PR #36; the existing
manual fallback is preserved. Local workflow configuration assertions and a
comparison of explicit Rust extraction diagnostics against all tracked Rust
files passed, as did the whitespace check. The scanner workflow,
native credential source, Foundation workflow, compiler, and lockfiles are
unchanged from the recorded source; main's newer strategy correction still
requires final-head hosted checks. Those results must be inspected before merge;
prior runs are not final-head CI acceptance.

Read-only API recheck during cleanup: prerequisite #28 is closed, repository
code-scanning alerts return `[]`, and default setup is `not-configured`. Empty
alerts do not prove safety. Main has no replacement workflow until manual merge,
so the scanning gap remains escalated to the lead. No security settings changed.
After integration, verify actual main analyses and alert triage, then separately
configure and test missing/failed-analysis enforcement before promoting scanning
to a merge requirement. Issue #30 remains incomplete for those acceptance steps.

## Historical preparation and baseline

The sections below retain the setup sequence and its then-pending evidence;
the hosted results above supersede their pending statements.

## Mac-runner correction — 14 November 2026 UTC

Scope: `.github/workflows/codeql.yml`, the contributor maintenance notes, and
this evidence record. The branch incorporates main at
`4957ea16af9740da64544a1e9911d34f96a77373`; no application source, compiler,
Foundation workflow or required merge check is changed.

The new workflow uses `macos-15` for Rust and `ubuntu-latest` for
JavaScript/TypeScript, Actions and Python. All use build mode `none` and the
standard default queries, without custom packs or path exclusions. Actions and
Python preserve the other working default-setup analyses. Swift remains excluded
because its standalone harness failed default autobuild; no Swift coverage is
claimed. Pushes to main, pull requests to main and a weekly schedule trigger
analysis. No `pull_request_target` event, application launch or credential test
is introduced. The scanner alone receives `security-events: write` to upload
results; ordinary Foundation permissions remain `contents: read`.

Runner limitation: `macos-15` uses Apple Silicon (arm64). CodeQL's
[platform requirements](https://codeql.github.com/docs/codeql-overview/system-requirements/)
label Apple Silicon support beta; build mode `none` runs natively. If actual
extraction is inadequate on this runner, the bounded fallback is
`macos-15-intel`, followed by renewed extraction and final-head CI verification.
No successful architecture-specific coverage is claimed before execution.

The checkout and Rust installer pins reuse #28's reviewed revisions. The Rust
installer explicitly preserves **1.100.0**. CodeQL `init` and `analyze` use
[`33d7b85da1a04171e211befe2c739303b9a4c066`](https://github.com/github/codeql-action/commit/33d7b85da1a04171e211befe2c739303b9a4c066),
resolved from **v4.39.3**, the same action version used by the initial default
scan. The annotated tag is unsigned; GitHub reports the target commit signature
valid. Reviewed both action manifests and their initialization/finalization,
query-running and upload entry paths. Both run bundled Node 24 code and contain
no nested action references. Default tool selection and standard queries remain
upstream-managed, rather than introducing a separate toolchain pin. This is a
bounded provenance/configuration review, not an audit of all bundled code.
The existing monthly Actions Dependabot configuration covers this new workflow;
the contributor guide records paired updates and the existing manual fallback.

Default setup must be disabled before this workflow can upload results; GitHub
blocks advanced uploads while default setup is enabled. That transition and the
first advanced run will be recorded in the issue/PR. Until the workflow is merged,
main has no advanced workflow, so default-branch scanning cannot be claimed.
No required scanning check will be promoted before actual default-branch and
representative-PR evidence, alert triage and missing/failed-analysis enforcement.

Local preparation checks passed: Ruby YAML parsing plus assertions for the
language/runner matrix, event scope, least-privilege permissions, fail-fast
behavior, immutable action references, Rust 1.100.0 and build mode `none`.
Foundation workflow, compiler and application lockfiles match current main
byte-for-byte; whitespace check passed. No repository actionlint gate is wired.
Independent Standards/security review (Codex / GPT-6.1 Sol) found no workflow
blocker and requested that the contributor guide describe coverage as pending;
that wording is corrected. Independent Spec review (OpenCode Free Trial /
DeepSeek V4.1 Flash Free) agreed on scope and requested the architecture/beta
limitation above. Both documentation findings are resolved; hosted execution
remains outstanding. No action-update proposal has been observed; the documented
manual fallback applies.

Required next evidence: full final-head Foundation checks; successful per-language
CodeQL jobs; Rust extraction with the native module's semantic analyzer available,
including macro/type diagnostics, not merely disappearance of one warning or a
9/9 file count; and default-branch validation after manual merge.

## Baseline and setup — 14 November 2026 UTC

Repository: `moultonlavah/metten`. Source revision:
`6861d8e264b4a78cacbf91e89bb1145a8775b27d`. Prerequisite #28 is closed and its
pinned workflow is present. Before this work, the default-setup API reported
`not-configured`, no CodeQL workflow existed, and the repository had no rulesets.

Enabled GitHub-managed default setup with the `default` query suite and default
`remote` threat model. No custom workflow, query pack, path exclusion, compiler
change or application change was introduced. GitHub owns the generated workflow;
repository-maintained workflow actions retain their reviewed #28 pins.

An explicit REST request selecting `javascript-typescript` and `rust` failed
with HTTP 422. The exact request was:

```sh
gh api --method PATCH repos/moultonlavah/metten/code-scanning/default-setup \
  -f state=configured -f query_suite=default \
  -f 'languages[]=javascript-typescript' -f 'languages[]=rust'
```

The response stated: `Invalid property /languages/1: rust is not a possible
value. Must be one of the following: actions, c-cpp, csharp, go, java-kotlin,
javascript-typescript, python, ruby, swift.` This was the observed PATCH
validation response, not a claim that GitHub cannot analyze Rust. Retrying
with automatic language detection (omitting `languages`) was accepted and
started separate jobs for Rust, JavaScript/TypeScript, Actions, Python and Swift.
Do not work around the API mismatch by silently omitting Rust.

Initial default-branch run:
[43563413565](https://github.com/moultonlavah/metten/actions/runs/43563413565).
CodeQL version observed in completed analyses: **2.27.4**. Final default setup
was configured at `2026-11-14T05:56:57Z`, weekly schedule, standard runners,
default suite and remote threat model. The GET response lists `actions`,
`javascript`, `javascript-typescript`, `python`, `rust`, `typescript`; GitHub
automatically omitted Swift after its failed setup job. No manual language
removal was performed. The setup run overall concluded success despite its failed Swift job; that
overall status does not clear the individual failure or Rust limitation.

## Per-language evidence

### JavaScript/TypeScript

[Job 132351899537](https://github.com/moultonlavah/metten/actions/runs/43563413565/job/132351899537)
succeeded. Analysis `2166460035`, category `/language:javascript-typescript`,
reports zero results and no error on the source revision above.
The extraction log reports **5/5 JavaScript and 4/4 TypeScript files**, plus the
HTML entry point and GitHub Actions workflow. Its named extraction entries
match all tracked JavaScript/TypeScript sources:

- `src/contract.ts`, `src/main.ts`, `src/view.d.mts`, `src/vite-env.d.ts`;
- `src/view.mjs` and the four `tests/*.test.mjs` files.

No repository-configured exclusions were added. These are static source
coverage counts, not execution coverage or proof that every defect is detected.

### Rust

Expected tracked source inventory is nine files: the engine's `lib.rs`,
`storage.rs`, `strategy.rs`; its three integration-test files; and
`src-tauri/build.rs`, `src-tauri/src/main.rs`, `src-tauri/src/credential.rs`.
The native credential module is gated by `target_os = "macos"`; coverage must
explicitly account for that boundary, not infer it from engine analysis.

[Rust job 132351899545](https://github.com/moultonlavah/metten/actions/runs/43563413565/job/132351899545)
succeeded. Analysis `2166473517` reports zero results, 26 rules, no API-level
error or warning, category `/language:rust`, build mode `none`, and runner
`ubuntu-latest`. The query pack was `codeql/rust-queries` **0.1.46**. Its log
reports **9/9 Rust files extracted without error**, zero with errors.

However, at `05:51:44Z` the extractor explicitly reported:

```text
src-tauri/src/credential.rs:1:1: semantic analyzer unavailable (not included as a module): macro expansion will be skipped.
```

The run used `1.100.0-x86_64-unknown-linux-gnu` with no Cargo target or cfg
overrides. The nine-file count therefore does not establish adequate native
credential analysis. This is the blocking coverage limitation, not a discovered
application vulnerability. The smallest proposed supported setup change is
advanced setup running Rust in build mode `none` on a macOS runner, retaining
standard queries and JavaScript/TypeScript analysis, with #28-reviewed action
pins. A full Rust build is not required merely because Rust is compiled. This
proposal has not been implemented or proven; repeat extraction diagnostics and
default-branch/PR acceptance before promoting any gate.

GitHub documents build mode `none` for Rust: rust-analyzer may run build scripts
and compile macros without building the complete app. Generated code, dependency
resolution and platform configuration therefore need inspection in the actual
run; file enumeration alone does not establish complete semantic coverage.

### Other automatically detected languages

Actions and Python jobs succeeded with zero results (analyses `2166459504` and
`2166459994`). The Swift job
[132351899574](https://github.com/moultonlavah/metten/actions/runs/43563413565/job/132351899574)
failed: autobuild found neither an Xcode project/workspace nor a Swift package.
The repository contains a standalone `scripts/verify-credential.swift` harness,
not a Swift app project. Its uploaded analysis `2166462000` records
`unsuccessful execution`; zero
results on that record are not a passing scan. No successful Swift analysis is
claimed. Omitting this ancillary language is smaller than adding a Swift build,
and GitHub already omitted it automatically. That does not fix the required Rust
coverage gap or make this issue complete. If Swift is
retained in scanning scope, the supported correction is advanced setup with an
explicit compile command for that script; do not execute the credential harness
to obtain extraction. Any new workflow actions must satisfy #28.

## Protections and remaining acceptance

Baseline branch protection on `main`: strict required `mac-foundation` from
GitHub Actions (app ID 15368); enforced for administrators; stale reviews
dismissed; conversation resolution required; force pushes and deletion disabled.
Auto-merge is disabled. Ordinary workflow defaults are read-only and cannot
approve pull requests; `.github/workflows/checks.yml` keeps `contents: read`.
These settings have not been weakened.

After the initial run, the paginated repository alerts API returned `[]`: there
were no initial alerts to triage, fix or dismiss. This does not clear the Rust
coverage limitation. No local alert ledger or unsupported dismissals were added.

Before promotion, inspect any subsequent alerts, fix confirmed in-scope findings
through linked issues, and record any justified dismissals individually in
GitHub. Demonstrate both languages on the default branch and a representative
pull request. Then verify the actual required checks and that missing or failed
analysis blocks merging. A successful JavaScript scan cannot substitute for
missing Rust evidence. No merge gate is claimed until that sequence passes.

The existing Foundation workflow supplies TypeScript checking, Rust formatting
and clippy, interface/native tests, release-mode hour tests and the packaged
Mac build. There is no separate JavaScript lint gate. Final-candidate CI
remains pending; its exact commit and run will be recorded on issue #30. Independent checkpoint reviews: Codex / GPT-6.1 Sol
found no standards violations; OpenCode Free Trial / DeepSeek V4.1 Flash Free
identified missing detail for the PATCH rejection and failed Swift analysis.
Both details are now recorded above. Its remaining observations were correctly
unmet acceptance criteria. The lead checked the complete language/setup flow
and added the final Rust result and concrete native-coverage blocker. No broader
code changes or additional review round were needed for these evidence updates.

## Primary references

- [Default setup](https://docs.github.com/en/code-security/how-tos/find-and-fix-code-vulnerabilities/configure-code-scanning/configure-code-scanning)
- [Rust build mode and requirements](https://docs.github.com/en/code-security/reference/code-scanning/codeql/build-options-for-compiled-languages)
- [Merge-protection behavior and exceptions](https://docs.github.com/en/enterprise-cloud%40latest/code-security/concepts/code-scanning/merge-protection)
