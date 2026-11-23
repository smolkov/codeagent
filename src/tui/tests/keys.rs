//! Keys, exit, the connection, commands and the shell helpers.

#![allow(clippy::wildcard_imports)]

use super::*;

// Keys and exit.

#[test]
fn esc_and_ctrl_c_cancel_a_running_turn() {
    for input in [key(KeyCode::Esc), ctrl('c')] {
        let mut app = app();
        app.update(live(SessionEvent::State {
            state: State::Working,
        }));
        assert_eq!(
            requests(&app.update(input)),
            [Request::Cancel {
                session: "project".into()
            }]
        );
    }
}

#[test]
fn without_a_turn_ctrl_c_clears_the_input_and_esc_does_nothing() {
    let mut app = app();
    typed(&mut app, "abc");
    assert!(requests(&app.update(key(KeyCode::Esc))).is_empty());
    assert_eq!(app.editor.text(), "abc");
    assert!(requests(&app.update(ctrl('c'))).is_empty());
    assert!(app.editor.is_empty());
}

#[test]
fn ctrl_c_twice_detaches_only_within_a_second() {
    let mut app = app();
    assert_eq!(exit(&app.update(ctrl('c'))), None);
    app.now += Duration::from_millis(1500);
    assert_eq!(exit(&app.update(ctrl('c'))), None);
    app.now += Duration::from_millis(500);
    assert_eq!(exit(&app.update(ctrl('c'))), detached("project"));
}

#[test]
fn ctrl_d_detaches_with_empty_input_only() {
    let mut app = app();
    typed(&mut app, "a");
    assert_eq!(exit(&app.update(ctrl('d'))), None);
    app.update(ctrl('c'));
    assert_eq!(exit(&app.update(ctrl('d'))), detached("project"));
}

#[test]
fn a_detach_uses_the_current_name() {
    let mut app = app();
    app.update(live(SessionEvent::Renamed { name: "new".into() }));
    assert_eq!(exit(&app.update(ctrl('d'))), detached("new"));
}

#[test]
fn session_ended_disables_the_input_and_any_key_exits() {
    let mut app = app();
    app.update(live(SessionEvent::Ended {
        reason: "killed".into(),
    }));
    assert_eq!(notices(&app), ["session ended: killed"]);
    assert!(screen(&mut app, 80, 24).contains("any key exits"));
    let paste = app.update(Input::Paste("x".into()));
    assert!(exit(&paste).is_none());
    assert!(app.editor.is_empty());
    assert_eq!(
        exit(&app.update(ch('x'))),
        Some(Exit::Ended {
            name: "project".into(),
            reason: "killed".into()
        })
    );
}

#[test]
fn an_ended_session_does_not_reconnect() {
    let mut app = app();
    app.update(live(SessionEvent::Ended {
        reason: "server stopped".into(),
    }));
    assert!(!app.update(Input::Disconnected).contains(&Effect::Reconnect));
    assert!(app.lost_since.is_none());
    // No more requests for the session row either.
    app.update(Input::Tick);
    assert!(requests(&app.update(Input::Tick)).is_empty());
}

// The connection.

pub(super) fn lost() -> App {
    let mut app = app_with(vec![
        SessionEvent::UserMessage {
            text: "hi".into(),
            images: Vec::new(),
        },
        SessionEvent::ToolCall(done(call("c1", "read_file", CallState::Running), "a\nb")),
    ]);
    screen(&mut app, 80, 24);
    assert!(app.update(Input::Disconnected).contains(&Effect::Reconnect));
    app
}

#[test]
fn a_lost_connection_is_retried_for_ten_seconds() {
    let mut app = lost();
    assert!(screen(&mut app, 80, 24).contains("server disconnected"));
    app.now += RECONNECT_FOR - Duration::from_millis(1);
    assert_eq!(app.update(Input::ReconnectFailed), [Effect::Reconnect]);
    app.now += Duration::from_millis(1);
    assert_eq!(
        app.update(Input::ReconnectFailed),
        [Effect::Exit(Exit::Lost)]
    );
}

#[test]
fn the_input_stays_while_disconnected() {
    let mut app = lost();
    typed(&mut app, "keep");
    assert!(requests(&app.update(key(KeyCode::Enter))).is_empty());
    assert_eq!(app.editor.text(), "keep");
    assert!(screen(&mut app, 100, 24).contains("the input is kept"));
}

