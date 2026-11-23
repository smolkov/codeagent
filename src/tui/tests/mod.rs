//! State tests of `App::update` and frames with the test backend, without a
//! terminal or a server.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::style::Color;
use serde_json::json;

use super::app::{
    App, Attachment, Dialog, Effect, Exit, Input, LOST, RECONNECT_FOR, Settings, clock,
};
use super::commands::COMMANDS;
use super::conversation::{Expansion, Kind};
use super::keys::Focus;
use super::prompt::Mode;
use super::style::Look;
use super::{check_terminal, exit_line, exit_result, first_session, parse_offset, save, view};
use crate::agent::{
    Answer, CallInfo, CallResult, CallState, Closed, EndReason, LIMIT_CHOICES, Limit,
    PERMISSION_CHOICES, Permission, Question, QuestionKind, SessionEvent, SessionRef, State,
    ToolCallEvent, TurnEnd, TurnInfo,
};
use crate::config::{ModelRef, SavedDefault, Tool};
use crate::error::Error;
use crate::permissions::Reason;
use crate::protocol::{
    DefaultFrom, ModelInfo, Notice, Payload, Request, ServerMessage, SessionInfo, TokenTotals,
};
use crate::tools::{Details, PathDetails, Text};

const DIFF: &str = "@@ -1,1 +1,1 @@\n-for i in 0..lines.len() - 1 {\n+for i in 0..lines.len() {\n";

fn settings(unicode: bool, colour: bool) -> Settings {
    Settings {
        look: Look::new(unicode, colour),
        client: "tui pts/3".to_owned(),
        tool_output_lines: 20,
        scroll_lines: 3,
        timestamps: None,
        windows: HashMap::from([("anthropic/sonnet".to_owned(), 200_000)]),
        tui: crate::config::Tui {
            symbols: if unicode {
                crate::config::Symbols::Unicode
            } else {
                crate::config::Symbols::Ascii
            },
            // Finished calls are one line, as before previews; the tests of
            // previews set `changes`.
            tool_preview: crate::config::ToolPreview::None,
            ..crate::config::Tui::default()
        },
        theming: super::app::Theming {
            themes: crate::config::themes::file_themes(&[]).0,
            warnings: Vec::new(),
            file: super::app::FileTui {
                tui: crate::config::Tui::default(),
                keys: Vec::new(),
                stamp: None,
            },
            colouring: if colour {
                super::style::Colouring::Ansi256
            } else {
                super::style::Colouring::None
            },
            unicode_locale: unicode,
            theme_env: None,
            offset: 3600,
            user_file: "/home/u/.config/codeagent/config.toml".into(),
            home: Some("/home/u".into()),
        },
        warnings: Vec::new(),
    }
}

fn info() -> SessionInfo {
    SessionInfo {
        id: 1,
        name: "project".to_owned(),
        workspace: "~/ws/project".to_owned(),
        path: "/ws/project".into(),
        model: "anthropic/sonnet".to_owned(),
        state: State::Idle,
        created: 0,
        updated: 0,
        turns: 0,
        queued: 0,
        question: None,
        usage: Default::default(),
        tmp: None,
    }
}

fn replayed(event: SessionEvent) -> ServerMessage {
    ServerMessage::Event {
        session: 1,
        replay: true,
        event,
    }
}

fn attachment(events: Vec<SessionEvent>) -> Attachment {
    Attachment {
        session: info(),
        sessions: vec![info()],
        backlog: events.into_iter().map(replayed).collect(),
    }
}

fn app_with(events: Vec<SessionEvent>) -> App {
    App::new(settings(true, true), attachment(events), Instant::now())
}

fn app() -> App {
    app_with(Vec::new())
}

fn live(event: SessionEvent) -> Input {
    Input::Server(ServerMessage::Event {
        session: 1,
        replay: false,
        event,
    })
}

fn press(code: KeyCode, modifiers: KeyModifiers) -> Input {
    Input::Key(KeyEvent::new(code, modifiers))
}

fn key(code: KeyCode) -> Input {
    press(code, KeyModifiers::NONE)
}

fn ch(c: char) -> Input {
    key(KeyCode::Char(c))
}

fn ctrl(c: char) -> Input {
    press(KeyCode::Char(c), KeyModifiers::CONTROL)
}

fn typed(app: &mut App, text: &str) {
    for c in text.chars() {
        app.update(ch(c));
    }
}

fn requests(effects: &[Effect]) -> Vec<Request> {
    effects
        .iter()
        .filter_map(|e| match e {
            Effect::Request(r) => Some(r.clone()),
            _ => None,
        })
        .collect()
}

fn exit(effects: &[Effect]) -> Option<Exit> {
    effects.iter().find_map(|e| match e {
        Effect::Exit(exit) => Some(exit.clone()),
        _ => None,
    })
}

fn detached(name: &str) -> Option<Exit> {
    Some(Exit::Detached {
        name: name.to_owned(),
    })
}

