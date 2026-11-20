# Project instructions

- RFC: 0038
- Status: draft
- Milestone: Context management (`todo.md`, "Next milestones")
- Spec: new [`spec/context.md`](../spec/context.md) (the part "Project
  instructions"); [`spec/agent-loop.md`](../spec/agent-loop.md), "System
  prompt", "Stable prefix", "Secrets";
  [`spec/config.md`](../spec/config.md), "Files", "Project files", new
  "Context"; [`spec/security.md`](../spec/security.md), "Entry points and
  guards", "Accepted risks"; [`spec/sessions.md`](../spec/sessions.md),
  "Persistence"; [`spec/tui.md`](../spec/tui.md), status view;
  [`spec/commit.md`](../spec/commit.md), "Agent file";
  [`spec/architecture.md`](../spec/architecture.md), "Protocol"

## Summary

A session reads the project's instruction file (`AGENTS.md`, `AGENT.md`,
`agent.md`) at the workspace root, and a personal one next to the user
config file, and puts their text into the system prompt. The model then
knows the project's rules (build and test commands, conventions, what not
to touch) without the user pasting them into each session. The files are
read again at the start of each turn; a change shows a notice. `/status`
lists the files that are in use with their size. `[context] instructions =
false` turns it off.

## Motivation

- Almost every repository that is worked on with coding agents has an
  `AGENTS.md` (or a `CLAUDE.md`). OpenCode, Claude Code, Codex and others
  read it on their own. `codeagent` does not: today only `codeagent
  commit` reads it (RFC 0013). In a session the model does not know the
  project's rules until it happens to read the file, and often it does
  not.
- Users work around it by asking "read AGENTS.md first" in each session.
  That costs a tool call and a question (`read_file` asks outside the
  rules), and it is forgotten after the history is trimmed.
- `spec/agent-loop.md`, "System prompt", already says project instructions
  belong to `context.md` (planned). This RFC writes that part of
  `context.md`; the rest of context management (shortening the history
  when it outgrows the window) is a later RFC.
- It is small: the file search, the link check and the cut exist in
  `src/commit/prompt.rs`; the prompt already has a section for skills
  (RFC 0023) to follow.

## Scope

In:

- Finding the project file and the user file, reading, checking and
  cutting them.
- The section in the system prompt, its place and its effect on the
  stable prefix and the prompt cache.
- Reading again per turn, the event when the text changes, the notice.
- `/status` in the TUI and in `codeagent chat`; `codeagent run`.
- `[context] instructions`.
- `codeagent commit` using the same search.
- The security row and the accepted risk.

Out:

- Instruction files in subdirectories, read when the model works there
  (open question 2).
- `@file` includes inside an instruction file.
- Other names than those below, set in the config (open question 1).
- A question before a new repository's file is used (open question 4).
- Shortening the history when it no longer fits: a later RFC, also in
  `spec/context.md`.

## Design

### Which files

Two files, each optional, in this order: **(proposal)**

| Source | Where | Names, the first found counts |
|--------|-------|-------------------------------|
| User | The directory of the user config file: `~/.config/codeagent/` (with `--config`, the directory of that file) | `AGENTS.md` |
| Project | The workspace root | `AGENTS.md`, `AGENT.md`, `agent.md`, then `CLAUDE.md` |

- `CLAUDE.md` is read only when none of the three is there, so a
  repository written for Claude Code works unchanged and one with both
  does not get the rules twice. **(proposal; open question 1)**
- The workspace root is the session's workspace (`-C`, `DIR`), not the
  repository root: a session in a subdirectory of a repository reads the
  file of that directory. **(proposal; open question 2)**
