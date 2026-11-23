# TUI: boxes

The model picker, the language server box and the skills box. Part of the [TUI](tui.md) specification.

## Model picker

Each session has its own provider and model. The model picker chooses them.
It opens with `Ctrl-O`, `/model` without an argument, or when creating a
session; then its title is "Model for a new session", it starts on
`default_model`, and it has no `current` or history marks.

```
┌ Model for "project" ──────────────────────────────────────────────────────┐
│ Filter: _                                                                 │
│                                                                           │
│ anthropic                                                                 │
│ ▶ sonnet      claude-sonnet-5-5         200k   ✓ current, default         │
│   opus        claude-opus-5-5           200k                              │
│ local                                                                     │
│   qwen        qwen3-coder:30b            32k   ⚠ history 41k > 32k        │
│                                                                           │
│     already the default for new sessions                                  │
│                                                                           │
│                                                                           │
│ Enter select ─ Space default ─ Esc cancel                                 │
└───────────────────────────────────────────────────────────────────────────┘
```

- The picker lists exactly the providers and models of the configuration
  (`spec/config.md`), grouped by provider, in config order. There is no
  free-text model id: a model that is not configured cannot be chosen.
- Typing filters by provider name, model name and model id.
- Each row shows the model's config name, its `id` and context window. Marks:
  current model of the session, `default_model`, a provider whose API key is
  missing (`⚠ no key`; still selectable, the error comes on first use), and a
  model whose context window is smaller than the session's history
  (`⚠ history 41k > 32k`, from the last context size). Until
  `spec/context.md` exists such a model is only marked.
- The list comes from the server (the `models` request), since its
  configuration and environment are what the requests use. The TUI takes context
  windows from the same answer; it asks when the picker opens and when the
  configuration was reloaded.
- `Enter` selects and closes; on the current model it only closes. `Esc` closes.
  `Up` / `Down` select; other keys edit the filter, except `Space` (below). With
  no match the list says `no model` and `Enter` does nothing.
- **Default for new sessions** (RFC 0021). Under the list, `[ ] Also make it the
  default for new sessions`; `Space` ticks or unticks it (model names and ids
  have no spaces, so `Space` does not edit the filter; pasted text still does).
  With the tick, `Enter` also saves the selected model as `default_model` in
  the user file the server reads (see [`config.md`](config.md), "Writing the
  user file"); on the session's current model it only saves. `Esc` saves
  nothing. The tick starts unticked each time the picker opens.
  - On the row that already is the user file's default the line reads
    `already the default for new sessions` and `Space` does nothing.
  - With the tick, the line under it names the file:
    `saved as default_model in ~/.config/codeagent/config.toml`.
  - When the default does not come from the user file, that line warns,
    ticked or not: `⚠ CODEAGENT_MODEL=local/qwen overrides the saved default`, or
    `⚠ .codeagent/config.toml of this workspace sets default_model = local/qwen`.
    The save still happens: other workspaces and later starts use it.
  - After the save the conversation shows
    `default model: anthropic/opus (saved in …)`, and the warning again if
    something else wins; a failed save shows
    `default model not saved: …` and keeps the selection.
  - The next start uses the saved model: `codeagent`, `codeagent new`, `/new` and the
    session list's `n`, `codeagent run` and `codeagent commit` without a model, also
    after a server restart. A running server reads the changed file before
    it creates a session or answers `models`, so a start right after the save
    gets it, not only after the next 2 s poll.
- The picker does not open while a question is open, and a question that opens
  closes it: keys belong to the prompt bar then.
- `/model <provider>/<model>` (or `/model <model>` when unambiguous) switches
  without the picker; an unknown name is an error that lists the configured
  models.
- If no model is configured, the picker shows the path of the user config file
  and a short example instead of a list.

Changing the model of a running session:

- The new provider and model apply from the next model request. A request that
  is already running finishes with the old model. **(assumption)**
- The conversation history is kept and converted to the new provider's
  format. If it does not fit the new context window, the oldest parts are
  shortened (see `spec/context.md`); the TUI asks for confirmation first.
- A notice in the conversation records each change, e.g.
  "model: anthropic/sonnet → local/qwen". Choosing the model the session already
  has adds no notice. A log of an older `codeagent` has no old model; its notice is
  "model: local/qwen".
- The change applies to this session only. Other sessions and
  `default_model` are not affected, unless the default tick is set (above) or
  `/model … --default` is used.

## Language servers

`/lsp` opens a box with the language servers of the codeagent server (see
[`config.md`](config.md), "LSP"), one row each:

```
┌ Language servers ─────────────────────────────────────────────────────────┐
│ ▶ rust-analyzer  .         ready  ─ codeagent, fix                             │
│   pyright        ../docs   failed: exited with status 1                   │
│                                                                           │
│ Last lines of rust-analyzer:                                              │
│ INFO waiting for flycheck                                                 │
│                                                                           │
│ r restart ─ k stop ─ Up Down choose ─ Esc close                           │
└───────────────────────────────────────────────────────────────────────────┘
```

- A row has the server's name, its root (relative to the session's
  workspace, else absolute), its state (`starting`, `indexing 42 %`,
  `ready`, `stopped`, `failed: <reason>`) and the sessions that used it, by
  name. A failed server is red.
