//! The status box (RFC 0007).

#![allow(clippy::wildcard_imports)]

use super::*;

// The status box (RFC 0007).

pub(super) const CREATED: u64 = 1_793_491_200;

/// A status after a turn with tool calls and questions, `state` now.
pub(super) fn status_of(state: State) -> crate::protocol::SessionStatus {
    use crate::agent::{ToolCallEvent, TurnEnd};
    use crate::protocol::{
        ClientInfo, DirInfo, LimitsInfo, ModelStatus, SessionStatus, Stats, Status,
    };
    let t = CREATED * 1000;
    let tool = |name: &str, state: CallState| {
        SessionEvent::ToolCall(ToolCallEvent {
            details: None,
            id: "c".into(),
            name: name.into(),
            state,
            summary: None,
            arguments: serde_json::Value::Null,
            output: None,
            result: None,
        })
    };
    let log = [
        (
            t,
            SessionEvent::State {
                state: State::Working,
            },
        ),
        (
            t,
            SessionEvent::UserMessage {
                text: "go".into(),
                images: Vec::new(),
            },
        ),
        (
            t + 2_000,
            SessionEvent::Usage {
                input_tokens: 2_100_000,
                output_tokens: 84_300,
                cache_read_tokens: 0,
                cache_write_tokens: 0,
                context_tokens: 170_000,
                model: String::new(),
            },
        ),
        (t + 3_000, tool("read_file", CallState::Done)),
        (t + 3_000, tool("read_file", CallState::Done)),
        (t + 4_000, tool("edit_file", CallState::Denied)),
        (
            t + 4_000,
            SessionEvent::QuestionClosed {
                id: "q1".into(),
                closed: Closed::Answered {
                    answer: Answer::Allow,
                    by: "tui pts/3".into(),
                },
            },
        ),
        (
            t + 5_000,
            SessionEvent::Retry {
                attempt: 1,
                max: 3,
                reason: "HTTP 503".into(),
            },
        ),
        (
            t + 60_000,
            SessionEvent::TurnEnd(TurnEnd {
                reason: EndReason::Done,
                requests: 1,
                tool_calls: 3,
                input_tokens: 0,
                output_tokens: 0,
                cache_read_tokens: 0,
            }),
        ),
        (t + 60_000, SessionEvent::State { state: State::Idle }),
        (
            t + 70_000,
            SessionEvent::ModelChanged {
                from: "anthropic/haiku".into(),
                model: "anthropic/sonnet".into(),
            },
        ),
    ];
    let mut stats = Stats::of_log(log.iter().map(|(at, e)| (Some(*at), e)));
    if state == State::Working {
        stats.count(&SessionEvent::State { state }, Some(t + 100_000));
        stats.count(
            &SessionEvent::UserMessage {
                text: "more".into(),
                images: Vec::new(),
            },
            Some(t + 100_000),
        );
    }
    SessionStatus {
        session: SessionInfo {
            state,
            created: CREATED,
            updated: 0,
            turns: 0,
            queued: 1,
            ..info()
        },
        stats,
        clients: vec![
            ClientInfo {
                name: "chat pts/5".into(),
                answers: false,
            },
            ClientInfo {
                name: "tui pts/3".into(),
                answers: true,
            },
        ],
        dir: Some("~/.local/state/codeagent/sessions/1".into()),
        size: Some(1_234_567),
        extra_dirs: vec![DirInfo {
            path: "~/ws/shared-lib".into(),
            read_only: false,
            exists: false,
        }],
        scratch: None,
        scratch_size: None,
        skills: Vec::new(),
        model: Some(ModelStatus {
            info: ModelInfo {
                model: "anthropic/sonnet".into(),
                provider: "anthropic".into(),
                name: "sonnet".into(),
                id: "claude-sonnet-5-5".into(),
                context_window: 200_000,
                default: true,
                key: "env".into(),
            },
            api: "anthropic".into(),
            base_url: "https://api.anthropic.com".into(),
            temperature: None,
            max_output_tokens: None,
            max_retries: 3,
        }),
        limits: Some(LimitsInfo {
            max_requests: 50,
            max_tool_calls: 120,
        }),
        now: t + 7_800_000,
        server: Status {
            pid: 4242,
            sessions: 3,
            running: 2,
            busy: 1,
            clients: 3,
            version: "0.4.0".into(),
        },
    }
}

pub(super) fn status_answer(status: crate::protocol::SessionStatus) -> Input {
    Input::Response {
        request: Request::SessionStatus {
            session: status.session.name.clone(),
        },
        result: Ok(Payload::SessionStatus {
            status: Box::new(status),
        }),
    }
}

pub(super) fn status_requests(effects: &[Effect]) -> Vec<Request> {
    requests(effects)
        .into_iter()
        .filter(|r| matches!(r, Request::SessionStatus { .. }))
        .collect()
}