#[test]
fn reconnecting_rebuilds_the_conversation_and_keeps_the_view() {
    let mut app = app_with(vec![
        SessionEvent::UserMessage {
            text: "hi".into(),
            images: Vec::new(),
        },
        SessionEvent::ToolCall(done(call("c1", "read_file", CallState::Running), "a\nb")),
    ]);
    notices_in(&mut app, 100);
    select_last(&mut app);
    app.selected = Some(1);
    app.update(key(KeyCode::Enter));
    app.update(key(KeyCode::Tab));
    typed(&mut app, "unsent");
    screen(&mut app, 80, 24);
    app.update(key(KeyCode::PageUp));
    screen(&mut app, 80, 24);
    let top = app.top;
    app.update(Input::Disconnected);
    let log = vec![
        SessionEvent::UserMessage {
            text: "hi".into(),
            images: Vec::new(),
        },
        SessionEvent::ToolCall(done(call("c1", "read_file", CallState::Running), "a\nb")),
    ];
    let mut events = log;
    for i in 0..100 {
        events.push(SessionEvent::Notice {
            text: format!("line {i}"),
        });
    }
    app.update(Input::Attached(Box::new(attachment(events))));
    assert!(app.lost_since.is_none());
    assert_eq!(app.conversation.entries.len(), 102);
    assert_eq!(app.editor.text(), "unsent");
    screen(&mut app, 80, 24);
    assert_eq!(app.top, top);
    assert!(!app.follow && !app.new_output);
    // The call stays expanded.
    app.update(press(KeyCode::Home, KeyModifiers::CONTROL));
    let text = screen(&mut app, 80, 24);
    assert!(text.contains("▾ read"), "{text}");
}

#[test]
fn a_question_still_open_after_reconnect_keeps_its_prompt() {
    let mut app = asked();
    app.update(ch('e'));
    typed(&mut app, "half a mess");
    app.update(Input::Disconnected);
    let mut session = info();
    session.state = State::Waiting;
    session.question = Some(permission());
    app.update(Input::Attached(Box::new(Attachment {
        session: session.clone(),
        sessions: vec![session],
        backlog: Vec::new(),
    })));
    let prompt = app.prompt.as_ref().unwrap();
    assert!(matches!(&prompt.mode, Mode::Explain(e) if e.text() == "half a mess"));
}

#[test]
fn a_question_answered_while_away_closes_on_reconnect() {
    let mut app = asked();
    app.update(Input::Disconnected);
    app.update(Input::Attached(Box::new(attachment(Vec::new()))));
    assert!(app.prompt.is_none());
}

#[test]
fn an_answer_lost_with_the_connection_can_be_given_again() {
    let mut app = asked();
    let effects = app.update(ch('y'));
    let request = requests(&effects).remove(0);
    app.update(Input::Response {
        request,
        result: Err(LOST.into()),
    });
    let prompt = app.prompt.as_ref().unwrap();
    assert!(prompt.error.is_none() && !prompt.sending);
    assert_eq!(answer_of(&app.update(ch('y'))), Answer::Allow);
}

#[test]
fn a_lost_connection_is_no_error_entry() {
    let mut app = app();
    app.update(Input::Response {
        request: Request::Cancel {
            session: "project".into(),
        },
        result: Err(LOST.into()),
    });
    assert!(kinds(&app).is_empty());
}

// Commands.

pub(super) fn command(app: &mut App, text: &str) -> Vec<Effect> {
    typed(app, text);
    app.update(key(KeyCode::Enter))
}

#[test]
fn help_detach_and_quit() {
    let mut app = app();
    command(&mut app, "/help");
    assert_eq!(app.dialog, Some(Dialog::Help));
    assert!(app.editor.is_empty());
    for name in ["/detach", "/quit"] {
        let mut app = self::app();
        assert_eq!(exit(&command(&mut app, name)), detached("project"));
    }
}

#[test]
fn model_with_an_argument() {
    let mut app = app();
    assert_eq!(
        requests(&command(&mut app, "/model local/qwen")),
        [Request::SetModel {
            session: "project".into(),
            model: "local/qwen".into()
        }]
    );
    assert!(app.editor.is_empty());
}

#[test]
fn a_rejected_model_is_shown() {
    let mut app = app();
    let request = requests(&command(&mut app, "/model nope")).remove(0);
    app.update(Input::Response {
        request,
        result: Err("unknown model nope".into()),
    });
    assert_eq!(kinds(&app), [Kind::Error("unknown model nope".into())]);
}

#[test]
fn rename_needs_a_name() {
    let mut app = app();
    // In the list, `Enter` completes it, so that the name can be typed.
    assert!(requests(&command(&mut app, "/rename")).is_empty());
    assert_eq!(app.editor.text(), "/rename ");
    assert!(app.command_list.is_none());
    assert!(notices(&app).is_empty());
    // With the list closed, the text is a command as it stands.
    app.editor.clear();
    typed(&mut app, "/rename");
    app.update(key(KeyCode::Esc));
    assert!(requests(&app.update(key(KeyCode::Enter))).is_empty());
    assert_eq!(notices(&app), ["usage: /rename NAME"]);
    app.editor.clear();
    assert_eq!(
        requests(&command(&mut app, "/rename docs")),
        [Request::Rename {
            session: "project".into(),
            name: "docs".into()
        }]
    );
}

