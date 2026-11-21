# Daily EGM documentation drift report — 21 November 2026

Pass: 2026-11-21, started 09:00 CST (America/Chicago).
Accountable lead: Codex. **No current drift established in the selected working copy, using fresh boundary inspection and identity-verified prior source inspection.** This is a static documentation audit with the explicit evidence limits below, not a new acceptance or independent resource/security certification.

## Scope and authoritative baseline

- Working copy: `/Users/connormoulton/Desktop/metten`; selected HEAD `868609baffc25f06ecb43d8760e7cdd60f684a68`.
- The fresh inventory is **241 distinct tracked plus nonignored untracked paths**, including hidden configuration. Derived with `git ls-files --cached --others --exclude-standard`, deduplicated; no fixed issue list or yesterday-only projection. The inventory and exact starting SHA-256 identities appear at the end.
- All maintained components are included, including issue #4, its complete shared-file additions, strategy implementation/tests, learning examples, command/permission artifacts, dependencies, modules, APIs, storage/schema and UI. Only ignored/private/secret/runtime records, Git internals, installed dependencies and generated build/cache outputs are excluded. No excluded records were opened.
- Nine pre-existing modified documentation/template files and five pre-existing untracked dated reports were preserved. Today's report did not exist at the start.
- Remote `origin = https://github.com/moultonpoint/metten.git`; fresh `git ls-remote --symref origin HEAD` at 09:00 CST resolved `refs/heads/main` to **8bb7aac3704c9b197e58e10ed9256483cbf9f2e8**. A delivery-preparation query returned the same commit. These objects were already available; no fetch, pull, ref update or checkout.
- Read the published immutable root AGENTS.md, verification/discovery procedure, code map and maintainability guidance at that commit, plus local instructions, current ownership map and applicable contracts. Local inventory has only root AGENTS.md; published-tree instruction discovery found no additional entry to load. This session loaded the published entry and discovery route; no claim is made about delivery to other sessions.
- Published navigation includes later source/paper implementation absent from this selected checkout. Published process guidance does not convert that remote code into implemented local scope. This report covers the selected working copy, not every path on newer remote main.
- Implemented local scope: MIT licensing, controlled persisted non-trading session, isolated dummy credentials, controlled format-2 rules and immutable local versions, supporting CI/tools/assets/documentation. Connected sources, financial paper/live execution/accounting/recovery and release qualification remain future scope here; absence is not a completed-scope regression.
- Used installed `egm-drift-report` skill at `/Users/connormoulton/.codex/plugins/cache/egm/egm/0.1.1/skills/egm-drift-report/SKILL.md` and its bundled `docs/codex.md`. No bootstrap, design-review workflow, delegation or new thread. Set this thread's title to exactly **Daily Drift Report**. No schedule operation.

## Evidence method, reuse and coverage limits

Current source reads were sequential. Read shared session/strategy protocol and storage files, native main/credential source, UI/types/helpers/HTML/CSS, maintained command permissions, manifests/configuration, workflows, scripts and interface/Python/session/storage tests without the former foundation-only filter. Current docs were compared in both directions with the independently derived maintained inventory.