- Below the rows: the last lines of the selected server's standard error.
- `r` restarts the selected server (it forgets its crashes), `k` stops it
  (it starts again when the model uses it); the box shows the new list, or
  the error. `Enter` or `F5` asks again. `Up` / `Down` select; `Esc`, `q`
  and `Ctrl-C` close.
- With no server yet: "No language server has run. One starts when the
  model uses lsp."

`lsp` calls in the conversation show the operation, symbol and place
(`lsp references parse src/parse.rs:3`) and, when done, a count
(`✓ 3 references`, `✓ 1 error, 2 warnings`, `✓ 12 symbols`); see
[`tools.md`](tools.md), "lsp".

## Skills

`/skills` opens a box with the skills found for the session's workspace and
its active ones (see [`skills.md`](skills.md)), from the `skills` request. It
opens with the answer; a request that fails shows the error in the
conversation.

```
┌ Skills for "project" ────────────────────────────────────────────────────────┐
│ Filter:                                                                      │
│                                                                              │
│ ~/ws/team-skills                                                             │
│ ▶ [x] release-check   Steps before tagging a release of a Rust crate. Use w… │
│   [ ] api-style       How our HTTP handlers are written and tested.          │
│ .claude/skills                                                               │
│   [ ] migrations      Writing and checking database migrations.              │
│ ~/.claude/skills                                                             │
│   [ ] pdf             Read, fill and merge PDF files.                        │
│       release-check   shadowed by ~/ws/team-skills/release-check             │
│ Steps before tagging a release of a Rust crate. Use when the user asks to p… │
│ 1 active ─ ⚠ 1 skipped: ~/.agents/skills/old/SKILL.md: no description        │
│ Space toggle ─ Enter apply ─ Esc cancel                                      │
└──────────────────────────────────────────────────────────────────────────────┘
```

| Key | Action |
|-----|--------|
| `Up` / `Down` | Select the previous / next skill; shadowed rows and headings are passed over. |
| `Space` | Tick or untick the selected skill. |
| `Enter` | Apply: the ticked skills become the session's active skills (`set_skills`), and the box closes. Without a change it only closes. |
| `Esc` | Close without changes. |
| other keys | Edit the filter: name and description, any case. |

- Rows are grouped by directory, in order of priority; a heading shows the
  directory relative to the workspace, as `~/…` in the server's home
  directory, or absolute. Directories without skills have no heading.
- The box opens with the session's active skills ticked. An active skill
  that is not found any more is listed first as
  `[x] name   not found any more`; unticking it is the only change it
  allows. An active skill whose file changed shows `changed on disk` before
  its description.
- A ticked row is in the accent colour; a shadowed one is dim and names the
  skill that wins.
- The description is cut to the box width with `…`. The line above the
  status line shows the selected skill's full description, cut only at the
  box width, when its row cuts it.
- The status line counts the ticked skills and the warnings (skipped files,
  missing configured directories), the first one in full.
- Without skills, the box lists the directories searched and an example of
  a `SKILL.md`.
- `Space` is free for the tick because skill names have no spaces; pasted
  text goes into the filter.
- A change shows the notice `skills: + release-check, − pdf` in the
  conversation of every attached client, from the `skills_changed` event.
- The box does not open while a question is open, and a question that opens
  closes it, as for the model picker. The end of the session closes it.
- In ASCII mode the ticks stay `[x]` / `[ ]`; the pointer and the
  separators follow `tui.symbols`.

`/skills NAME` turns that skill on if it is off and off if it is on, without
the box, with the same notice. An unknown name is an error from the server
that lists the found names.

The Session tab of the status box lists the active skills with their size
in tokens (estimated), and marks one that changed on disk or is not found
any more.
