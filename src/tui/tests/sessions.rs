//! Sessions: switching, the session list, new sessions (RFC 0005).

#![allow(clippy::wildcard_imports)]

use super::*;

// Sessions: switching, the session list, new sessions (RFC 0005).

pub(super) fn session(id: u64, name: &str) -> SessionInfo {
    SessionInfo {
        id,
        name: name.to_owned(),
        ..info()
    }
}

pub(super) fn three() -> Vec<SessionInfo> {
    vec![session(1, "project"), session(2, "docs"), session(3, "web")]
}

pub(super) fn sessions_answer(sessions: Vec<SessionInfo>) -> Input {
    Input::Response {
        request: Request::List,
        result: Ok(Payload::Sessions { sessions }),
    }
}

/// An app that knows three sessions, attached to the first.
pub(super) fn among_three() -> App {
    let mut app = app();
    app.update(sessions_answer(three()));
    app
}

pub(super) fn switches(effects: &[Effect]) -> Vec<String> {
    effects
        .iter()
        .filter_map(|e| match e {
            Effect::Switch(name) => Some(name.clone()),
            _ => None,
        })
        .collect()
}

/// The attachment a switch to session `id` brings.
pub(super) fn attached_to(id: u64, name: &str, events: Vec<SessionEvent>) -> Input {
    Input::Attached(Box::new(Attachment {
        session: session(id, name),
        sessions: three(),
        backlog: events
            .into_iter()
            .map(|event| ServerMessage::Event {
                session: id,
                replay: true,
                event,
            })
            .collect(),
    }))
}

#[test]
fn ctrl_n_and_ctrl_p_go_round_the_sessions() {
    let mut app = among_three();
    assert_eq!(switches(&app.update(ctrl('n'))), ["docs"]);
    assert_eq!(app.switching.as_deref(), Some("docs"));
    assert!(screen(&mut app, 80, 24).contains("attaching docs"));
    // One switch at a time.
    assert!(switches(&app.update(ctrl('n'))).is_empty());
    app.update(Input::SwitchFailed("no session named docs".into()));
    assert!(app.switching.is_none());
    assert!(kinds(&app).contains(&Kind::Error("no session named docs".into())));
    // Backwards from the first is the last.
    assert_eq!(switches(&app.update(ctrl('p'))), ["web"]);
}

#[test]
fn with_one_session_ctrl_n_does_nothing() {
    let mut app = app();
    app.update(sessions_answer(vec![info()]));
    assert!(switches(&app.update(ctrl('n'))).is_empty());
    assert!(switches(&app.update(ctrl('p'))).is_empty());
}

#[test]
fn a_switch_keeps_the_view_of_each_session() {
    let mut app = among_three();
    typed(&mut app, "half a thought");
    app.update(ctrl('n'));
    app.update(attached_to(
        2,
        "docs",
        vec![SessionEvent::UserMessage {
            text: "in docs".into(),
            images: Vec::new(),
        }],
    ));
    assert_eq!(app.session.name, "docs");
    assert!(app.switching.is_none());
    assert!(app.editor.is_empty());
    assert_eq!(kinds(&app), [Kind::User("in docs".into())]);
    // Events of the session left are not shown.
    app.update(live(SessionEvent::Notice {
        text: "from project".into(),
    }));
    assert_eq!(kinds(&app).len(), 1);
    typed(&mut app, "x");
    app.update(attached_to(1, "project", Vec::new()));
    assert_eq!(app.editor.text(), "half a thought");
    app.update(attached_to(2, "docs", Vec::new()));
    assert_eq!(app.editor.text(), "x");
}

#[test]
fn attach_by_name() {
    let mut app = among_three();
    // Not in the last list: a notice, and the input stays.
    let effects = command(&mut app, "/attach nope");
    assert!(switches(&effects).is_empty());
    assert_eq!(
        notices(&app),
        ["no session named nope; /sessions lists them"]
    );
    assert_eq!(app.editor.text(), "/attach nope");
    app.editor.clear();
    assert_eq!(switches(&command(&mut app, "/attach web")), ["web"]);
    assert!(app.editor.is_empty());
    // The attached one: nothing.
    let mut app = among_three();
    assert!(switches(&command(&mut app, "/attach project")).is_empty());
}

