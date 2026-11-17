# Issue tracker: GitHub

Repository: `moultonpoint/metten`. Issue bodies own acceptance; the [product specification](../../METTEN_DESIGN_DRAFT_2026-11-12.md) and accepted [ADRs](../adr/) own behavior. The [implementation plan](../implementation-plan.md) links the product sequence. Projects display these same issues, never a separate task ledger. Requests belong in issues, not PRs; when a skill asks to publish a ticket, create an issue.

## Work recipe

1. **Read:** use `gh` to read the full issue body, comments, labels, native blockers and parent/children. Follow current evidence links. Use explicit repository arguments outside this checkout and UTF-8 `--body-file` files for multiline writes. Done when the outcome, evidence and next action are understood.
2. **Classify:** apply the metadata and triage rules below. Human intake starts with `needs-triage`; skills may emit their defined role directly. Done when category, priority and one current triage label are explicit, or a missing priority is visibly untriaged.
3. **Check start:** implementation requires a prepared brief, satisfied blockers and authority for the actual action. If body/native blockers disagree, respect every unresolved blocker and reconcile both before starting. Publish prerequisites first; add native dependency links using numeric issue database IDs, plus body links. Report native-access limits. Prepared but blocked work stays `ready-for-agent` in Backlog. A requested human decision/action on `ready-for-human` may proceed within existing authority while dependent implementation waits.
4. **Work:** the lead assigns, schedules and prioritizes; agents maintain routine tags, stage and evidence. Keep one bounded implementation issue per session. For selected work, use one accountable human assignee, even when a differently authorized executor performs it. Add a persistent session link once in the assignment comment when available; the owner communicates through the lead. Record outcome, allowed actions, exclusions, checks and stopping condition. Move authorized active work to In progress. On a halt, record reason, next actor/action, move to Review / waiting and reapply triage.
5. **Verify:** check every acceptance criterion against applicable evidence. Source checks cite tested source; default-branch acceptance cites actual merged source/run; settings checks cite settings, time and candidate. Reuse unchanged evidence with explicit limits. Ordinary test execution is work, not a request for missing information. Use `Refs #N` in PRs while post-merge/settings acceptance remains; do not use `Closes`/`Fixes` to close early. Submitted work or outstanding verification belongs in Review / waiting.
6. **Close:** the assigned owner/lead updates the acceptance record and leaves one concise completion comment linking PR/commit/run or observed results and remaining limits. Remove the current triage label (any except `wontfix`) and close completed only when all acceptance is evidenced within existing authority. Set Done, preserving GitHub’s close reason. Apply the closure branches below when relevant.

Closure branches:

- **Not planned:** retain `wontfix` and a reason; lack of time stays Backlog.
- **Reopen:** re-evaluate current facts, priority, triage and stage rather than restoring stale readiness.
- **Child outcome:** add a one-line outcome/link to the parent. Not-planned/superseded children leave required parent outcomes unmet; keep the parent open until its own requirements and required child outcomes are evidenced or the owner explicitly authorizes consequential scope change. Never move unmet acceptance to a follow-up merely to close the original.

No issue, label or assignment authorizes credentials, accounts, paid services, deployment or real-money validation. Obtain exact owner authority for those actions. Follow [SECURITY.md](../../SECURITY.md) for undisclosed vulnerabilities; keep confidential details out of issues/Projects. Public-advisory follow-ups may cite public-safe evidence.

## Metadata and triage

Use one primary category label: `product`, `security`, `maintenance` or `documentation`. Retain useful secondary labels such as `dependencies` and `github_actions`. Native Feature/Bug/Task types are supported here; set the appropriate existing type without editing account-wide definitions. If unavailable later, defer types instead of inventing labels.

Priority currently uses **only** `priority:urgent`, `priority:high`, `priority:medium`, `priority:low` labels. Owner-approved access was restored on 14 November 2026; the account issue-fields API returned no definitions, so no existing native Priority was available to reuse. Retain this fallback rather than creating shared account definitions for this setup. Do not add a duplicate Project Priority field. A later migration to accessible native Priority (Urgent/High/Medium/Low) must map values and remove old labels. Account-wide definition changes affecting other repositories require an owner decision.

| Priority | Meaning |
| --- | --- |
| Urgent | Credible active/imminent harm or a failed current safety control. Lead confirms with evidence; anyone flags concerns immediately. |
| High | Material unresolved risk, important missing safeguard or selected delivery blocker needing next attention. |
| Medium | Normal dependency-ordered delivery. |
| Low | Optional work outside the selected outcome. |