The earlier [expanded inspection](2026-11-14_DRIFT_REPORT.md#authority-identity-and-method) records complete sequential evaluator, public strategy-test and shared-source inspection at `96ad37ab547751f1f3b4b55d392bc42be3563e9c`. Reuse that unchanged inspection only with verified correspondence:

- Compared 136 non-README paths under crates/src/src-tauri/tests/scripts/assets with their immutable blobs at that revision: **133 byte-identical, three changed**. Root `index.html` is an additional freshly read shared input, giving 137 component inputs in this comparison scope; no issue-based exclusion.
- The three differences are `crates/engine/src/strategy.rs`, its public `tests/strategy.rs`, and `scripts/verify-credential.swift`. Read the entire two-file strategy diff and the current Swift source. Strategy source/tests match the final identities in [F12's diagnostic correction](2026-11-14_DRIFT_REPORT.md#f12-diagnostic-correction-and-sol-review--2026-11-14t104821737765-0600): **330994592799c22bb12685dbe55a5c4ebc1b0ac3f5765f26222b168a6a760575** and **7e4476d008eeb2e26d43cb3a50305a5a6a23e3b3636e0e79d3ff45cf58b7a540**.
- The diagnostic change uses an owned-or-borrowed Rust accepted_form while retaining a serialized string, separates instance overload from planned-memory rejection, reports indexed per-measurement costs without echoing author identifiers, and is covered by the recorded public preview/save regression. No new execution of that regression.
- The operative design is **9b5f1d2738fad3f77c99d70ea3cfe6e7436b370e07a5056daa08dc23f3fc872d**; the teaching file is **a79bd747078601cd16bc292c7896cdcb9f2fe3db0feebf412284a50a2bd50aa5**. Both match prior complete inspection. Read current approval/authority and phase/evidence navigation; archived drafting statements do not supersede later acceptance.
- Swift source is **e98d408323d4a24dcbd0fced8e56792751087d1598c37973f635a2318c61ffcb**, exactly the issue-31 recorded source. Current read includes bundle-specific scan paths, retained-child shutdown and final cleanup.
- Reuse the full unchanged evaluator/history/continuation/resource inspection and unchanged reference teaching examples. **The 7,212-line evaluator and 3,126-line public strategy test file were not both reread line by line today.** No new universal processing/memory proof or capacity certification is claimed. Fingerprints establish source correspondence, not that an earlier review was infallible.
- Reused historical acceptance remains bounded to its original artifact/source and behavior. Current issue-4/30/31 evidence indexes and relevant final corrections were consulted. Historical proposal/review bodies, old evidence chronology and every older report were not exhaustively reread. They have documented archival ownership; only the indexed approved issue-4 contract is operative.
- All **341 relative file/directory links** checked in active Markdown documents resolve. This checks local targets, not all heading anchors, external redirects, hosted policy/settings or upstream service facts.
- Every inventory component has an ownership route below. No newly discovered unowned implementation path was established. Static export coverage establishes inventory/ownership/provenance correspondence only; no visual, legal, device or print recertification.

This run does not claim a complete fresh line-by-line audit or clean application acceptance. Its no-new-drift conclusion depends partly on the precisely identified prior inspection. Unverified runtime/security/distribution behavior remains unverified.

## Maintained inventory reconciled to ownership

| Inventory group | Current owner |
| --- | --- |
| Root manifests, both locks, toolchain, TypeScript/ignore settings | README, CONTRIBUTING and package contracts |
| Engine session, strategy, storage; embedded/public/integration tests | crates and engine/source READMEs, strategy reference and indexed operative issue-4 design |
| Native startup, admission, shutdown, credential module | desktop/source READMEs; issues 2/3/4/31 evidence |
| Native build/config/capability and four permission TOMLs; two icons | desktop package README |
| Frontend main/types/view/declaration/Vite declaration, HTML/CSS | src README; strategy guides |
| Four JavaScript and two Python regression files | tests README |
| Swift verifier and staged-check helper | scripts README and verification procedure |
| Foundation, CodeQL and Dependabot | CONTRIBUTING; verification 28/29/30/PR-33/PR-34 |
| Issue/PR templates | issue workflow |
| Assets, brand master/specification, SVG/PNG/PDF exports and font license/provenance | assets and brand READMEs; desktop icons separately owned above |
| Product, accepted ADRs, root/agent docs and implementation sequence | docs README and AGENTS.md |
| Strategy guides/reference and examples/learning.json | strategies README |
| Approved issue-4 design plus remaining proposal/review/source captures | docs README, issue-4 current index; remaining proposal tree historical |
| Verification records and all dated reports, including nonignored untracked reports | docs README and EGM README |

## Resolved findings carried forward

Yesterday's original F1/F2/F3 are **resolved locally**, as recorded in [the 20 November correction appendix](2026-11-20_DRIFT_REPORT.md#owner-authorized-documentation-corrections--2026-11-20t142709620860-0600). Current map includes CodeQL and issue-29/30/PR-33/34 evidence; active repository destinations use moultonpoint; daily coverage explicitly includes issue #4 and new maintained components, including weekends. Historical owner URLs remain historical and are not reopened as active-link drift.

Older expanded-audit component/test/helper/format omissions, F12 memory-error diagnostic and original F8 private-reporting destination/storage findings remain resolved. The report is not evidence of current hosted reporting settings, redirects, notification delivery or tracker state.

## docs/egm/README.md

No drift detected. Evidence reuse and verification limits above apply.

## docs/README.md

No drift detected. Evidence reuse and verification limits above apply.

## README.md

No drift detected. Evidence reuse and verification limits above apply.

## CONTRIBUTING.md

No drift detected. Evidence reuse and verification limits above apply.

## SECURITY.md

No drift detected. Evidence reuse and verification limits above apply.

## AGENTS.md

No drift detected. Evidence reuse and verification limits above apply.

## GLOSSARY.md

No drift detected. Evidence reuse and verification limits above apply.

## LICENSE

No drift detected. Evidence reuse and verification limits above apply.

## docs/implementation-plan.md

No drift detected. Evidence reuse and verification limits above apply.

## docs/agents/code-map.md

No drift detected. Evidence reuse and verification limits above apply.

## docs/agents/maintainability.md

No drift detected. Evidence reuse and verification limits above apply.

## docs/agents/verification.md

No drift detected. Evidence reuse and verification limits above apply.

## docs/agents/issue-tracker.md

No drift detected. Evidence reuse and verification limits above apply.

## docs/agents/domain.md

No drift detected. Evidence reuse and verification limits above apply.

## docs/agents/triage-labels.md

No drift detected. Evidence reuse and verification limits above apply.

## METTEN_DESIGN_DRAFT_2026-11-12.md

No drift detected. Evidence reuse and verification limits above apply.

## docs/adr/0001-rust-engine-and-tauri-boundaries.md

No drift detected. Evidence reuse and verification limits above apply.

## docs/adr/0002-mit-license.md

No drift detected. Evidence reuse and verification limits above apply.

## crates/README.md

No drift detected. Evidence reuse and verification limits above apply.

## crates/engine/README.md

No drift detected. Evidence reuse and verification limits above apply.

## crates/engine/src/README.md

No drift detected. Evidence reuse and verification limits above apply.

## src-tauri/README.md

No drift detected. Evidence reuse and verification limits above apply.

## src-tauri/src/README.md

No drift detected. Evidence reuse and verification limits above apply.

## src/README.md

No drift detected. Evidence reuse and verification limits above apply.

## tests/README.md

No drift detected. Evidence reuse and verification limits above apply.

## scripts/README.md

No drift detected. Evidence reuse and verification limits above apply.

## .github/workflows/checks.yml

No drift detected. Evidence reuse and verification limits above apply.

## .github/workflows/codeql.yml

No drift detected. Evidence reuse and verification limits above apply.

## .github/dependabot.yml

No drift detected. Evidence reuse and verification limits above apply.

## .github/ISSUE_TEMPLATE/bug_report.md

No drift detected. Evidence reuse and verification limits above apply.

## .github/ISSUE_TEMPLATE/work_proposal.md

No drift detected. Evidence reuse and verification limits above apply.

## .github/pull_request_template.md

No drift detected. Evidence reuse and verification limits above apply.

## docs/strategies/README.md

No drift detected. Evidence reuse and verification limits above apply.

## docs/strategies/authoring.md

No drift detected. Evidence reuse and verification limits above apply.

## docs/strategies/agent-authoring.md

No drift detected. Evidence reuse and verification limits above apply.

## docs/strategies/reference.md

No drift detected. Evidence reuse and verification limits above apply.

## docs/proposals/issue-4-egm/strategy-rules-design.md

No drift detected. Evidence reuse and verification limits above apply.

## docs/verification/issue-4.md

No drift detected within its historical evidence scope. Evidence reuse and verification limits above apply.

## docs/verification/issue-2.md

No drift detected within its historical evidence scope. Evidence reuse and verification limits above apply.

## docs/verification/issue-3.md

No drift detected within its historical evidence scope. Evidence reuse and verification limits above apply.

## docs/verification/issue-28.md

No drift detected within its historical evidence scope. Evidence reuse and verification limits above apply.

## docs/verification/issue-29.md

No drift detected within its historical evidence scope. Evidence reuse and verification limits above apply.

## docs/verification/issue-30.md

No drift detected within its historical evidence scope. Evidence reuse and verification limits above apply.

## docs/verification/issue-31.md

No drift detected within its historical evidence scope. Evidence reuse and verification limits above apply.

## docs/verification/pr-33.md

No drift detected within its historical evidence scope. Evidence reuse and verification limits above apply.

## docs/verification/pr-34.md

No drift detected within its historical evidence scope. Evidence reuse and verification limits above apply.

## assets/README.md

No drift detected within inventory/ownership/provenance scope. Evidence reuse and verification limits above apply.

## assets/brand/README.md

No drift detected within inventory/ownership/provenance scope. Evidence reuse and verification limits above apply.

## assets/brand/specifications.json

No drift detected within inventory/ownership/provenance scope. Evidence reuse and verification limits above apply.

## assets/brand/licenses/font-source.json

No drift detected within inventory/ownership/provenance scope. Evidence reuse and verification limits above apply.

## Final verification limits and disposition

No fresh acceptance tests, other tests, builds or executable verifier checks ran. No browser, app, container, credential/account/private runtime record, real provider, paid service or trading operation was accessed. No source/contract fix, staging, commit, push, tracker or schedule change.

Current hosted settings/CI/security alerts, external links, physical power-loss durability, GUI/accessibility, signed upgrade continuity, real credentials/providers, financial paper/live behavior and fills remain unverified here. Existing explicit limitations in the evidence records are not new drift or newly passed checks. The no-new-drift conclusion is a bounded static result supported by fresh boundary inspection and exact-identity prior evidence, not an unqualified clean certification.

Before writing, all **241** starting files retained their byte counts and SHA-256 identities, with no added/removed paths; selected HEAD was unchanged. Starting modified files remain concurrent work. Final preservation and published-guidance refresh are recorded below after creation.

## Starting inventory and source identities

This manifest records all 241 maintained input paths, not ignored/private/runtime data. Byte counts and hashes refer to the start of this pass. It supports future evidence reuse; it is not an additional behavioral specification.

| Path | Bytes | SHA-256 |
| --- | ---: | --- |
| `.github/ISSUE_TEMPLATE/bug_report.md` | 873 | `54f86f8074d284b9669340ffe0cfff5a48f75eaa386ded008359dc9b7415d908` |
| `.github/ISSUE_TEMPLATE/work_proposal.md` | 720 | `c6d13aa627297db095119658e01342866a92920ee9bda5e371a69ec4ca7935d1` |
| `.github/dependabot.yml` | 753 | `9c5e610f87c393435f6434df5ee83fad2c9bf098264dd86591c29a8126073a6e` |
| `.github/pull_request_template.md` | 620 | `d100a5a320528c10c7f46394681476780b2954f9a59ebb22c505848d156724a8` |
| `.github/workflows/checks.yml` | 1068 | `27a633dc1019f158301467299b9e89522115c072766ce17080fbf087e0961ad0` |
| `.github/workflows/codeql.yml` | 1840 | `7dbb38522366c9d169092c70d9febaaad58c2a5d061782f91428a1c72b65a957` |
| `.gitignore` | 386 | `6434b02b300046bbc8440a22c6314f574288ff1757df87336478697108f22ad8` |
| `AGENTS.md` | 3426 | `a8a94353471c303f99b0701a89c3a8718c1dcd0e0545f1bd513fed04c7d89ee2` |
| `CONTRIBUTING.md` | 7269 | `4ff773098df606ced14331a25d7a510fc0b1c846b5d21a060951d710c60b2e20` |
| `Cargo.lock` | 111236 | `5ecec006f50454544c383d2d0ebf6edf40177de2fd626c6b038333f48a5a77ad` |
| `Cargo.toml` | 68 | `df9fcf76e7cf2a988581525a7b503f375ccebfc3e18d45420cecb969ef41bbd5` |
| `GLOSSARY.md` | 2544 | `a5dc2438c4c5f26738d91ec302359d9fc12a3381f8d787fab58ed5611715c573` |
| `LICENSE` | 1076 | `703d643410a1a3f9876a5d182e20086bb7bcb8313cd3cce1646d2226b5882b51` |
| `METTEN_DESIGN_DRAFT_2026-11-12.md` | 30466 | `d7c2a296448dcce80e611bfe5ba056c2ea413a40b50356eca42834ae1d3fcc1f` |
| `README.md` | 7354 | `05bd41a95d9ae3790fbd3371baaa41d2ce26727604747148bfd742861fc5a669` |
| `SECURITY.md` | 1564 | `556cf002c32595af5b962bb3f61b7dde5a0752f3d8a19cc85a40fd930db222b2` |
| `assets/README.md` | 2187 | `a40c2911b9ae6c17db796c1027de91e88fc927cf39217c292304bf24dd1d9ec4` |
| `assets/brand/README.md` | 1786 | `f9b7b9575a9e868b94b35a1e2c471806bd0d731daa35400cef102d36187d3301` |
| `assets/brand/brand-guide.pdf` | 490265 | `3195cf35aa3794d6e54057f04af8cade065dcccb519b91d51b3d4aed4c6c2864` |
| `assets/brand/exports/pdf/metten-a-lockup-chalk.pdf` | 3068 | `98e2c4e56748513e32ecd2d1c73498102f8e1680db8c67cebc7aeaa39cbd58c9` |
| `assets/brand/exports/pdf/metten-a-lockup-ink.pdf` | 3070 | `1092c109f9df7a81490a52d42352a1f1a1cde122124a3ba078278909dba313ad` |
| `assets/brand/exports/pdf/metten-a-lockup-primary.pdf` | 3084 | `8348d6f6ffee7a860fce4d6d4aed1038f1f0dfce553c59b22f4cd4b11fe3ba21` |
| `assets/brand/exports/pdf/metten-a-lockup-reversed.pdf` | 3081 | `9f64180e88bc5cfcb99f77b473ccc30f746acbb946e6704dbf4f4465616406e8` |
| `assets/brand/exports/pdf/metten-a-symbol-chalk.pdf` | 1489 | `49e6345be5e506f7e03511d138206cf78c52e12e5164970a94d1f2ff35d87801` |
| `assets/brand/exports/pdf/metten-a-symbol-ink.pdf` | 1492 | `9e6e9014267194476e1bd4dd671f6005e7b4c86fb0864394178e472749834f01` |
| `assets/brand/exports/pdf/metten-a-symbol-primary.pdf` | 1506 | `596d5d13298c08cb5028da174626a75260a7bc25dc8df5bb895b19f02887799c` |
| `assets/brand/exports/pdf/metten-a-symbol-reversed.pdf` | 1503 | `65a45988380c631ee16527da98c11d5ed4501ecbf97187aa459e5b9a6b792c95` |
| `assets/brand/exports/pdf/metten-a-wordmark-chalk.pdf` | 5288 | `42b5d9203a5e54ee8379fd22aee5fe8295cae5dd558122cb0b2f8c459fabef33` |
| `assets/brand/exports/pdf/metten-a-wordmark-ink.pdf` | 5295 | `ce383e3f17bcf28d756360f2118b1c6f4f1a3e796c6801ec5b10f161d4e10038` |
| `assets/brand/exports/pdf/metten-b-micro-chalk.pdf` | 1473 | `28911ce40d2613c4ed22ebcc07f0ddb86efdc5bfb7c58a7b27be99c44d42100c` |
| `assets/brand/exports/pdf/metten-b-micro-ink.pdf` | 1476 | `730e46d9b8edb115d860b420c56a9b5faf8ad9d6e47325f26f9f36048807f526` |
| `assets/brand/exports/pdf/metten-b-micro-primary.pdf` | 1490 | `acf1d62a4aa61e93735f96941e0f74212bbb4d047945b67d4c011b7262585955` |
| `assets/brand/exports/pdf/metten-b-micro-reversed.pdf` | 1488 | `0ed05156ac7cc63ed836c1754d983752b587b300e6049feefc6d7d531f594cd5` |
| `assets/brand/exports/pdf/metten-b-small-chalk.pdf` | 1486 | `29a4f34a7351ff3c4c84b4978c0a7708eec595e554761c37b2bdae5294542d5d` |
| `assets/brand/exports/pdf/metten-b-small-ink.pdf` | 1489 | `52293712c29abbdef83123acaf88d26487c91ba478489f9a650e22736d76a41f` |
| `assets/brand/exports/pdf/metten-b-small-primary.pdf` | 1503 | `f8546e969b0226760e7a5daf296340d562b112ec08c615538f6b5be1c436c157` |
| `assets/brand/exports/pdf/metten-b-small-reversed.pdf` | 1500 | `949bbecb886ba9c86554310d8f473d403d49a461ec24058b136d2fcd65623937` |
| `assets/brand/exports/png/metten-a-lockup-chalk-1200.png` | 19971 | `c047d474f45265cf68c0e4aac0f497b2b7f994987b58c1ddda7866cf53a542c0` |
| `assets/brand/exports/png/metten-a-lockup-chalk-2400.png` | 42635 | `4e38095e137a526f1f316a308109c91e90a793e5b98715f81bd600d79bc02c03` |
| `assets/brand/exports/png/metten-a-lockup-ink-1200.png` | 20203 | `e7ab8a0b147ccd237973ae692bb414f47406903c417652eb64c4f8ad7772dbfd` |
| `assets/brand/exports/png/metten-a-lockup-ink-2400.png` | 42917 | `b58d687949ee5c13010274b6d69a426b35b4ed51b0d0a925c43be5f47a6ac9b2` |
| `assets/brand/exports/png/metten-a-lockup-primary-1200.png` | 20197 | `e32c90fa036be752e58970c2fbe0844a3af86d885f74bb7b04dadcd5c73213f1` |
| `assets/brand/exports/png/metten-a-lockup-primary-2400.png` | 42908 | `0faa358af2b5f3bf4b914e482f6bad7c48eadfe379c78d1053f776ad2cb69857` |
| `assets/brand/exports/png/metten-a-lockup-reversed-1200.png` | 19982 | `2db4c1c2cb04d059fbb9ec2609f06044d13bd4b87fa9072e202664f5e669a764` |
| `assets/brand/exports/png/metten-a-lockup-reversed-2400.png` | 42638 | `50d2ea6463fbe79d89b56ee3adbaf66aa501124f5dc62a15308e1437e1b238ec` |
| `assets/brand/exports/png/metten-a-symbol-chalk-256.png` | 4917 | `7fb3d6e18f37dd380d03ddd20e2ddc670ebefeec38a0e27b945ed77429aab658` |
| `assets/brand/exports/png/metten-a-symbol-chalk-48.png` | 972 | `e007ab297539aa0a92df23bb0eda045112ea1dc03aef23401969f9a8691b485e` |
| `assets/brand/exports/png/metten-a-symbol-chalk-48@2x.png` | 1811 | `c34a83157f3c5305648dbebf46026b9b9130a58b6afb4f9f4f9b2462627436d0` |
| `assets/brand/exports/png/metten-a-symbol-chalk-512.png` | 10371 | `ca5951361b2cd57139f2a6758f0946739de7dc80300432b1026ce687801290ed` |
| `assets/brand/exports/png/metten-a-symbol-ink-256.png` | 4968 | `890df95314c707465e19fea6cfa107455eb8a1e23cc894a1294787eb81313583` |
| `assets/brand/exports/png/metten-a-symbol-ink-48.png` | 971 | `ff78e9baa0d48673d2ed62b4a1a48cc96fbaac5bc62d1d258764157b908891c5` |
| `assets/brand/exports/png/metten-a-symbol-ink-48@2x.png` | 1827 | `a15c8e0c7b39c596118352508fac7a264afa80d5e39733a591abfaa3e5744390` |
| `assets/brand/exports/png/metten-a-symbol-ink-512.png` | 10457 | `884d7c19f2d398c00d67b9f9b0206038d69087241c4c594c490c3aa7283b140e` |
| `assets/brand/exports/png/metten-a-symbol-primary-256.png` | 4942 | `4f317ef8061dad2cd65dfae42f64b072ad05abe3b7341ef296bb094f64393e94` |
| `assets/brand/exports/png/metten-a-symbol-primary-48.png` | 932 | `eef7e8116c0621961d7e75e8b9089dfb7a0df7c624d3bd13183bd9c9111961ba` |
| `assets/brand/exports/png/metten-a-symbol-primary-48@2x.png` | 1830 | `de045de16243a9483e089e00fc05b9853c9f8f24fd30493fe8fa37091fc352c7` |
| `assets/brand/exports/png/metten-a-symbol-primary-512.png` | 10450 | `e650f2f407c8cbd804a2ffd7f18034fefa14b8ca5626350a69fcef99feb0eacd` |
| `assets/brand/exports/png/metten-a-symbol-reversed-256.png` | 4915 | `814249599a4fa163f80dcd348b02d0076482d74912b4b3858f00a8e46904445c` |
| `assets/brand/exports/png/metten-a-symbol-reversed-48.png` | 944 | `9a3ef5bd0237a1a7d8e41bb8bca54edf81d3acaaa891866588dbdb67d523348f` |
| `assets/brand/exports/png/metten-a-symbol-reversed-48@2x.png` | 1808 | `94d40b4fb482a19614140762c80a784c10e11e6234c778e5479681df10691df3` |
| `assets/brand/exports/png/metten-a-symbol-reversed-512.png` | 10401 | `ceb1c5f4a247bd465abd5e16dd03176e04dffdd14ac20d72a13d1e2db377cb97` |
| `assets/brand/exports/png/metten-a-wordmark-chalk-1200.png` | 26906 | `7098f4f7785dc90dec3e94738ee70aac459c792218df89812197cb447fff7ee2` |
| `assets/brand/exports/png/metten-a-wordmark-chalk-2400.png` | 67714 | `d4721c4c3550c6d425828555a3bb8d9db968b1919b6ae64ead85adcb5408e620` |
| `assets/brand/exports/png/metten-a-wordmark-ink-1200.png` | 26582 | `6a9aba672767155938eb7b05ca25789fc286e6b9ecfe4f09ddc2cc8a6c259071` |
| `assets/brand/exports/png/metten-a-wordmark-ink-2400.png` | 66642 | `96e9d9454a52907832e61665ceef60b65321f20359c5b856f37714671b730c1e` |
| `assets/brand/exports/png/metten-b-micro-chalk-16.png` | 454 | `dd986a40c3e06d85dce232c66d1a6c744441c09e87a0f7bd5cd144e97ea4d242` |
| `assets/brand/exports/png/metten-b-micro-chalk-16@2x.png` | 750 | `546e638081299f65b25f4afa43a1760ac5f4d14b00c1de95b3655b798a1dd194` |
| `assets/brand/exports/png/metten-b-micro-chalk-256.png` | 5411 | `02aa61c9b7f73d72068ff62709591d2248af5f2f9484a855ff52164f9b6cebb9` |
| `assets/brand/exports/png/metten-b-micro-ink-16.png` | 458 | `9e92ccf79b1f51a6b6f955e8a73b1ead55540d804561874524a8fd65159bc84c` |
| `assets/brand/exports/png/metten-b-micro-ink-16@2x.png` | 763 | `c4a2f89de4a4d89048dc659153ceca3b2447914d432dddf9b3e9f6282951ad52` |
| `assets/brand/exports/png/metten-b-micro-ink-256.png` | 5457 | `872548e1ebc873dc12a5a7cf3ab5f1f1a96ba6ffeb4bccb915a3ffe8697a168e` |
| `assets/brand/exports/png/metten-b-micro-primary-16.png` | 458 | `6f41d0a1b6520070f704acfffefb3c01688499eece94a6a723d80469d2ed17fd` |
| `assets/brand/exports/png/metten-b-micro-primary-16@2x.png` | 758 | `fc771caa762f0026b30fa805a648a330735c081a3a9f4a2eefb4772112096cd4` |
| `assets/brand/exports/png/metten-b-micro-primary-256.png` | 5441 | `310840b6522327822e7820396e0b12a99723b7add280fa15e740b5e7140b4851` |
| `assets/brand/exports/png/metten-b-micro-reversed-16.png` | 453 | `433dbf0ded7c07d803112588638e8217fb6a1db6ab54caffdf15fe266c673722` |
| `assets/brand/exports/png/metten-b-micro-reversed-16@2x.png` | 749 | `818f00cab019b06300d9334d1ce901ed6e1f4abcdf53a081643bde2153147523` |
| `assets/brand/exports/png/metten-b-micro-reversed-256.png` | 5411 | `2cd744ac5f779db4d055355b0bbcfbe82c2e94be387162603970bcd1397e8467` |
| `assets/brand/exports/png/metten-b-small-chalk-24.png` | 562 | `090271547fba086f3f0eb78ad41687d71c15e8fcc62f7c9b4e8eb46e2457fcaf` |
| `assets/brand/exports/png/metten-b-small-chalk-24@2x.png` | 971 | `52f865cacce5e9209855eea13acdfe08004f72628af9d21cfff2b3e0c61b6b43` |
| `assets/brand/exports/png/metten-b-small-chalk-256.png` | 4910 | `9fa01b8038cfe7b9598357a0e8ff5e438ebb61301f2cac5a8d8837c0b2466496` |
| `assets/brand/exports/png/metten-b-small-chalk-32.png` | 696 | `2b9042746e77cb041b85957378ff20fa68b9fa1f0af1091be551d36a890c95dd` |
| `assets/brand/exports/png/metten-b-small-chalk-32@2x.png` | 1290 | `29918d26f301a27687c07950ff1d1b76a5b0958ff2160688ae42978e369fe182` |
| `assets/brand/exports/png/metten-b-small-ink-24.png` | 580 | `22577ecf0d9ad97c52a078de8660fe95a53d853808e9a318ed723afa2f2acaef` |
| `assets/brand/exports/png/metten-b-small-ink-24@2x.png` | 971 | `48eb55449b036ca6fc7050ed39e412e8894e79437f8a2f00abefd8ede796b3f0` |
| `assets/brand/exports/png/metten-b-small-ink-256.png` | 4976 | `aeb94b0321099ecf05e0f9e604bddb11401a03effdc2a381355667cfbd9e3dfb` |
| `assets/brand/exports/png/metten-b-small-ink-32.png` | 687 | `3abc0cc935040b4d9c1b45449972d663128fa545bbe0851654cdb51bd93e2bda` |
| `assets/brand/exports/png/metten-b-small-ink-32@2x.png` | 1307 | `8526ceeea85e96aa570a405ca78edecbf515003eaf2f3f50992039bed11b1d7d` |
| `assets/brand/exports/png/metten-b-small-primary-24.png` | 584 | `8c624959b30eeb6d6865e463475a5caf3baff8fc7cb6fc6ada4f3c71acb7a12d` |
| `assets/brand/exports/png/metten-b-small-primary-24@2x.png` | 944 | `f7915e2e4eaa47f08b112589654c3770feddefa6ef96f32413e0be406267a69f` |
| `assets/brand/exports/png/metten-b-small-primary-256.png` | 4949 | `6c412eeedda9e89504a504b2296b17409045f5a651ec8c9691b19f6f3652bb54` |
| `assets/brand/exports/png/metten-b-small-primary-32.png` | 684 | `208630278fa48b4604004d89ca5d5684fb37e90583a6149acec640971db429ca` |
| `assets/brand/exports/png/metten-b-small-primary-32@2x.png` | 1287 | `2d024e504bb26319a4003614b41696750626719a26cce0740a9fd9d82664c37b` |
| `assets/brand/exports/png/metten-b-small-reversed-24.png` | 575 | `1fb5fde96c5f7ebd52ed0ed13eedc0653155029c64911e7bacdc411df9c75ca0` |
| `assets/brand/exports/png/metten-b-small-reversed-24@2x.png` | 945 | `0ad3c966b6ef380f2ba80f83a925fb7316cb5e834d85c05efa83fa98d1bb5ca4` |
| `assets/brand/exports/png/metten-b-small-reversed-256.png` | 4914 | `83fc65459e68e44fabba6431c59c821f730e529a933cfaa250e616c2dce05220` |
| `assets/brand/exports/png/metten-b-small-reversed-32.png` | 686 | `8cac4a085101be59c7a92fc5cc1c0b6038d000475a3e9737ceec25e19f79d725` |
| `assets/brand/exports/png/metten-b-small-reversed-32@2x.png` | 1280 | `06a9de951e01e0b5d5444b950a31615e50baf549a11217aafd50ec9f770ae611` |
| `assets/brand/exports/svg/metten-a-lockup-chalk.svg` | 6203 | `272705b4f3ebf17b4c7a7b55086dbc44a17119f5c907be07c2189345e8e00fe6` |
| `assets/brand/exports/svg/metten-a-lockup-ink.svg` | 6201 | `309ee643093735b54b0c517fe416b7d28f02210a4e98b41f170672d68015f016` |
| `assets/brand/exports/svg/metten-a-lockup-primary.svg` | 6205 | `d383dc8412d4af92e7340e62632102b010dd95998a0eab7c2326195ad3f43ccc` |
| `assets/brand/exports/svg/metten-a-lockup-reversed.svg` | 6206 | `3f377fac6ea92e5f4ad5556223cf0f4297a90621e964afbff68331851d4ee412` |
| `assets/brand/exports/svg/metten-a-symbol-chalk.svg` | 1972 | `622a42b57437ab87c54424f7c9eaafb5009ef64e6d3a7f03e965ad6a64551553` |
| `assets/brand/exports/svg/metten-a-symbol-ink.svg` | 1970 | `a1925a1e4bbc560f9c8f97f3e70fe36b3a6f2f32ff89abbbadfdfaa2bf4b448e` |
| `assets/brand/exports/svg/metten-a-symbol-primary.svg` | 1974 | `ba6beb36fbd990af92122e1a30ea38c9986799345eefef05b841e39077637661` |
| `assets/brand/exports/svg/metten-a-symbol-reversed.svg` | 1975 | `768b5dddab34b897abc1b9720dbb9f0c0bc76a7e2776af6e335b22b0c933c084` |
| `assets/brand/exports/svg/metten-a-wordmark-chalk.svg` | 4743 | `64e69a413693fe2ba73f1fc6ac35bc83ebdaf5c0245a03def8b3c89dbf91a24d` |
| `assets/brand/exports/svg/metten-a-wordmark-ink.svg` | 4741 | `241eda58dbf649f364e8777ff31db353924d326e105af8dae1792547d245b161` |
| `assets/brand/exports/svg/metten-b-micro-chalk.svg` | 1850 | `d26a08c41f47c28c24e3efd475471f158daec6905d28d853c248e926b11dc2e8` |
| `assets/brand/exports/svg/metten-b-micro-ink.svg` | 1848 | `095195d7294ca314b02d19a2a8cabefedafc084b26808cbe8fecdb346c8497db` |
| `assets/brand/exports/svg/metten-b-micro-primary.svg` | 1852 | `087e88ab70b1ea70c0b93973be5b9c0991386f1b1f2d2b9186375f6295e83b74` |
| `assets/brand/exports/svg/metten-b-micro-reversed.svg` | 1853 | `f232ad1875dccb690c9d364d4a96a8bed7144ab3fa13e36bd57057d5fc0e47fd` |
| `assets/brand/exports/svg/metten-b-small-chalk.svg` | 1859 | `ec2ddfd275690cff89850c980c1895fe1993556299bacb0e405cd7ebd2f5ed9f` |
| `assets/brand/exports/svg/metten-b-small-ink.svg` | 1857 | `0b51f15cad637fbcdb4f92625d8627d479f217708998f989985125d059cf3bd0` |
| `assets/brand/exports/svg/metten-b-small-primary.svg` | 1861 | `fff2691a16005ae1f59b215db1bc4fd52dbecc61780751931bb1203f451e9fe3` |
| `assets/brand/exports/svg/metten-b-small-reversed.svg` | 1862 | `a961b90e3ba44d7b0fa9d62903d16a32997ca5c07c3557093f026ce0a4720d8b` |
| `assets/brand/licenses/Clash-Grotesk-FFL.txt` | 12734 | `145e7fe2429a3336ba215c070ef722000e01348a3e1baaa127e871bb5012f554` |
| `assets/brand/licenses/Inter-OFL.txt` | 4377 | `5b9321a4298cfeb6b34354164a1c3afc3db114569984c502b9b35d988fd58c57` |
| `assets/brand/licenses/font-source.json` | 460 | `d67faa931b20e9bcc8f4a3d31a1e572409279fa67af2740a195dd25609b07a9c` |
| `assets/brand/source/metten-master.svg` | 9295 | `ea966980e67433feb3e7bc5d0cc37d36928eb9e49eab176ea1c56950ed015428` |
| `assets/brand/specifications.json` | 2379 | `c2596b23dd1be73fe6d97e80ad7e3a6e41fc831b0e68e6f3b934d42625474c33` |
| `assets/icon.svg` | 302 | `293c80fc79c3e637b269ac1c617b9dfa2351b9b31aec35bd19094ce7adb7d28f` |
| `crates/README.md` | 2759 | `dbbcf1b35be9bb9d9fb85f2e3a994ed71c29050e874b801c6cc615dd2f85e429` |
| `crates/engine/Cargo.toml` | 280 | `c59d3a0f0e91032f6900adc7624c12f18bf5fdbd2fbb58c20b26c1656a80fee0` |
| `crates/engine/README.md` | 4084 | `34cbf3fc09d84b109a4dd8869c64198e8c01ad23fc506de136c130822a5ca4b8` |
| `crates/engine/src/README.md` | 8969 | `277ccad2d300319be82552bec68a0ae6fde50342ccfdc0194220f12cf98a5086` |
| `crates/engine/src/lib.rs` | 21085 | `ca5ddd6c0eb79aa91b8dad2c8e784c34c897bfaae11dfae8bb615c7278acaa77` |
| `crates/engine/src/storage.rs` | 13886 | `df2c0d838166f49b99a5cc3322269105a695836410df10f2d0da7db11eac94c6` |
| `crates/engine/src/strategy.rs` | 279458 | `330994592799c22bb12685dbe55a5c4ebc1b0ac3f5765f26222b168a6a760575` |
| `crates/engine/tests/session.rs` | 7120 | `f6684bb26ff5052b1bfee02bd9f3deed4c4606f7887d9732a25222b60b56188e` |
| `crates/engine/tests/strategy.rs` | 111841 | `7e4476d008eeb2e26d43cb3a50305a5a6a23e3b3636e0e79d3ff45cf58b7a540` |
| `crates/engine/tests/strategy_storage.rs` | 16115 | `a8b5a359ce1e3575facedc40f54e72f02bfa3956da951025ebc60a392f59eddb` |
| `docs/README.md` | 6231 | `b09c1303061701b90074d1de750242012d083494f2e9252c2a778fe30f4f7c9e` |
| `docs/adr/0001-rust-engine-and-tauri-boundaries.md` | 3180 | `3fdd4120eb8a7968eec12d4ae446f71f16791dd93b6d93270e275b494969a48b` |
| `docs/adr/0002-mit-license.md` | 464 | `cedac407570d5634cf2ef8ed57c336084646e419a22705a44bb55a1595774775` |
| `docs/agents/code-map.md` | 9527 | `0ef722f86b912176690ee0e7e21e41a9ae30184676dd9d8d30b1ac7af61987d2` |
| `docs/agents/domain.md` | 632 | `72470a51613f365756b6f8136a841791ac07698e69c0c4880c3706dad9d8c88c` |
| `docs/agents/issue-tracker.md` | 10907 | `0620a07e5473182344abd4a1470dc31af0cb636bc793db1f4b7a15c5bedb3242` |
| `docs/agents/maintainability.md` | 9526 | `1a5cb0740c33b88a9692f5bdcb175cfe5b122559f644acbcc1c1d07b3162c280` |
| `docs/agents/triage-labels.md` | 574 | `f55b05b3eb208fe287cee84509056b28f169100cbeedc512781c2423c3dffda6` |
| `docs/agents/verification.md` | 11260 | `7d7c56a6b70da15b50d06ac60f03f9d623522012ce4a34eee0111a422531d2ee` |
| `docs/egm/2026-11-13_DRIFT_REPORT.md` | 6477 | `787acd1d4ddb02008319fa1998d9a77301381d4429023784783e30f00a2e110a` |
| `docs/egm/2026-11-14_DRIFT_REPORT.md` | 80586 | `a48609966b60dccc4bed57bd8a68fc52a989bf21ce8f8b0bcd4325c994beeaa3` |
| `docs/egm/2026-11-15_DRIFT_REPORT.md` | 18795 | `e6125f1f7b0224a2b68315373cdc8b6211196a071d2c587ef0397ed27143a341` |
| `docs/egm/2026-11-16_DRIFT_REPORT.md` | 19282 | `6cdc40da173f6dd366d783b58f773564ee64ebe19247f85a1465beb5c043cca2` |
| `docs/egm/2026-11-17_DRIFT_REPORT.md` | 24333 | `ffd6e59d05cbc961563020ebbf3ec1df0e4be5effdbefd26c79dbe6ed1b1f58b` |
| `docs/egm/2026-11-19_DRIFT_REPORT.md` | 20846 | `6db1ee5a9ab2b46a58445d42ea9e534567df570d3376ab7c0a28cc278a27817f` |
| `docs/egm/2026-11-20_DRIFT_REPORT.md` | 29505 | `97b73bd4566a7c27817139cd1046632167136e2b9ff43c5db09d34dd79654d85` |
| `docs/egm/README.md` | 13314 | `9bc5b1563a003eaa5f13848e50ff0b79864d2be20b57bd16ecd1ced11360c6d5` |
| `docs/implementation-plan.md` | 6050 | `d5a0576f241c6ce742b026ebbfd52ef0f545a2a9023390a8375df46297960d01` |
| `docs/proposals/issue-4-council-record.txt` | 12433 | `e62e1cd75173d278883918e725a7621b52bb9ce6fbb7531ac55bbe124514a295` |
| `docs/proposals/issue-4-egm-handoff.md` | 32287 | `887d6d694506d01f6b2e8ccb170ebac699c7bbad175d8f5e6317e53750626064` |
| `docs/proposals/issue-4-egm/brainstorm_review_report.md` | 12226 | `e16cebb28ab4633ed710eab116259f65dd82436e452d4992eebbda75e8aa9a5b` |
| `docs/proposals/issue-4-egm/comprehension_report.md` | 2539 | `2145449efa05785d75656a1af765a3c35c69e1f52567c9cf901d6d4d10b3a0d0` |
| `docs/proposals/issue-4-egm/concepts_brief.md` | 13118 | `5151fcf1579fcb4f6811ac71d6319ecf99131b4e89c25b1423eb862420773806` |
| `docs/proposals/issue-4-egm/consolidated_review_report.md` | 17852 | `15d496d2f4fc02e39d12ad5177c2743e2ff6d6fab534bb73f7634f1cc5002d9f` |
| `docs/proposals/issue-4-egm/critic_report.md` | 2208 | `375da79f163ce10db5a866e1d69e7b57040980383bfd93670cc9482cec2e7e5e` |
| `docs/proposals/issue-4-egm/design_draft.md` | 142598 | `4f39df3e732b27d79a43bc3dc95048cf5368337634c15bcd71e454bb149dd8e5` |
| `docs/proposals/issue-4-egm/detailed-implementation-adversarial-review.md` | 19272 | `6e13ae2a3df20c163fb6a1c0dc9b074f54807914d05094fda9bda6f906b4e199` |
| `docs/proposals/issue-4-egm/focused_recheck_report.md` | 15980 | `3fcbbd3d1ec5e750edbfd0adec958132ca6c960fab518ae457206bce4f9f6356` |
| `docs/proposals/issue-4-egm/focused_resolution_report.md` | 5072 | `f0d8471e3cfc94ca428b291806689974074c5316f7ed47c55e86750d57afee08` |
| `docs/proposals/issue-4-egm/goldfish_review_report.md` | 44356 | `5c59625a274905f50e5b7d9042a55e02ab6182bc91eae38896d96f6e1f5762f3` |
| `docs/proposals/issue-4-egm/implementation_readiness_report.md` | 2401 | `068d82861144a1f06599a52bc0b5a2664625d7d5592f5775f7d3a2db7707a2a8` |
| `docs/proposals/issue-4-egm/intermediate_zoom_out_report.md` | 5387 | `6d47dc5babd85b52e2d4f5cebe7a42337daebad42fc21564f902a4c7752bc0b1` |
| `docs/proposals/issue-4-egm/review-sources/kalshi-15-minute-guide.md` | 11739 | `969ff1c845d12e40e8dbe85bf8e45fa8415d29daa95ead2b7123322fa7ebd0e7` |
| `docs/proposals/issue-4-egm/review-sources/kalshi-15-minute-markets.md` | 2528 | `1f52b932894c9dd768e909f70161fdacd16875e583ede868bcb475adb9b76d55` |
| `docs/proposals/issue-4-egm/review_setup.md` | 4151 | `d7fa9c77e3ff243276e96f497b15a05c16a35dd5c4455e27efb85f1c6d482bc6` |
| `docs/proposals/issue-4-egm/reviews/codebase_details.md` | 6322 | `f9feed91150134806e5c2920c70308f6acb38bb8acb90b9b85882c6e1f704773` |
| `docs/proposals/issue-4-egm/reviews/comprehension.md` | 7016 | `698b06fc7702a51101fe29f7d3ccc7f64dd31e67c3cacdc460f31d9de484d49d` |
| `docs/proposals/issue-4-egm/reviews/design.md` | 6772 | `f9c8776344072da9c106e9f6ecf63d7d6bedb85d201a559736c65e2cc11a71a1` |
| `docs/proposals/issue-4-egm/reviews/focused-recheck-codebase.md` | 4950 | `aa9ec73f4d38d7c5ed81ba6347fe3292e273085f1aa716a2ac0ac15fdc88bed9` |
| `docs/proposals/issue-4-egm/reviews/focused-recheck-comprehension.md` | 3100 | `1a0f82b449d69aa0d8f7c13b36f24e588916e85d0c153852d226e31b2fa44ae0` |
| `docs/proposals/issue-4-egm/reviews/focused-recheck-lead-clarification.md` | 2615 | `e661bd79c7746c6ee75daff04633e3b0f992f2263b3108775781d3606761e52f` |
| `docs/proposals/issue-4-egm/reviews/focused-recheck-readiness.md` | 3629 | `dee285a515d1ff425688ed561d1933f7d1dc6d41540942012462577eb59786b3` |
| `docs/proposals/issue-4-egm/reviews/focused-recheck-style.md` | 2388 | `573873eb418d6fb549259494c26ba66d18095738e54429c810ea48089e02ea43` |
| `docs/proposals/issue-4-egm/reviews/implementation_readiness.md` | 6746 | `2f8adff4d0af5d593625c3d6052dab1b43ae7fc72d5a6ac1e3b3289e929558c4` |
| `docs/proposals/issue-4-egm/reviews/inventive_catalyst.md` | 6153 | `8a6a11b95ed8ceef58889d3faa9b002171addb483f82fc468a2e7f487df1dbe6` |
| `docs/proposals/issue-4-egm/reviews/stage-5-inventive.md` | 1970 | `77f66f12be0f3943173e8d9a6d8a3b4a1678d0d035d99c4dd83bd077ce7cdbb7` |
| `docs/proposals/issue-4-egm/reviews/stage-5-zoom.md` | 4551 | `10652609664d823ad8f93307d7e4d15b7a795d677583d5854573593dd0ef242d` |
| `docs/proposals/issue-4-egm/reviews/stage-7-design.md` | 3360 | `4c453004b2d6aab23f60264623f38f428cf7a2beb9f6c5839098dadea435846a` |
| `docs/proposals/issue-4-egm/reviews/stage-7-structure.md` | 1533 | `1dde4ba85006b8a8d14a28c83a6ff3442b5c2edad13494b753820e76b6aa6ed4` |
| `docs/proposals/issue-4-egm/reviews/stage-7-style.md` | 3204 | `8c8fb4ba1d54cc0d657b56c248b1a7261c8df915126bf57457d47109e70f2810` |
| `docs/proposals/issue-4-egm/reviews/stage-9-codebase.md` | 5468 | `46b99f85ed7099bdeba4077bb8f2507f6bf9196c05119bc3e036d828b49b3db4` |
| `docs/proposals/issue-4-egm/reviews/stage-9-readiness.md` | 4110 | `fa6d9678cdf7af39aa16cf67cd66d19eeb7d74601b773e714489b15af0505a3e` |
| `docs/proposals/issue-4-egm/reviews/stage-9-style.md` | 2476 | `c2cd2cbfc7d0ca06d85ba26b8ffcda147e3f582f840b7104c2ec3e34eeedb990` |
| `docs/proposals/issue-4-egm/reviews/structural.md` | 5954 | `d4a03a90cf496f2207df8be7df6c2e4a6d5522f98f00c4cd2b2e07674e6f6641` |
| `docs/proposals/issue-4-egm/reviews/zoom_out.md` | 6424 | `d4c1934bb10030235017e69b45e43ae82030e103668dfe1e9df1f935b6e8d268` |
| `docs/proposals/issue-4-egm/strategy-rules-design.md` | 142782 | `9b5f1d2738fad3f77c99d70ea3cfe6e7436b370e07a5056daa08dc23f3fc872d` |
| `docs/proposals/issue-4-egm/strategy-rules-design.reviewed-2026-11-13.md` | 121913 | `3f6f8c23492a41131f56cb81e899860da5565a28519f61ac79ffbe081731c853` |
| `docs/proposals/issue-4-egm/working_hypothesis.md` | 7596 | `0376aae0a80ceb7680ef5b2cb623dfd124ff817c874e7109ed61ed5f0d6fcd5f` |
| `docs/proposals/issue-4-egm/zoom_out_report.md` | 1345 | `6d9ebd66c027d9f70ca886bf19047102b32097b5b434dada09bf890ec16e7d6a` |
| `docs/proposals/issue-4-strategy-contract.txt` | 34027 | `0063d5fc904b3a4ceb3bf1fa8c77b3400ea4b538fae242d46bf73cd5e18c757b` |
| `docs/strategies/README.md` | 1129 | `4d6e69ebb7862e3662f261eb2086391692b0233b8127c3f5c1cb113b5011c472` |
| `docs/strategies/agent-authoring.md` | 9174 | `afbf20545d8a6a240bba440e504bfdeb7ffc2ecdd0fbd524cae9da485ad3be32` |
| `docs/strategies/authoring.md` | 8068 | `50554902721dc1b3a1e52141eedb007321875f15dfe7fa93dcceb758ec028d02` |
| `docs/strategies/reference.md` | 39954 | `54d67c1583ee091355ef9154d2af4ec0b5dba6e277e6a4b85b04d466bd5bbcf4` |
| `docs/verification/issue-2.md` | 16028 | `e07cb5e4610fdde6a51e7ac37c4b94860511d1bdee60ae23dcc02b2857ad6c7a` |
| `docs/verification/issue-28.md` | 4012 | `3cc75ed9117c0f06f227de3f45adae1a5adba0f50748270b6c6dbd5268767a29` |
| `docs/verification/issue-29.md` | 8814 | `da4686bbd7a8d836d10fccffd68e9d3ea3c1def08207627783bd958c70a79d30` |
| `docs/verification/issue-3.md` | 23365 | `f2a8c49ac381d9e3f92f47ed76649c9c2882fa9bd5df0aa82f46e5a34c2c60fa` |
| `docs/verification/issue-30.md` | 47528 | `b356fc44c0d28f97597ab9c5a183df427d9590936eda1037d6c04bef96c1901b` |
| `docs/verification/issue-31.md` | 23686 | `18f1c11f1d0090cedaf36ac9cd871abf4125fae7b8f79138caaf9df5b1e71940` |
| `docs/verification/issue-4.md` | 247893 | `590411f4e8e0ba5b7f8d0207e3357f44928752f258463b3d617c39f107d7861d` |
| `docs/verification/pr-33.md` | 4758 | `0127fb27d8e6da259a0f5af60044393e716cb43464aca9ba2ccbd6fa363efeee` |
| `docs/verification/pr-34.md` | 5675 | `54e7ca4039c192dc6b66a2fd725294a1b4ee4da347b2464d01ed973286e16c68` |
| `examples/learning.json` | 1609 | `a79bd747078601cd16bc292c7896cdcb9f2fe3db0feebf412284a50a2bd50aa5` |
| `index.html` | 3002 | `aa36a41bf6dfd06537d60e2572ea54e39b2b452484b13ca2e021fe0bc8de29e1` |
| `package-lock.json` | 48808 | `af893d318470044a133dd39e0581df2f9b345d789e28604c1c74e55e4f9fa021` |
| `package.json` | 435 | `143e996b97233817024d21a786284d83e7a41ce85732c72c71fdac43dbfa6440` |
| `rust-toolchain.toml` | 87 | `6b1cf8341d9a08d76c6deb1e9df90a0b208d0ef0d00ee05e0173e9cd4f0f425d` |
| `scripts/README.md` | 4533 | `55a8bdf95f1f5cffccb41ab2fb43914dcbdfe7d5a880ee9ea21eb866977900b5` |
| `scripts/check-staged.py` | 1869 | `4ac2e3bbff160d425d72f08b77cdec75cc4899be4a184c1945191cd33a5c9c10` |
| `scripts/verify-credential.swift` | 34083 | `e98d408323d4a24dcbd0fced8e56792751087d1598c37973f635a2318c61ffcb` |
| `src-tauri/Cargo.toml` | 518 | `26e377a97f222315d9eeb983d50deda3a94a681dce9c5e424fd333636ec3e3e3` |
| `src-tauri/README.md` | 6888 | `8b5514756372ef41f852bc1126b92de5ec62696ace926392aa966235a69f2bb8` |
| `src-tauri/build.rs` | 287 | `84c13e583b5ffdb712037c573abed66fd1e0a6e9ef9def925b3e646caa931159` |
| `src-tauri/capabilities/main.json` | 241 | `2fe654ede98c1490e109f06f64f85b6ea1113805225abb305f55542add86f84a` |
| `src-tauri/icons/icon.icns` | 40745 | `68d930a483a179d4468328d5995ff2f999f86e5422b7fae60db14e0a4c0c3dab` |
| `src-tauri/icons/icon.png` | 6554 | `c002af38a269a8a02945e4fd3b41da748a14f984598d99de1054e185dea15727` |
| `src-tauri/permissions/autogenerated/control.toml` | 342 | `6062bd007690db2c4e8c8b0b1c6f2eac00a938372beedf370b0367cae43f4608` |
| `src-tauri/permissions/autogenerated/credential.toml` | 360 | `1a7761451cd0c60b73ab6ab3e3fd08a93b9056705d72811ac384033c58ebfe7a` |
| `src-tauri/permissions/autogenerated/snapshot.toml` | 348 | `8319753d7e129f2f39c2930e3fda68336d9cb6464d2d3a543ab1051b473de900` |
| `src-tauri/permissions/autogenerated/strategies.toml` | 360 | `331c31324427ab1fbbafa749c98a3ee8b40da2d76dba86c5e1da18751ec6f5e8` |
| `src-tauri/src/README.md` | 9562 | `084c22506f5ac45bc2d41650727e1706098b32b5cc461b0c403161146b239f8d` |
| `src-tauri/src/credential.rs` | 22242 | `c243bacc16ce591c1cde8bbd2f7bb21a7d35a945a5b2ffb653d4269e3d742c7f` |
| `src-tauri/src/main.rs` | 16204 | `a0221b50117e83c2f6f010ae16c23416990aa0bbcfece505083918661fd1ff50` |
| `src-tauri/tauri.conf.json` | 1015 | `24c3f2e971a8d3806b0b8c8db8db5963a16dd73880e961a5f62ecac07597fcb8` |
| `src/README.md` | 6064 | `2a2cee08e55eb4380c86fea5c67977c7d27da4abfa95c4017e54a89a372e894d` |
| `src/contract.ts` | 1784 | `ed2bd38d9bfef0c2639a1fd79beba7d10c8e1e2ef92350852c8c93fb70364505` |
| `src/main.ts` | 11727 | `836f1580cdefa94d47cb1612cf1be682004963c9b0a543b6f9ed6492a9eee934` |
| `src/style.css` | 1523 | `d2f4166630f48d73d92e74dc0c95cd46eea85835ed6782966ed43c14164a0194` |
| `src/view.d.mts` | 284 | `b6df689ef18496e752a828632c39cae153f91064b8d5183f518d590c75e3d2f2` |
| `src/view.mjs` | 1482 | `b1b8c31f21e65243dfc065b2a3855bc5f51c25c76e1a27c5cbefc943b83f83a0` |
| `src/vite-env.d.ts` | 38 | `65996936fbb042915f7b74a200fcdde7e410f32a669b1ab9597cfaa4b0faddb5` |
| `tests/README.md` | 5082 | `0f002691ec7be46ba6326fa66763fbaecfaa44a5ad87df2a1bd38d1401174da8` |
| `tests/check-staged.test.py` | 3655 | `9245206b3dcdbe070376a3ca89a62044eb80e6f20afc56d73ec79f957a0eb0c2` |
| `tests/credential-runner.test.py` | 2523 | `1637493cd66dd0e660a417cd3ee9925aa7be7b5898e30a90de3f903e106a6c75` |
| `tests/credential.test.mjs` | 1738 | `fc79e979217ecadbca34f408216ba114b05baecaf84504ab6d70e95785aae844` |
| `tests/refresh.test.mjs` | 4550 | `2f286d136964d5a2b9eba9d61c33e1e334cef7e2c03c451f735febea8232336a` |
| `tests/strategy.test.mjs` | 7290 | `c27844dca82fae4f1d94cb0caefe24ddb56f19515e97b3d6c2e8516646ac6994` |
| `tests/view.test.mjs` | 883 | `171a5117e3faa9a4042770e194fe1759e7636942c9ae4954bdf5126c2527ed89` |
| `tsconfig.json` | 211 | `8cb859be4b79e6c1942e6bb2acfa038c4f2af578a18c80d81fccb5d9220008a5` |



## Post-write preservation — 2026-11-21T09:04:58.978361-06:00

All 241 pre-existing maintained paths retain their exact starting byte counts and SHA-256 identities. The sole added path is this report (242 total). HEAD remains 868609baffc25f06ecb43d8760e7cdd60f684a68. Delivery remote refresh at 09:04:48 CST again returned main at 8bb7aac3704c9b197e58e10ed9256483cbf9f2e8; published-tree enumeration confirms only root AGENTS.md. No affected guidance changed, so loaded immutable guidance and applicable evidence were reused.

Before this append: 46621 bytes, SHA-256 `be0161e9f43a488bec949afda4cbac02e7606148927d996d7eb1245493672df1`. The append preserves the exact previous byte prefix and verifies that fact afterward.


## Corrected audit disposition and publication scope — 2026-11-21T15:46:48.434882-06:00

**Coverage limitation: this was an audit of the older selected checkout, not a complete audit of current main. Do not use the original no-drift disposition as a clean full-repository result for main.** The original observations, manifest and per-document sections above are retained as historical statements about HEAD 868609b and its uncommitted documents.

The earlier summary overstated completion: source reads in this pass included truncated outputs, and the large evaluator/test files and some reference material were covered through prior inspection rather than complete fresh reads. Exact identities support bounded evidence reuse; they do not satisfy a promise of complete fresh source inspection. The unqualified per-document “No drift detected” statements above must be read within those limits. No complete clean audit is claimed. A complete current-main audit remains outstanding.

Current main at publication preparation is 8bb7aac3704c9b197e58e10ed9256483cbf9f2e8. It includes source/provider and controlled paper additions absent from the selected checkout. Those additions were not audited by the original pass. Its EGM map still excludes issue #4 from daily runs and enumerates older engine/native boundaries; docs/README.md still points to that exclusion. This is current documentation drift against Connor's 20 November authorization and the maintained inventory, even though the older local copies had already been corrected.

Recommended correction, implemented in the publication branch: publish the full daily scope, inventory-driven ownership routes for all maintained engine/native/UI/tooling paths, resolved-finding navigation and the corrected report. Preserve main's newer lint, evidence, presentation and paper documentation. This documentation correction does not validate the new source/provider/paper implementation. No schedule change is needed or performed.

The owner subsequently authorized a new branch, PR and merge. Publication uses a separate worktree based on that freshly resolved main; it does not reset, switch, stage or commit the original working copy or import its unrelated nine-file modifications. The 20 November report is included only because this report links its resolution appendix. Earlier untracked daily reports remain local.

Original report before this correction: 47347 bytes; SHA-256 `80e18904b9bdc6e24806b329d84ed7d7ebfb3afc72ff8c3a4d69e8fdb62e02f2`. Appending preserves and verifies the exact previous prefix. No fresh application tests, builds, GUI, credentials or trading operations are implied.
