//! Replay, events, the input and scrolling.

#![allow(clippy::wildcard_imports)]

use super::*;

// Replay and events.

#[test]
fn replay_builds_entries() {
    let app = app_with(vec![
        SessionEvent::UserMessage {
            text: "hi".into(),
            images: Vec::new(),
        },
        SessionEvent::TextDelta { text: "he".into() },
        SessionEvent::TextDone {
            text: "hello".into(),
        },
        SessionEvent::ToolCall(call("c1", "read_file", CallState::Running)),
        SessionEvent::ToolCall(done(call("c1", "read_file", CallState::Running), "a\nb")),
        SessionEvent::Notice {
            text: "turn cancelled".into(),
        },
        SessionEvent::Error {
            message: "boom".into(),
        },
    ]);
    let kinds = kinds(&app);
    assert_eq!(kinds.len(), 5, "{kinds:?}");
    assert_eq!(kinds[0], Kind::User("hi".into()));
    assert_eq!(kinds[1], Kind::Text("hello".into()));
    assert!(matches!(&kinds[2], Kind::Call(c) if c.state == CallState::Done));
    assert_eq!(kinds[3], Kind::Notice("turn cancelled".into()));
    assert_eq!(kinds[4], Kind::Error("boom".into()));
}

#[test]
fn delta_then_done() {
    let mut app = app();
    app.update(live(SessionEvent::TextDelta { text: "a".into() }));
    app.update(live(SessionEvent::TextDelta { text: "b".into() }));
    assert_eq!(kinds(&app), [Kind::Text("ab".into())]);
    app.update(live(SessionEvent::TextDone {
        text: "whole".into(),
    }));
    assert_eq!(kinds(&app), [Kind::Text("whole".into())]);
    // The next block is a new entry.
    app.update(live(SessionEvent::TextDelta { text: "c".into() }));
    assert_eq!(kinds(&app).len(), 2);
}

#[test]
fn a_done_without_deltas_is_an_entry() {
    let mut app = app();
    app.update(live(SessionEvent::TextDone { text: "x".into() }));
    app.update(live(SessionEvent::TextDone {
        text: String::new(),
    }));
    assert_eq!(kinds(&app), [Kind::Text("x".into())]);
}

#[test]
fn tool_call_states_and_live_output() {
    let mut app = app();
    let mut event = call("c1", "run_command", CallState::Pending);
    event.summary = None;
    event.arguments = serde_json::Value::Null;
    app.update(live(SessionEvent::ToolCall(event)));
    let mut running = call("c1", "run_command", CallState::Running);
    running.output = Some("one\n".into());
    app.update(live(SessionEvent::ToolCall(running.clone())));
    running.output = Some("two\n".into());
    running.summary = None;
    running.arguments = serde_json::Value::Null;
    app.update(live(SessionEvent::ToolCall(running)));
    let kinds = kinds(&app);
    let [Kind::Call(c)] = kinds.as_slice() else {
        panic!("not one call: {kinds:?}");
    };
    assert_eq!(c.output, "one\ntwo\n");
    assert_eq!(c.summary.as_deref(), Some("src/parse.rs"));
    assert_eq!(c.arguments["path"], "src/parse.rs");
    assert!(app.conversation.running_tool());
    app.update(live(SessionEvent::ToolCall(done(
        call("c1", "run_command", CallState::Running),
        "exit 0",
    ))));
    assert!(!app.conversation.running_tool());
    assert_eq!(app.tool_calls, 1);
}

#[test]
fn an_id_used_again_is_a_new_call() {
    let mut app = app();
    let first = call("call_1", "read_file", CallState::Running);
    app.update(live(SessionEvent::ToolCall(first.clone())));
    app.update(live(SessionEvent::ToolCall(done(first.clone(), "old"))));
    app.update(live(SessionEvent::ToolCall(first.clone())));
    app.update(live(SessionEvent::ToolCall(done(first, "new"))));
    let kinds = kinds(&app);
    let [Kind::Call(a), Kind::Call(b)] = kinds.as_slice() else {
        panic!("not two calls: {kinds:?}");
    };
    assert_eq!(a.result.as_ref().unwrap().text, "old");
    assert_eq!(b.result.as_ref().unwrap().text, "new");
}

#[test]
fn model_rename_and_usage_update_the_session() {
    let mut app = app();
    app.update(live(SessionEvent::ModelChanged {
        from: "anthropic/sonnet".into(),
        model: "local/qwen".into(),
    }));
    app.update(live(SessionEvent::Renamed { name: "new".into() }));
    app.update(live(SessionEvent::Usage {
        input_tokens: 1,
        output_tokens: 2,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        context_tokens: 3,
        model: String::new(),
    }));
    assert_eq!(app.session.model, "local/qwen");
    assert_eq!(app.session.name, "new");
    assert_eq!(app.context_tokens, 3);
    assert_eq!(app.requests, 1);
    assert_eq!(
        notices(&app),
        [
            "model: anthropic/sonnet → local/qwen",
            "session renamed to new"
        ]
    );
}

