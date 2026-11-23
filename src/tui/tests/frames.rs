//! Frames, performance, token counters and the answer marker.

#![allow(clippy::wildcard_imports)]

use super::*;

// Frames.

pub(super) fn idle_session() -> App {
    let mut app = app_with(vec![
        SessionEvent::UserMessage {
            text: "fix the failing test".into(),
            images: Vec::new(),
        },
        SessionEvent::TextDone {
            text: "The test fails because `parse` drops the last line. Let me look.".into(),
        },
        SessionEvent::ToolCall(done(
            call("c1", "read_file", CallState::Running),
            "one\ntwo\nthree",
        )),
        SessionEvent::Usage {
            input_tokens: 12_000,
            output_tokens: 300,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            context_tokens: 12_300,
            model: String::new(),
        },
        SessionEvent::TurnEnd(TurnEnd {
            reason: EndReason::Done,
            requests: 1,
            tool_calls: 1,
            input_tokens: 12_000,
            output_tokens: 300,
            cache_read_tokens: 0,
        }),
    ]);
    app.place = app.place.map(|mut place| {
        place.total = 3;
        place.waiting = 1;
        place
    });
    app
}

pub(super) fn with_prompt(mut app: App) -> App {
    app.update(live(SessionEvent::ToolCall(call(
        "c2",
        "edit_file",
        CallState::Waiting,
    ))));
    let mut question = permission();
    if let QuestionKind::Permission(p) = &mut question.kind {
        p.call.id = "c2".into();
    }
    app.update(live(SessionEvent::Question(Box::new(question))));
    app
}

#[test]
fn frame_idle_80x24() {
    let mut app = idle_session();
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}

#[test]
fn frame_idle_120x40() {
    let mut app = idle_session();
    insta::assert_snapshot!(screen(&mut app, 120, 40));
}

#[test]
fn frame_permission_prompt_with_a_diff() {
    let mut app = with_prompt(idle_session());
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}

#[test]
fn frame_limit_prompt() {
    let mut app = idle_session();
    app.update(live(SessionEvent::Question(Box::new(limit()))));
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}

#[test]
fn frame_command_with_cut_output() {
    let mut app = long_command(30);
    select_last(&mut app);
    app.update(key(KeyCode::Enter));
    insta::assert_snapshot!(screen(&mut app, 80, 30));
}

#[test]
fn frame_ascii_symbols() {
    let events = vec![
        SessionEvent::UserMessage {
            text: "fix the failing test".into(),
            images: Vec::new(),
        },
        SessionEvent::ToolCall(done(
            call("c1", "read_file", CallState::Running),
            "one\ntwo",
        )),
    ];
    let app = App::new(settings(false, true), attachment(events), Instant::now());
    let mut app = with_prompt(app);
    let text = screen(&mut app, 80, 24);
    assert!(text.is_ascii(), "{text}");
    insta::assert_snapshot!(text);
}

#[test]
fn frame_terminal_too_small() {
    let mut app = idle_session();
    let small = screen(&mut app, 39, 10);
    assert!(small.contains("terminal too small"), "{small}");
    assert!(!small.contains("fix the failing test"));
    insta::assert_snapshot!(small);
    // At the smallest size all parts fit, also with a prompt.
    let mut app = with_prompt(idle_session());
    let least = screen(&mut app, 40, 10);
    assert!(!least.contains("terminal too small"), "{least}");
    assert!(
        least.contains("[y] yes") || least.contains("asks"),
        "{least}"
    );
    let mut app = idle_session();
    assert!(screen(&mut app, 40, 9).contains("terminal too small"));
}

#[test]
fn there_is_no_box_around_the_screen() {
    for (unicode, sides) in [(true, "│┌┐└┘├┤"), (false, "|+")] {
        let mut app = App::new(
            settings(unicode, true),
            attachment(Vec::new()),
            Instant::now(),
        );
        let text = screen(&mut app, 80, 24);
        assert!(!text.contains(|c| sides.contains(c)), "{text}");
        let lines: Vec<&str> = text.lines().collect();
        // Lines above and below the input, then the two footer rows.
        let rule = if unicode { "─" } else { "-" }.repeat(80);
        assert_eq!(lines[19], rule);
        assert!(lines[20].starts_with('>'), "{text}");
        assert_eq!(lines[21], rule);
        assert!(lines[22].starts_with("  idle"), "{text}");
        assert!(lines[23].starts_with("  project"), "{text}");
    }
}

