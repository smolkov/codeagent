# TUI: questions

Permission prompts, the limit prompt and questions from the model. Part of the [TUI](tui.md) specification.

## Permission prompts

When a tool call needs permission, the session sends a permission question
with the call's details, the reason and the proposed rules (see
[`sessions.md`](sessions.md), "Questions", and [`tools.md`](tools.md),
"Questions"). The same question may be answered in another TUI or with
`codeagent chat`; the TUI then closes its prompt and shows who answered.

The TUI:

1. Expands the tool call and scrolls it into view, showing the question's
   details: diff or content, command parts with their decisions, working
   directory.
2. Shows the prompt bar with the reason (`asks: default for edit_file`) and
   the choices.
3. Takes keys for the prompt bar only, until the user answers or cancels.
   What answers nothing still works: `PgUp` / `PgDn`, `Ctrl-Home` / `Ctrl-End`
   and the mouse wheel scroll, to read what is asked; `Ctrl-N` / `Ctrl-P`
   attach to another session and `Ctrl-S` opens the session list over the
   prompt, which then takes the keys until it closes; `Ctrl-D` detaches. The
   question stays open, for later or for another client: attaching to the
   session again shows the prompt again. `Ctrl-C` acts like `Esc`.

| Key   | Action |
|-------|--------|
| `y`   | Allow this call once. |
| `a`   | Allow this call and add an allow rule for the session. The prompt bar shows the proposed rule (this command, this file); `Tab` widens it step by step (e.g. `cargo test`, `src/**`), `Enter` confirms, `Esc` goes back. See [`config.md`](config.md), "Session rules". |
| `n`   | Deny; the model is told the call was denied. |
| `e`   | Deny and type a short message to the model explaining why or what to do instead. |
| `Esc` | Deny and cancel the turn. |

Edits are shown as a unified diff with added lines green and removed lines
red. Commands are shown exactly as they will run, with the working directory.

Starting a language server (see [`tools.md`](tools.md), "lsp") is asked in
the same prompt bar, below the `lsp` call's own details:

```
Start language server rust-analyzer in ~/project?
command: rust-analyzer  It may run build scripts and macros of the project.
[y] yes  [n] no  [e] explain  [Esc] cancel
```

It has no `a`: "yes" already lasts for the session.


A question whose text holds hidden characters (control characters,
bidirectional controls, zero-width characters) shows them as `⟨U+202E⟩`
(`␛` for `ESC`) and starts with the line `note: this question contains N
hidden characters, shown as ⟨…⟩` in the warning colour (RFC 0035; see
[`command.md`](command.md), "Terminal output"). The answer, and an
"always" rule, use the question as the server sent it. Elsewhere the TUI
drops control characters and bidirectional controls.

## Limit prompt

When a turn reaches its limit of model requests or tool calls, the session
waits (see [`sessions.md`](sessions.md), "Background work") and the TUI shows
the prompt bar:

```
Turn limit reached (50 requests, 120 tool calls). Continue?
[y] yes  [n] stop
```

| Key   | Action |
|-------|--------|
| `y`   | Continue the turn with a new budget of the same size. |
| `n`   | Stop the turn; a "limit reached" notice is added. |
| `Esc` | Same as `n`. |

The prompt takes keys only for the prompt bar, as for permission prompts.

## Questions from the model

A question from the model (`ask_user`, see [`tools.md`](tools.md),
"ask_user") opens at the bottom of the screen, in the place of the input,
where the user's eyes are when they type. The tool call line shows
`ask_user Database` waiting; the session row counts the question in `●`,
as for other questions.

```
● Both work with the current code; the choice decides the migration.

▾ ask_user Database                                                     ? ask
── Question ─ 1/2 Database ───────────────────────────────────────────────────
Both work with the current code; the choice decides the migration.

Which database should the cache use?

▶ 1 SQLite (recommended)
      One file next to the binary, no server.
  2 Postgres
      Shared with the API; needs the server running.
  3 Type an answer

Up Down choose ─ Enter next ─ Tab question ─ Esc skip ─ Ctrl-C cancel turn
──────────────────────────────────────────────────────────────────────────────
  asking · /help · Ctrl-D detach
  project · ~/ws/project · anthropic/sonnet · 1/3 ●1 · 12.3k/200k · in 45.1k out 3.2k
```

- It takes the rows of the input and the rules around it, directly above
  the status row: a rule with the title on top, a rule below, no side
  borders, the full width of the terminal.
- **The input is covered** while the question is open: it is not drawn and
  takes no keys. Its text, cursor and history position are kept, and come
  back as they were when the question closes (answered, skipped, cancelled,
  or answered by another client).
- **Height:** as many rows as the question needs, keeping 3 rows of the
  conversation above it (fewer on screens below 13 rows, none at the
  smallest, 10 rows). A taller question scrolls inside its place; its keys
  line stays its last row.
- **The conversation** takes the rows above and follows its end, so the
  call that asks is the last line above the question. The mouse wheel over
  it scrolls it; over the question, it scrolls the question.
- The activity line is hidden while the question is open; the status row
  says `asking`.
- A box that opens over a question (`Ctrl-S`, the model picker) takes the
  question's place at the bottom (see [Boxes](tui.md#boxes)); when it closes, the
  question is there again.

| Key | Action |
|-----|--------|
| `Up` / `Down` | Select an option or the "Type an answer" row. |
| `1`–`9` | Select that row; as `Space` on it. |
| `Space` | With `multiple`: tick or untick the option. Without: choose it. On the text row: type. |
| `Enter` | On an option: choose it (with `multiple`, go on with the ticked ones, or this one if none is ticked) and go to the next question. On the text row: type; `Enter` again keeps the text and goes on, `Esc` goes back. |
| `Tab` / `Shift-Tab` | Next or previous question, keeping what was chosen; after the last, the review page. |
| `PgUp` / `PgDn` | Scroll the question when it is taller than its place. |
| `Esc` | Skip: answers `decline`; asks `[y] yes  [n] no` first when something was chosen. |
| `Ctrl-C` | Cancel the turn (`cancel`). |

- With `multiple`, options show `[x]` or `[ ]`; a chosen option is drawn
  in the accent colour.
- With one question and no `multiple`, `Enter` on an option sends at once.
  Otherwise the review page after the last question lists each answer;
  `Enter` sends, or goes to the first question without an answer.
- A text on the row of a question that has a choice is sent with it ("The
  user added").
- `context` and the model's `text` are drawn as Markdown.
- While the question is open the keys belong to it. `Ctrl-N` / `Ctrl-P`,
  `Ctrl-S` and `Ctrl-D` (when no text is typed) work as during a permission
  prompt; the question stays open. What was chosen is kept while the TUI
  stays on the session; switching to another session drops it.
- A rejected answer shows `not accepted: …` at the top of the question.
- When another client answers, the question closes and the conversation shows
  `answered by codeagent chat: reply`.