#[test]
fn events_of_other_sessions_are_ignored() {
    let mut app = app();
    let effects = app.update(Input::Server(ServerMessage::Event {
        session: 2,
        replay: false,
        event: SessionEvent::Notice { text: "x".into() },
    }));
    assert!(effects.is_empty());
    assert!(kinds(&app).is_empty());
}

#[test]
fn server_notices_are_shown() {
    let mut app = app();
    app.update(Input::Server(ServerMessage::Notice {
        notice: Notice::ConfigReloaded,
    }));
    app.update(Input::Server(ServerMessage::Notice {
        notice: Notice::ConfigRejected {
            errors: vec!["bad".into()],
        },
    }));
    assert_eq!(
        kinds(&app),
        [
            Kind::Notice("configuration reloaded".into()),
            Kind::Error("configuration rejected: bad".into())
        ]
    );
}

#[test]
fn streaming_text_only_asks_for_a_redraw() {
    let mut app = app();
    let effects = app.update(live(SessionEvent::TextDelta { text: "a".into() }));
    assert_eq!(effects, [Effect::Redraw]);
}

#[test]
fn an_idle_tick_draws_nothing() {
    let mut app = app();
    assert!(app.update(Input::Tick).is_empty());
    // Every second tick asks for the session list, for the session row.
    assert_eq!(requests(&app.update(Input::Tick)), [Request::List]);
    app.update(live(SessionEvent::ToolCall(call(
        "c1",
        "read_file",
        CallState::Running,
    ))));
    assert!(app.update(Input::Tick).contains(&Effect::Redraw));
}

#[test]
fn the_session_row_counts_sessions_and_waiting_ones() {
    let mut app = app();
    assert_eq!(app.place.unwrap().total, 1);
    let mut waiting = info();
    waiting.id = 2;
    waiting.question = Some(limit());
    let result = Ok(Payload::Sessions {
        sessions: vec![waiting, info()],
    });
    app.update(Input::Response {
        request: Request::List,
        result,
    });
    let place = app.place.unwrap();
    assert_eq!((place.position, place.total, place.waiting), (2, 2, 1));
    assert!(screen(&mut app, 80, 24).contains("2/2 ●1"));
}

// The input.

#[test]
fn enter_sends_and_shows_it_as_queued() {
    let mut app = app();
    typed(&mut app, "hello");
    let effects = app.update(key(KeyCode::Enter));
    assert_eq!(
        requests(&effects),
        [Request::Send {
            session: "project".into(),
            text: "hello".into(),
            images: Vec::new(),
        }]
    );
    assert!(app.editor.is_empty());
    assert_eq!(app.conversation.queued, ["hello"]);
    assert!(screen(&mut app, 80, 24).contains("> hello (queued)"));
    app.update(live(SessionEvent::UserMessage {
        text: "hello".into(),
        images: Vec::new(),
    }));
    assert!(app.conversation.queued.is_empty());
    assert_eq!(kinds(&app), [Kind::User("hello".into())]);
    assert!(!screen(&mut app, 80, 24).contains("queued"));
}

#[test]
fn messages_can_be_sent_while_a_turn_runs() {
    let mut app = app();
    app.update(live(SessionEvent::State {
        state: State::Working,
    }));
    typed(&mut app, "next");
    assert_eq!(requests(&app.update(key(KeyCode::Enter))).len(), 1);
    assert_eq!(app.conversation.queued, ["next"]);
}

#[test]
fn a_failed_send_comes_back_to_the_input() {
    let mut app = app();
    typed(&mut app, "hello");
    let effects = app.update(key(KeyCode::Enter));
    let request = requests(&effects).remove(0);
    app.update(Input::Response {
        request,
        result: Err("no such session".into()),
    });
    assert_eq!(app.editor.text(), "hello");
    assert!(app.conversation.queued.is_empty());
    assert_eq!(kinds(&app), [Kind::Error("no such session".into())]);
}

#[test]
fn alt_and_shift_enter_insert_a_newline() {
    let mut app = app();
    typed(&mut app, "a");
    app.update(press(KeyCode::Enter, KeyModifiers::ALT));
    typed(&mut app, "b");
    app.update(press(KeyCode::Enter, KeyModifiers::SHIFT));
    typed(&mut app, "c");
    assert_eq!(app.editor.text(), "a\nb\nc");
}

#[test]
fn slash_slash_sends_a_message() {
    let mut app = app();
    typed(&mut app, "//etc/passwd");
    assert_eq!(
        requests(&app.update(key(KeyCode::Enter))),
        [Request::Send {
            session: "project".into(),
            text: "/etc/passwd".into(),
            images: Vec::new(),
        }]
    );
}

#[test]
fn empty_input_sends_nothing() {
    let mut app = app();
    typed(&mut app, "  ");
    assert!(requests(&app.update(key(KeyCode::Enter))).is_empty());
}

