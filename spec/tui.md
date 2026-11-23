# TUI

Interactive sessions are shown in a full-screen terminal user interface (TUI).
The TUI is a client of the `codeagent` server: it attaches to one session at a
time, and can detach, switch to another session or start a new one. Sessions
keep running without a TUI (see `spec/sessions.md`).
One-shot mode (`codeagent "task"`) and runs where standard output is not a
terminal do not use the TUI; they print plain text (see
[`command.md`](command.md)).

Statements marked **(assumption)** need a decision from the user.

## Topic files

The TUI is specified in this file and in topic files. A reference
like "`tui.md`, "Copy"" means the section of that name, in whichever
file it is now:

| File | Sections |
|------|----------|
| [`tui-sessions.md`](tui-sessions.md) | Sessions, Filter, Delete, Switching sessions |
| [`tui-conversation.md`](tui-conversation.md) | Conversation entries, Activity line, Copy, Images, Details view, Cancellation and exit |
| [`tui-questions.md`](tui-questions.md) | Permission prompts, Limit prompt, Questions from the model |
| [`tui-input.md`](tui-input.md) | Input editor, Command list, Commit |
| [`tui-status.md`](tui-status.md) | Token counters, Status box, Global tab |
| [`tui-boxes.md`](tui-boxes.md) | Model picker, Language servers, Skills |
| [`tui-settings.md`](tui-settings.md) | Settings, Theme list, Appearance, Syntax highlighting |
| `tui.md` (this file) | Goals, Layout, Boxes, Keys, Performance, Implementation notes, Open questions |

## Goals

- Show the conversation, tool calls and their results as they happen,
  including the model's answer while it streams.
- Make every permission decision visible and quick: the change or command is
  shown in full before the user answers.
- Stay responsive: the screen redraws and keys work while the model answers
  or a command runs.
- Never leave the terminal broken: on exit, error or panic in a dependency,
  the terminal is restored (raw mode off, cursor shown, main screen back).

## Layout

```
> fix the failing test

● The test fails because `parse` drops the last line. Let me look.

▸ read src/parse.rs                                               ✓ 120 lines
▾ edit src/parse.rs                                                     ? ask
  - for i in 0..lines.len() - 1 {
  + for i in 0..lines.len() {

──────────────────────────────────────────────────────────────────────────────
edit: src/parse.rs  asks: default for edit_file
[y] yes  [a] always  [n] no  [e] explain  [Esc] cancel
──────────────────────────────────────────────────────────────────────────────
> _
──────────────────────────────────────────────────────────────────────────────
  asking · Esc cancel · /help · Ctrl-D detach
  project · ~/ws/project · anthropic/sonnet · 1/3 ●1 · 12.3k/200k · in 45.1k out 3.2k
```

There is no box around the screen: the conversation takes the full width, and
plain lines set off the prompt bar and the input. From top to bottom:

