# TUI: conversation

What the conversation shows, and how its text is copied and opened. Part of the [TUI](tui.md) specification.

## Conversation entries

| Entry          | Display |
|----------------|---------|
| User message   | Prefixed with `>`, in the user colour. |
| Model text     | `●` (`*` in ASCII, in the accent colour) before its first row; the other rows indented by two columns, so the text forms one column. Rendered Markdown: headings, emphasis, lists, inline code, code blocks with syntax highlighting, tables. A table is drawn as columns two cells apart, as wide as their text, with the head in bold and a rule under it; when it is wider than the screen, the widest columns are narrowed first (down to 8 cells) and their cells wrap. Column alignment (`:-`, `-:`, `:-:`) is kept. Streams in as it arrives, with the marker from its first row. An answer cut by tool calls gets the marker on each part. |
| Tool call      | One summary line: marker, the tool's short name, main argument, state. |
| Tool preview   | Below the summary of a finished edit, write or command, and of a failed call: see below. |
| Tool details   | Below the summary when expanded: diff for edits, file excerpt for reads, command and output for commands. |
| Tool group     | Finished calls of one tool in a row as one line, e.g. `▸ Ran 5 commands`; see "Tool groups" below. |
| Error          | Red line with the error message; the session continues. |
| Notice         | Dim line for events such as "turn cancelled" or "limit reached". |

The tools are shown by short names: `read` (`read_file`), `ls`
(`list_files`), `search` (`search_files`), `edit` (`edit_file`), `write`
(`write_file`), `run` (`run_command`); other tools by their own name. The
short name is on the summary line, in the prompt bar of a permission
question and in the title of the details view. Where the user reads or
types a name the configuration uses, the real name stays: the rule text of
a question (`asks: default for edit_file`), the rules `a` proposes, the
status box, the settings and configuration errors (see
[`tools.md`](tools.md), "Display").