- The checks are those of `codeagent commit` (`spec/commit.md`, "Agent
  file"), so both read the same file:
  - a regular file, or a link to one inside the workspace; a link that
    leaves the workspace is not followed;
  - not when the `read_file` rules deny it (also the built-in rules for
    secret files): such a file is left out, and `/status` says why;
  - read as UTF-8, bad bytes replaced;
  - cut at 32 000 characters, with a line saying so (below).
- The user file has the same checks except the workspace boundary: it is
  the user's own file, next to their config.
- An empty file, or one with only white space, counts as not there.

### System prompt

The text goes into a section after the fixed text and before the skills
section ([`skills.md`](../spec/skills.md), "System prompt"): **(proposal)**

```
Instructions for this project, from the user and from the workspace.
Follow them; what the user says in the conversation wins over them.

<instructions source="user" file="~/.config/codeagent/AGENTS.md">
…
</instructions>

<instructions source="project" file="AGENTS.md">
…
</instructions>
```

- The user file comes first and the project file second, so the more
  specific text is nearer to the conversation. **(proposal)**
- `file` is the path relative to the workspace, or with `~/` for the user
  file.
- A cut file ends with the line
  `[cut: the file has N characters; read the rest with read_file]`.
- The fixed text loses nothing: the rules on data, denials and asking
  stay, and the section cannot change permissions. The file is an
  instruction to the model, not a configuration; what the model does
  still goes through the rules.
- The text goes through the session's redactor (RFC 0030) like the rest of
  the prompt, so a key written into the file becomes a placeholder.
- With no file found the prompt is the same as today.

### When the files are read

- At the start of each turn, when the system prompt is built
  ([`agent-loop.md`](../spec/agent-loop.md), "Stable prefix"). All
  requests of a turn send the same text, also when the file changes during
  the turn (e.g. the model edits `AGENTS.md`). **(proposal)**
- A turn whose text is the same as the last one's keeps the prefix, so
  the prompt cache keeps working. A changed file costs one uncached
  prefix, once.
- This differs from skills, whose text is kept with the session until they
  are turned off and on again (`spec/skills.md`, "Active skills"). An
  instruction file belongs to the workspace and is edited with the work;
  a session that keeps an old copy would follow rules the project no
  longer has. **(proposal; open question 3)**

### Event and notice

When the set of files or their text differs from the last turn of the
session (also at the first turn), the session writes the event
`instructions` before the turn's first request: **(proposal)**

```json
{"type":"instructions","files":[
  {"source":"project","file":"AGENTS.md","chars":4210,"cut":false}
]}
```

- `files` is empty when the last file went away.
- The event carries names and sizes, not the text: the text is in the
  workspace, and the log should not keep a second copy of it. A resumed
  session reads the files again at its next turn.
- Clients show a notice:
  - first turn: `instructions: AGENTS.md (1.1k tokens)`;
  - later: `instructions changed: AGENTS.md`, or
    `instructions removed: AGENTS.md`;
  - with a cut file: `… (cut at 32000 characters)`.
  The notice is a line like `model: …` in the TUI, in `codeagent chat` and
  on standard error of `codeagent run`.
- An older client ignores the unknown event; it is a new protocol version
  (the next free one, `12` unless another RFC takes it first).
- `codeagent export` and `codeagent import` carry the event; a log of an
  older `codeagent` has none, and its sessions read the files from their
  next turn on.

### Status

`/status` (TUI and `codeagent chat`) gets an **Instructions** line, after
the skills:

```
Instructions  AGENTS.md  1.1k tokens · ~/.config/codeagent/AGENTS.md  0.2k tokens
```

- The estimate is the characters divided by four, rounded up, as for
  skills.
- A file that was found but left out is named with the reason:
  `AGENTS.md: denied by rule (config.toml:12)`,
  `AGENTS.md: link leaves the workspace`, or
  `off ([context] instructions = false)`.
- The values come from the last `instructions` event and from
  `session_status`, which gets a field `instructions` with the same
  entries and the reasons for files left out.

### Configuration

```toml
[context]
instructions = true   # read AGENTS.md and the user file; false: neither
```

- User file only, like `[session]`. A project file may set it to `false`
  (it only takes something away) and nothing else in `[context]`; any
  other key or value there is an error naming it. **(proposal)**
- A change applies from the next turn, because the files are read per
  turn.

### codeagent commit

`codeagent commit` keeps its own prompt (`spec/commit.md`) and keeps
sending only the project file, at the repository root. The search moves
from `src/commit/prompt.rs` to `src/context/instructions.rs` and both use
it, so the names and checks cannot drift apart. `CLAUDE.md` as a fallback
applies to `commit` too. **(proposal)**

### Modules

| Module | Change |
|--------|--------|
| `src/context/instructions.rs` (new) | `find(root, user_dir, protected) -> Instructions`: the files, their text, cut flags, files left out with reasons. |
| `src/commit/prompt.rs` | Uses `context::instructions` for the project file. |
| `src/agent/prompt.rs` | `with_instructions(base, &Instructions)` before `with_skills`. |
| `src/agent/mod.rs` | Reads the files at the start of a turn; compares with the last; emits `instructions`. |
| `src/agent/events.rs`, `src/protocol/` | The `instructions` event; `session_status` field; version. |
| `src/config/` | `[context] instructions`, project-file check. |
| `src/tui/`, `src/command/chat/`, `src/command/run/`, `src/text/status.rs` | Notice and status line. |

### Security

The project file comes with the repository, so its author is the
"repository author" attacker of `spec/security.md`. What it can do:

- **It can steer the model.** That is its purpose, and the same as with
  every other agent that reads `AGENTS.md`. It cannot widen permissions:
  every tool call still goes through the rules, the built-in rules and the
  questions; the file is not configuration.
- **It is not data any more.** Today the system prompt says file contents
  are data. The project file is the one file read as instructions; the
  prompt names it so the model can tell it apart from files it reads with
  tools.
- **Limits:** one file per source, 32 000 characters, read only at the
  workspace root, links not followed out of the workspace, denied files
  left out.
- **Terminal:** only the file name and sizes are printed, through the safe
  terminal text of RFC 0035.
- **Turning it on or widening it:** a project file can only turn it off;
  an imported session carries only names and sizes.

New row in "Entry points and guards":

| Source | Trusted | Guard |
|--------|---------|-------|
| Project instruction file (`AGENTS.md`) | No: comes with the repository | Read as instructions for the model, not as configuration: the rules and questions still decide every call. Root only, links kept in the workspace, `read_file` rules apply, cut at 32 000 characters, secrets hidden; can be turned off, not widened, by a project file. |

New accepted risk:

| Risk | Why it stays |
|------|--------------|
| A repository's `AGENTS.md` can tell the model to do things the user did not ask for. | Reading it is what users expect from a coding agent; the permission rules bound what the model can do, and `/status` and the notice show that a file is in use. A question before a new repository's file is used is open question 4. |

## Spec changes

- `spec/context.md` (new): "Project instructions" with the sections of the
  design: files, prompt, reading per turn, event, status, limits. A
  section "History size" says it is planned.
- `spec/readme.md`, `spec/architecture.md`, `spec/SUMMARY.md`:
  `context.md` from `planned` to `partly`, in the index and the book.
- `spec/agent-loop.md`: "System prompt" replaces the assumption about
  project instructions with a link to `context.md`, and names the section
  order (fixed text, instructions, skills); "Stable prefix" says the
  instructions are read once per turn.
- `spec/config.md`: new "Context" with `[context] instructions`; "Project
  files" allows `instructions = false`.
- `spec/sessions.md`: the `instructions` event in "Persistence".
- `spec/architecture.md`: protocol version and the event.
- `spec/tui.md`: the notice and the status line.
- `spec/commit.md`: "Agent file" links to `context.md` and adds
  `CLAUDE.md`.
- `spec/security.md`: the row and the accepted risk above.
- `readme.md`: a short part on project instructions and the config key.

## Testing

- `context::instructions`: each name and the order; `CLAUDE.md` only when
  none of the others; a link inside and one outside the workspace; a
  dangling link; a directory with the name; a denied file; bad UTF-8; the
  cut at 32 000 characters; an empty file; the user file with
  `--config`.
- Prompt: no files gives today's prompt byte for byte; one and both files;
  the section before the skills; the cut line; a key in the file becomes a
  placeholder.
- Agent loop with the fake provider:
  - all requests of a turn have the same system prompt when the file
    changes during the turn;
  - the next turn has the new text, and one `instructions` event;
  - an unchanged file gives no event and the same prefix;
  - a removed file gives an event with empty `files`;
  - `instructions = false`: no section, no event.
- Config: `[context]` in the user file; `instructions = false` in a project
  file; `true` or another key in a project file is an error with
  file:line.
- Protocol: the event and the `session_status` field round trip; an
  unknown event is ignored by the client of the old version.
- Persistence: resume, export and import with the event; a log without it.
- `codeagent commit`: the existing tests pass with the moved search; one
  for `CLAUDE.md`.
- Clients: the notice texts and the status line in the TUI and in
  `codeagent chat` (unit tests on the renderers); a `codeagent run` end to
  end test against the local model server, checking that the request's
  system prompt has the file.

## Implementation plan

1. **Find and read the files**
   - Status: todo
   - Files: `src/context/mod.rs`, `src/context/instructions.rs`,
     `src/lib.rs`, `src/commit/prompt.rs`
   - Result: one search for both sources; `codeagent commit` uses it.
   - Tests: the `context::instructions` tests; the commit tests.
2. **Configuration**
   - Status: todo
   - Files: `src/config/`, `spec/config.md`
   - Result: `[context] instructions` is read and checked.
   - Tests: the config tests above.
3. **System prompt**
   - Status: todo
   - Files: `src/agent/prompt.rs`, `src/agent/mod.rs`,
     `spec/context.md`, `spec/agent-loop.md`
   - Result: sessions and `codeagent run` send the instructions, read per
     turn.
   - Tests: the prompt and agent loop tests, without the event.
4. **Event, protocol and status**
   - Status: todo
   - Files: `src/agent/events.rs`, `src/protocol/`, `src/session/`,
     `src/server/requests.rs`, `spec/sessions.md`,
     `spec/architecture.md`
   - Result: the `instructions` event in the log and on the wire; the
     `session_status` field.
   - Tests: the event tests, protocol and persistence tests.
5. **Clients**
   - Status: todo
   - Files: `src/tui/`, `src/command/chat/`, `src/command/run/`,
     `src/text/status.rs`, `spec/tui.md`
   - Result: notices and the status line.
   - Tests: the client tests and the `codeagent run` end to end test.
6. **Spec, security, readme**
   - Status: todo
   - Files: `spec/readme.md`, `spec/SUMMARY.md`, `spec/commit.md`,
     `spec/security.md`, `readme.md`, `todo.md`
   - Result: the spec says what is built; the RFC is `done`.
   - Tests: `ci/check.sh`, `ci/test.sh`.

## Alternatives

- **Let the model read `AGENTS.md` with `read_file`, told so by the fixed
  prompt.** No new code, but a tool call and often a question per session,
  the text is trimmed away in long sessions, and models skip it.
- **Keep the text with the session, as for skills.** A stable prompt for
  the whole session, but the session follows rules the project changed,
  and the log keeps a copy of a workspace file. Reading per turn costs at
  most one uncached prefix per change.
- **Put the text into the first user message instead of the system
  prompt.** It would be trimmed and summarized with the history, and a
  change in the middle of a session would have nowhere to go.
- **Read every instruction file from the workspace root down to the
  current directory, as some agents do.** `codeagent` has no current
  directory inside the workspace; the model moves around with paths.
  Files in subdirectories are open question 2.

## Open questions

1. Names: is `CLAUDE.md` as a fallback right, or should the names be a
   config list (`[context] files = ["AGENTS.md", "CLAUDE.md"]`)? Should
   `.opencode/` or `.github/copilot-instructions.md` count?
2. Subdirectories: read an `AGENTS.md` of a subdirectory when the model
   first reads or edits a file there (added to the result, as the
   diagnostics after an edit are)? And should a session in a subdirectory
   of a repository also read the file at the repository root?
3. Reading per turn or once per session: is the cost of one uncached
   prefix per change acceptable, or should a change wait for `/reload` or
   a new session?
4. A new repository: should the first use of a workspace's file ask the
   user (show the file, remember the answer by its hash), as some editors
   do for workspace trust? It protects against a hostile cloned
   repository, but asks for every repository once and again after each
   edit of the file.
5. Size: is 32 000 characters per file right, or should the limit follow
   the model's context window (e.g. at most 5 % of it)?
