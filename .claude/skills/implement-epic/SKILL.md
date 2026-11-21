---
name: implement-epic
description: Implement a whole Bears epic unattended, from its planned tasks to a pull request with green CI, by orchestrating one task-implementer subagent per task and a task-reviewer for each.
argument-hint: <epic-id>
disable-model-invocation: true
---

Implement the Bears epic `$ARGUMENTS` from start to a pull request with green CI, without
the user watching. They read one thing at the end: the pull request and your final report.

You are the orchestrator. You write no feature code and read no full diffs: each task goes
to a `task-implementer` subagent and each diff to a `task-reviewer` subagent, so that your
context lasts for the whole epic. What only you can do is the part between the tasks:
choosing the order, telling each agent what the others built, deciding on review findings,
integrating the branches, and judging whether the epic as a whole is done.

Use the installed `bea` CLI for all task tracking during the run, not the Bears MCP tools:
the MCP server writes to the main checkout whichever directory an agent works in.

Nothing in a run touches hardware. No agent flashes a board, runs probe-rs, opens a CAN
interface or edits a KiCad file. What only a real board can show is left to the user as
`manual` tasks and listed in the pull request.

## Before starting

Stop and tell the user, instead of working around it, if any of these fails:

- `bea show <epic> --json` is an epic with planned sub-tasks (`bea list --epic <epic> -a --json`).
- Every epic or task in the epic's own `depends_on` is done. `bea ready --epic` does not
  check this, so check it yourself.
- The working tree is clean, and `main` is up to date with `origin/main`.
- The checks of every component the epic touches pass on `main`. The table "Which checks a
  change needs" in the root `AGENTS.md` says which `just` recipes those are.

Then read the epic's body, every task's body and dependencies (`bea show <id> --json` for
each; the list output leaves the dependencies out), the root `AGENTS.md` and the scoped
`AGENTS.md` of every component the epic touches. You need to know the epic well enough to
tell a correct seam from a wrong one. Do not read the code the tasks will change.

Create the epic's branch from `main`: `<type>/<short-slug>-<epic-id>`, for example
`feat/cell-voltage-broadcast-kss`. If the branch already exists, this is a resumed run:
check it out, see what is committed, `bea release` any task left in progress with no
commits, and continue with the loop.

## The loop

Repeat until `bea ready --epic <epic> --json` returns nothing and no agent is running.

A task tagged `manual` is the user's to do: never give it to an implementer. Tasks of this
epic can depend on tasks of other epics, and those you do not implement either. So the
loop can end with tasks still open: blocked by a manual task, by a task outside the epic,
or by a task you released. That is a partial run; see "Finishing the epic". Most epics
here end as partial runs, because the bench checks come last and are manual.

### 1. Choose the batch

Take the ready tasks. Components are separate directories, and each firmware is its own
cargo workspace with its own `target/` and `Cargo.lock`, so tasks in different components
can often run together. What they share is `crates/`, the root `Cargo.toml` and
`Cargo.lock`, the justfiles, `.github/` and the wiki pages in `site/`:

- Two tasks run together only when their bodies make it clear that they work in different
  components and neither changes anything shared: for example the VCM firmware beside the
  manager UI, a `wiring/` or `site/` task beside a Rust task, or two firmwares that both
  only consume what an earlier task put in `crates/`.
- A task that changes a shared crate, the bootloader protocol, the flash layout, a
  justfile or a workflow runs alone: every other component builds against it.
- If you cannot tell, run them in sequence.
- No more than two agents at once.

A single task, and the first task of a parallel batch, works in the main checkout on the
epic's branch. The other task of a batch gets its own worktree: spawn it with
`isolation: "worktree"`. The project's settings base such a worktree on the current HEAD,
so the main checkout must be clean and committed when you spawn it. A build in a fresh
worktree starts from nothing.

### 2. Spawn the implementers

One `task-implementer` per task, all of a batch in the same message. Its prompt holds:

- the task id and the epic id (it reads the bodies itself; do not paste them);
- where it works: the main checkout on the epic's branch, or its worktree;
- the seams it builds on, from earlier implementers' reports: the messages, parameters,
  types, functions, crate features, recipes or CLI commands that exist now and where they
  are;
- decisions already made in this epic that bind it;
- for a batch: which other task runs beside it and what it owns, so that it keeps out.

### 3. When an implementer reports

Check the report against the repository, briefly: the commits exist
(`git log --oneline <base>..<head>`), the tree is clean (nothing under
`site/static/generated/`, no stray build output), `git diff --stat <base>..<head>` touches
the component the task is about and no `.kicad_sch` or `.kicad_pcb` file. Do not read the
diff.

If it stopped without finishing, decide: answer it with SendMessage if you can resolve what
blocked it from the epic or `AGENTS.md`; otherwise `bea release <id>`, note it for the
final report, and go on with the tasks that do not depend on it.

Otherwise spawn a `task-reviewer` with the task id, the directory and the commit range, and
nothing else. It must not see the implementer's report: a reviewer handed the author's
account tends to confirm it.

### 4. When a reviewer reports

Decide on every finding yourself; the reviewer advises.

- Blocking and should-fix findings you agree with go back to the same implementer with
  SendMessage, so that it fixes them with its context intact. Send the findings as the
  reviewer wrote them, with the `file:line` and the failing case.