#[test]
fn kill_asks_first() {
    let mut app = app();
    assert!(requests(&command(&mut app, "/kill")).is_empty());
    assert_eq!(app.dialog, Some(Dialog::ConfirmKill));
    assert!(screen(&mut app, 80, 24).contains("Delete project?"));
    // Other keys do nothing, `n` closes.
    assert!(requests(&app.update(ch('x'))).is_empty());
    assert_eq!(app.dialog, Some(Dialog::ConfirmKill));
    app.update(ch('n'));
    assert_eq!(app.dialog, None);
    command(&mut app, "/kill");
    assert_eq!(
        requests(&app.update(ch('y'))),
        [Request::Kill {
            session: "project".into()
        }]
    );
}

#[test]
fn later_commands_say_so() {
    for name in ["clear", "permissions", "commit"] {
        let mut app = app();
        assert!(requests(&command(&mut app, &format!("/{name}"))).is_empty());
        assert_eq!(
            notices(&app),
            [format!("`/{name}` comes with a later version")]
        );
        assert!(app.editor.is_empty());
    }
}

#[test]
fn unknown_commands_stay_in_the_input() {
    let mut app = app();
    // In the list, `Enter` does nothing: it already says `no command`.
    assert!(requests(&command(&mut app, "/nope")).is_empty());
    assert_eq!(app.editor.text(), "/nope");
    assert!(app.command_list.is_some());
    assert!(notices(&app).is_empty());
    // With the list closed, it gets a notice.
    app.update(key(KeyCode::Esc));
    assert!(requests(&app.update(key(KeyCode::Enter))).is_empty());
    assert_eq!(app.editor.text(), "/nope");
    assert!(notices(&app)[0].starts_with("unknown command /nope"));
}

// Helpers of the shell.

#[test]
fn exit_lines_and_codes() {
    let detached = Exit::Detached {
        name: "docs".into(),
    };
    assert_eq!(
        exit_line(&detached).unwrap(),
        "codeagent: detached from docs; \"codeagent attach docs\" to return"
    );
    assert!(exit_result(&detached).is_ok());
    let ended = Exit::Ended {
        name: "docs".into(),
        reason: "killed".into(),
    };
    assert_eq!(
        exit_line(&ended).unwrap(),
        "codeagent: session docs ended: killed"
    );
    assert!(exit_result(&ended).is_ok());
    assert_eq!(exit_line(&Exit::Lost), None);
    let lost = exit_result(&Exit::Lost).unwrap_err();
    assert_eq!(lost.to_string(), "server disconnected");
    assert_eq!(lost.exit_code(), 1);
    assert_eq!(exit_line(&Exit::Signal), None);
    assert_eq!(exit_result(&Exit::Signal).unwrap_err().exit_code(), 1);
}

#[test]
fn no_terminal_is_a_usage_error() {
    assert!(check_terminal(true, true).is_ok());
    for (stdin, stdout) in [(false, true), (true, false)] {
        let error = check_terminal(stdin, stdout).unwrap_err();
        assert_eq!(error.exit_code(), 2);
        assert!(error.to_string().starts_with("the TUI needs a terminal"));
    }
}

#[test]
fn attach_without_a_name() {
    let named = |name: &str| SessionInfo {
        name: name.to_owned(),
        ..info()
    };
    assert_eq!(first_session(&[named("a")]).unwrap(), ("a".into(), false));
    let none = first_session(&[]).unwrap_err();
    assert!(matches!(&none, Error::Failed(m) if m == "no sessions"));
    assert_eq!(none.exit_code(), 1);
    // Several: the first, and the user chooses in the session list.
    let several = first_session(&[named("a"), named("b"), named("c")]).unwrap();
    assert_eq!(several, ("a".into(), true));
}

#[test]
fn clock_and_offsets() {
    assert_eq!(clock(0, 0), "00:00");
    assert_eq!(clock(3_600 + 120, 0), "01:02");
    assert_eq!(clock(0, 5 * 3600 + 1800), "05:30");
    assert_eq!(clock(1800, -3600), "23:30");
    assert_eq!(parse_offset("+0530"), Some(19_800));
    assert_eq!(parse_offset("-0100"), Some(-3600));
    assert_eq!(parse_offset("UTC"), None);
}

#[test]
fn timestamps_prefix_live_entries_only() {
    let mut settings = settings(true, true);
    settings.timestamps = Some(0);
    let mut app = App::new(
        settings,
        attachment(vec![SessionEvent::UserMessage {
            text: "old".into(),
            images: Vec::new(),
        }]),
        Instant::now(),
    );
    app.update(live(SessionEvent::UserMessage {
        text: "new".into(),
        images: Vec::new(),
    }));
    let text = screen(&mut app, 80, 24);
    let line_of = |wanted: &str| {
        text.lines()
            .find(|l| l.contains(wanted))
            .unwrap_or_else(|| panic!("no {wanted} in {text}"))
            .to_owned()
    };
    assert!(line_of("> old").starts_with("> old"));
    let new = line_of("> new");
    assert_eq!(new.chars().nth(2), Some(':'), "{new}");
    assert!(new.get(5..).unwrap().starts_with(" > new"), "{new}");
}