#[test]
fn the_session_list_opens_with_the_answer() {
    for open in ["ctrl-s", "/sessions", "/attach"] {
        let mut app = among_three();
        let effects = if open == "ctrl-s" {
            app.update(ctrl('s'))
        } else {
            command(&mut app, open)
        };
        assert_eq!(requests(&effects), [Request::List], "{open}");
        assert!(app.session_list.is_none());
        app.update(sessions_answer(three()));
        let list = app.session_list.as_ref().expect(open);
        assert_eq!(list.chosen().unwrap().name, "project");
    }
}

/// The session list open on three sessions.
pub(super) fn listing() -> App {
    let mut app = among_three();
    app.update(ctrl('s'));
    app.update(sessions_answer(three()));
    app
}

#[test]
fn enter_in_the_list_switches() {
    let mut app = listing();
    app.update(key(KeyCode::Down));
    assert_eq!(switches(&app.update(key(KeyCode::Enter))), ["docs"]);
    app.update(attached_to(2, "docs", Vec::new()));
    assert!(app.session_list.is_none());
    // Enter on the attached session only closes.
    let mut app = listing();
    assert!(switches(&app.update(key(KeyCode::Enter))).is_empty());
    assert!(app.session_list.is_none());
    // Esc closes and leaves nothing.
    let mut app = listing();
    assert!(exit(&app.update(key(KeyCode::Esc))).is_none());
    assert!(app.session_list.is_none());
}

#[test]
fn rename_and_kill_from_the_list() {
    let mut app = listing();
    app.update(key(KeyCode::Down));
    app.update(ch('r'));
    typed(&mut app, "2");
    let effects = app.update(key(KeyCode::Enter));
    assert_eq!(
        requests(&effects),
        [Request::Rename {
            session: "docs".into(),
            name: "docs2".into()
        }]
    );
    // A refused name is shown in the box.
    app.update(Input::Response {
        request: requests(&effects).remove(0),
        result: Err("the name docs2 is taken".into()),
    });
    assert_eq!(
        app.session_list.as_ref().unwrap().message.as_deref(),
        Some("the name docs2 is taken")
    );
    assert!(screen(&mut app, 80, 24).contains("the name docs2 is taken"));
    app.update(ch('d'));
    assert!(screen(&mut app, 80, 24).contains("Delete docs? No turns yet."));
    assert_eq!(
        requests(&app.update(ch('y'))),
        [Request::Kill {
            session: "docs".into()
        }]
    );
}

#[test]
fn a_question_closes_the_list() {
    let mut app = listing();
    app.update(live(SessionEvent::Question(Box::new(permission()))));
    assert!(app.session_list.is_none());
    // And a list asked for does not open over the prompt.
    app.update(sessions_answer(three()));
    assert!(app.session_list.is_none());
}

/// The models, with `opus` as `default_model`: not the session's model.
pub(super) fn default_opus() -> Vec<ModelInfo> {
    model_list()
        .into_iter()
        .map(|m| ModelInfo {
            default: m.name == "opus",
            ..m
        })
        .collect()
}

#[test]
fn new_starts_on_the_default_model() {
    let mut app = among_three();
    assert_eq!(requests(&command(&mut app, "/new")), [models_request()]);
    app.update(models_answer(default_opus()));
    let picker = app.picker.as_ref().expect("picker");
    assert_eq!(picker.chosen().unwrap().model, "anthropic/opus");
    let text = screen(&mut app, 80, 24);
    assert!(text.contains("Model for a new session"), "{text}");
    // No `current` mark: the new session has no model yet.
    assert!(!text.contains("current"), "{text}");
    // One Enter: the default model.
    let effects = app.update(key(KeyCode::Enter));
    assert_eq!(
        requests(&effects),
        [Request::New {
            workspace: "/ws/project".into(),
            name: None,
            model: Some("anthropic/opus".into()),
            skills: Vec::new(),
        }]
    );
    assert!(app.picker.is_none());
    // The answer switches to it, with a note: the workspace is shared.
    let effects = app.update(Input::Response {
        request: requests(&effects).remove(0),
        result: Ok(Payload::Session {
            session: Box::new(session(4, "project-2")),
        }),
    });
    assert_eq!(switches(&effects), ["project-2"]);
    app.update(attached_to(4, "project-2", Vec::new()));
    assert_eq!(
        notices(&app),
        ["note: project, docs, web also work in ~/ws/project; sessions may change the same files"]
    );
}

