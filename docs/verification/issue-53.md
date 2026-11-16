# Issue #53: correctness and type-aware lint

## Current evidence

Implementation candidate on `t3/implement-issue-53`, based on
`c1a6fa973ea0aee8f635a42a9f44923d1ec321df`. Review, final regression and exact-head
CI are pending. This record does not establish merged-source acceptance.

## Instruction and scope baseline

On 16 November 2026, freshly queried `git ls-remote --symref origin HEAD` resolved
`https://github.com/moultonlavah/metten.git`, `refs/heads/main`, to
`c1a6fa973ea0aee8f635a42a9f44923d1ec321df`; selected HEAD matched and the worktree
was clean. Read immutable root AGENTS.md, verification.md, maintainability.md,
code-map.md, issue-tracker.md and triage-labels.md under docs/agents; corresponding
local instructions match. Published inventory has no nested AGENTS.md. Also read
CONTRIBUTING, product specification, glossary, source/test contracts, coverage map
and issue #53 including comments/native blockers. #52 and #51 are closed and their
final layout is present. No engine/desktop implementation or domain change is made;
conditional ADR/domain procedures do not add work to this tooling-only scope.

Expected path: package/lockfile → one Oxlint config → two npm commands → existing
Foundation job → contributor usage and existing code/coverage maps. No departures,
production fixes, suppressions, compiler changes or new abstractions were needed.
The two development dependencies implement the issue's named requirement.

## Candidate and coverage

Installed exact `oxlint@1.94.0`, `oxlint-tsgolint@7.0.2007`; Oxlint's installed
optional peer requires `oxlint-tsgolint >=7.0.2007`. No overrides; TypeScript stays
7.0.2 and Rust stays 1.100.0. Initial installation under host Node 25.9.0 succeeded
with zero audit findings. Probe runs use temporary npm-exec Node 24.22.3.
Official [type-aware integration](https://oxc.rs/docs/guide/usage/linter/type-aware)
and [configuration](https://oxc.rs/docs/guide/usage/linter/config) were consulted.

`npm run lint -- --debug files` selects exactly:

- src/contract.ts, src/main.ts, src/view.ts, src/vite-env.d.ts
- tests/credential.test.mjs, tests/refresh.test.mjs, tests/strategy.test.mjs,
  tests/ui-harness.mjs, tests/ui-harness.test.mjs, tests/view.test.mjs

Tracked/nonignored inventory contains no other maintained JS/TS or JS/TS config.
The config enables correctness warnings for TypeScript/unicorn/oxc plus core
rules and explicitly enables typescript/no-floating-promises as error. Browser
source/Node test environments are separate. Both npm commands use `src tests`,
`--type-aware --max-warnings 0`; fix adds only `--fix`. Explicit ignore patterns
exclude node_modules, dist, target, src-tauri/gen, private-security and .scratch;
Git ignore rules also exclude private/generated inputs. JSON/YAML configuration
is not executable JS/TS and is outside lint. No formatting or other-language lint.

Ordinary JS correctness coverage is distinct from the TypeScript program's typed
rules. Declaration selection is not comprehensive declaration validation:
`skipLibCheck` remains enabled; a temporary missing declaration type passed lint.
The existing compiler and VM checks remain required. Existing async chains were
inspected: credential/file/preview/save chains catch errors; refresh, list and
detail functions have their existing error paths. Explicit-void sites remain
permitted; this does not prove universal rejection safety or change error UX.

## Enforcement probes

Temporary append-only probes ran through **npm run lint**, under Node 24.22.3.
A Python try/finally wrapper saved original bytes, restored them after each probe
and on SIGINT/SIGTERM/SIGHUP, asserted byte equality, and required a clean lint
between probes. The first assertion expected a slash-form diagnostic name while
Oxlint prints parentheses; it failed after detecting the intended rule and restored
all bytes. Correcting only that assertion produced this completed run:

| Path | Temporary probe | Intended rule | Exit |
| --- | --- | --- | --- |
| src/main.ts | bare Promise.reject(new Error("probe")) | typescript/no-floating-promises | 1 |
| src/main.ts | same promise with catch handler | handled control | 0 |
| src/main.ts | same promise prefixed with void | accepted explicit-void limit | 0 |
| src/view.ts | exported function returning twice | eslint/no-unreachable warning | 1 |
| tests/ui-harness.mjs | exported function returning twice | eslint/no-unreachable warning | 1 |

These are parsed semantic/rule failures, including a maintained JavaScript helper;
no suppression-comment or parse-only probe substitutes for defect detection.
Warning probes prove --max-warnings 0 enforcement. All probes were removed and
all original source bytes matched. No production source or test behavior changed.

## Repository enforcement

Read current main branch protection on 16 November 2026: strict required check
`mac-foundation`, GitHub Actions app 15368, zero required approving reviews.
The existing job now runs lint immediately after npm ci, retaining every gate.
Active ruleset 27343255, "Verified CodeQL", was also present; no merge/security
settings were changed. Exact-head Foundation/CodeQL and merged acceptance remain
pending until linked evidence is available.
