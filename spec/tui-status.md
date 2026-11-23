# TUI: status

Token counters and the status box. Part of the [TUI](tui.md) specification.

## Token counters

The session row ends with the tokens the session has used. They are the first
thing cut on a narrow terminal, and left out until the first `usage` event.

- **Input** and **output** are the sums over all requests of the session.
  Input counts the history again with every request, because that is what is
  sent; it is not the context size.
- Anthropic's cached input is included in input, so the counter is not a bill.
- A cancelled request counts what the provider had reported by then. A provider
  that reports nothing adds nothing.

The TUI sums the `usage` events of the log it replays, and starts again on a
reconnect, so a reconnect never counts anything twice.

When the provider retries a request, the conversation shows a notice:
`retrying (2/3): HTTP 503: busy`.

## Status box

The status box has two tabs, named in its title: the session's name, and
**Global**. `/status` opens it on the first, `/stats` on the second.

`/status [NAME]` opens a box with the status of a session and statistics over
its whole log, from the `session_status` request (see
[`architecture.md`](architecture.md), "Protocol"). `NAME` is completed from the
session list, the attached session included; a name that is not in the list
stays in the input with a notice. A saved session is shown without being
resumed. The box opens at once ("asking the server…") and fills with the
answer; a first answer that fails closes it with the error.

```
┌ Status ─ project │ Global ───────────────────────────────────────────────────┐
│ Session                                                                      │
│   name         project  (id 1)                                               │
│   state        working                                                       │
│   workspace    ~/ws/project                                                  │
│   extra dirs   ~/ws/shared-lib  ⚠ missing                                    │
│   scratch      /run/user/1000/codeagent/sessions/…/tmp  (34 kB)              │
│   created      2026-11-01 01:00  (2h 10m ago)                                │
│   last active  2h 08m ago                                                    │
│   clients      2: chat pts/5 (watching), tui pts/3 (this)                    │
│   queued       1 message                                                     │
│   saved in     ~/.local/state/codeagent/sessions/1  (1.2 MB)                      │
│                                                                              │
│ Model                                                                        │
│   model        anthropic/sonnet  (claude-sonnet-5-5)                         │
│   api          anthropic  https://api.anthropic.com                          │
│   key          set in the environment                                        │
│   context      170k / 200k  █████████████████░░░  85%                        │
│   settings     temperature default ─ max output default ─ max_retries 3      │
│   changes      1; first anthropic/haiku                                      │
│                                                                              │
│ This turn                                                                    │
│   requests     0 / 50                                                        │
│   tool calls   0 / 120                                                       │
│   running      2h 08m                                                        │
│                                                                              │
│ Totals                                                                       │
│   turns        1: 1 done                                                     │
│   requests     1 ─ 1 retry                                                   │
│   tokens       in 2.1M ─ out 84.3k                                           │
│   working      2h 09m of 2h 10m                                              │
│                                                                              │
│ Tool calls     calls  done  error  denied  cancelled                         │
│   read_file        2     2      0       0          0                         │
│   edit_file        1     0      0       1          0                         │
│   all              3     2      0       1          0                         │
│                                                                              │
│ Questions      1: 1 yes                                                      │
│                by tui pts/3 1                                                │
│                                                                              │
│ Server         0.4.0 ─ pid 4242 ─ 3 sessions, 2 running ─ 3 clients          │
│                                                                              │
│ Tab global ─ Up Down PgUp PgDn scroll ─ Esc close                            │
└──────────────────────────────────────────────────────────────────────────────┘
```

- The `secrets` row counts the secrets hidden from the model in the
  session (`3 hidden from the model`; see [`agent-loop.md`](agent-loop.md),
  "Secrets"); it is left out at zero.
- Rows that do not apply are left out: extra dirs when there are none,
  scratch (the session's scratch directory and the size of its files, see
  [`sessions.md`](sessions.md), "Scratch directory") when it does not exist, the
  clients and "This turn" of a saved session, "queued" without a queue,
  "changes" when the model never changed. "This turn" is shown while a turn
  runs; its counts start again after `continue` on a limit question.
- The model rows are the configuration in effect now for the session's
  workspace; a model that is no longer configured is marked. The key row says
  where the key comes from; `api_key_cmd` is not run. The context row turns to
  the warning colour from 80 % of the context window on.
- Times come from the event log. A log of an older `codeagent` has none: its
  stretches count as unknown, and "working" then shows `≥`; a session with no
  known time shows `?`. "working" counts the time in `working`, not the time
  waiting for an answer.
- "requests" counts the model requests that answered: from `usage` events, and
  from the counts at the end of a turn for requests that reported no usage.
  "retries" counts `retry` events, "error" the `error` events.
- The box updates while it is open: every second while the session works, and
  at the next second after an event of the attached session changed it. It
  does not poll an idle session. A refresh that fails keeps what is shown.
- `Up` / `Down`, `PgUp` / `PgDn` and `Home` / `End` scroll a box taller than
  the screen; `Esc`, `q` and `Ctrl-C` close it. A question that opens closes
  the box, as it closes the other boxes; so does a switch to another session,
  and the end of the session it shows.