#[test]
fn new_with_a_name_and_another_model() {
    let mut app = among_three();
    command(&mut app, "/new spike");
    app.update(models_answer(default_opus()));
    app.update(key(KeyCode::Down));
    assert_eq!(
        requests(&app.update(key(KeyCode::Enter))),
        [Request::New {
            workspace: "/ws/project".into(),
            name: Some("spike".into()),
            model: Some("local/qwen".into()),
            skills: Vec::new(),
        }]
    );
    // Esc creates nothing.
    let mut app = among_three();
    command(&mut app, "/new");
    app.update(models_answer(default_opus()));
    assert!(requests(&app.update(key(KeyCode::Esc))).is_empty());
    assert!(app.picker.is_none());
}

#[test]
fn n_in_the_list_asks_workspace_and_name_then_the_model() {
    let mut app = listing();
    app.update(ch('n'));
    assert!(screen(&mut app, 80, 24).contains("Workspace: ~/ws/project"));
    app.update(key(KeyCode::Enter));
    typed(&mut app, "side");
    let effects = app.update(key(KeyCode::Enter));
    assert!(
        matches!(
            requests(&effects).as_slice(),
            [Request::Models { workspace: Some(_) }]
        ),
        "{effects:?}"
    );
    assert!(app.session_list.is_none());
    app.update(models_answer(default_opus()));
    let effects = app.update(key(KeyCode::Enter));
    let sent = requests(&effects);
    let [Request::New { name, model, .. }] = sent.as_slice() else {
        panic!("{effects:?}");
    };
    assert_eq!(name.as_deref(), Some("side"));
    assert_eq!(model.as_deref(), Some("anthropic/opus"));
}

#[test]
fn the_list_opens_when_the_session_ended() {
    let mut app = among_three();
    let effects = app.update(live(SessionEvent::Ended {
        reason: "killed".into(),
    }));
    assert_eq!(requests(&effects), [Request::List]);
    app.update(sessions_answer(vec![session(2, "docs"), session(3, "web")]));
    let list = app.session_list.as_ref().expect("list");
    assert!(list.exit_on_esc);
    // The list keeps up while it is open.
    app.update(Input::Tick);
    assert_eq!(requests(&app.update(Input::Tick)), [Request::List]);
    // Another session: attached, and the TUI goes on.
    assert_eq!(switches(&app.update(key(KeyCode::Enter))), ["docs"]);
    app.update(attached_to(2, "docs", Vec::new()));
    assert!(app.ended.is_none());
    assert!(exit(&app.update(ch('x'))).is_none());

    // Esc leaves, as the session ended.
    let mut app = among_three();
    app.update(live(SessionEvent::Ended {
        reason: "killed".into(),
    }));
    app.update(sessions_answer(Vec::new()));
    let text = screen(&mut app, 80, 24);
    assert!(text.contains("no sessions"), "{text}");
    assert!(text.contains("n new ─ Esc exit"), "{text}");
    assert_eq!(
        exit(&app.update(key(KeyCode::Esc))),
        Some(Exit::Ended {
            name: "project".into(),
            reason: "killed".into()
        })
    );
}

#[test]
fn a_stopping_server_opens_no_list() {
    let mut app = among_three();
    app.update(Input::Server(ServerMessage::Notice {
        notice: Notice::Stopping,
    }));
    let effects = app.update(live(SessionEvent::Ended {
        reason: "the server stopped".into(),
    }));
    assert!(requests(&effects).is_empty());
}

#[test]
fn attach_without_a_name_chooses_in_the_list() {
    let mut app = app();
    app.sessions = three();
    app.choose_session();
    assert!(app.session_list.as_ref().unwrap().exit_on_esc);
    assert_eq!(exit(&app.update(key(KeyCode::Esc))), detached("project"));
}

/// Sessions of all states, made at fixed ages before now.
pub(super) fn various() -> Vec<SessionInfo> {
    let now = crate::agent::now_secs();
    vec![
        SessionInfo {
            created: now - 12 * 60,
            updated: 0,
            turns: 0,
            state: State::Working,
            ..session(1, "project")
        },
        SessionInfo {
            created: now - 2 * 3600,
            updated: 0,
            turns: 0,
            model: "local/qwen".into(),
            ..session(2, "project-2")
        },
        SessionInfo {
            created: now - 3 * 60,
            updated: 0,
            turns: 0,
            state: State::Waiting,
            workspace: "~/ws/docs".into(),
            model: "anthropic/opus".into(),
            question: Some(permission()),
            ..session(3, "docs")
        },
        SessionInfo {
            created: now - 30 * 86_400,
            updated: now - 12 * 86_400,
            turns: 14,
            usage: TokenTotals {
                input_tokens: 200_000,
                output_tokens: 10_000,
                cache_read_tokens: 0,
            },
            state: State::Saved,
            workspace: "~/ws/old".into(),
            ..session(4, "old")
        },
    ]
}