#[test]
fn the_cursor_is_in_the_input() {
    let mut app = app();
    typed(&mut app, "abc");
    let mut terminal = screen_of(&mut app, 80, 24);
    let position = terminal.get_cursor_position().unwrap();
    assert_eq!((position.x, position.y), (5, 20));
}

#[test]
fn long_input_grows_and_stays_within_the_screen() {
    let mut app = app();
    for _ in 0..30 {
        app.update(press(KeyCode::Enter, KeyModifiers::ALT));
    }
    typed(&mut app, "last");
    let text = screen(&mut app, 80, 24);
    assert!(text.contains("  last"), "{text}");
    let areas = view::areas(&app, Rect::new(0, 0, 80, 24)).unwrap();
    assert_eq!(areas.input.height, 6);
    assert!(areas.conversation.height > 0);
}

// Performance.

/// A frame must not reflow everything. The bound is generous, for a debug
/// build on a busy machine: full reflows of 10 000 lines per frame exceed it.
#[test]
fn ten_thousand_lines_scroll_quickly() {
    let mut app = app();
    let text: String = (0..1000)
        .map(|i| format!("Paragraph {i} with some `code` in it.\n\n"))
        .collect();
    for i in 0..10 {
        app.update(live(SessionEvent::UserMessage {
            text: format!("question {i}"),
            images: Vec::new(),
        }));
        app.update(live(SessionEvent::TextDone { text: text.clone() }));
    }
    let size = Rect::new(0, 0, 120, 40);
    app.layout(size);
    assert!(app.conversation.total() >= 10_000);
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    let start = Instant::now();
    for _ in 0..100 {
        app.update(key(KeyCode::PageUp));
        app.layout(size);
        terminal.draw(|frame| view::draw(&app, frame)).unwrap();
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(3),
        "100 frames took {elapsed:?}"
    );
    // A streaming delta only rebuilds its own entry.
    let start = Instant::now();
    for _ in 0..100 {
        app.update(live(SessionEvent::TextDelta { text: "x".into() }));
        app.layout(size);
        terminal.draw(|frame| view::draw(&app, frame)).unwrap();
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(3),
        "100 frames took {elapsed:?}"
    );
}

// Token counters.

pub(super) fn usage(input_tokens: u64, output_tokens: u64, context_tokens: u64) -> SessionEvent {
    SessionEvent::Usage {
        input_tokens,
        output_tokens,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        context_tokens,
        model: String::new(),
    }
}

pub(super) fn totals(input_tokens: u64, output_tokens: u64) -> Option<TokenTotals> {
    Some(TokenTotals {
        input_tokens,
        output_tokens,
        cache_read_tokens: 0,
    })
}

/// The session row, at a width where nothing is cut.
pub(super) fn session_row(app: &mut App) -> String {
    let text = screen(app, 200, 24);
    text.lines().last().unwrap().trim().to_owned()
}

#[test]
fn counters_are_hidden_before_the_first_usage() {
    let mut app = app();
    assert_eq!(app.usage, None);
    assert!(!session_row(&mut app).contains(" in "));
    // Zero tokens reported is still a report.
    app.update(live(usage(0, 0, 0)));
    assert!(session_row(&mut app).ends_with("· in 0 out 0"));
}

#[test]
fn counters_sum_live_and_replayed_events() {
    let mut app = app_with(vec![usage(40_000, 3_000, 12_000)]);
    assert_eq!(app.usage, totals(40_000, 3_000));
    app.update(live(usage(5_100, 200, 12_300)));
    assert_eq!(app.usage, totals(45_100, 3_200));
    assert_eq!(app.context_tokens, 12_300);
    assert_eq!(
        session_row(&mut app),
        "project · ~/ws/project · anthropic/sonnet · 1/1 · 12.3k/200k · in 45.1k out 3.2k"
    );
}

#[test]
fn a_reconnect_does_not_double_the_counters() {
    let log = vec![usage(100, 10, 110), usage(200, 20, 220)];
    let mut app = app_with(log.clone());
    assert_eq!(app.usage, totals(300, 30));
    app.update(Input::Disconnected);
    // The session info carries the totals too; only the log may count.
    let mut session = info();
    session.usage = totals(300, 30).unwrap();
    let again = Attachment {
        session: session.clone(),
        sessions: vec![session],
        backlog: log.into_iter().map(replayed).collect(),
    };
    app.update(Input::Attached(Box::new(again)));
    assert_eq!(app.usage, totals(300, 30));
    assert_eq!(app.context_tokens, 220);
}