- With ASCII symbols, the bar is `[####......]`, separators are `-` and the
  warning mark is `!`.
- `Tab`, `Shift-Tab`, `Left` and `Right` switch the tab. Each tab keeps its
  scroll position. The tab in use is bold in the title. A box opened by
  `/stats` asks for the session's status when its tab is first shown.

### Global tab

`/stats [RANGE]` opens the status box on the Global tab: statistics over the
logs of all saved sessions of the server, from the `global_stats` request (see
[`architecture.md`](architecture.md), "Protocol"; how they are counted is in
[`sessions.md`](sessions.md), "Statistics"). `RANGE` is `7d` (the default),
`30d` or `all`; another word stays in the input with its usage. It works for
all sessions, whichever is attached.

```
┌ Status ─ project │ Global ─────────────────────────── 7 days │ 30 days │ all ┐
│ Totals         2026-11-04 – 2026-11-10                                       │
│   sessions     12 active of 31 ─ 4 running                                   │
│   turns        87: 80 done, 3 cancelled, 2 limit, 2 failed                   │
│   requests     1.4k ─ 21 retries ─ 3 errors                                  │
│   tokens       in 10.9M ─ out 340k                                           │
│   working      6h 40m                                                        │
│                                                                              │
│ Models            sessions  requests      in     out   share                 │
│   anthropic/sonnet       9      1.1k    9.9M    300k   ██████████░░  90%     │
│   openai/gpt-old         4       300      1M     40k   █░░░░░░░░░░░   9%     │
│   all                   12      1.4k   10.9M    340k                         │
│                                                                              │
│ Tokens per day  █ in  █ out                                                  │
│    5M ┤                                                                      │
│       │                                    ▅▅▅▅▅▅▅▅                          │
│       │                                    ████████                          │
│       │                                    ████████                          │
│  2.5M ┤         ▆▆▆▆▆▆▆▆                   ████████ ▂▂▂▂▂▂▂▂                 │
│       │         ████████                   ████████ ████████                 │
│       │▇▇▇▇▇▇▇▇ ████████          ▄▄▄▄▄▄▄▄ ████████ ████████                 │
│       │████████ ████████          ████████ ████████ ████████ ▇▇▇▇▇▇▇▇        │
│     0 ┼───────────────────────────────────────────────────────────────       │
│        Wed      Thu      Fri      Sat      Sun      Mon      Tue             │
│        max 4.1M on Sun 11-08 ─ avg 1.6M                                      │
│                                                                              │
│ Tab session ─ 7 3 a range ─ R count again ─ Up Down scroll ─ Esc close       │
└──────────────────────────────────────────────────────────────────────────────┘
```

- **Range:** the last 7 or 30 days, today included, or all time, in the
  client's time zone. At the right of the title, the range in use is bold; a
  box too narrow for them names only that one after the tabs. `7`, `3` and `a`
  choose it, `r` goes to the next one. Each answer is kept while the box is
  open, so a range shown before comes back at once.
- **Totals:** the dates of the range; the sessions with an event in it, of all
  saved sessions, and how many run now; turns, requests, retries, errors and
  tokens as on the Session tab; the time in `working` in the range, with `≥`
  when some stretch has no time. A log that cannot be read is left out and
  counted in a warning row "skipped".
- **Models:** one row per `provider/model`, most tokens first: the sessions
  that used it in the range, its requests, input and output tokens, and its
  share of all tokens with a bar in the `user` colour (`[###.....]` in ASCII).
  A model that is no longer configured is dim; tokens that cannot be given to a
  model are in a row `unknown`. With more than one model, a row `all` sums
  them. No model used: `Models  none`.
- **Tokens per day:** a bar chart, input (`user` colour) and output (`accent`
  colour) stacked, 8 rows high, the top of a bar in eighth blocks; in ASCII
  `#` and `+` in whole cells. The y axis is labelled at 0, the middle and the
  top, which is 1, 2, 4, 5 or 8 times a power of ten. One column per day, as
  wide as the box allows (at most 8 cells and a gap); days without tokens are
  empty columns. Labels: the weekday for 7 days, `MM-DD` every 7th day for 30
  days. For all time, a column is a day while they fit, then a week (from
  Monday), then a month (`YYYY-MM`; the oldest are left out when even those do
  not fit); the line under the chart says so. That line names the highest
  column and the mean per day of the range. Tokens of events without a time
  are not in the chart; a line says how many there are. A range without tokens
  says `no tokens in this range`; a box narrower than 40 cells leaves the
  chart out and says so.
- **Asking:** the tab asks with `global_stats` when it is shown and the range
  has no answer yet, and when the range changes ("counting the logs of all
  sessions…" until it comes). It does not poll: `R` counts again, and the
  title says how old an answer is once it is older than a minute
  (`Global ─ 2m 30s old`). A refresh that fails keeps what is shown; a first
  answer that fails shows the error in the tab, or closes the box with it when
  nothing else is shown.
