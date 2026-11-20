# Contributing to Metten

Metten is in early development. The controlled paper prototype uses synthetic inputs and an accelerated clock; live trading is unavailable. Read the [README](README.md) for setup and the [product specification](METTEN_DESIGN_DRAFT_2026-11-12.md) for intended behavior.

## Find work

Check [open issues](https://github.com/moultonpoint/metten/issues) and their blockers before starting. Discuss substantial changes in an issue first. The [implementation plan](docs/implementation-plan.md) describes the planned sequence, and [ADR 0001](docs/adr/0001-rust-engine-and-tauri-boundaries.md) explains the engine and desktop boundary.

## Set up and check changes

For features, fixes, refactors, tests, tooling and reviews, use the [maintainability standard](docs/agents/maintainability.md): find the owner and contract, make the smallest complete change, and leave meaningful verification and a clear route for the next contributor.

Use macOS, Node.js 24 or later, Apple's Xcode Command Line Tools, and the Rust toolchain pinned in `rust-toolchain.toml`. Run `npm ci` and `npm run tauri dev` to start the local foundation. The README explains its test session and isolated workspace.

Run checks relevant to your change. The current automated workflow runs these on macOS:

```sh
python3 tests/credential-runner.test.py
python3 tests/check-staged.test.py
npm run lint
npm run check
npm run build
cargo fmt --all --check
cargo test --workspace --locked
cargo test -p metten-engine --release --locked --test strategy -- --ignored --nocapture
cargo clippy --workspace --all-targets --locked -- -D warnings
npm run tauri build -- --bundles app
```

`npm run lint` runs pinned Oxlint correctness rules and type-aware
`typescript/no-floating-promises` with zero warnings allowed; `npm run lint:fix`
uses the same command/configuration with safe automatic fixes. Review fixes before
committing. The single [configuration](.oxlintrc.json) covers maintained `src`
TypeScript (including declarations and `view.ts`) and all `tests` JavaScript,
including `ui-harness.mjs`. Browser globals apply to source; Node globals to tests.
There are currently no maintained JS/TS configuration files outside those paths;
add any future configuration entry points to both npm commands. Dependencies,
build/generated output and private files are excluded. Python, Swift, Rust,
formatting and hooks are outside this lint command.

Type-aware rules use resolved TypeScript programs; JavaScript tests receive
ordinary correctness checks, not a new JS type-checking promise. Declaration
files are selected, but `skipLibCheck` and applicable-rule limits still apply.
`npm run check` remains the implementation/type-contract check. The promise rule
uses its default `ignoreVoid: true`: bare floating typed promises fail, while
handled chains and explicit `void` pass. **Discarding a rejected promise with
`void` can still produce an unhandled rejection.** Lint does not establish safe
error handling at every site, trading correctness or recovery correctness.
See [issue #53 evidence](docs/verification/issue-53.md) for coverage and enforcement
probes. Foundation runs this same lint command after installation, alongside all
existing checks; repository merge/security settings are maintained separately.

Describe what you changed, link the issue, and report checks and any limitations in your pull request. Do not present paper or simulated results as proof of live behavior.

## Protect users and their data

Use controlled inputs and dummy credentials for tests. Never commit secrets, private runtime records, or account data. An issue or contribution does not authorize access to real credentials or accounts, paid services, deployment, or real-money validation. Do not operate another trading application or import its code or strategy thresholds. See the [product specification](METTEN_DESIGN_DRAFT_2026-11-12.md) for the intended safety boundaries.

Report vulnerabilities privately using the instructions in [SECURITY.md](SECURITY.md). Do not post secrets or exploitable vulnerability details in a public issue.

## Maintain build-action pins

External actions in `.github/workflows/` use full commit SHAs, with
upstream versions or branch names in comments. `.github/dependabot.yml` requests
weekly update proposals for the repository’s GitHub Actions. This
configuration takes effect on the default branch. Proposals require source review and
passing Foundation checks before merge; they are never automatic approval.

Connor Moulton (@moultonpoint) owns a manual review on the first of each month
for `dtolnay/rust-toolchain`'s `master`-derived pin, and checks that the other two
pins have received proposals or manually reviews them if automation is silent.
Dependabot branch-derived SHA updates are not relied on: its current updater
returns without updating when no version tag is available. See the
[issue #28 evidence](docs/verification/issue-28.md) for the source and limitation.

For each update, resolve the version/ref in the upstream repository, review the
old-to-new source diff and action entry points (including nested action refs),
and record the upstream ref, full SHA and review findings in the update issue or
PR. Change the SHA and readable comment together. Preserve Node 24, restricted
`contents: read` permissions, and every existing check, including the release
full-hour tests and packaged Mac build. Rust **1.100.0** stays fixed: a compiler
upgrade needs a separate issue and reassessment of the engine's
compiler-dependent resource proof. Run the complete Foundation checks workflow
on the exact proposed revision before accepting any pin update.

## Review dependency updates

Dependabot checks npm, the root Cargo workspace (both members and the shared
lockfile), and GitHub Actions weekly. Patch/minor version updates are grouped
per ecosystem; major upgrades remain separate proposals. Grouping is not proof
of compatibility. All merges remain manual. Security-update PRs are enabled in
repository settings and follow GitHub's security processing, independently of
the weekly version-update schedule; the version groups do not delay them.

For each proposal, record the affected checks and evidence in its PR before
merge. Require passing Foundation checks on the exact proposed revision,
including the native tests, release full-hour tests and packaged Mac build.
Review Tauri's npm API/CLI and Rust crates together for a supported combination;
separate ecosystem proposals must not be merged into an incompatible state.
If coordinated changes are needed, combine them in one reviewed candidate and
check that revision. Do not silently advance Rust **1.100.0** to accept an update:
leave an update requiring a newer compiler unmerged and open a separate compiler
issue with the affected resource evidence.

Re-run the isolated dummy-credential, storage or IPC packaged checks when an
update affects those boundaries. Reassess engine resource proofs when relevant
allocation, layout, history or processing assumptions change. Record why those
checks are affected or why existing evidence still applies; do not repeat
unrelated manual checks or present an old build's results as evidence for a new
artifact. Follow the action-pin review route above for action updates, including
the first-of-month manual fallback. See the [issue #29 record](docs/verification/issue-29.md)
for actual settings and activation evidence still required.

## Maintain security scanning

`.github/workflows/codeql.yml` runs the standard CodeQL queries. Rust runs on
macOS; the recorded extraction diagnostics include the Mac-only credential module
without the original platform-exclusion warning. This closes that specific gap,
not full security acceptance. JavaScript/TypeScript, Actions and Python run on
Linux. Swift's standalone
verification harness is not scanned. The scanner does not execute that harness
or access credentials. See [issue #30 evidence](docs/verification/issue-30.md)
for coverage limitations and the verified enforcement/test results. The owner-approved
merge rule is active under #40. Controlled tests proved missing, failed,
incomplete and skipped Rust analysis blocked merging, while complete analysis
allowed eligibility. The evidence record retains neutral-job and external-fork
verification limits, which the owner excluded from closure requirements. No claim
of complete security acceptance is made.

The CodeQL `init` and `analyze` actions share one reviewed upstream release SHA.
Update both together through the documented Dependabot/pin-maintenance route, with the
same manual fallback owner above. Review action entry points and release changes,
then inspect each language's extraction diagnostics and run all Foundation checks
on the final revision. A successful job or file count alone does not prove native
coverage. Keep the pinned Rust compiler and ordinary workflow permissions intact.

## Source integration checks

Use the published maintainability standard from freshly resolved default-branch guidance even in this older worktree. `npm run lint` runs pinned Oxlint correctness rules and type-aware `typescript/no-floating-promises` with zero warnings. `lint:fix` adds safe fixes to the same configuration; inspect any edits. Actual maintained paths include src/view.ts and all tests JavaScript; browser and Node globals are separate. Lint does not replace TypeScript, behavior tests or rejection handling at explicit-void sites.

Run `cargo test -p metten --locked` and the public engine source tests for changed native/source boundaries. Run `python3 tests/credential-runner.test.py` after shared bootstrap/runner changes. The isolated packaged commands and proof limits are in [scripts](scripts/README.md#isolated-sources-walkthrough-step-5). Calibration/qualification belongs to deferred issue16; no test grants provider/account/ordinary credential access or rights. Reuse unchanged release-hour arithmetic evidence only within its named version/assumptions; the complete Foundation job retains that gate.

The capacity campaigns, collectors and diagnostics have been deleted by owner direction. No opt-in performance command remains. Complete the issue 5/6 prototype with ordinary product-behavior tests; issue16 owns any later measurements justified by the integrated application. Historical failures remain documented and establish no supported envelope. Never run all ignored tests together: remaining ignored engine/Keychain checks have their own explicit selection and authority requirements.

## Controlled paper checks

Run `python3 tests/paper-runner.test.py` with the ordinary checks. After `npm run tauri build -- --bundles app`, run `python3 scripts/verify-paper.py target/release/bundle/macos/Metten.app`. The latter resolves the bundle executable from Info.plist, uses a new owned workspace and invokes the headless synthetic-only protocol. It does not open a window, use credentials or connect providers. Preserve any workspace/PID the runner reports as exit-unconfirmed; never force cleanup of a live database owner. See [paper guide](docs/strategies/paper.md) and [actual evidence](docs/verification/issue-6.md).