#[test]
fn counters_are_cut_first_on_a_narrow_terminal() {
    let mut app = app_with(vec![usage(45_100, 3_200, 12_300)]);
    let text = screen(&mut app, 80, 24);
    let row = text.lines().last().unwrap();
    assert!(row.contains("12.3k/200k"), "{row}");
    assert!(!row.contains("out 3.2k"), "{row}");
}

#[test]
fn a_model_without_a_known_window_shows_the_size_alone() {
    let mut app = app();
    app.settings.windows.clear();
    // Nothing known yet: no size to show.
    assert_eq!(
        session_row(&mut app),
        "project · ~/ws/project · anthropic/sonnet · 1/1"
    );
    app.update(live(usage(12_000, 300, 12_300)));
    assert_eq!(
        session_row(&mut app),
        "project · ~/ws/project · anthropic/sonnet · 1/1 · 12.3k · in 12k out 300"
    );
}

// The answer marker (RFC 0022).

pub(super) fn answers() -> Vec<SessionEvent> {
    vec![
        SessionEvent::UserMessage {
            text: "fix the failing test".into(),
            images: Vec::new(),
        },
        SessionEvent::TextDone {
            text: "The test fails because `parse` drops the last line of the input, \
                   which the loop below never reaches:\n\n\
                   - `lines.len() - 1` stops one early\n\
                   - the last line has no newline\n\n\
                   ```rust\nfor i in 0..lines.len() - 1 {\n```\n\nLet me look."
                .into(),
        },
        SessionEvent::ToolCall(done(
            call("c1", "read_file", CallState::Running),
            "one\ntwo",
        )),
        SessionEvent::TextDone {
            text: "Fixed.".into(),
        },
    ]
}

#[test]
fn frame_answers_with_their_marker() {
    let mut app = App::new(settings(true, false), attachment(answers()), Instant::now());
    let text = screen(&mut app, 60, 24);
    assert!(text.contains("● The test fails"), "{text}");
    assert!(text.contains("● Fixed."), "{text}");
    // Wrapped rows, list items and code are indented with the text.
    for row in text.lines().filter(|l| l.contains("lines.len()")) {
        assert!(row.starts_with("  "), "{row:?}\n{text}");
    }
    insta::assert_snapshot!(text);
}

#[test]
fn frame_answers_with_their_marker_ascii() {
    let mut app = App::new(
        settings(false, false),
        attachment(answers()),
        Instant::now(),
    );
    let text = screen(&mut app, 60, 24);
    assert!(text.is_ascii(), "{text}");
    assert!(text.contains("* The test fails"), "{text}");
    insta::assert_snapshot!(text);
}

#[test]
fn a_streaming_answer_has_its_marker_from_the_first_row() {
    let mut app = App::new(
        settings(true, false),
        attachment(Vec::new()),
        Instant::now(),
    );
    app.update(live(SessionEvent::TextDelta { text: "Let".into() }));
    let text = screen(&mut app, 60, 12);
    assert!(text.contains("● Let"), "{text}");
}

#[test]
fn a_selection_leaves_out_the_answer_marker_and_indent() {
    let mut app = previewing(answers());
    let first = row_of(&mut app, "● The test fails");
    let last = row_of(&mut app, "● Fixed.");
    app.update(mouse(MouseEventKind::Down(MouseButton::Left), 0, first));
    app.update(mouse(MouseEventKind::Drag(MouseButton::Left), 79, last));
    let effects = app.update(mouse(MouseEventKind::Up(MouseButton::Left), 79, last));
    let copied = copies(&effects).concat();
    assert!(copied.starts_with("The test fails because"), "{copied:?}");
    assert!(copied.contains("\n• lines.len() - 1"), "{copied:?}");
    assert!(copied.contains("\n\nread src/parse.rs\n\n"), "{copied:?}");
    assert!(copied.ends_with("Fixed."), "{copied:?}");
    assert!(!copied.contains('●'), "{copied:?}");
    assert!(!copied.contains("\n  "), "{copied:?}");
}
