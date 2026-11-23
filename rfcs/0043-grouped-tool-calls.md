# Grouped tool calls

- RFC: 0043
- Status: done
- Milestone: none
- Spec: [`spec/tui.md`](../spec/tui.md), "Conversation entries",
  "Keys", "Copy", "Settings"; [`spec/config.md`](../spec/config.md),
  "TUI"; RFC 0022

## Summary

The TUI shows finished tool calls of the same tool that follow each
other, with no model text between them, as one line:

```
▸ Ran 5 commands                                               ✓ 5 ok
```

A click or `Enter` on that line opens the group: its calls appear below
it exactly as they look today, each with its own summary, preview,
click, `Enter` and details view. A second click closes it. Only the view
changes; the protocol, the session log and what the model sees stay as
they are.

## Motivation

- A model that explores runs many calls in a row: five commands, eight
  reads. Each is a line, and each finished command shows its last six
  lines (`tui.tool_preview = "changes"`). The model's answer and the
  diffs that matter scroll out of view behind output the user does not
  need to read.
- `tui.tool_preview = "none"` hides the previews, but for every call, and
  still leaves one line per call.
- The information must stay one click away: a user who wants to know
  what ran, and what it printed, opens the group.

## Scope

In: grouping in the TUI conversation, the summary line, open and close
by mouse and keys, the settings, copy of a group.

Out: `codeagent chat` and `codeagent run` output; the protocol and the
session log; the details view (it still shows one call); mixed groups of
different tools (see "Open questions").

## Design

### What forms a group

A group is a run of tool call entries in one turn that:

- are of the same tool, by its short name (`run`, `read`, …);
- follow each other with no model text, user message, question or
  notice between them;
- are all finished: done, failed or denied;
- are at least `tui.group_tools_min` long (default 2).

Never grouped **(proposal)**:

- a call waiting for permission: it stays expanded, as today;
- a call that runs: it shows its live output; when it finishes, it joins
  the group before it, if any;
- `edit_file` and `write_file`: their diffs are what the user wants to
  see, so each stays its own entry with its preview.

Groups are built by the TUI from its entries each time the conversation
is laid out, so a resumed or switched-to session groups the same way,
and a call that arrives later extends the group.

### Summary line

Marker, text, state, aligned like a call's summary:

| Short name | Text |
|------------|------|
| `run` | `Ran N commands` |
| `read` | `Read N files` |
| `ls` | `Listed N directories` |
| `search` | `Searched N times` |
| `lsp` | `N lsp queries` |
| others | `N NAME calls` |

State: `✓ N ok` when all are done; otherwise the counts of what is not
ok, worst first: `✗ 1 failed`, `⊘ 2 denied`, `✗ 1 failed, ⊘ 1 denied`.
**(proposal)** Failed and denied calls stay inside the group; the count
on its line tells the user to open it. The ASCII symbols of
`tui.symbols` apply as for calls.

### Open and closed

- Closed (`▸`, the default): only the summary line. The calls' previews
  are hidden.
- Open (`▾`): the summary line, then each call indented by two columns,
  as it would show without the group: its own `▸`/`▾`, its preview by
  `tui.tool_preview`, its cycle of closed, short and whole.
- What the user chose for a group wins over the default, as for calls,
  and is kept when switching sessions, like expanded calls today
  (`spec/tui.md`, "Switching sessions").

### Keys and mouse

`spec/tui.md`, "Keys":

| Input | Action |
|-------|--------|
| Click on a group's summary line | Open or close it. The focus does not move. |
| `Enter` on a selected group | Open or close it. |
| `Up`/`Down` | Select the group line, and in an open group its calls. |
| `o` on a selected group | Nothing **(proposal)**; `o` acts on a selected call. |
| `Ctrl-T` | As today for the last turn; it also opens its groups, or closes them with the rest. |

A double click on a group line acts as a single click.

### Copy

**(proposal)** `y` on a selected closed group copies the commands (or
paths) of its calls, one per line; on an open group, as today, what is
shown. A drag selection copies what is on screen.

### Settings

`spec/config.md`, "TUI", and the settings box:

```toml
[tui]
group_tools     = true   # group finished calls of the same tool
group_tools_min = 2      # fewest calls that form a group, 2–100
```

`group_tools = false` shows every call as today. Both apply at once to
the whole conversation.

### Security checklist

1. *Input:* none new; the entries exist already.
2. *Writes:* none; the open or closed state of a group is TUI memory,
   like expanded calls.
