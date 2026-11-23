# TUI: settings and appearance

The settings box, the theme list and how the TUI looks. Part of the [TUI](tui.md) specification.

## Settings

`/settings` shows the `[tui]` keys of the user file
([`config.md`](config.md), "TUI") on the whole screen, all rows but the
two footer rows (RFC 0024); the conversation and the input are not drawn
while it is open, and the input's text is kept:

```
── Settings ─ ~/.config/codeagent/config.toml ──────────────────────────────────────
  theme              terminal  Colour theme
  symbols            auto      Unicode or ASCII symbols
▶ mouse             *on        Capture the mouse: wheel, drag copies
  tool_output_lines  20        Lines of tool output before it is cut
  tool_preview       changes   What finished tool calls show
  group_tools        on        Group calls of the same tool
  group_tools_min    2         Fewest calls that form a group
  clipboard          auto      How copied text reaches the clipboard
  syntax_highlight   on        Colour code by its language
  timestamps         off       Time of each entry
  scroll_lines       3         Lines per mouse-wheel step
  max_fps            30        Redraw limit
  activity           playful   Line while the session works

mouse: on: the wheel scrolls, a drag selects and copies, a click opens a call;
off: the terminal selects
Enter change ─ Left/Right value ─ d default ─ s save ─ Esc close
──────────────────────────────────────────────────────────────────────────────
  idle · /help · Ctrl-D detach
  project · ~/ws/project · anthropic/sonnet · 1/3 · 12.3k/200k
```

- The help of the selected key is word wrapped above the keys line. The
  theme list opened from the settings lies at the bottom of this screen.

- One row per key, with the value and a description; the line under the list
  explains the selected key. A key the file does not set shows its default,
  dim. `*` marks a value changed and not saved.
- The `theme` row says when something else decides it: `⚠ CODEAGENT_THEME=nord for
  this run`, or `⚠ colours are off (NO_COLOR)`. `symbols = unicode` in a locale
  that is not UTF-8 is marked too, and still applied.
- **Preview:** a change applies to this TUI at once; other TUIs see it only once
  it is saved.
- The title names the file `s` writes: the user file of this TUI (`--config` or
  the default path).

