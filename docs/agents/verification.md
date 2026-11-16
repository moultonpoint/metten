# Evidence and checkpoint verification

## Published-instruction discovery

The lead establishes this baseline at a new substantive task/session or handoff, including planning and reviews in older worktrees. Reviewers use the pinned baseline within a round. The lead checks again at delivery or a known relevant update, then repeats only affected evidence. A file update alone does not refresh a running session.

1. **Resolve published guidance.** Record selected code `HEAD` and relevant uncommitted scope. Identify the authoritative remote/default branch from repository metadata, then query the remote now, for example with `git ls-remote --symref <remote> HEAD`. Record the returned branch, immutable commit and query time. A cached `origin/main` is insufficient. Surface genuine remote ambiguity rather than guessing.
2. **Read the pinned sources.** Read root `AGENTS.md`, this procedure and applicable linked standards at that commit, using `git show <commit>:<path>` or an authenticated hosting API. Resolve same-repository relative and branch-form links at the same commit. Independently inspect applicable nested instructions in the published tree, and read local instructions/contracts and approved task documents. Record external required guidance separately with its available version/date. Read the applicable sources, not every repository document.
3. **Preserve the selected work.** If objects are missing, fetch objects without updating existing refs, index or working files. With Git supporting these options, use `git fetch --no-tags --no-write-fetch-head --refmap= <remote> <commit>`. If the host refuses SHA retrieval, use the same options with the fully qualified default-branch ref as the source and no destination ref; then verify the originally queried commit is available. A remote rewrite/race may require a fresh query and new baseline. Preserve existing tracking refs and unfinished files; leave checkout, reset, stash and rebase outside this operation. Added objects are permitted; this is read-only with respect to selected source/state, not the entire Git object store.
4. **Reconcile and record.** In the existing task work record (plan, issue or PR), name one authoritative baseline; other summaries link to it. Include code revision/uncommitted scope, remote/default branch and guidance commit/query time, paths actually read, missing/unread required paths with reasons, and differences/decisions. Follow instruction hierarchy and exact owner-approved exceptions: consistent local requirements may add detail; a branch file cannot grant itself new authority. Reconcile routine differences; leave unresolved consequential conflicts or unavailable required guidance explicitly unverified. Bounded investigation can continue, but dependent readiness/compliance claims cannot. Published process guidance does not silently amend product behavior or accepted task scope.
5. **Review and deliver.** Each reviewer verifies the root entry exists in the pinned tree, traverses applicable links independently, compares the discovered set with the brief, and names consulted/missing documents. The lead's list is navigation, not the entire requirement set. Where direct access is unavailable, reviewers may traverse the same complete, immutable source bundle within approved sharing limits; distinguish supplied sources from direct retrieval and record coverage gaps. Missing required content prevents a full compliance claim. The lead checks changed guidance before delivery and records evidence reuse or affected re-review. A material change alters an applicable required-document set, owner/contract, behavior/validation requirement, safety/authority boundary or required check; unrelated/wording changes need no repeated review. An unavailable final query leaves currentness unverified. Scores cannot replace this evidence.

Handoffs carry the selected code and guidance revisions, relevant unfinished work, missing evidence and this refresh requirement in the existing RISEN instructions. Keep one baseline per task unit, rather than re-querying independently for each reviewer or message.

### Bootstrap delivery and activation

The minimal remote-query/published-entry rule must also live in a verified shared standing-instruction source outside versioned worktrees; a pointer only in this repository cannot reach an older branch. Verify the supported host's actual source and delivery behavior. If no shared source is available, supply the exact insertion text, mark prevention **blocked/not installed**, and name the next actor/action.