Tool call state is one of: running (spinner, moved by the frame tick of
[Performance](tui.md#performance)), done (`✓` plus a short result,
e.g. line count or exit code), failed (`✗`), denied (`⊘`), waiting for
permission (`?`).

Tool calls are collapsed (`▸`) by default and expanded (`▾`) while they wait
for permission. Long outputs are cut to `tui.tool_output_lines` lines (default
20) in the view, with a "… N more lines" marker; the full text stays
available in the expanded entry and the [details view](#details-view). See
[`config.md`](config.md), "TUI".

```
▸ read src/parse.rs                                               ✓ 120 lines
▾ edit src/parse.rs                                               ✓ edited …
  @@ -12,3 +12,3 @@
  - for i in 0..lines.len() - 1 {
  + for i in 0..lines.len() {
▾ run cargo test parse                                            ✓ exit code 0
  $ cargo test parse
  … 38 lines
  test parse::last_line ... ok
  test result: ok. 4 passed; 0 failed; 0 ignored
  exit code 0
```

**Previews.** With `tui.tool_preview = "changes"` (the default), a finished
call shows a preview (`▾`) below its summary:

- `edit_file`, `write_file`: the diff, without its `---` / `+++` header, up to
  `tool_output_lines` lines; a new file its first lines.
- `run_command`: the command and the **last** 6 lines of its output, with
  `… N lines` above them for what is hidden. While a command runs, the same
  shows its live output.
- A failed call of any tool: its error, up to `tool_output_lines` lines.
- `read_file`, `list_files`, `search_files` and others that did not fail: no
  preview; they stay one line.

`"all"` expands every call, `"none"` keeps every finished call on one line.
`Enter` on a call with a preview expands it, as from collapsed; what the user
chose for a call wins over the setting. `Ctrl-T` shows the previews of all
calls of the last turn (expanding calls without one), or collapses them all
when any of them shows more than its summary.

The details of an edit come with the call's `tool_call` event (see
[`architecture.md`](architecture.md), "Protocol"), also when no question was
asked, so its diff is known after it ran.

**Tool groups** (RFC 0043). With `tui.group_tools = true` (the default),
finished calls of one tool that follow each other show as one line:

```
▸ Ran 5 commands                                                    ✓ 5 ok
▸ Read 3 files                                                      ✓ 3 ok
▸ Ran 4 commands                                      ✗ 1 failed, ⊘ 1 denied
```

- A group is a run of at least `tui.group_tools_min` (default 2) call
  entries of the same tool with no other entry between them (model text,
  a message, a notice), all finished: done, failed, denied or cancelled.
- Never grouped: `edit_file` and `write_file` (their diffs are the point),
  `ask_user` and `report`, a call waiting for permission (it stays
  expanded) and a call that runs (it shows its live output, and joins the
  group before it when it finishes).
- The line: `Ran N commands` (`run_command`), `Read N files`
  (`read_file`), `Listed N directories` (`list_files`), `Searched N times`
  (`search_files`), `N lsp queries` (`lsp`), else `N NAME calls`. Its
  state is `✓ N ok` when all are done, else what is not ok:
  `✗ N failed`, `⊘ N denied`, `⊘ N cancelled`, joined by `, `, in the
  error colour when one failed. Failed and denied calls stay in the group.
- Closed (`▸`, the default), the group is its line; the previews of its
  calls are hidden. Open (`▾`), its calls follow the line, two columns in,
  each as it shows without the group: its own marker, preview by
  `tui.tool_preview`, click, `Enter` and details view.
- Groups are worked out by the TUI from its entries: the protocol, the
  log and the model's history do not know them, and a resumed session
  groups the same. Which groups are open is kept by the id of their first
  call, like expanded calls.

## Activity line

A line above the input while the session works, so that the user sees at
once that something happens and for how long:

```
✻ Prestidigitating… (14s · ↓ 1.2k tokens · Esc cancel)
```

- **Place:** the last row of the conversation area, with an empty row above
  it. The conversation is two rows shorter while it is shown and follows the
  end as before. It is not part of the conversation: not saved, not
  scrolled, not selected. When the user scrolled up, it stays at the bottom.
- **Hidden** when the session is `idle` or `saved`, while a permission, limit
  or question prompt is open (the prompt says what is needed), after the
  session ended, below 14 rows of terminal, and with `tui.activity = "off"`.

| Phase | When | Text |
|-------|------|------|
| Thinking | after the request is sent, before its text | the verb of the turn: `Prestidigitating…` |
| Writing | text deltas arrive | the verb, as above |
| Tool | a tool call is pending or runs | `Running cargo test…`, `Reading src/parse.rs…`, `Editing src/parse.rs…`; with several calls the first is named and `+2` follows |
| Retry | after a `retry` event, until the next text or tool call | `Retrying (2/3): rate limited…` |
| Cancelling | after `Esc` or `Ctrl-C` cancelled the turn, until it ends | `Cancelling…` |

- **Verb:** one of about 50 playful gerunds (`Pondering`, `Untangling`,
  `Percolating`, …), chosen at random when the turn starts and kept for the
  turn. With `tui.activity = "plain"` the two phases say `Thinking…` and
  `Writing…`.
- **Tool text:** the tool as a verb (`run_command` Running, `read_file`
  Reading, `edit_file` Editing, `write_file` Writing, `list_files` Listing,
  `search_files` Searching; other tools `Running NAME`) and the main argument,
  as on the call's summary line.
- **In brackets:**
  - The time since the turn started: `14s`, `2m 05s`, `1h 03m`. The TUI notes
    it when it sees `working` after `idle`; after an attach to a session that
    already works it asks `session_status` once for the start of the running
    turn. Until the answer the time is left out.
  - `↓ 1.2k tokens`: the output tokens of the turn. Between `usage` events
    they are estimated from the text received (4 characters per token), and
    the count of a `usage` event replaces the estimate. Left out while it is
    0.
  - `Esc cancel`, while the input is empty and has the focus, so that `Esc`
    really cancels; not while cancelling.
- **Room:** parts that do not fit are dropped from the right: the hint, then
  the tokens, then the time. Then the text is cut with `…`. The glyph stays.
- After the turn the line goes; no "done in 14s" entry is left, the
  `turn_end` notice is enough.

Animation:

- **Glyph:** `· ✢ ✳ ✶ ✻ ✽ ✻ ✶ ✳ ✢`, one per frame, in the `accent` colour;
  with ASCII symbols `| / - \`.
- **Shimmer:** the verb is drawn `dim`, with a band of three characters in
  the `text` colour that moves from left to right about once per two seconds.
  Not with `plain`, and with `NO_COLOR` the verb is plain text and only the
  glyph moves.
- **Stalled:** while thinking, when no event has come for 30 s, the glyph
  turns to the `warning` colour. The time stays. Nothing else changes: the
  provider's own timeouts and retries decide whether the request failed.

## Copy

```
> fix the failing test

● The test fails because `parse` drops the last line. Let me look.
  for i in 0..lines.len() {          ← selected text in reverse colours
▸ read src/parse.rs                                               ✓ 120 lines
```

- **Selection:** with `tui.mouse = true`, a left-button drag in the
  conversation or the details view selects text, shown in reverse colours;
  the release copies it. The selection stays shown until `Esc`, a key or a
  click clears it; `Esc` then does nothing else. A double click selects and
  copies a word (on a call's summary line it opens the call instead, see
  [Keys](tui.md#keys)) (letters, digits and `_-./:~@#%+`, without a `.` or `:` at
  its end), a triple click the line with its wrapped rows. Dragging above or
  below the area scrolls it, so a selection can be longer than the screen.
  While the view follows the end, a selection stops the following until it is
  cleared, so the text does not move under the mouse.
- **What is copied** is the text as it is read, not as it is drawn: rows the
  TUI wrapped are joined again; the markers of the TUI (`▸ ▾`, the `●` and indent
  of an answer, the `> ` of a message, quote marks, indentation, the state of a call, `… N more lines`,
  rules) are left out; list markers stay; trailing spaces go. Code blocks are
  drawn without fences, so their rows copy as their source.
- **`y`** copies the selected entry: a message or answer as its Markdown
  source; for a tool call:

  | Tool | Text |
  |------|------|
  | `edit_file`, `write_file` | The diff as a unified diff with `a/PATH` and `b/PATH` headers, so it applies with `git apply`; a new file is its content. Without details (a log of an older `codeagent`), the old and new text as `-` and `+` lines. |
  | `run_command` | `$ command`, then the output with its exit code. |
  | others, and failed calls | The result. |
  | a closed tool group | The commands, or the main arguments, of its calls, one per line. |

- **`Y`** copies the last code block of the selected answer, or of the last
  answer when the selected entry has none. **`/copy`** and **`Ctrl-Y`** copy
  the last answer, `/copy N` the answer N back, `/copy code` its last code
  block.
- **The clipboard:** `tui.clipboard` ([`config.md`](config.md), "TUI").
  `auto` writes the terminal's OSC 52 sequence (which works over SSH; in tmux
  it needs `set-clipboard on`) and, when the session is local (neither
  `SSH_TTY` nor `SSH_CONNECTION` is set), also gives the text to `wl-copy`
  (Wayland), `xclip` or `xsel` (X11) or `pbcopy` (macOS), the first one
  installed. `osc52` and `command` use only one of the two; `off` copies
  nothing and says so. OSC 52 carries at most 100 kB of text; the rest is
  left out of it.
