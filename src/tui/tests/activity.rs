//! The activity line.

#![allow(clippy::wildcard_imports)]

use super::*;

// The activity line.

pub(super) fn state_event(state: State) -> Input {
    live(SessionEvent::State { state })
}

/// A turn that started 14 s ago, with 1.2k tokens of text so far.
pub(super) fn thinking() -> App {
    let mut app = app();
    app.update(state_event(State::Working));
    app.update(live(SessionEvent::UserMessage {
        text: "fix the failing test".into(),
        images: Vec::new(),
    }));
    app.update(live(SessionEvent::TextDelta {
        text: "The test fails because `parse` drops the last line. Let me look.".into(),
    }));
    app.update(live(SessionEvent::TextDone {
        text: "The test fails because `parse` drops the last line. Let me look.".into(),
    }));
    app.update(live(SessionEvent::Usage {
        input_tokens: 4000,
        output_tokens: 1200,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        context_tokens: 5200,
        model: String::new(),
    }));
    app.update(live(SessionEvent::ToolCall(call(
        "c1",
        "read_file",
        CallState::Running,
    ))));
    app.update(live(SessionEvent::ToolCall(done(
        call("c1", "read_file", CallState::Running),
        "a\nb",
    ))));
    app.now += Duration::from_secs(14);
    app
}

pub(super) fn activity_row(app: &mut App, width: u16, height: u16) -> Option<String> {
    screen(app, width, height)
        .lines()
        .find(|l| {
            l.contains('(') && (l.starts_with(['·', '✢', '✳', '✶', '✻', '✽', '|', '/', '-', '\\']))
        })
        .map(|l| l.trim_end().to_owned())
}

#[test]
fn frame_activity_thinking() {
    let mut app = thinking();
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}

#[test]
fn the_activity_line_comes_with_the_turn_and_goes_with_it() {
    let mut app = app();
    assert_eq!(activity_row(&mut app, 80, 24), None);
    app.update(state_event(State::Working));
    let row = activity_row(&mut app, 80, 24).unwrap();
    // No text yet: no tokens; the time from the start.
    assert!(row.ends_with("(0s · Esc cancel)"), "{row}");
    let mut app = thinking();
    let row = activity_row(&mut app, 80, 24).unwrap();
    assert!(row.ends_with("(14s · ↓ 1.2k tokens · Esc cancel)"), "{row}");
    app.update(state_event(State::Idle));
    assert_eq!(activity_row(&mut app, 80, 24), None);
    // The conversation has its rows back.
    assert!(!screen(&mut app, 80, 24).contains("tokens"));
}

#[test]
fn the_activity_line_names_running_tools() {
    let mut app = thinking();
    let mut command = call("c2", "run_command", CallState::Running);
    command.summary = Some("cargo test".into());
    app.update(live(SessionEvent::ToolCall(command)));
    let row = activity_row(&mut app, 80, 24).unwrap();
    assert!(row.contains(" Running cargo test… ("), "{row}");
    for id in ["c3", "c4"] {
        app.update(live(SessionEvent::ToolCall(call(
            id,
            "edit_file",
            CallState::Pending,
        ))));
    }
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}

#[test]
fn the_activity_line_after_a_retry_and_a_cancel() {
    let mut app = thinking();
    app.update(live(SessionEvent::Retry {
        attempt: 2,
        max: 3,
        reason: "rate limited".into(),
    }));
    let row = activity_row(&mut app, 80, 24).unwrap();
    assert!(row.contains(" Retrying (2/3): rate limited… (14s"), "{row}");
    app.update(live(SessionEvent::TextDelta { text: "ok".into() }));
    let row = activity_row(&mut app, 80, 24).unwrap();
    assert!(!row.contains("Retrying"), "{row}");
    let effects = app.update(key(KeyCode::Esc));
    assert!(matches!(
        requests(&effects).as_slice(),
        [Request::Cancel { .. }]
    ));
    let row = activity_row(&mut app, 80, 24).unwrap();
    // The hint is gone: Esc did its work.
    assert!(row.ends_with("Cancelling… (14s · ↓ 1.2k tokens)"), "{row}");
}