- A finding you judge wrong, or outside the task, you drop, with the reason kept for the
  final report. A finding that is real but belongs to another task becomes a Bears task
  (`bea create … --parent <epic>`, with `bea dep add` where it blocks something).
- A finding about safety-relevant behaviour (contactors, precharge, limits, fault
  handling) is never dropped on your own judgment. It is fixed, or it goes into the pull
  request under "decisions made" with the reviewer's case, for the user to rule on.
- After fixes, send the reviewer the new range with SendMessage for a second look at just
  those findings. Two rounds at most: what is still open after the second becomes a Bears
  task under the epic, and you move on.

### 5. Integrate

A task done in the main checkout is already on the epic's branch. For a worktree task:

1. `git -C <worktree> rebase <epic-branch>`. A conflict only in a `Cargo.lock` is resolved
   by taking the epic branch's version and building that workspace. Any other conflict:
   abort the rebase and send it to the task's implementer, which knows both the code and
   its worktree.
2. `git merge --ff-only <task-branch>` in the main checkout. Never a merge commit.
3. `git worktree remove <worktree>` and delete the task's branch.

After a rebased task is integrated, run on the epic's branch the checks of every component
that either task of the batch touched, from the table in the root `AGENTS.md`. A change in
`crates/` means `just host ci` and a build of every firmware that uses the crate. Send the
output to a file in your scratchpad and read only the end of it. A failure goes to the
implementer whose task it concerns.

Discovered work from the implementers' reports becomes Bears tasks now: under the epic if
the epic is not done without it, otherwise on its own with the right priority. Anything
that needs a board, a probe or KiCad is tagged `manual`. Commit new and changed `.bears/`
files that no implementer committed (`chore: track work found in the <name> epic (<epic>)`).

## Finishing the epic

When the loop has ended, with every task done or with the rest blocked (a partial run):

1. Run `just ci` on the epic's branch, output to a file. It covers every component, so it
   also catches a firmware the epic did not mean to touch. Fix failures through the
   implementer concerned. If a recipe fails for a reason that is not the epic's (a missing
   local tool, a component that was already failing on `main`), say so in the report.
2. Spawn a `task-reviewer` for an epic review of `main..HEAD`. Handle its findings as in
   step 4, sending each to the implementer of the task it concerns.
3. Read the seams yourself, and only those: the diff of the public items in `crates/`
   (message, parameter and protocol definitions), every `memory.x` and anything describing
   the flash layout, the `Cargo.toml` files, the justfiles, `.github/`, the `AGENTS.md`
   files and the wiki pages in `site/`. Check them against the epic's body.
4. `bea done <epic>` if it did not auto-close, and commit it:
   `chore: complete the <name> epic (<epic>)`. In a partial run, leave the epic open and
   skip this step; the pull request's title and description say that it is a part, and list
   each open task with what blocks it.
5. Rebase on `origin/main` if it moved, push, and open the pull request with
   `gh pr create`. Its description, written for a reader who has not followed the run:
   what the epic delivers, in the user's terms; the tasks with their ids; **breaking
   changes**: every wire format, parameter or flash-layout change, with which units must
   be reflashed and in which order, and whether a unit running the old firmware still
   works beside the new; **safety**: every change to safety-relevant behaviour, stated as
   what the unit now does in which condition; **verify on hardware**: the open `manual`
   tasks, and for everything else the tests cannot judge, the recipe to run and what to
   observe or measure, since nothing in the run was executed on a board; **decisions
   made**: every place where an implementer or you chose between readings, with the
   reason, so the user can overrule one cheaply; findings you dropped and why; tasks you
   created; anything not done.
6. Watch CI (`gh pr checks <number> --watch`). The workflows are path-filtered, so only
   those of the touched components run. A failing job is fixed through the implementer
   concerned and pushed, until every check is green.

Do not merge the pull request. Never create a tag or a release, and never bump a version,
even when a task asks for it: leave that task open and report it.

## Deciding without the user

The user is not there, so do not stop to ask what you can decide. Where a task is open to
two readings, take the one that fits the epic's body and `AGENTS.md`, and record it under
"decisions made". Prefer the reading that is easier to change later.

Stop and report, with the epic's branch left as it is, only when going on would mean one
of these:

- a choice about safety-relevant behaviour that the epic does not make: a limit, a timing,
  what a unit does on a fault or on losing a peer;
- a breaking change to a CAN message, a parameter, the bootloader protocol or the flash
  layout that the epic does not call for;
- a change to a KiCad file, or work that cannot be finished without a board change;
- flashing, probing or otherwise driving hardware;
- working around a missing capability of aphid or wirebug instead of reporting it;
- a change to the repository's `.bears/` files beyond normal task tracking;
- deleting or rewriting work outside the epic;
- a task that contradicts `AGENTS.md`;
- the same task failing after two implementers have tried it and nothing else is ready.

## The final report

A few lines: the pull request's link and whether CI is green; the tasks done and any
released; what has to be verified on hardware before merging; the decisions the user
should look at first, safety-relevant ones before the rest; the Bears tasks you created.
If the run stopped early, lead with what stopped it and what the user needs to decide.