- After a copy the footer says `copied 12 lines` for two seconds. The TUI
  cannot know whether the terminal took the text, so it says `copied`. A
  clipboard command that cannot be started says `cannot copy: …`.

## Images

A message may hold images (RFC 0034), e.g. a screenshot:

```
────────────────────────────────────────────────────────────
images: #1 PNG 1366×306 33 KB · #2 GIF 20×10 1 KB
> compare [Image #1] with [Image #2]
────────────────────────────────────────────────────────────
```

- **`Ctrl-V`** (or `Alt-V`, for terminals that keep `Ctrl-V` for their own
  paste) reads the clipboard: its first image of `image/png`,
  `image/jpeg`, `image/webp` or `image/gif`, else its text, which is
  pasted as bracketed paste would. The terminal passes only text, so the TUI
  asks the desktop itself: `wl-paste` (Wayland), `xclip` (X11) or
  `osascript` (macOS, PNG only; text with `pbpaste`). Each program has 2 s.
  The footer says `pasting…`, then `image #1: PNG 1366×306 33 KB`, or why
  nothing came: `the clipboard is empty`, `Ctrl-V: …`.
- `tui.clipboard` decides whether it may: `auto` reads only when the session
  is local (over SSH the programs would read the remote clipboard),
  `command` always, `osc52` and `off` never. Without access the footer says
  so and points to `/image PATH`.