#[test]
fn the_esc_hint_is_shown_only_when_esc_cancels() {
    let mut app = thinking();
    assert!(
        activity_row(&mut app, 80, 24)
            .unwrap()
            .contains("Esc cancel")
    );
    typed(&mut app, "next");
    assert!(
        !activity_row(&mut app, 80, 24)
            .unwrap()
            .contains("Esc cancel")
    );
    app.update(ctrl('u'));
    app.update(key(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Conversation);
    assert!(
        !activity_row(&mut app, 80, 24)
            .unwrap()
            .contains("Esc cancel")
    );
}

#[test]
fn the_activity_line_hides_while_a_prompt_is_open() {
    let mut app = thinking();
    app.update(state_event(State::Waiting));
    app.update(live(SessionEvent::ToolCall(call(
        "c1",
        "edit_file",
        CallState::Waiting,
    ))));
    app.update(live(SessionEvent::Question(Box::new(permission()))));
    assert!(app.prompt.is_some());
    assert_eq!(activity_row(&mut app, 80, 24), None);
    app.update(live(SessionEvent::QuestionClosed {
        id: "q1".into(),
        closed: Closed::Answered {
            answer: Answer::Allow,
            by: "tui pts/3".into(),
        },
    }));
    app.update(state_event(State::Working));
    let row = activity_row(&mut app, 80, 24).unwrap();
    // The same turn: its time goes on.
    assert!(row.contains("(14s"), "{row}");
}

#[test]
fn the_activity_line_drops_parts_on_a_narrow_or_low_screen() {
    let mut app = thinking();
    let verb = app.activity.verb;
    let row = activity_row(&mut app, 40, 24).unwrap();
    assert!(
        row.ends_with(&format!("{verb}… (14s · ↓ 1.2k tokens)")),
        "{row}"
    );
    assert!(!row.contains("Esc"), "{row}");
    app.update(live(SessionEvent::Retry {
        attempt: 2,
        max: 3,
        reason: "the provider is overloaded, try again later".into(),
    }));
    // The time goes last; then the text is cut.
    let row = activity_row(&mut app, 40, 24).unwrap();
    assert_eq!(row, "· Retrying (2/3): the provider is overl…");
    // Too low: no line.
    assert_eq!(activity_row(&mut app, 80, 13), None);
    assert!(!screen(&mut app, 80, 13).contains("Retrying"));
}

#[test]
fn frame_activity_ascii_and_plain() {
    let mut app = App::new(
        settings(false, false),
        attachment(Vec::new()),
        Instant::now(),
    );
    app.update(state_event(State::Working));
    app.update(live(SessionEvent::TextDelta {
        text: "x".repeat(4800),
    }));
    let mut tui = app.settings.tui.clone();
    tui.activity = crate::config::ActivityLine::Plain;
    app.apply(tui);
    app.now += Duration::from_secs(125);
    let row = activity_row(&mut app, 80, 24).unwrap();
    assert_eq!(row, "| Writing... (2m 05s - v 1.2k tokens - Esc cancel)");
    app.update(live(SessionEvent::ToolCall(call(
        "c1",
        "read_file",
        CallState::Running,
    ))));
    app.update(Input::Frame);
    let row = activity_row(&mut app, 80, 24).unwrap();
    assert_eq!(
        row,
        "/ Reading src/parse.rs... (2m 05s - v 1.2k tokens - Esc cancel)"
    );
}

#[test]
fn activity_off_shows_no_line_but_spinners_still_move() {
    let mut app = thinking();
    let mut tui = app.settings.tui.clone();
    tui.activity = crate::config::ActivityLine::Off;
    app.apply(tui);
    assert_eq!(activity_row(&mut app, 80, 24), None);
    // Nothing moves: no frames.
    assert!(!app.animating());
    assert!(app.update(Input::Frame).is_empty());
    app.update(live(SessionEvent::ToolCall(call(
        "c2",
        "read_file",
        CallState::Running,
    ))));
    assert!(app.animating());
    let before = screen(&mut app, 80, 24);
    assert_eq!(app.update(Input::Frame), [Effect::Redraw]);
    assert_ne!(screen(&mut app, 80, 24), before, "the spinner moved");
    // The second tick does not move it again.
    let before = screen(&mut app, 80, 24);
    app.update(Input::Tick);
    assert_eq!(screen(&mut app, 80, 24), before);
}

#[test]
fn frames_run_only_while_the_session_works() {
    let mut app = app();
    assert!(!app.animating());
    assert!(app.update(Input::Frame).is_empty());
    app.update(state_event(State::Working));
    assert!(app.animating());
    let glyph = |app: &mut App| activity_row(app, 80, 24).unwrap().chars().next();
    let first = glyph(&mut app);
    assert_eq!(app.update(Input::Frame), [Effect::Redraw]);
    assert_ne!(glyph(&mut app), first);
    app.update(state_event(State::Idle));
    assert!(!app.animating());
    assert!(app.update(Input::Frame).is_empty());
}

#[test]
fn a_quiet_model_turns_the_glyph_to_the_warning_colour() {
    let mut app = thinking();
    let glyph_colour = |app: &mut App| {
        let terminal = screen_of(app, 80, 24);
        let buffer = terminal.backend().buffer().clone();
        (0..24)
            .map(|y| buffer[(0, y)].clone())
            .find(|c| c.symbol() == "·")
            .map(|c| c.fg)
    };
    let accent = app.settings.look.styles.accent.fg;
    let warning = app.settings.look.styles.warning.fg;
    assert_ne!(accent, warning);
    assert_eq!(glyph_colour(&mut app), accent);
    app.now += Duration::from_secs(30);
    assert_eq!(glyph_colour(&mut app), warning);
    // The time stays.
    assert!(activity_row(&mut app, 80, 24).unwrap().contains("(44s"));
}

#[test]
fn an_attach_to_a_working_session_takes_the_start_from_the_server() {
    let working = SessionInfo {
        state: State::Working,
        ..info()
    };
    let mut app = App::new(
        settings(true, true),
        Attachment {
            session: working.clone(),
            sessions: vec![working],
            backlog: vec![
                replayed(SessionEvent::State {
                    state: State::Working,
                }),
                replayed(SessionEvent::UserMessage {
                    text: "go".into(),
                    images: Vec::new(),
                }),
            ],
        },
        Instant::now() + Duration::from_secs(3 * 3600),
    );
    // The time is left out until the answer.
    let row = activity_row(&mut app, 80, 24).unwrap();
    assert!(row.ends_with("(Esc cancel)"), "{row}");
    let effects = app.update(Input::Tick);
    assert_eq!(
        status_requests(&effects),
        [Request::SessionStatus {
            session: "project".into()
        }]
    );
    // Once only.
    assert!(status_requests(&app.update(Input::Frame)).is_empty());
    app.update(status_answer(status_of(State::Working)));
    // No box opened for it.
    assert!(app.status_box.is_none());
    let row = activity_row(&mut app, 80, 24).unwrap();
    // The turn started at t + 100 s; the server's clock says t + 7800 s.
    assert!(row.contains("(2h 08m · Esc cancel)"), "{row}");
}

pub(super) fn global_answer(
    range: crate::protocol::Range,
    stats: crate::protocol::GlobalStats,
) -> Input {
    Input::Response {
        request: Request::GlobalStats {
            range,
            offset: 3_600,
        },
        result: Ok(Payload::GlobalStats {
            stats: Box::new(stats),
        }),
    }
}

pub(super) fn global_requests(effects: &[Effect]) -> Vec<Request> {
    requests(effects)
        .into_iter()
        .filter(|r| matches!(r, Request::GlobalStats { .. }))
        .collect()
}

pub(super) fn week_stats() -> crate::protocol::GlobalStats {
    crate::text::global::tests::week()
}

#[test]
fn stats_opens_the_global_tab_and_asks() {
    use crate::protocol::Range;
    let mut app = app();
    let effects = command(&mut app, "/stats");
    assert_eq!(
        requests(&effects),
        [Request::GlobalStats {
            range: Range::Week,
            offset: 3_600
        }]
    );
    assert!(app.editor.is_empty());
    assert!(screen(&mut app, 80, 24).contains("counting the logs"));
    // Not twice at a time, and no polling.
    assert!(global_requests(&app.update(Input::Tick)).is_empty());
    app.update(global_answer(Range::Week, week_stats()));
    assert!(global_requests(&app.update(Input::Tick)).is_empty());
    let text = screen(&mut app, 80, 50);
    assert!(text.contains("12 active of 31"), "{text}");
    // The Session tab asks for the status once it is shown.
    let effects = app.update(key(KeyCode::Tab));
    assert_eq!(
        status_requests(&effects),
        [Request::SessionStatus {
            session: "project".into()
        }]
    );
    app.update(status_answer(status_of(State::Idle)));
    assert!(screen(&mut app, 80, 50).contains("Tool calls"));
    // Back again: the answer is kept.
    assert!(requests(&app.update(key(KeyCode::Left))).is_empty());
    assert!(screen(&mut app, 80, 50).contains("12 active of 31"));
}

#[test]
fn stats_with_a_range() {
    use crate::protocol::Range;
    let mut app = app();
    let effects = command(&mut app, "/stats 30d");
    assert_eq!(
        global_requests(&effects),
        [Request::GlobalStats {
            range: Range::Month,
            offset: 3_600
        }]
    );
    // A wrong range stays in the input.
    let mut app = self::app();
    let effects = command(&mut app, "/stats 2w");
    assert!(requests(&effects).is_empty());
    assert_eq!(app.editor.text(), "/stats 2w");
    assert!(notices(&app).contains(&"usage: /stats [7d|30d|all]".to_owned()));
    // The argument is completed.
    let mut app = self::app();
    typed(&mut app, "/stats ");
    let values: Vec<String> = app.arguments().into_iter().map(|v| v.value).collect();
    assert_eq!(values, ["7d", "30d", "all"]);
}

#[test]
fn ranges_and_counting_again() {
    use crate::protocol::Range;
    let mut app = app();
    command(&mut app, "/stats");
    app.update(global_answer(Range::Week, week_stats()));
    app.update(key(KeyCode::Down));
    // `a`: all time, asked once, from the first line.
    let effects = app.update(ch('a'));
    assert_eq!(
        global_requests(&effects),
        [Request::GlobalStats {
            range: Range::All,
            offset: 3_600
        }]
    );
    let open = app.status_box.as_ref().unwrap();
    assert_eq!((open.range, open.top), (Range::All, 0));
    // `7` again: the kept answer, no request.
    assert!(global_requests(&app.update(ch('7'))).is_empty());
    // `r` goes round.
    assert_eq!(global_requests(&app.update(ch('r'))).len(), 1);
    assert_eq!(app.status_box.as_ref().unwrap().range, Range::Month);
    app.update(ch('7'));
    // `R` counts again, once at a time.
    assert_eq!(global_requests(&app.update(ch('R'))).len(), 1);
    assert!(global_requests(&app.update(ch('R'))).is_empty());
    // A refresh that fails keeps what is shown.
    app.update(Input::Response {
        request: Request::GlobalStats {
            range: Range::Week,
            offset: 0,
        },
        result: Err("busy".into()),
    });
    assert!(screen(&mut app, 80, 50).contains("12 active of 31"));
    // Range keys do nothing on the Session tab.
    app.update(key(KeyCode::Tab));
    assert!(global_requests(&app.update(ch('a'))).is_empty());
    assert_eq!(app.status_box.as_ref().unwrap().range, Range::Week);
}

#[test]
fn a_first_global_answer_that_fails_closes_the_box() {
    use crate::protocol::Range;
    let mut app = app();
    command(&mut app, "/stats");
    app.update(Input::Response {
        request: Request::GlobalStats {
            range: Range::Week,
            offset: 0,
        },
        result: Err("cannot count".into()),
    });
    assert!(app.status_box.is_none());
    assert!(kinds(&app).contains(&Kind::Error("cannot count".into())));
}

#[test]
fn an_old_answer_says_its_age() {
    use crate::protocol::Range;
    let mut app = app();
    command(&mut app, "/stats");
    app.update(global_answer(Range::Week, week_stats()));
    assert!(!screen(&mut app, 80, 50).contains(" old "));
    app.now += Duration::from_secs(150);
    let text = screen(&mut app, 80, 50);
    assert!(text.contains("Global ─ 2m 30s old"), "{text}");
}

#[test]
fn frame_global_tab() {
    use crate::protocol::Range;
    let mut app = app();
    command(&mut app, "/stats");
    app.update(global_answer(Range::Week, week_stats()));
    insta::assert_snapshot!(screen(&mut app, 80, 40));
}

#[test]
fn frame_global_tab_ascii() {
    use crate::protocol::Range;
    let mut app = App::new(
        settings(false, false),
        attachment(Vec::new()),
        Instant::now(),
    );
    command(&mut app, "/stats");
    app.update(global_answer(Range::Week, week_stats()));
    insta::assert_snapshot!(screen(&mut app, 80, 40));
}

#[test]
fn frame_global_tab_narrow() {
    use crate::protocol::Range;
    let mut app = app();
    command(&mut app, "/stats");
    app.update(global_answer(Range::Week, week_stats()));
    insta::assert_snapshot!(screen(&mut app, 42, 40));
}