Severity and size are separate from priority. Missing priority remains visibly untriaged, never Low by default. #39's High concerns assessment urgency: advisory severity is Medium, practical exposure unknown. Record dated limits and the next assessment step; revisit on new evidence and ordinary open-queue review. #40 is preventive security work; #8/#10 are safety-sensitive product requirements, not current incidents by default.

Choose exactly one current label from the first four [triage labels](triage-labels.md), in this order (definitions live only there):

1. A specific decision/action is needed on this issue → `ready-for-human`; record decision, next actor and action in the body.
2. Otherwise a necessary fact is missing → `needs-info`; state the missing fact and who can supply it.
3. Otherwise the brief is unevaluated/unprepared → `needs-triage`.
4. Otherwise the brief is prepared → `ready-for-agent`, including dependency-blocked work.

A decision on a prerequisite does not make every dependent `ready-for-human`.

| Stage | Use |
| --- | --- |
| Backlog | Not started, including prepared blocked work and pre-start human decisions. |
| In progress | Active authorized work. |
| Review / waiting | Submitted work, outstanding verification, or halt after starting. |
| Done | Closed; retain completed/not-planned reason. |

## Project configuration and access

**14 November 2026:** owner-approved `project` and `read:user` access resolved the initial connection block. No existing Projects or account issue-field definitions were returned. Created the private [Metten Project](https://github.com/users/moultonlavah/projects/1), linked to this repository; kept priority labels and existing native issue types. No shared account definitions changed. [Issue #41](https://github.com/moultonlavah/metten/issues/41) holds the dated inventory, observed settings, walkthroughs and remaining publication evidence.

The Project uses only these views; reuse its existing fields for future updates:

| View | Filter |
| --- | --- |
| [Queue (mandatory)](https://github.com/users/moultonpoint/projects/1/views/1) | `repo:moultonpoint/metten is:issue is:open` |
| [Product](https://github.com/users/moultonpoint/projects/1/views/2) | Queue filter plus `label:product` |
| [Security](https://github.com/users/moultonpoint/projects/1/views/3) | Queue filter plus `label:security` |

Views show labels (category, triage, priority), Status (the stages above) and assignee; Queue also shows parent issue. Queue and Security use saved flat display defaults so children remain visible independently of parents. Native blocked icons are shown beside affected issues; follow issue links for exact dependency details. Verify every current open issue is in Queue; check native blocker indicators and direct dependency/body links. Report missing indicators honestly. Filters across different fields combine with AND; do not promise cross-field OR or a blocked-only filter. Drop unsupported optional views/sorts, retain complete Queue.

With priority labels there is **no native Priority sort**. During triage inspect temporary `label:priority:urgent`, then high, medium, low, then missing-priority items (negate all four labels); return to Queue. If migrated to native Priority, sort by that field. Add no duplicate ranking, Effort/Severity/date fields or extra permanent views.

All 20 open issues in the setup inventory were manually seeded. Manually seed any future omitted existing issues: built-in auto-add does not backfill. Only built-in auto-add (Metten repository selected, `is:issue is:open`) and closed-issue → Done are enabled. Other default workflows, including automatic issue closure, merge/linked-PR stage changes, item-added stage assignment and automatic sub-issue addition, are disabled. Do not enable board → issue closure. Verify behavior on an explicitly identified disposable setup item with cleanup, or label it a walkthrough. Preserve manual merges; no bot, stale closure, automatic security dismissal, paid service or organization migration.

Milestones are optional, finite and never dependencies or inherited child membership. If useful, create at most Repository security baseline (#30/#39/#40) and First complete paper purchase (#5/#6), with explicit membership/exit evidence and no unsupported dates. Parent/child counts are not effort; unavailable evidence remains required. None were created for this setup: existing blockers already express the sequence.

## Upkeep

At each work session, make one small queue check: selected issue's tags, blocker/authority state, assignee, stage and next action. While unresolved work exists, the lead makes a short weekly manual sweep of open-queue coverage, missing/conflicting metadata, changed evidence/priorities (including #39), halted work and parent outcomes. Record necessary changes in the affected issues. No scheduled automation or second report ledger.

Configuration references: GitHub's [issue fields](https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/managing-issue-fields), [Project filters](https://docs.github.com/en/issues/planning-and-tracking-with-projects/customizing-views-in-your-project/filtering-projects), [dependencies](https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/creating-issue-dependencies) and [auto-add](https://docs.github.com/en/issues/planning-and-tracking-with-projects/automating-your-project/adding-items-automatically).
