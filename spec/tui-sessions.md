# TUI: sessions

The session list and switching between sessions. Part of the [TUI](tui.md) specification.

## Sessions

The TUI shows exactly one session, the attached one. Other sessions keep
running on the server; the session row counts the ones waiting for an answer to a
question (see [`sessions.md`](sessions.md), "Background work"), so the user
knows to switch to them.

The session list opens with `Ctrl-S`, `/sessions`, or `codeagent attach` without a
name:

```
┌ Sessions ────────────────────────────────────────────────────────────────────┐
│       NAME       WORKSPACE     MODEL             STATE      USED             │
│     ✓ project    ~/ws/project  anthropic/sonnet  working    12m              │
│ ▶ ■   project-2  ~/ws/project  local/qwen        idle       2h               │
│       docs       ~/ws/docs     anthropic/opus    ● waiting  3m               │
│   ■   old        ~/ws/old      anthropic/sonnet  saved      12d              │
│                                                                              │
│ 2 marked                                                                     │
│ Enter attach ─ Space mark ─ a all ─ d delete ─ / filter ─ n new ─ r rename … │
└──────────────────────────────────────────────────────────────────────────────┘
```

- The list holds every session the server knows, running and saved, in the
  server's order, which is also the order of `1/3` and of `Ctrl-N` / `Ctrl-P`.
  `✓` marks the attached session; it is selected when the list opens. `■`
  (ASCII `#`) marks a session marked for deleting.
- `USED` is the time since the session last changed (`SessionInfo.updated`;
  from an older server without it, since it was created).
- It asks the server for the sessions when it opens and every 2 s while open;
  the selection stays on the same session.
- On a narrow terminal the time, then the workspace, are left out; the key
  line leaves out `a all` first, then `r rename`, then `n new`.
- A question that opens in the attached session closes the list: keys belong to
  the prompt bar then.
- The line above the keys shows, when nothing is asked, the filter, how many
  sessions are shown of how many, and how many are marked, each only when it
  applies: `filter: saved >7d ─ 2 of 9 ─ 2 marked`. A message (a refused name,
  the result of a delete) takes its place.

| Key in list | Action |
|-------------|--------|
| `Up` / `Down`, `k` / `j` | Select a session among those shown; they stop at the ends. |
| `Enter`     | Attach to the selected session (detaches from the current one). On the attached session it only closes the list. |
| `Space`     | Mark or unmark the selected session, then select the next one. |
| `a`         | Mark every session shown; if all shown are marked already, unmark them. |
| `d`, `Del` | Delete: the marked sessions if any are marked (also those the filter hides), else the selected one, after a `y`/`n` confirmation. |
| `/`         | Filter: edit the filter text in the line above the keys. |
| `n`         | New session: asks for the workspace (default: the attached session's; `~` is the home directory, a relative path starts there) and the name (empty: the default name), then opens the model picker on `default_model`; `Enter` there creates the session and attaches. `Esc` goes back one step; in the picker it creates nothing. |
| `r`         | Rename the selected session, in place; `Enter` renames, `Esc` goes back. A refused name is shown in the list. |
| `Esc`       | Clear the marks if any; else clear the filter if set; else close the list, stay on the current session. Where the list stands for a choice not yet made (after the attached session ended, or for `codeagent attach` without a name), `Esc` exits the TUI. |

Marks belong to sessions, not to rows: they stay through the refresh and a
change of the filter. A session the server no longer lists loses its mark.
Marks end when the list closes.

### Filter

`/` opens the filter, starting with the current text: `Filter: saved >7d`.
Each key edits it, and the list follows. `Enter` or `Down` goes back to the
list with the filter kept; `Esc` empties it and goes back.

The text is split into words; a session is shown if every word matches:

| Word | Matches |
|------|---------|
| `>7d`, `>12h`, `>30m`, `>2w`, `>90s` | Not used for more than that. |
| `<1d` and the like | Used within that time. |
| Anything else | A part of the name, the workspace as shown, the model or the state (`idle`, `working`, `waiting`, `saved`), ignoring case. |

A word that looks like a time but does not parse (`>7x`) matches as text,
so it hides everything and shows that the filter is wrong. The attached
session is shown only if it matches. If the selected session is filtered
out, the first one shown is selected. With nothing shown, the list says `no
session matches`, and `Enter`, `d`, `r` and `Space` do nothing.

So "delete all saved sessions not used for a week" is `/ saved >7d`,
`Enter`, `a`, `d`, `y`.

### Delete

The confirmation says what is lost:

```
Delete old? 14 turns, 210k tokens. [y] yes  [n] no
Delete project? It is working: the turn is cancelled. 3 turns, 41k tokens. [y] yes  [n] no
Delete docs? Its question is closed unanswered. No turns yet. [y] yes  [n] no
Delete 5 sessions? 1 is working, 1 waits for an answer. [y] yes  [n] no
```

The tokens are input plus output. `y` sends one `kill` per session, in list
order, and clears the marks; `n` or `Esc` goes back with the marks kept.
When all are answered, the line says `deleted 5 sessions`, or with a failure
`deleted 4 sessions; docs: no session named docs` (for example a session
renamed by another client meanwhile), and the list is asked for again.
There is no undo: the files are removed at once. Deleting the attached
session, alone or among others, works as `/kill`: the TUI shows a notice and
the list stays open.

`/new [NAME]` creates a session in the attached session's workspace the same
way: the model picker opens on `default_model`, so `Enter` alone starts the new
session with it. If other sessions work in that workspace, the new session's
conversation starts with the note `codeagent new` prints.

## Switching sessions

- Each session keeps the TUI's view state while the TUI is attached to
  another one: scroll position, expanded tool calls, open tool groups and
  unsent input. It is
  restored on switching back. **(assumption)**
- On attach, the TUI loads the session's full history from the server, then
  follows it live. A turn that ran while detached appears as if it had been
  watched.
- A switch attaches on a new connection to the server and drops the old one,
  which detaches it. Until the new session is there, the old one stays on
  screen and the status row says `attaching NAME…`; if it fails, the error is
  shown and the old session stays. A question open in the session left stays
  open for other clients and is counted in `●`.
- If the attached session is killed (by this or another client), the TUI
  shows a notice and opens the session list. If no session is left, the list
  says `no sessions` and offers `n` new or `Esc` exit. `Esc` exits with `0`.
  Until the list is there, any key exits. For `codeagent server stop` the TUI
  opens no list: the notice, the input is disabled, and any key exits with
  `0`; it does not start the server again.
- If the connection to the server is lost, the TUI shows "server
  disconnected" and retries every 500 ms for 10 s, starting the server if
  needed, since sessions are saved. The conversation is rebuilt from the log;
  scroll position, expanded tool calls and unsent input are kept. After 10 s
  it exits with an error; the terminal is restored. **(assumption)**