#[test]
fn status_opens_the_box_and_asks() {
    let mut app = app();
    let effects = command(&mut app, "/status");
    assert_eq!(
        status_requests(&effects),
        [Request::SessionStatus {
            session: "project".into()
        }]
    );
    assert!(app.editor.is_empty());
    assert!(screen(&mut app, 80, 24).contains("asking the server"));
    app.update(status_answer(status_of(State::Idle)));
    let text = screen(&mut app, 80, 50);
    assert!(text.contains("Tool calls"), "{text}");
    // Keys go to the box: `q` closes it and types nothing.
    app.update(ch('q'));
    assert!(app.status_box.is_none());
    assert!(app.editor.is_empty());
}

#[test]
fn the_status_box() {
    let mut app = app();
    command(&mut app, "/status");
    app.update(status_answer(status_of(State::Working)));
    insta::assert_snapshot!(screen(&mut app, 80, 56));
}

#[test]
fn the_status_box_in_ascii_and_scrolled() {
    let mut app = App::new(
        settings(false, false),
        attachment(Vec::new()),
        Instant::now(),
    );
    command(&mut app, "/status");
    app.update(status_answer(status_of(State::Idle)));
    let first = screen(&mut app, 80, 24);
    assert!(first.contains("Session"), "{first}");
    assert!(!first.contains("Server"), "{first}");
    app.update(key(KeyCode::End));
    let last = screen(&mut app, 80, 24);
    assert!(last.contains("Server"), "{last}");
    assert!(last.contains("0.4.0 - pid 4242"), "{last}");
    assert!(last.contains("Esc close"), "{last}");
    app.update(key(KeyCode::Home));
    assert_eq!(app.status_box.as_ref().unwrap().top, 0);
}

#[test]
fn the_status_box_refreshes_while_the_session_works() {
    let mut app = app();
    command(&mut app, "/status");
    // Not twice at a time.
    assert!(status_requests(&app.update(Input::Tick)).is_empty());
    app.update(status_answer(status_of(State::Working)));
    assert_eq!(status_requests(&app.update(Input::Tick)).len(), 1);
    assert!(status_requests(&app.update(Input::Tick)).is_empty());
    app.update(status_answer(status_of(State::Idle)));
    // Idle: no polling, until an event changes it.
    assert!(status_requests(&app.update(Input::Tick)).is_empty());
    app.update(live(SessionEvent::Usage {
        input_tokens: 1,
        output_tokens: 1,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        context_tokens: 2,
        model: String::new(),
    }));
    assert_eq!(status_requests(&app.update(Input::Tick)).len(), 1);
    // A failed refresh keeps what is shown.
    app.update(Input::Response {
        request: Request::SessionStatus {
            session: "project".into(),
        },
        result: Err("busy".into()),
    });
    assert!(app.status_box.as_ref().unwrap().status.is_some());
}

#[test]
fn status_of_another_session() {
    let mut app = among_three();
    let effects = command(&mut app, "/status docs");
    assert_eq!(
        status_requests(&effects),
        [Request::SessionStatus {
            session: "docs".into()
        }]
    );
    // An unknown name stays in the input.
    app.update(key(KeyCode::Esc));
    let effects = command(&mut app, "/status nope");
    assert!(status_requests(&effects).is_empty());
    assert_eq!(app.editor.text(), "/status nope");
    // A first answer that fails closes the box with the error.
    app.editor.clear();
    command(&mut app, "/status web");
    app.update(Input::Response {
        request: Request::SessionStatus {
            session: "web".into(),
        },
        result: Err("no session named web".into()),
    });
    assert!(app.status_box.is_none());
    assert!(kinds(&app).contains(&Kind::Error("no session named web".into())));
}

#[test]
fn the_status_box_closes_on_a_question_and_a_switch() {
    let mut app = among_three();
    command(&mut app, "/status");
    app.update(status_answer(status_of(State::Idle)));
    app.update(live(SessionEvent::ToolCall(call(
        "c1",
        "edit_file",
        CallState::Waiting,
    ))));
    app.update(live(SessionEvent::Question(Box::new(permission()))));
    assert!(app.status_box.is_none());

    let mut app = among_three();
    command(&mut app, "/status");
    app.update(ctrl('n'));
    app.update(attached_to(2, "docs", Vec::new()));
    assert!(app.status_box.is_none());
}

#[test]
fn status_completes_session_names_with_the_attached_one() {
    let mut app = among_three();
    typed(&mut app, "/status ");
    let values: Vec<String> = app.arguments().into_iter().map(|v| v.value).collect();
    assert_eq!(values, ["project", "docs", "web"]);
}

#[test]
fn a_retry_is_a_notice() {
    let mut app = app();
    app.update(live(SessionEvent::Retry {
        attempt: 2,
        max: 3,
        reason: "rate limited".into(),
    }));
    assert!(kinds(&app).contains(&Kind::Notice("retrying (2/3): rate limited".into())));
}