| Area           | Content |
|----------------|---------|
| Conversation   | Scrollable history of the session: user messages, model text, tool calls. |
| Activity line  | Shown only while the session works: what it does, for how long, and the tokens so far. See [Activity line](tui-conversation.md#activity-line). |
| Prompt bar     | Shown only while a question (permission or limit) is open; replaces nothing else. |
| Input          | Multi-line editor for the next user message, with a line above and below it. Covered while a question from the model or a box is open: they take its rows (see [Questions from the model](tui-questions.md#questions-from-the-model), [Boxes](#boxes)). |
| Status row     | Current state (idle, waiting for model, running tool, asking), turn counters, key hints. |
| Session row    | Session name, workspace path, provider and model (`anthropic/sonnet`), position among all sessions (`1/3`), number of other sessions waiting for an answer (`●1`), context usage (tokens used / limit; only the tokens if the server does not list the model, nothing until the first count), tokens the session has used (`in 45.1k out 3.2k`). |

While the session works and no question is open, the activity line takes the
last row of the conversation area, with an empty row above it:

```
▸ read src/parse.rs                                               ✓ 120 lines

✻ Prestidigitating… (14s · ↓ 1.2k tokens · Esc cancel)
──────────────────────────────────────────────────────────────────────────────
> _
```

The conversation area takes all remaining height. If the terminal is smaller
than 40 × 10 cells, `codeagent` shows only a "terminal too small" message until it
is resized.

### Boxes

The boxes that commands open lie at the bottom of the screen, in the place
of the input, as a question from the model does (RFC 0024): the model
picker, the session list, the skills box, the status box (both tabs), the
language servers, the theme list, help and the delete confirmation.

```
● The parser drops the last line; the fix is in src/parse.rs.

▸ edit src/parse.rs (line 12)
── Model for "project" ────────────────────────────────────────────────────────
Filter: _

anthropic
▶ sonnet  claude-sonnet-5-5  200k  ✓ current, default
  opus    claude-opus-5-5    200k

[ ] Also make it the default for new sessions

Enter select ─ Space default ─ Esc cancel
──────────────────────────────────────────────────────────────────────────────
  idle · /help · Ctrl-D detach
  project · ~/ws/project · anthropic/sonnet · 1/3 · 12.3k/200k
```

- A box takes the rows of the input and of the rules around it, directly
  above the status row: a rule with its title on top, a rule below, no
  side borders, the full width of the terminal. A title at the right (the
  ranges of the Global tab) is at the right end of the top rule.
- The input is covered and takes no keys while a box is open; its text,
  cursor and history position come back when the box closes. The
  activity line is hidden; the status row still shows the state.
- **Height:** as many rows as the box needs, up to the screen less the two
  footer rows, the two rules and 3 rows of conversation, which stay above
  it. On a small screen a box gets at least 8 rows (or the rows it needs)
  before rows are kept for the conversation. A box that needs more rows
  than it gets scrolls inside, as before; its keys line stays its last row.
- The conversation takes the rows above and keeps its place. The wheel
  over it scrolls it; the wheel over the status box scrolls the status
  box.
- One box is at the bottom at a time: the one that takes the keys. A box
  opened over a question (the session list with `Ctrl-S`) takes the
  question's place until it closes. A question that opens closes the
  boxes.
- The sketches of the box sections below show their rows; the frame is
  the one above.
- The settings and the details view take the whole screen instead (see
  [Settings](tui-settings.md#settings), [Details view](tui-conversation.md#details-view)).

## Keys

| Key                  | Action |
|----------------------|--------|
| `Esc` / `Ctrl-C`     | Cancel the running turn. With no running turn, `Ctrl-C` clears the input. |
| `Ctrl-C` twice within 1 s, or `Ctrl-D` with empty input | Detach and exit the TUI; the session keeps running. |
| `Ctrl-S`             | Open the session list. |
| `Ctrl-O`             | Open the model picker for the attached session. |
| `Ctrl-N` / `Ctrl-P`  | Attach to the next / previous session in the list, round the end. With one session, nothing. |
| `PgUp` / `PgDn`      | Scroll the conversation. |
| `Ctrl-Home` / `Ctrl-End` | Jump to start / end of the conversation. |
| `Tab`                | Move focus between input and conversation. |
| `Enter` (conversation focus) | Expand or collapse the selected tool call; on a tool group, open or close it. |
| `Up` / `Down` (conversation focus) | Select the previous / next entry. A closed tool group is one entry; the calls of an open one are entries of their own. |
| `o` (conversation focus) | Open the selected tool call in the [details view](tui-conversation.md#details-view). On a closed tool group it does nothing but say `o opens a tool call`. |
| `y` / `Y` (conversation focus) | Copy the selected entry / the last code block of the selected answer. See [Copy](tui-conversation.md#copy). |
| `?` (conversation focus) | Show help. |
| `Ctrl-Y`             | Copy the last answer, as `/copy`. |
| `Ctrl-T`             | Show or hide the previews of the tool calls of the last turn, and open or close its tool groups. |
| `Ctrl-V` / `Alt-V` (input focus) | Paste an image from the clipboard, else its text. See [Images](tui-conversation.md#images). |
| Click on a tool call | On its summary line: closed → short view (the preview, or for a call without one its details cut at `tui.tool_output_lines`); short or full → closed. A call whose preview `tui.tool_preview` shows counts as short. The focus does not move. A click on its `… N lines` row opens the details view. |
| Double click on a tool call | On its summary line: the whole result, or closed when it was whole. The first click acts at once and the second one, on the same row within 400 ms, turns the call to whole, so a single click never waits. A third click acts as a single click. |
| Click on a tool group | On its line: open or close it. A double click acts as a single click. The focus does not move. |

`Ctrl-Z` does nothing for now: the TUI does not suspend.

While new output arrives, the view follows the end of the conversation,
unless the user scrolled up; then it stays put and the status line shows
"↓ new output". `Ctrl-End` resumes following.

`Enter` on a selected call keeps its own cycle: closed, expanded, whole
result (when cut), closed. The first call of a group is selected as the
group, so `Enter` on it opens and closes the group; a click on its own
summary line, inside the open group, changes its view.

Mouse wheel scrolls the conversation (over a question from the model, the
question), a drag selects text and copies it (see
[Copy](tui-conversation.md#copy)). Mouse capture can be turned off with `tui.mouse = false` so
the terminal's own text selection works (see [`config.md`](config.md), "TUI");
with it on, `Shift`-drag uses the terminal's selection in most terminals.

With the focus on the conversation, the footer says `Enter expand ─ o open`
when a call is selected, and `y copy`, `? help` and, with the mouse on,
`drag copy`.

## Performance

- Redraw at most 30 times per second (`tui.max_fps`); streaming text is
  batched between frames.
- An idle TUI wakes once per second. While the session works and the
  activity line or a tool call spinner is on the screen, a frame tick runs
  every 120 ms: it moves the glyph, the shimmer and the spinners, and only
  the activity line and the running calls are built again.
- Input never waits for the network or a tool: the UI runs on its own task and
  receives events from the server over the client connection.
- Switching to a session with a long history shows its latest page within
  200 ms; older history loads in the background. **(assumption)** The first
  version replays the whole log before its first frame; paged loading is built
  if that turns out to be too slow.
- A session with 10 000 conversation lines scrolls without visible delay.

## Implementation notes

- Library: `ratatui` with the `crossterm` backend.
- The TUI only displays state and sends user actions to the server; it does
  not call the provider or run tools itself. The agent loop runs in the
  server and does not depend on the UI, so one-shot mode and tests can run
  without a terminal.
- Rendering is tested with `ratatui`'s test backend: given a state, the
  produced screen buffer is compared with an expected one.

## Open questions

1. Is an external editor (`Ctrl-G` opens `$EDITOR` for the input) wanted?
2. Answered ([RFC 0006](../rfcs/0006-tui-settings.md)): `/theme [NAME]`
   switches the theme and saves it as the default; see [Theme list](tui-settings.md#theme-list).
3. Should keys be configurable (`[tui.keys]`)?