| Key | Action |
|-----|--------|
| `Up` / `Down` | Select a row. |
| `Left` / `Right` | Previous / next value: on/off, one of a list, one step of a number, the themes in the order of the theme list. |
| `Enter`, `Space` | On/off: toggle. List: next value. Number: type it in the row; `Enter` takes it, `Esc` goes back, a value out of range says so and stays. Theme: open the [theme list](#theme-list); its `Enter` only sets the row. |
| `d` | Back to the default; the key is removed from the file on save. |
| `s` | Save all changes. |
| `Esc` | Close. With unsaved changes it asks `save changes? [s] save  [n] discard  [Esc] back`; discarding puts the file's values back. |

Saving (also for the theme list):

- The keys are set or removed in `[tui]` of the user file; the table is added if
  it is missing. Everything else stays as it is, comments and order included.
- The new text is loaded like at start first. If that fails, nothing is written
  and the box shows the error.
- If the file changed since it was read, nothing is written either:
  `the file changed since it was read; open it again`.
- The file is replaced atomically with its mode kept; a link is written through
  to its target. The server notices it and sends `config_reloaded`; every TUI
  then applies the new `[tui]`.

Opening the box or the theme list reads the user file and the theme files
again, so a theme file edited on its own is listed. A question that opens
closes the box and the theme list and undoes their preview, as for the model
picker.

## Theme list

`/theme` opens a list of all themes ([`config.md`](config.md), "Themes"):

```
┌ Theme ───────────────────────────────────────────────────────────────────────┐
│ Filter:                                                                      │
│                                                                              │
│ built in                                                                     │
│   terminal          ✓ current                                                │
│ ▶ dark                                                                       │
│   light                                                                      │
│ bundled                                                                      │
│   atom-one-dark     base dark                                                │
│   nord              base dark                                                │
│   gruvbox-dark      base dark                                                │
│   solarized-light   base light                                               │
│   emacs             base light                                               │
│   flexoki-dark      base dark                                                │
│   flexoki-light     base light                                               │
│   material-oceanic  base dark                                                │
│   monokai           base dark                                                │
│   nightfox          base dark                                                │
│   one-light         base light                                               │
│   solarized-dark    base dark                                                │
│   textual-dark      base dark                                                │
│   tokyo-night       base dark                                                │
│ theme files                                                                  │
│   bad               ⚠ themes/bad.toml:1: base: …                             │
│                                                                              │
│ Enter take and save as default ─ Esc back to terminal                        │
└──────────────────────────────────────────────────────────────────────────────┘
```

- Groups: built in, bundled (a theme file that replaces one shows its path
  there), theme files by name, then the `[themes.NAME]` tables of the user file
  in file order. A broken theme file is listed dim with its error and cannot
  be taken. Typing filters by name; `Up` / `Down` select.
- **Preview:** the selected theme applies at once, on the real conversation.
  With the selection on a broken file or no match, the last theme shown stays.
  `Esc` goes back to the theme in effect when the list opened.
- `Enter` takes the theme and saves it into the user file as `tui.theme`, the
  default theme: every TUI started from then on uses it, and open ones switch
  to it on `config_reloaded`. Nothing is saved before `Enter`. Opened from the
  settings box, `Enter` only sets the row there.
- `/theme NAME` does the same without the list; an unknown name is a notice
  that names `/theme`, and the input stays. After `/theme ` the argument list
  offers the theme names.
- With `CODEAGENT_THEME` set, the title says so. A theme taken in the list wins over
  it for the rest of the run, and is saved for later runs.
- With `NO_COLOR` the title says that colours are off.

## Appearance

- Colours come from the theme set in `tui.theme`: `terminal` (default, the
  terminal's 16-colour palette, so it follows the terminal's own theme),
  the built-in `dark` and `light`, or a custom theme. `NO_COLOR` turns
  colours off. Themes and all other TUI settings: [`config.md`](config.md),
  "TUI".
- Unicode symbols (`✓ ✗ ▸ ▾ ●`) fall back to ASCII (`+ x > v *`) when the
  locale is not UTF-8, or always with `tui.symbols = "ascii"`.
- Resizing the terminal re-wraps all text; no content is lost.

### Syntax highlighting

With `tui.syntax_highlight = true` (the default), code is coloured by its
language:

| Place | Language from |
|-------|---------------|
| Code blocks of answers and of questions from the model (prompt bar, ask box) | The fence: ```` ```rust ````, `rs`, `py`, `sh`, `js`, … An indented block or an unknown name is not highlighted. |
| Diffs of `edit_file` and `write_file`: previews, expanded calls, permission questions, the details view, `/diff` | The file's extension (and `Makefile`, `Dockerfile`), else its first line (`#!/bin/sh`). |
| New files of `write_file` | As for diffs. |
| The `$ command` line of `run_command` | Shell. The output is not highlighted. |

- **Diffs:** the `+` and `-` keep the `diff_add` and `diff_remove` colours;
  the code after them is highlighted. Each hunk is highlighted twice, its old
  side (`-` and context lines) and its new side (`+` and context lines), so a
  comment opened on a `-` line does not colour the `+` lines. The `@@` line
  stays `accent`. With true colour and a theme that sets `background`,
  changed lines also get a tint of their diff colour behind them. A file of
  no known language keeps whole lines in the diff colours.
- **Colours** come from the syntax theme the theme's `code` key names
  ([`config.md`](config.md), "Themes"). Without one: `ansi` for `terminal`,
  `base16-ocean.dark` for `dark`, `InspiredGitHub` for `light`. The bundled
  themes use the syntax colours of the same scheme: `atom-one-dark`, `nord`,
  `gruvbox-dark`, `emacs`, `flexoki-dark`, `flexoki-light`,
  `material-oceanic`, `monokai`, `nightfox`, `one-light`, `textual-dark` and
  `tokyo-night` are `codeagent`'s own, made from each scheme's palette
  (keywords, strings, numbers, functions, types, variables, operators, tags,
  comments in italic); `solarized-light` uses `Solarized (light)` and
  `solarized-dark` `Solarized (dark)`. In
  them, as in the published schemes, every word the grammar calls storage
  (`let`, `fn`, `struct`, also `i32`) has the keyword colour, and type
  names (`P` in `struct P`, `String`) the type colour. `ansi`
  is `codeagent`'s own too, on the terminal's 16 colours: keywords magenta, strings
  green, numbers and constants yellow, functions blue, types cyan, comments
  bright black and italic. There the words that start a definition (`fn`,
  `struct`, `enum`, `impl`, `class`) are keywords; other storage words
  (`let`, `i32`; the grammars do not tell them apart) are cyan like the type
  names. Other syntax themes are
  exact with true colour and the nearest of the 256 colours otherwise. Only
  foregrounds, bold and italic are used, never a syntax theme's background.
  `NO_COLOR` turns it off.
- **Loading:** the grammars load in the background when the TUI starts; until
  then code is shown in its plain style, and it is laid out again when they
  are there.
- **Streaming:** an open code block is highlighted line by line as it grows;
  the line without its newline yet keeps the `code` style until it is
  complete. Highlighted texts are cached (the last 512), and a text that grows
  goes on from where it stopped.
- **Limits:** a block or diff side of more than 5000 lines, or with a line
  longer than 2000 characters, is not highlighted.
- Copying is not changed: colours are not copied.
- Languages: those `syntect` bundles (Rust, Python, JavaScript, Go, C, C++,
  Java, Shell, JSON, YAML, Markdown, HTML, CSS, SQL, Ruby, PHP, Makefile, …).
  TOML and TypeScript are not among them; a `ts` fence uses JavaScript.