pub(super) fn listing_various(unicode: bool) -> App {
    let mut app = App::new(
        settings(unicode, true),
        attachment(Vec::new()),
        Instant::now(),
    );
    app.update(ctrl('s'));
    app.update(sessions_answer(various()));
    app.update(key(KeyCode::Down));
    app
}

#[test]
fn frame_session_list() {
    insta::assert_snapshot!(screen(&mut listing_various(true), 80, 24));
}

#[test]
fn frame_session_list_ascii() {
    let text = screen(&mut listing_various(false), 80, 24);
    assert!(text.is_ascii(), "{text}");
    insta::assert_snapshot!(text);
}

#[test]
fn frame_session_list_narrow() {
    let text = screen(&mut listing_various(true), 44, 12);
    assert!(!text.contains("USED"), "{text}");
    insta::assert_snapshot!(text);
}

#[test]
fn frame_session_list_marked_and_filtered() {
    let mut app = listing_various(true);
    app.update(ch(' '));
    app.update(ch('/'));
    typed(&mut app, "project");
    app.update(key(KeyCode::Enter));
    let text = screen(&mut app, 80, 24);
    assert!(
        text.contains("filter: project ─ 2 of 4 ─ 1 marked"),
        "{text}"
    );
    insta::assert_snapshot!(text);
}

#[test]
fn frame_session_list_filter_without_match() {
    let mut app = listing_various(true);
    app.update(ch('/'));
    typed(&mut app, "saved >30d");
    let text = screen(&mut app, 80, 24);
    assert!(text.contains("no session matches"), "{text}");
    assert!(text.contains("Filter: saved >30d"), "{text}");
}

#[test]
fn frame_session_list_delete_one() {
    let mut app = listing_various(true);
    app.update(key(KeyCode::Down));
    app.update(key(KeyCode::Down));
    app.update(ch('d'));
    let text = screen(&mut app, 80, 24);
    assert!(
        text.contains("Delete old? 14 turns, 210k tokens. [y] yes  [n] no"),
        "{text}"
    );
    insta::assert_snapshot!(text);
}

#[test]
fn frame_session_list_delete_several() {
    let mut app = listing_various(true);
    app.update(ch('a'));
    app.update(ch('d'));
    let text = screen(&mut app, 80, 24);
    assert!(
        text.contains("Delete 4 sessions? 1 is working, 1 waits for an answer."),
        "{text}"
    );
    insta::assert_snapshot!(text);
}

#[test]
fn deleting_marked_sessions_sends_one_kill_each() {
    let mut app = listing_various(true);
    app.update(ch('a'));
    app.update(ch('d'));
    let kills = requests(&app.update(ch('y')));
    let names: Vec<&str> = kills
        .iter()
        .map(|r| match r {
            Request::Kill { session } => session.as_str(),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(names, ["project", "project-2", "docs", "old"]);
    // The result line waits for all answers; then the list is asked again.
    let answer = |name: &str, result: Result<Payload, String>| Input::Response {
        request: Request::Kill {
            session: name.into(),
        },
        result,
    };
    for name in ["project-2", "docs", "old"] {
        let effects = app.update(answer(name, Ok(Payload::Done)));
        assert!(requests(&effects).is_empty());
    }
    assert!(!screen(&mut app, 80, 24).contains("deleted"));
    let effects = app.update(answer("project", Err("no session named project".into())));
    assert_eq!(requests(&effects), [Request::List]);
    let text = screen(&mut app, 80, 24);
    assert!(
        text.contains("deleted 3 sessions; project: no session named project"),
        "{text}"
    );
    insta::assert_snapshot!(text);
    // The attached session among them: the list stays open.
    assert!(app.session_list.is_some());
}

#[test]
fn frame_session_list_rename() {
    let mut app = listing_various(true);
    app.update(ch('r'));
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}

#[test]
fn frame_picker_for_a_new_session() {
    let mut app = among_three();
    command(&mut app, "/new");
    app.update(models_answer(default_opus()));
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}
