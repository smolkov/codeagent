# Cheap tools: read less, reuse results

- RFC: 0041
- Status: draft
- Milestone: Context management, with RFC 0028
- Spec: [`spec/tools.md`](../spec/tools.md), every tool section,
  "Result size", "Display", "Paths";
  [`spec/agent-loop.md`](../spec/agent-loop.md), "System prompt",
  "Trimmed results";
  [`spec/config.md`](../spec/config.md), "Permissions";
  [`spec/providers.md`](../spec/providers.md);
  RFC 0008 (trimming), RFC 0028 (cached input, bench), RFC 0012 (a
  new tool)

## Summary

Three tool features that cut the input tokens of a turn, in the order
they pay off. None changes what the model may do; each changes how many
characters of the workspace leave the tool layer.

1. **A tree argument of `list_files`.** `tree: true` lists files
   recursively as an outline (directories once, entries below), not
   one full path per line. Getting the lay of a repository costs a
   tenth of the text: a thousand paths become a few hundred lines with
   the prefix written once.
2. **A `ranges` argument of `search_files`.** `ranges: 0` answers with
   `path:line` positions only; `ranges: N` keeps at most `N`
   characters around each match instead of the fixed 300 of today.
   "Where do I look next" stops paying for line text the next
   `read_file` returns anyway.
3. **An `outline` operation for `lsp`**: a symbol map of a directory
   from `document_symbols`, the shape the model needs to pick files
   instead of reading them.

All three are measured with `ci/bench.sh` (RFC 0028, "Measuring"):
kept only when no task is done less often and the input tokens fall.
Step 3 is **(proposal)**: if the bench says `ranges` and the tree cover
navigation, it is dropped.

## Motivation

The tools are honest but wasteful, and the waste is input tokens:

- `list_files` answers with one full path per line, up to 1 000
  entries (`MAX_ENTRIES`, `src/tools/list_files.rs:17`). A model asked
  to find the tests reads a wall of `tests/foo/bar.rs` lines whose
  shared prefixes say more than the lines themselves. Ten files in
  one directory are one line in a tree and ten lines today.
- `search_files` answers with up to 200 matching lines at 300
  characters each (`MAX_MATCHES` and `LINE_CHARS`,
  `src/tools/search_files.rs:21`), inside the 30 000-character cap of
  the spec. Most of a match is the line's neighbours, not the match:
  when the model wants positions, the next call is `read_file` on a
  few of them and the line text was spent for nothing.
- The agent has no cheap way to see code shape: `document_symbols`
  does it for one file (`Operation::DocumentSymbols`,
  `src/tools/lsp.rs`), the workspace-wide alternatives are reading
  files or guessing. The cost of finding where to edit dominates a
  turn's input: three of the four bench tasks under `bench/tasks/`
  are navigation problems (`find-the-caller` solves nothing else).

The big wins are already taken and stay taken: the prompt prefix is
shared and cached (RFC 0028, step 3), old results go out as
placeholders (RFC 0008), command output is cleaned before it counts
(RFC 0028, step 4). What is left is the first read of the workspace
itself, and that is what these three features cut.

## Scope

In: the three features, the tool descriptions, their spec sections,
bench numbers per step.

Out: request caching and compaction (RFC 0028), trimming and
placeholders (RFC 0008), new tools and new permission rows (the
features ride the tools that exist).

## Design

### 1. `tree` argument of `list_files`

```json
{ "path": "src", "pattern": "**", "tree": true }
```

- `tree` (default `false`) lists recursively: each directory once,
  ending in `/`, its files below it indented one level. The entry
  list, not an `ls`-style column format, keeps it compact.
- The walk and rules do not change (`src/tools/walk.rs`:
  `.gitignore` respected, `.git` skipped, links not followed, `deny`
  rules applied per entry).
- `MAX_ENTRIES` still counts shown files, and the
  `… more entries; use a narrower path or pattern` line stays, so a
  tree grows no further than a flat list. Directories are not counted,
  they are the outline.

Result, `path: "src"` on this project:

```
src/agent/
  history.rs
  mod.rs
tools/
  list_files.rs
  mod.rs
```

### 2. `ranges` argument of `search_files`

```json
{ "pattern": "fn parse\\(", "ranges": 0 }
```