fn kinds(app: &App) -> Vec<Kind> {
    app.conversation
        .entries
        .iter()
        .map(|e| e.kind.clone())
        .collect()
}

fn notices(app: &App) -> Vec<String> {
    kinds(app)
        .into_iter()
        .filter_map(|k| match k {
            Kind::Notice(text) => Some(text),
            _ => None,
        })
        .collect()
}

fn call(id: &str, name: &str, state: CallState) -> ToolCallEvent {
    ToolCallEvent {
        details: None,
        id: id.to_owned(),
        name: name.to_owned(),
        state,
        summary: Some("src/parse.rs".to_owned()),
        arguments: json!({"path": "src/parse.rs"}),
        output: None,
        result: None,
    }
}

fn done(mut event: ToolCallEvent, text: &str) -> ToolCallEvent {
    event.state = CallState::Done;
    event.result = Some(CallResult {
        text: text.to_owned(),
        is_error: false,
    });
    event
}

fn permission() -> Question {
    let path = PathDetails {
        written: "src/parse.rs".to_owned(),
        resolved: "src/parse.rs".to_owned(),
        area: "workspace".to_owned(),
    };
    Question {
        id: "q1".to_owned(),
        session: SessionRef::default(),
        opened: 0,
        turn: TurnInfo {
            number: 1,
            requests: 1,
            tool_calls: 1,
        },
        summary: "edit_file: src/parse.rs (line 1)".to_owned(),
        choices: PERMISSION_CHOICES.to_vec(),
        kind: QuestionKind::Permission(Box::new(Permission {
            call: CallInfo {
                tool: Tool::EditFile,
                id: "c1".to_owned(),
                arguments: json!({"path": "src/parse.rs"}),
            },
            details: Details::EditFile {
                path,
                diff: Text {
                    text: DIFF.to_owned(),
                    cut: false,
                },
                first_line: 1,
                last_line: 1,
                occurrences: 1,
                size: 10,
                modified: None,
            },
            reason: Reason::Default {
                tool: Tool::EditFile,
            },
            reason_text: "default for edit_file".to_owned(),
            proposals: vec![
                vec!["edit_file(src/parse.rs)".to_owned()],
                vec!["edit_file(src/**)".to_owned()],
            ],
            text: None,
        })),
    }
}

fn limit() -> Question {
    Question {
        id: "q2".to_owned(),
        session: SessionRef::default(),
        opened: 0,
        turn: TurnInfo {
            number: 1,
            requests: 50,
            tool_calls: 120,
        },
        summary: "limit".to_owned(),
        choices: LIMIT_CHOICES.to_vec(),
        kind: QuestionKind::Limit(Limit {
            max_requests: 50,
            max_tool_calls: 120,
            requests: 50,
            tool_calls: 120,
        }),
    }
}

/// An app with the edit of `c1` waiting for an answer.
fn asked() -> App {
    let mut app = app();
    app.update(live(SessionEvent::State {
        state: State::Waiting,
    }));
    app.update(live(SessionEvent::ToolCall(call(
        "c1",
        "edit_file",
        CallState::Waiting,
    ))));
    app.update(live(SessionEvent::Question(Box::new(permission()))));
    app
}

fn answer_of(effects: &[Effect]) -> Answer {
    match requests(effects).as_slice() {
        [Request::Answer { answer, .. }] => answer.clone(),
        other => panic!("{other:?}"),
    }
}

fn screen_of(app: &mut App, width: u16, height: u16) -> Terminal<TestBackend> {
    app.layout(Rect::new(0, 0, width, height));
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| view::draw(app, frame)).unwrap();
    terminal
}

/// The screen as text, one line per row.
fn screen(app: &mut App, width: u16, height: u16) -> String {
    let terminal = screen_of(app, width, height);
    let buffer = terminal.backend().buffer();
    let area = *buffer.area();
    let mut out = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            out.push_str(buffer[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}
use crate::config::edit::Setting;

// The topics, one module each; their helpers are shared through these
// imports (see `AGENTS.md`, keep test files small).
mod activity;
mod boxes;
mod calls;
mod clicks;
mod commands;
mod events;
mod frames;
mod images;
mod keys;
mod models;
mod prompts;
mod sessions;
mod settings;
mod status;

#[allow(unused_imports)]
use activity::*;
#[allow(unused_imports)]
use boxes::*;
#[allow(unused_imports)]
use calls::*;
#[allow(unused_imports)]
use clicks::*;
#[allow(unused_imports)]
use commands::*;
#[allow(unused_imports)]
use events::*;
#[allow(unused_imports)]
use frames::*;
#[allow(unused_imports)]
use keys::*;
#[allow(unused_imports)]
use models::*;
#[allow(unused_imports)]
use prompts::*;
#[allow(unused_imports)]
use sessions::*;
#[allow(unused_imports)]
use settings::*;
#[allow(unused_imports)]
use status::*;
