# TUI: input

The input editor, the command list and the commit dialog. Part of the [TUI](tui.md) specification.

## Input editor

- `Enter` sends the message; `Shift-Enter` or `Alt-Enter` inserts a newline.
  **(assumption: terminals that cannot report `Shift-Enter` use `Alt-Enter`)**
- Standard line editing: arrows, `Home`/`End`, `Ctrl-A`/`Ctrl-E`,
  `Ctrl-W` delete word, `Ctrl-U` delete to line start.
- `Up`/`Down` on the first/last line of the input browse previous messages of
  this session.
- Pasted text is inserted as is, including newlines (bracketed paste); it is
  not sent until `Enter`.
- Images can be part of a message; see [Images](tui-conversation.md#images).
- The user may type and send while a turn runs. The message goes to the server
  at once, which queues it until the turn ends (other clients see the queue
  too); the TUI shows it as "queued" until the turn that takes it starts.
- Lines starting with `/` are commands for `codeagent`, not messages to the model:

| Command    | Action |
|------------|--------|
| `/help`    | Show keys and commands. |
| `/clear`   | Start a new conversation in the same session. |
| `/model [PROVIDER/MODEL]` | Change the session's provider and model; without an argument, open the model picker. With `--default` (before or after the model), also save it as `default_model`; `/model --default` saves the session's model without changing it. |
| `/permissions` | Show the permission rules in effect: configuration, project file and session rules, with their source. |
| `/sessions` | Open the session list. |
| `/new [NAME]` | Start a new session in the same workspace and attach to it; the model picker opens on `default_model` first. |
| `/attach [NAME]` | Attach to another session; without a name, open the session list. A name that is not in the list stays in the input with a notice. |
| `/rename NAME` | Rename the attached session. |
| `/status [NAME]` | Open the status box: the full status of the attached session, or of session `NAME`, running or saved, and its statistics. See [Status box](tui-status.md#status-box). |
| `/stats [RANGE]` | Open the status box on its Global tab: statistics over all sessions of the server, for `7d` (the default), `30d` or `all`. See [Global tab](tui-status.md#global-tab). |
| `/skills [NAME]` | Choose the session's skills in the skills box; with a name, turn that skill on or off. See [Skills](tui-boxes.md#skills). |
| `/lsp` | Open the language server box: the servers of the codeagent server, their state and the sessions using them; restart or stop one. See [Language servers](tui-boxes.md#language-servers). |
| `/settings` | Open the settings box: the `[tui]` settings, changed with a preview and saved into the user file. See [Settings](tui-settings.md#settings). |
| `/theme [NAME]` | Open the theme list; with a name, switch to that theme. Either way the theme is saved as the default. See [Theme list](tui-settings.md#theme-list). |
| `/kill`, `/delete` | Delete the attached session, after the confirmation of [Delete](tui-sessions.md#delete), and open the session list. |
| `/commit [HINT]` | Generate a commit message for all changes in the workspace and commit them after confirmation; see [Commit](#commit) and [`commit.md`](commit.md). |
| `/copy [N\|code]` | Copy the last answer of the model as Markdown; `N` the answer N back (`/copy 2` is the one before the last); `code` the last code block of the last answer. See [Copy](tui-conversation.md#copy). |
| `/diff` | Open the details view with the diffs of all edits and writes of the last turn, one after the other. |
| `/image PATH` | Add the image file `PATH` (relative to the workspace, or `~/…`) to the message at the cursor. See [Images](tui-conversation.md#images). |
| `/detach`, `/quit` | Detach and exit the TUI; the session keeps running. |

### Command list

Typing `/` into the empty input, with focus on the input, opens the command
list above the input. Pasted text and a message from the history do not open it.

```
──────────────────────────────────────────────────────────────────────────────
▶ /model [PROVIDER/MODEL]  Change the model; without one, choose it
  /permissions             later version
  /rename NAME             Rename the session
  /commit [HINT]           later version
──────────────────────────────────────────────────────────────────────────────
> /m_
──────────────────────────────────────────────────────────────────────────────
```

- The list shows every command with its arguments and a one-line
  description, in the order of the table above. A command that is not built yet
  is dim and says `later version`; `Enter` on it says so too.
- It lies over the last rows of the conversation and shows at most 8 rows; the
  selected row stays visible. The layout does not change, so the conversation
  does not jump.
- Each further character searches the list: commands with a name that contains
  the typed text, in any case, those that start with it first, each in table
  order. The best match is selected. `/quit` shares a row with `/detach`, and
  either name finds it.
- If nothing matches, the list shows "no command" and `Enter` sends nothing;
  the input stays so it can be corrected. To send a message that starts with
  `/` to the model, start it with `//`; the first `/` is removed, and the second
  `/` closes the list. **(assumption)**
- A space or a new line closes the list too: arguments follow.
- A question that opens, and `Ctrl-O`, close the list and keep the text. Keys
  belong to the prompt bar or the picker then.
- A closed list opens again only from an empty input. With it closed, `Enter`
  runs the text as it stands; an unknown command gets a notice and stays in the
  input, and `/rename` without a name gets its usage.

| Key in list         | Action |
|---------------------|--------|
| `Up` / `Down`       | Select the previous / next command; they stop at the ends. |
| `Tab`               | Complete the selected command into the input and close the list, so arguments can be typed. With no match, nothing. Focus does not change. |
| `Enter`             | Run the selected command. If it needs an argument, complete it like `Tab` instead. |
| `Esc`               | Close the list and nothing else: the typed text stays and a running turn goes on. A second `Esc` cancels the turn. |
| `Backspace` on the `/` | Delete the `/` and close the list. |

Other keys work as without the list: the conversation scrolls, and `Ctrl-C`
cancels the turn or clears the input, which closes the list.

Every box the user can open has a command in the table above. The prompt bar has
none: it opens when the agent asks a question.

After `/attach `, `/model `, `/skills `, `/stats `, `/status ` and `/theme `
(typed, or completed with `Tab`), a list of values lies in the same place:
`/attach` offers the other sessions, with their workspace and state, `/model`
the server's models, with their id and which one is `current` and `default`,
`/skills` the skills that can be turned on, with their description and
`active` before it for active ones, `/stats` the ranges, `/status` the
sessions, `/theme` the themes, with where they come from. Typing searches them
like commands, by name, and models and skills also by their id or
description. `Up` / `Down` select, `Tab` completes
the selected value, `Enter` runs the command with it, and with no match
(`no match`) `Enter` runs the text as typed. `Esc` closes the list until the
input no longer is such a command. A second word closes it.

While the command list is open, `Up`/`Down` select in the list instead of
browsing the message history.

### Commit

`/commit [HINT]` commits all changes in the session's workspace with a
message written by the model. Nothing is committed without the user's
confirmation. Which changes are included and how the message is generated is
described in [`commit.md`](commit.md); `codeagent commit` on the command line uses
the same rules but only prints the message.

Steps:

1. Collect the changes of the workspace and generate the message (see
   [`commit.md`](commit.md)). If there is nothing to commit or the directory
   cannot be committed, a notice explains why and nothing else happens.
   The request and answer are not added to the conversation history; the
   conversation only gets a notice. **(assumption)**
2. Confirm: the message is shown in a dialog together with the list of files
   that will be committed:

   ```
   ┌ Commit 3 files in ~/ws/project ──────────────────────────────────────────┐
   │ Fix last-line loss in parse                                              │
   │                                                                          │
   │ The loop stopped one line early, so the last line of every input was     │
   │ dropped. Iterate over all lines and add a test for single-line input.    │
   │                                                                          │
   │ M src/parse.rs   A tests/parse_single_line.rs   M Cargo.lock             │
   ├──────────────────────────────────────────────────────────────────────────┤
   │ Enter commit ─ e edit ─ r regenerate ─ d diff ─ Esc cancel               │
   └──────────────────────────────────────────────────────────────────────────┘
   ```

   | Key     | Action |
   |---------|--------|
   | `Enter` | Commit with the shown message. |
   | `e`     | Edit the message in the dialog (multi-line editor, same keys as the input). |
   | `r`     | Generate a new message. |
   | `d`     | Show the full diff that will be committed. |
   | `Esc`   | Cancel; nothing is staged or committed. |

3. Commit: stage the changes of the workspace (`git add -A -- <workspace>`)
   and commit only them (`git commit -F - -- <workspace>`). Changes already
   staged outside the workspace stay staged but are not committed. Git hooks
   run as usual; `codeagent` never passes `--no-verify`.
4. Result: the conversation gets a notice with the short hash and subject. If
   git fails (hook rejects, no `user.name`), its output is shown and the
   dialog opens again with the message, so the user can retry or cancel.

`/commit` never pushes, amends, or changes branches. If files change while
the dialog is open, `Enter` first shows "changes differ from the generated
message" and offers `r` regenerate or `Enter` commit anyway.

With a remote provider, the dialog header names the provider that received
the diff.