- `ranges` (default: today's form) sets how much of a matching line
  follows the `path:line:` prefix:
  - `0`: positions only, one match per line, the closing line says
    how many files matched.
  - `N`: at most `N` characters around the match.
  - unset: the full line, cut to 300 characters, as today.
- The caps stay: 200 matches, the `… more matches in F files` closing
  line (`src/tools/search_files.rs:128`), the 30 000-character result
  cap of the spec.
- The result shapes are a prefix matter: `specified, shown and
  expanded` (the "Shown" rules of the spec) work unchanged, and a
  permission question shows `pattern` and `path` as today.

### 3. `outline` operation of `lsp` **(proposal)**

```json
{ "operation": "outline", "path": "src" }
```

- One answer for a directory: the top symbols of each file the
  language server has for it, in the compact form
  `document_symbols` already builds today (`outline()`,
  `src/tools/lsp.rs:1159`: `struct Parser            12-40`), at most
  500 symbols as for `workspace_symbols`, `(no symbols)` when the
  directory is empty.
- The files are the ones `list_files` would list: the same walk, the
  same per-file permission check with the `read_file` rules. This is
  **not** a workspace-wide symbol search; `workspace_symbols` stays
  the way to find a name.
- The one-tool rule holds (`spec/tools.md`, "lsp"): one more
  `operation` in the argument list, no new wire, no new permission
  row. The model description of `operation` gains `outline` with one
  sentence.

### 4. Result size, trimmed results, display

- The 30 000-character cap and the 20 000/10 000 split of
  `spec/tools.md`, "Result size" apply to trees and outlines as to any
  result; when a cap cuts one, the result ends with the narrower call
  to make (the `… string` of `search_files` today).
- Placeholders of older answers (`spec/agent-loop.md`, "Trimmed
  results") are orthogonal: a trimmed step's tree is a placeholder
  like any other result; the journal keeps the full text for the TUI
  (`History::keep_full`, `src/agent/history.rs`).
- The three additions are arguments to existing tools, so the tool
  list the model sees stays nine and the "Display" table of the spec
  stays as it is.

### The tools sell themselves

One sentence each in the tool description to the model (they live in
`src/tools/lsp.rs` and friends, not here): `tree` for the shape of a
directory, `ranges: 0` when positions are enough, `outline` for a
symbol map. The model that knows the cheap call makes it; the system
prompt's rule (RFC 0028, "Local texts") asks for the cheaper call
before the wider one.

## Dependencies

- The tree and the ranges are text changes in the tools' result
  builders; no new crate.
- `outline` speaks `document_symbols` to a running language server,
  per file, collected (`src/tools/lsp.rs`); the `Lsp` environment and
  the project lookup are already needed by the operations the spec
  lists. `outline` errors with the same "no language server for
  file.rs" result as the other `lsp` operations when none is
  configured.
- The bench of RFC 0028 exists (`ci/bench.sh`); each step needs one
  run set, no code.

## Spec changes

- [`spec/tools.md`](../spec/tools.md):
  - "list_files": a `tree` row in the argument table, the result
    shape, the cap and closing line, "Display": `ls PATH … as tree`.
  - "search_files": a `ranges` row and the three result forms.
  - "lsp": an `outline` row in the `operation` table, the 500-symbol
    cap, the per-file permission column.
  - "Result size": trees and outlines name the narrower call when the
    cap cuts them.
- [`spec/agent-loop.md`](../spec/agent-loop.md), "System prompt": the
  three one-sentence pitches.
- [`spec/config.md`](../spec/config.md), "Permissions": nothing new;
  the summary notes the tree rides the `list_files` rules and the
  outline the `read_file` rules.

## Testing

- `src/tools/tests.rs`: a `list_files` `tree` result on a small tree,
  a narrow `pattern`, a deny rule hiding a directory; `search_files`
  `ranges: 0` positions only, `ranges: 40` around the match, the closing
  line, binary files skipped as today; the `lsp` `outline` answer over
  the fake server the `lsp_tests` already run, the empty and cut
  cases.
- `src/tools/lsp_tests.rs`: `outline` next to the existing
  `document_symbols` case, and the no-server error.
- Bench, by hand, not in the pipelines (RFC 0028):
  `ci/bench.sh --model anthropic/sonnet --runs 3` before and after
  each step. A step is kept only when the tasks' success holds and
  the input tokens fall. The sums are written under
  `bench/readme.md`, "Results", with the date and the commit.
- The three features are additive arguments and operations; loading
  old sessions must keep working unchanged, covered by the store's
  loading tests for any session that mentions them.

## Implementation plan

1. **`tree` argument of `list_files`**
   - Status: todo
   - Files: `src/tools/list_files.rs`, `src/tools/tests.rs`,
     `spec/tools.md`, `bench/readme.md`
   - Result: recursive listings as an outline, walk and rules
     unchanged
   - Tests: the tools tests above; bench numbers for the navigation
     tasks
2. **`ranges` argument of `search_files`**
   - Status: todo
   - Files: `src/tools/search_files.rs`, `src/tools/tests.rs`,
     `spec/tools.md`, `bench/readme.md`
   - Result: position-only and short-range match lines
   - Tests: the tools tests above; bench numbers as above
3. **`outline` operation of `lsp` (proposal)**
   - Status: todo
   - Files: `src/tools/lsp.rs`, `src/tools/lsp_tests.rs`,
     `spec/tools.md`, `spec/agent-loop.md`, `bench/readme.md`
   - Result: a symbol map of a directory in one bounded answer
   - Tests: the lsp tests above; bench against step 2's numbers, and
     drop the step if input rises or success falls

## Alternatives

- **A new `dir_tree` tool.** Dropped: a new tool means a permission
  row, a display name, a rule in every user's configuration and a
  line in the model's tool list for a feature an argument to an
  `allow`-by-default tool covers. The project's standing decision is
  the opposite: `lsp` is one tool with operations "so the model sees
  one tool, not ten" (`spec/tools.md`, "lsp").
- **A fuzzy finder tool.** Dropped: relevance guesses hide paths the
  model must still check; the honest cheap result is the full list of
  names in fewer characters, plus the model's own `search_files` and
  `workspace_symbols`.
- **Making lines longer in `search_files` so the model sees more per
  match.** Dropped: it spends more per match, not less; `ranges: N`
  gives the model the dial with the default kept.
- **Compaction: rewriting old tool results into summaries in place.**
  Dropped: that is the trimming and placeholder machinery of RFC 0008
  and the accepted risk that fresh requests keep caching older
  results; a compaction RFC of its own would need its own security
  walk (the boundary of what the model still sees). Out of scope.

## Open questions

- **`outline` (step 3): keep it?** **(proposal: keep, the bench
  decides)** if steps 1 and 2 already cut the navigation tasks'
  input, step 3 is not built.
- **The tree's budget:** count files only, as today, or one shared
  budget for files and directories? **(proposal: files only)**
- **`ranges` default:** keep the 300-character lines as the default
  and positions as opt-in, or make positions the default?
  **(proposal: keep today's default; changing it moves the bench
  first)**