- **A dropped file:** pasted text that is one absolute path (`/…`, `~/…`,
  `file://…`, maybe quoted or with `\ ` for spaces) ending in `.png`,
  `.jpg`, `.jpeg`, `.gif` or `.webp` is read as an image; a file that is not
  one is pasted as the text it was.
- **`/image PATH`** adds an image file; relative paths are below the
  workspace. The file is read by the TUI as the user's own action, not by a
  tool, so no permission rule applies.
- The type comes from the first bytes, the width and height from the
  header: PNG, JPEG, GIF and WebP. Other files are refused (`not an image
  (PNG, JPEG, GIF or WebP)`), as are images over 5 MiB, more than 10 images
  in a message, or more than 11 MiB of images together.
- **In the input** an image is one character, drawn as `[Image #N]` in the
  accent colour: the cursor passes it in one step, `Backspace` or `Delete`
  removes it and its image. Numbers count from 1 per message in the order
  of pasting and are not reused until the message is sent, so `#2` stays
  `#2` when `#1` goes. Text is written around images as usual. Typing the
  text `[Image #1]` by hand gives text, not an image. The line above the
  input lists the images in it.
- **Sending:** the text keeps its placeholders, and each image follows the
  text up to its placeholder, so the model can refer to "image 1". A
  message of images alone is sent too. The server refuses images for a
  model without `images` ([`config.md`](config.md), "Model keys"); the
  message then comes back to the input with its images.
- **In the conversation** the message shows its placeholders and, below it,
  the `images:` line. The first image of a session adds the notice `images
  are sent as they are; secrets in them are not hidden`.
- **History:** `Up` brings back a message with its images while the TUI
  still has them (the last 8 messages with images sent from it); older
  ones, and those of other clients, come back as text.

## Details view

`o` on a selected call, a click on its `… N lines` row, or `/diff` opens a
view over the whole screen:

```
┌ edit src/parse.rs ─ ✓ edited src/parse.rs: replaced 1 occurrence ──────────┐
│ @@ -12,3 +12,3 @@                                                          │
│      let lines: Vec<&str> = text.lines().collect();                        │
│ -    for i in 0..lines.len() - 1 {                                         │
│ +    for i in 0..lines.len() {                                             │
│          out.push(parse_line(lines[i])?);                                  │
│                                                                            │
│ Up Down scroll ─ / search ─ n next ─ y copy ─ w wrap ─ Esc close           │
└────────────────────────────────────────────────────────────────────────────┘
```

- It shows all of the call, as its expanded entry does without the limit:
  for an edit the diff with its context, for a command the command, its
  directory and the whole output with the exit code. The output is the one
  the tool gave; a text cut at 1 MiB is said under it in the warning colour.
- `/diff` shows the diffs of the edits and writes of the last turn, each under
  its tool and file; the title counts the files. Without any, a notice says
  `the last turn changed no file`.
- Keys: `Up` / `Down` (also `k` / `j`), `PgUp` / `PgDn` (also `Space`),
  `Home` / `End` scroll; `/` types a search, `Enter` finds it (in any case)
  and `Esc` drops it; `n` / `N` find the next / previous match, round the end;
  the line found is marked, and `not found: …` says when there is none. `y`
  copies all of it, `w` turns wrapping off and on (without it, `Left` /
  `Right` scroll sideways), `Esc`, `q` or `Ctrl-C` close. The mouse wheel
  scrolls it and a drag selects and copies in it, as in the conversation.
- A question that opens closes it, as it closes the other boxes; so does a
  switch to another session.

## Cancellation and exit

- Cancelling a turn stops the model request and kills a running command
  (process group, `SIGTERM`, then `SIGKILL` after 2 s). Tool calls already
  done stay done; a notice is added to the conversation.
- Exiting the TUI only detaches; a running turn continues on the server. To
  stop it, cancel the turn or kill the session.
- On exit, the terminal is restored before anything else is printed. A panic
  hook and signal handlers (`SIGTERM`, `SIGHUP`) restore it too.
