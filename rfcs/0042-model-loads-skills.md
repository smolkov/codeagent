# The model turns on skills

- RFC: 0042
- Status: draft
- Milestone: none
- Spec: [`spec/skills.md`](../spec/skills.md), "Active skills",
  "System prompt", "Saving"; [`spec/tools.md`](../spec/tools.md);
  [`spec/agent-loop.md`](../spec/agent-loop.md), "Requests", "System
  prompt"; [`spec/config.md`](../spec/config.md), "Permissions",
  "Skills"; [`spec/sessions.md`](../spec/sessions.md), "Persistence";
  [`spec/tui.md`](../spec/tui.md), "Skills";
  [`spec/security.md`](../spec/security.md); RFC 0023

## Summary

A new tool, `use_skill`, lets the model turn on a skill that fits the
current task. Its description lists the skills the model may choose, by
name and description. A call turns the skill on exactly as `/skills NAME`
does: the same `skills_changed` event, the same block in the system
prompt, the same read access to its directory. The model can only turn
skills on; turning them off stays with the user.

It is off by default. `[skills] model` in the user file turns it on, for
the user's own skills only (`user`) or also for the workspace's (`all`).
A workspace skill is never turned on without a question.

## Motivation

- Today a skill helps only when the user remembers it and turns it on
  before the task (`spec/skills.md`: "The model does not choose skills
  itself"). A user with twenty skills does not keep them all in mind; the
  model reads the task and knows which one fits.
- Other agents that use the same `SKILL.md` format (Claude Code,
  OpenCode) let the model load skills; skills written for them expect it,
  e.g. descriptions that say "Use when the user asks to …".
- RFC 0023 left this out, for three reasons. This RFC answers each:
  - *Every session pays for the list.* Off by default; with it on, the
    list is bounded (see "Catalogue") and only skills the model may load
    are in it. No such skill, no tool, no tokens.
  - *A project skill from an untrusted repository could be pulled in.*
    Workspace skills are left out unless the user sets `model = "all"`,
    and then each one asks, also under `--yes`.
  - *The fixed tool list changes.* `use_skill` joins the tools that are
    offered only sometimes (`spec/agent-loop.md`, like `lsp`).

## Scope

In: the tool, its catalogue, the setting, the permission, the prompt
rebuild after a call, the event field and the notice.

Out: the model turning skills off; the model loading a skill's other
files without the file tools; skills for sub-agents (RFC 0036, see "Open
questions"); `allowed-tools` and other frontmatter keys, which stay
ignored.

## Design

### Setting

`spec/config.md`, "Skills". User file only, as all of `[skills]`:

```toml
[skills]
# Which skills the model may turn on itself with use_skill:
# none, user (skills.dirs and the home directory), all (also the
# workspace's). Default: none.
model = "user"
```

**(proposal)** Default `none`: a new release does not change what the
model sees, and the user opts in once.

Sources, by the table of `spec/skills.md`, "Where skills are found":

| `model` | Rows offered |
|---------|--------------|
| `none` | none; the tool is not offered |
| `user` | 1 (`skills.dirs`) and 5–7 (home directory) |
| `all` | 1–7; rows 2–4 (workspace) always ask |

A shadowed skill is never offered. An already active skill is not in the
catalogue.

### Catalogue

The tool's description holds the list, built once per turn with the
other tool specs (`spec/agent-loop.md`, "Requests"), so it does not break
the provider cache within a turn:

```
Turn on a skill: its instructions join the system prompt from your next
request, and its directory becomes readable. Use it when a skill below
fits the task; do not turn on skills the task does not need.

- release-check: Steps before tagging a release of a Rust crate. Use when the user asks to prepare or check a release.
- pdf: Read, merge and fill PDF files. …
```

- Order: the order of `spec/skills.md` (directory priority, then name).
- **(proposal)** At most 50 skills and 8 KiB of list; the rest are left
  out, and the last line says `… N more skills; ask the user to turn them
  on`. A description is cut at 300 characters in the list; the full one
  stays in `/skills`.
- Names and descriptions are cleaned like other text from files before
  they go into a prompt (control characters removed); they are data the
  user installed, not checked further.
- When no skill is offered, the tool is not offered.

### Tool

`spec/tools.md`, new section `use_skill`:

```json
{ "name": "release-check" }
```

- `name` must be a skill in the current catalogue. Anything else is an
  error result `unknown skill NAME; the skills you may turn on are listed
  in the tool's description`. The tool never takes a path, so it reads
  nothing the model names beyond the found skill's `SKILL.md`.
- The call reads `SKILL.md` again (it may have changed since the list was
  built); a file that now fails to load is an error result with the
  warning text.
- On success the result is short: `release-check turned on; its
  instructions are in the system prompt from your next request.` The text
  is not repeated in the result.
- Shown in the TUI as `skill release-check`, state `✓ turned on`.

### Permission

`spec/config.md`, "Permissions": a new tool name `use_skill`, default
`allow`, with patterns on the skill name (`use_skill(release-*)`).

- A workspace skill (rows 2–4) always asks, like a command the built-in
  rules mark unsafe: `--yes`, `allow` rules and an "always" answer do not
  reach it. **(proposal)** The question names the directory, so the user
  sees that the skill comes from the repository.
- A project file may set `use_skill = "deny"` or add `deny` rules, never
  `allow` (`spec/config.md`, "Project files").

### Limit

**(proposal)** At most 5 skills turned on by the model per session; then
the tool is no longer offered, and a call in the same turn is an error
`limit of 5 skills reached; ask the user`. Skills the user turns on do
not count. The limit is `[skills] model_max`, 1–50, user file only.

### Prompt rebuild

Today the system prompt is built once per turn (`spec/agent-loop.md`),
while `spec/skills.md` says a change applies "from the next model
request". After a successful `use_skill` the loop rebuilds the system
prompt before its next request, with the new skill block last.

- This breaks the provider's prompt cache once per skill; the history
  part stays cached where the provider caches by prefix after the system
  prompt **(assumption: Anthropic's cache is prefix-based and restarts
  at the system prompt)**.
- The same rule then holds for a user's `/skills` change during a turn;
  `spec/agent-loop.md` and `spec/skills.md` are made to say the same.

Rejected: the text as the tool result (see "Alternatives").

### Saving and display

- `skills_changed` (`spec/sessions.md`, "Persistence") gains `by:
  "user" | "model"`; a missing field reads as `user`, so older logs load
  unchanged. Replay and export are as today.
- TUI notice: `skills: + release-check (model)`. The `/skills` box marks
  a skill the model turned on with `model`; the user can turn it off
  there as any other.
- `codeagent chat` prints the notice; `codeagent run --report` lists the
  skills the model turned on.

### Security checklist

1. *Input, from which attacker:* the skill name from the model (prompt
   injection can choose it); names and descriptions from skill files,
   which for workspace skills come from the repository.
2. *Writes:* the session log (`skills_changed`). It widens later steps:
   the skill's directory becomes readable and its text an instruction of
   the user. Guard: only found skills, workspace skills always ask, a
   limit per session.
3. *Child processes:* none.
4. *Terminal output:* the name in the notice and the question, from a
   file; cleaned by `text::safe::terminal` like other names.
5. *Limit:* catalogue 50 skills / 8 KiB, 5 skills per session,
   `SKILL.md` at 64 KiB as today.
6. *Project or imported file:* a project file may only deny. An imported
   session replays the skills it had, as today; `by: "model"` changes
   nothing about that.

Row for `spec/security.md`, entry points: "`use_skill` | the model,
prompt injection | only found skills; off by default; workspace skills
always ask; limit per session".

Row for accepted risks: "A skill the user installed may be turned on by
the model for a task the user did not mean it for. The user opted in with
`[skills] model`; the notice shows it; `/skills` turns it off."

## Spec changes

- `spec/skills.md`: drop "The model does not choose skills itself";
  new section "Turned on by the model": setting, catalogue, permission,
  limit; "System prompt": the heading line becomes "These skills are
  on." when one came from the model **(proposal)**, since "The user
  turned on" is no longer true.
- `spec/tools.md`: new section `use_skill`; "Display" table: `skill`.
- `spec/agent-loop.md`, "Requests": `use_skill` among the tools offered
  only sometimes; the system prompt is rebuilt after a skill change.
- `spec/config.md`: `[skills] model`, `model_max`; `use_skill` in
  `[permissions]`; project files may only deny it.
- `spec/sessions.md`: the `by` field of `skills_changed`.
- `spec/tui.md`, "Skills": the `(model)` notice and mark.
- `spec/security.md`: the rows above.
- `readme.md`: the setting and the permission.

## Testing

- Config: `model` values, `model_max` range, `[skills]` in a project file
  still rejected, `use_skill = "allow"` in a project file rejected.
- Catalogue: not offered with `none`, without skills, or when all are
  active; `user` leaves out workspace skills; shadowed skills left out;
  cut at 50 skills and 8 KiB with the "more" line; control characters
  removed.
- Tool: unknown name; a name of a workspace skill under `user`; a
  `SKILL.md` broken since the list was built; success writes
  `skills_changed` with `by: "model"`.
- Permission: a workspace skill asks under `--yes` and with an `allow`
  rule; a project `deny` holds.
- Limit: the sixth call fails; the tool is gone in the next turn.
- Loop (`tests/run.rs`, fake server): the request after `use_skill` has
  the skill block in its system prompt; the request before has not.
- Session: resume and export/import keep the skill; an old log without
  `by` loads.

## Implementation plan

1. **Setting and permission**
   - Status: todo
   - Files: `src/config/mod.rs`, `src/config/file.rs`,
     `src/permissions/`, `spec/config.md`
   - Result: `[skills] model`, `model_max` and `use_skill` load and check
   - Tests: config and permission tests above
2. **Catalogue and tool**
   - Status: todo
   - Files: `src/skills/mod.rs`, `src/tools/use_skill.rs`,
     `src/tools/mod.rs`, `spec/tools.md`, `spec/skills.md`
   - Result: the tool is offered with its list and turns a skill on
   - Tests: catalogue and tool tests
3. **Prompt rebuild and limit**
   - Status: todo
   - Files: `src/agent/`, `spec/agent-loop.md`
   - Result: the next request carries the skill; the limit holds
   - Tests: loop tests in `tests/run.rs`
4. **Event, notice, box**
   - Status: todo
   - Files: `src/session/`, `src/tui/`, `src/command/chat.rs`,
     `spec/sessions.md`, `spec/tui.md`
   - Result: `by: "model"` saved and shown
   - Tests: session and TUI tests
5. **Security and readme**
   - Status: todo
   - Files: `spec/security.md`, `readme.md`, `todo.md`
   - Result: rows and documentation in place
   - Tests: `ci/check.sh`, `ci/test.sh`

## Alternatives

- **The skill text as the tool result.** No prompt change and no cache
  break. Rejected as in RFC 0023: the user cannot turn it off, trimming
  of old results (RFC 0008) would drop it, and a result is data, not an
  instruction (`spec/agent-loop.md`, "System prompt").
- **The catalogue in the system prompt.** Same tokens, but it changes
  the prompt when skills on disk change; in the tool description it is
  built with the tools and goes away with them.
- **On by default for user skills.** Less setup, but every user pays for
  the list and sees the model act on skills they installed for another
  agent. Can be changed later without a format change.
- **The model may also turn skills off.** Saves context in long sessions,
  but a skill the user chose could vanish behind their back; left to the
  user.
- **Ask for every skill.** Safer, but for the user's own skills the
  question adds little; `use_skill = "ask"` gives it to who wants it.

## Open questions

1. Default of `[skills] model`: `none` **(proposal)** or `user`?
2. Catalogue bounds: 50 skills, 8 KiB, 300 characters per description
   **(proposal)**.
3. Limit per session: 5 **(proposal)**, and whether it is a setting.
4. Agents (RFC 0036, draft): does an agent's session get the tool, and
   may an agent file limit which skills it offers?
5. Heading of the prompt section once the model turned a skill on.