3. *Child processes:* none.
4. *Terminal output:* the summary line is made of counts and fixed
   words; the calls inside are shown as today, cleaned by the same code.
5. *Limit:* a group is as long as the turn's tool calls
   (`limits.max_tool_calls`).
6. *Project or imported file:* `[tui]` is user file only.

No row for `spec/security.md` is needed.

## Spec changes

- `spec/tui.md`, "Conversation entries": a new entry "Tool group", the
  rules of "What forms a group", the summary line and an example.
  "Keys": the rows above. "Copy": `y` on a group. "Settings": the two
  keys. "Switching sessions": open groups are kept.
- `spec/config.md`, "TUI": `group_tools`, `group_tools_min`.
- `readme.md`: the two settings.

## Testing

TUI render tests (`src/tui/tests`), on a list of entries:

- five finished `run` calls in a row: one line `Ran 5 commands ✓ 5 ok`;
- one call alone, or fewer than `group_tools_min`: no group;
- model text between calls splits them in two groups;
- `run, run, read, read`: two groups;
- edits between commands: the edit stays its own entry and splits the
  commands;
- a running call and a call waiting for permission stay outside; the
  running one joins when it finishes;
- states: one failed, one denied, both, in ASCII too;
- click and `Enter` open and close; inside an open group a call's own
  click works; `Ctrl-T` opens and closes groups;
- `group_tools = false`: the view of today, line for line;
- a resumed session groups the same;
- `y` on a closed group copies the commands.

## Implementation plan

1. **Group model**
   - Status: done
   - Files: `src/tui/` (conversation entries)
   - Result: entries are grouped by the rules above, without rendering
   - Tests: grouping tests on entry lists
2. **Rendering**
   - Status: done
   - Files: `src/tui/` (conversation view)
   - Result: summary line, state, open and closed groups drawn
   - Tests: render tests
3. **Keys, mouse and copy**
   - Status: done
   - Files: `src/tui/` (keys, mouse, copy)
   - Result: click, `Enter`, selection, `Ctrl-T` and `y` work on groups
   - Tests: input tests
4. **Settings**
   - Status: done
   - Files: `src/config/mod.rs`, `src/tui/` (settings box),
     `spec/config.md`
   - Result: `group_tools` and `group_tools_min` load and apply at once
   - Tests: config tests, `group_tools = false` render test
5. **Spec and readme**
   - Status: done
   - Files: `spec/tui.md`, `readme.md`, `todo.md`
   - Result: documentation in place
   - Tests: `ci/check.sh`, `ci/test.sh`

## Alternatives

- **Group in the server, as one protocol entry.** Every client would
  group the same, but it needs a new protocol version and changes the
  session log for a question of display. Rejected.
- **Mixed groups** (`Ran 3 commands, read 2 files`): fewer lines in an
  exploring turn, but the summary grows long and the user cannot tell
  from it what failed. Left as an open question.
- **Only hide the previews** inside a run of calls. Less change, but
  leaves one line per call, which is the problem.
- **Group the whole turn's calls** behind one line. Shortest, but hides
  edits and the order of what the model did.

## Open questions

1. ~~Same tool or mixed groups~~: answered, same tool only.
2. ~~Edits and writes in groups~~: answered, kept out.
3. ~~Failed calls~~: answered, inside the group, counted on its line.
4. ~~The running turn's last group~~: answered, closed like the others.
5. ~~`y` on a closed group~~: answered, the commands only.

## Answers

1. to 5. The user asked to build it, so the proposals hold, as for RFC
   0023. Question 4 had none: a group is closed until the user opens it,
   also while its turn runs; a call that runs stays outside until it
   finishes, so its live output is seen.

## Notes from building

- No new entry kind: the first call of a group draws the group's line,
  the other calls draw no rows while it is closed. A row knows its offset
  in its entry, so a click tells the group's line from the first call's
  own summary below it.
- `o` on a closed group says `o opens a tool call`; on an open one it
  opens the first call.
- Cancelled calls join groups like denied ones and are counted as
  `⊘ N cancelled`. `ask_user` and `report` are kept out with edits and
  writes.
- Groups are worked out again only around the entry that changed: the
  calls in a row around it, so a long replay does not regroup everything
  per event.
- The tests of previews turn `group_tools` off; groups have tests of
  their own (`src/tui/tests.rs`, "Grouped tool calls").