Record installation, repository publication and session activation separately. Each explicitly targeted new session must report the loaded entry path/guidance commit and discovery result. Existing sessions need an authorized explicit refresh via a supported channel or restart/rebrief, with the same observable result; record each targeted session as refreshed, restarted, or unprotected. Do not infer delivery from a file write or claim coverage of unrelated/unknown sessions. A global override, another profile/host or instruction truncation can prevent delivery. Required delivery checks that cannot run remain blocked, and prevention incomplete for those targets. Installation and retrieval do not prove obedience; use task evidence and the [existing adoption trial](maintainability.md#adoption-and-existing-code-review).

### Verify instruction changes

Use a disposable local Git repository and bare remote to reproduce the failure: select an old commit in a worktree, publish a new entry link/standard only on the default branch, leave the local tracking ref stale, and add an uncommitted sentinel. Run the documented discovery commands, verify the published commit/document, and assert selected `HEAD`, index, tracking refs and working-file bytes are unchanged. Show that local-only discovery misses the new standard. Exercise object retrieval with both SHA and branch-ref forms.

Reuse the fixture for a current checkout, failed remote query, missing linked required document, and a publication between baseline and delivery. Walk through conflicting local/approved instructions, a reviewer brief omitting a required link, a mismatched review baseline, and the no-shared-source outcome: each must produce the documented decision or explicit missing-evidence/blocked status. Verify actual new-session delivery and existing-session refresh separately in the supported host. Record commands/inputs, expected and observed outcomes, source version and remaining limits in the existing work record. A fixture proves retrieval/state preservation; a walkthrough checks decision rules; neither proves host delivery or agent obedience. Check affected links and instruction consistency; application builds are unnecessary for an instruction-only change.

## Find current evidence

Start with the current-state index in the issue's `docs/verification/` record, then read the linked acceptance and proof sections. Dated failures and continuation prompts preserve history; they do not override later accepted evidence. Issue #4's [index](../verification/issue-4.md#current-evidence-index) separates those records.

For large documents, locate headings with `rg -n '^#'` and read explicit line ranges. Size tool output to include each requested range completely. If output truncates, retrieve the missing range before claiming the source was read. Summarize findings with source locations instead of repeatedly printing whole reports.

When delegation is requested or otherwise authorized, give each agent a bounded goal, owned functions/files, required source pointers, checks and stopping condition. Require implemented corrections and compact evidence. The lead integrates results and verifies the final source; agents coordinate shared-file edits before making them.

## Verify a mixed-work checkpoint

When a checkpoint combines inherited changes, untracked files or partial staging:

1. Stage the intended files explicitly and inspect `git diff --cached --stat` and the staged diff.
2. Run the affected existing checks against the staged snapshot. For example:

   ```sh
   python3 scripts/check-staged.py /Users/connormoulton/.cargo/bin/cargo test -p metten-engine --locked
   ```

   The helper checks staged whitespace, exports the immutable staged tree through a temporary index, and runs the supplied command in that disposable folder. It preserves the real index and working files and removes the temporary folder on success or failure. The export contains no Git metadata, installed dependencies or build cache; choose checks that work on source alone, or use an explicit shell command to install required locked dependencies first. It does not commit or impose a new approval gate.
3. Record the printed tree identity and actual check outcome. Commit only when the required affected checks succeed; if staging changes afterward, verify the changed evidence again.

Use this for mixed-work checkpoints rather than every documentation edit. A normal check of the original working folder cannot establish that unstaged dependencies were included.

## Handle check failures

Run dependent checks and mutations in separate tool calls or with fail-fast execution. A later successful command does not clear an earlier failure. The staged helper exits nonzero on a whitespace or supplied-command failure and does not execute the check after failed whitespace validation.

Markdown hard breaks and verbatim source captures can intentionally contain whitespace. Preserve established archived records. If importing a new archival record requires an exception, identify the exact files and whitespace in the evidence, run an explicitly scoped check with that exception, and report the exception as preserved formatting. The strict helper will continue to reject those staged warnings; do not describe that run as passing or silently disable checks for code or other files. Resolve the archival import separately before checking a mixed code checkpoint.

## Review responsibilities

Reviewers check whether claimed behavior and resource bounds cover the actual changed paths and final source. Passing sample workloads do not replace a general argument. Changes to the pinned Rust toolchain require reviewing the standard-library layout and movement assumptions in the processing and memory proofs. Keep these judgement calls here; mechanical failures belong in executable checks.

## Maintainability review

Apply the [maintainability standard](maintainability.md) to each programming change. In the existing issue/PR summary, identify the expected change path before editing, then explain departures from it, any added structure and current need, the meaningful check and remaining limits. Update the owning contract/map only where affected. For a small change, a few sentences suffice.

The reviewer checks those claims against the changed code and callers: ordinary changes remain local, state has a clear owner, and simplification preserves failure behavior and safety. Request a correction for a concrete violation; record unrelated improvement opportunities separately. Documentation edits need affected-link and consistency checks, not application execution solely to satisfy this review.