#[test]
fn message_history_comes_from_the_conversation() {
    let mut app = app_with(vec![
        SessionEvent::UserMessage {
            text: "one".into(),
            images: Vec::new(),
        },
        SessionEvent::UserMessage {
            text: "two".into(),
            images: Vec::new(),
        },
    ]);
    typed(&mut app, "draft");
    app.update(key(KeyCode::Up));
    assert_eq!(app.editor.text(), "two");
    app.update(key(KeyCode::Up));
    assert_eq!(app.editor.text(), "one");
    app.update(key(KeyCode::Down));
    app.update(key(KeyCode::Down));
    assert_eq!(app.editor.text(), "draft");
}

#[test]
fn paste_goes_to_the_input_and_is_not_sent() {
    let mut app = app();
    let effects = app.update(Input::Paste("a\nb".into()));
    assert!(requests(&effects).is_empty());
    assert_eq!(app.editor.text(), "a\nb");
}

#[test]
fn ctrl_z_does_nothing() {
    let mut app = app();
    typed(&mut app, "a");
    let effects = app.update(ctrl('z'));
    assert!(requests(&effects).is_empty() && exit(&effects).is_none());
    assert_eq!(app.editor.text(), "a");
}

#[test]
fn key_releases_are_ignored() {
    let mut app = app();
    let mut release = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
    release.kind = KeyEventKind::Release;
    assert!(app.update(Input::Key(release)).is_empty());
    assert!(app.editor.is_empty());
}

// Scrolling.

pub(super) fn notices_in(app: &mut App, count: usize) {
    for i in 0..count {
        app.update(live(SessionEvent::Notice {
            text: format!("line {i}"),
        }));
    }
}

#[test]
fn the_view_follows_the_end() {
    let mut app = app();
    notices_in(&mut app, 100);
    let text = screen(&mut app, 80, 24);
    assert!(text.contains("line 99"), "{text}");
    assert!(app.follow && !app.new_output);
}

#[test]
fn scrolled_up_it_stays_and_says_new_output() {
    let mut app = app();
    notices_in(&mut app, 100);
    screen(&mut app, 80, 24);
    app.update(key(KeyCode::PageUp));
    let text = screen(&mut app, 80, 24);
    let top = app.top;
    assert!(!app.follow);
    assert!(!text.contains("line 99"));
    notices_in(&mut app, 5);
    let text = screen(&mut app, 80, 24);
    assert_eq!(app.top, top);
    assert!(app.new_output);
    assert!(text.contains("↓ new output"), "{text}");
    app.update(press(KeyCode::End, KeyModifiers::CONTROL));
    let text = screen(&mut app, 80, 24);
    assert!(app.follow && !app.new_output);
    assert!(
        text.contains("line 99") && !text.contains("new output"),
        "{text}"
    );
}

#[test]
fn scrolling_down_to_the_end_resumes_following() {
    let mut app = app();
    notices_in(&mut app, 100);
    screen(&mut app, 80, 24);
    app.update(key(KeyCode::PageUp));
    screen(&mut app, 80, 24);
    app.update(key(KeyCode::PageDown));
    screen(&mut app, 80, 24);
    assert!(app.follow);
}

#[test]
fn ctrl_home_goes_to_the_start() {
    let mut app = app();
    notices_in(&mut app, 100);
    screen(&mut app, 80, 24);
    app.update(press(KeyCode::Home, KeyModifiers::CONTROL));
    let text = screen(&mut app, 80, 24);
    assert_eq!(app.top, 0);
    assert!(text.contains("line 0"));
}

#[test]
fn the_mouse_wheel_scrolls_by_scroll_lines() {
    let mut app = app();
    notices_in(&mut app, 100);
    screen(&mut app, 80, 24);
    let top = app.top;
    let wheel = |kind| {
        Input::Mouse(MouseEvent {
            kind,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        })
    };
    app.update(wheel(MouseEventKind::ScrollUp));
    screen(&mut app, 80, 24);
    assert_eq!(app.top, top - 3);
    app.update(wheel(MouseEventKind::ScrollDown));
    screen(&mut app, 80, 24);
    assert!(app.follow);
}

#[test]
fn resizing_rewraps_and_keeps_the_entry_at_the_top() {
    let mut app = app();
    let long = "word ".repeat(40);
    for _ in 0..20 {
        app.update(live(SessionEvent::Notice { text: long.clone() }));
    }
    screen(&mut app, 100, 24);
    app.update(press(KeyCode::Home, KeyModifiers::CONTROL));
    app.update(key(KeyCode::PageDown));
    screen(&mut app, 100, 24);
    let entry = app.conversation.entry_at(app.top).unwrap();
    screen(&mut app, 50, 24);
    assert_eq!(app.conversation.entry_at(app.top), Some(entry));
    assert_eq!(app.top, app.conversation.span_of(entry).unwrap().0);
}
