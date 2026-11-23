//! The model change on replay, the `models` request and the picker.

#![allow(clippy::wildcard_imports)]

use super::*;

// The model change on replay.

#[test]
fn replay_shows_the_old_model_from_the_event() {
    // `attached` sets the current model first; the notice must not use it.
    let mut session = info();
    session.model = "local/qwen".into();
    let attachment = Attachment {
        session: session.clone(),
        sessions: vec![session],
        backlog: vec![replayed(SessionEvent::ModelChanged {
            from: "anthropic/sonnet".into(),
            model: "local/qwen".into(),
        })],
    };
    let app = App::new(settings(true, true), attachment, Instant::now());
    assert_eq!(notices(&app), ["model: anthropic/sonnet → local/qwen"]);
    assert_eq!(app.session.model, "local/qwen");
}

#[test]
fn an_old_log_without_from_names_only_the_new_model() {
    let app = app_with(vec![SessionEvent::ModelChanged {
        from: String::new(),
        model: "local/qwen".into(),
    }]);
    assert_eq!(notices(&app), ["model: local/qwen"]);
}

// The `models` request and the picker.

pub(super) fn model(provider: &str, name: &str, id: &str, window: u64) -> ModelInfo {
    ModelInfo {
        model: format!("{provider}/{name}"),
        provider: provider.to_owned(),
        name: name.to_owned(),
        id: id.to_owned(),
        context_window: window,
        default: false,
        key: "env".to_owned(),
    }
}

pub(super) fn model_list() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            default: true,
            ..model("anthropic", "sonnet", "claude-sonnet-5-5", 200_000)
        },
        model("anthropic", "opus", "claude-opus-5-5", 200_000),
        ModelInfo {
            key: "missing".to_owned(),
            ..model("local", "qwen", "qwen3-coder:30b", 32_768)
        },
    ]
}

/// `models` for the test session's workspace.
pub(super) fn models_request() -> Request {
    Request::Models {
        workspace: Some("/ws/project".into()),
    }
}

pub(super) fn models_answer(models: Vec<ModelInfo>) -> Input {
    models_answer_from(models, DefaultFrom::User)
}

pub(super) fn models_answer_from(models: Vec<ModelInfo>, default_from: DefaultFrom) -> Input {
    Input::Response {
        request: models_request(),
        result: Ok(Payload::Models {
            models,
            config: "/home/u/.config/codeagent/config.toml".into(),
            default_from,
        }),
    }
}

/// An app with the picker open on the usual list.
pub(super) fn picking() -> App {
    let mut app = app();
    assert_eq!(requests(&app.update(ctrl('o'))), [models_request()]);
    assert!(app.picker.is_none(), "it opens with the answer");
    app.update(models_answer(model_list()));
    assert!(app.picker.is_some());
    app
}

pub(super) fn chosen(app: &App) -> String {
    let picker = app.picker.as_ref().unwrap();
    picker.chosen().unwrap().model.clone()
}

#[test]
fn slash_model_opens_the_picker() {
    let mut app = app();
    let effects = command(&mut app, "/model");
    assert_eq!(requests(&effects), [models_request()]);
    assert!(app.editor.is_empty());
    app.update(models_answer(model_list()));
    assert!(app.picker.is_some());
}

#[test]
fn ctrl_o_works_in_the_conversation_too() {
    let mut app = app();
    app.update(key(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Conversation);
    assert_eq!(requests(&app.update(ctrl('o'))), [models_request()]);
}

#[test]
fn the_answer_replaces_the_windows() {
    let mut app = app();
    assert_eq!(app.context_window(), Some(200_000));
    app.update(models_answer(vec![model(
        "anthropic",
        "sonnet",
        "x",
        50_000,
    )]));
    assert_eq!(app.context_window(), Some(50_000));
    // Without having asked for the picker, it does not open.
    assert!(app.picker.is_none());
    app.update(models_answer(vec![model("other", "m", "x", 1)]));
    assert_eq!(app.context_window(), None);
}

#[test]
fn a_reload_asks_for_the_models() {
    let mut app = app();
    let effects = app.update(Input::Server(ServerMessage::Notice {
        notice: Notice::ConfigReloaded,
    }));
    assert_eq!(requests(&effects), [models_request()]);
}

#[test]
fn a_reload_refreshes_an_open_picker() {
    let mut app = picking();
    app.update(key(KeyCode::Down));
    assert_eq!(chosen(&app), "anthropic/opus");
    let mut newer = model_list();
    newer.remove(0);
    app.update(models_answer(newer));
    assert_eq!(chosen(&app), "anthropic/opus");
    let picker = app.picker.as_ref().unwrap();
    assert_eq!(picker.visible().len(), 2);
}

#[test]
fn a_failed_models_request_goes_to_the_conversation() {
    let mut app = app();
    app.update(ctrl('o'));
    app.update(Input::Response {
        request: models_request(),
        result: Err("bad config".to_owned()),
    });
    assert!(app.picker.is_none());
    assert_eq!(kinds(&app), [Kind::Error("bad config".into())]);
    // A lost connection is no news, and a refresh fails quietly.
    let mut app = self::app();
    app.update(ctrl('o'));
    app.update(Input::Response {
        request: models_request(),
        result: Err(LOST.to_owned()),
    });
    assert!(kinds(&app).is_empty());
    let mut app = self::app();
    app.update(Input::Response {
        request: models_request(),
        result: Err("bad config".to_owned()),
    });
    assert!(kinds(&app).is_empty());
}

#[test]
fn the_picker_starts_on_the_current_model() {
    let mut session_on_opus = app();
    session_on_opus.session.model = "anthropic/opus".into();
    session_on_opus.update(ctrl('o'));
    session_on_opus.update(models_answer(model_list()));
    assert_eq!(chosen(&session_on_opus), "anthropic/opus");
}

#[test]
fn up_and_down_select_and_stop_at_the_ends() {
    let mut app = picking();
    assert_eq!(chosen(&app), "anthropic/sonnet");
    app.update(key(KeyCode::Up));
    assert_eq!(chosen(&app), "anthropic/sonnet");
    app.update(key(KeyCode::Down));
    app.update(key(KeyCode::Down));
    assert_eq!(chosen(&app), "local/qwen");
    app.update(key(KeyCode::Down));
    assert_eq!(chosen(&app), "local/qwen");
}

#[test]
fn typing_filters_by_provider_name_and_id() {
    for (filter, expected) in [
        ("local", vec!["local/qwen"]),
        ("OPUS", vec!["anthropic/opus"]),
        ("coder:30", vec!["local/qwen"]),
        ("claude-", vec!["anthropic/sonnet", "anthropic/opus"]),
        ("anthropic/s", vec!["anthropic/sonnet"]),
    ] {
        let mut app = picking();
        typed(&mut app, filter);
        let picker = app.picker.as_ref().unwrap();
        let shown: Vec<&str> = picker.visible().iter().map(|m| m.model.as_str()).collect();
        assert_eq!(shown, expected, "{filter}");
    }
    let mut app = picking();
    typed(&mut app, "qwenx");
    app.update(key(KeyCode::Backspace));
    assert_eq!(chosen(&app), "local/qwen");
    // Typing goes to the filter, not to the input.
    assert!(app.editor.is_empty());
}

#[test]
fn paste_goes_to_the_filter() {
    let mut app = picking();
    app.update(Input::Paste("qwen\n".to_owned()));
    assert!(app.editor.is_empty());
    let picker = app.picker.as_ref().unwrap();
    assert_eq!(picker.filter.text(), "qwen ");
}

/// The `default_model` saves among `effects`: path, model, workspace.
pub(super) fn default_saves(effects: &[Effect]) -> Vec<(String, String, String)> {
    effects
        .iter()
        .filter_map(|e| match e {
            Effect::SaveDefault {
                path,
                model,
                workspace,
            } => Some((
                path.display().to_string(),
                model.clone(),
                workspace.display().to_string(),
            )),
            _ => None,
        })
        .collect()
}

pub(super) const USER_FILE: &str = "/home/u/.config/codeagent/config.toml";

#[test]
fn space_ticks_the_default_and_leaves_the_filter() {
    let mut app = picking();
    app.update(key(KeyCode::Down));
    app.update(ch(' '));
    let picker = app.picker.as_ref().unwrap();
    assert!(picker.make_default);
    assert_eq!(picker.filter.text(), "");
    assert_eq!(chosen(&app), "anthropic/opus");
    app.update(ch(' '));
    assert!(!app.picker.as_ref().unwrap().make_default);
    // A pasted space still reaches the filter.
    app.update(Input::Paste("op us".to_owned()));
    assert_eq!(app.picker.as_ref().unwrap().filter.text(), "op us");
}

#[test]
fn space_on_the_default_does_nothing() {
    let mut app = picking();
    assert_eq!(chosen(&app), "anthropic/sonnet");
    app.update(ch(' '));
    let picker = app.picker.as_ref().unwrap();
    assert!(picker.already_default());
    assert!(!picker.make_default);
}

#[test]
fn enter_with_the_tick_switches_and_saves_the_default() {
    let mut app = picking();
    app.update(key(KeyCode::Down));
    app.update(ch(' '));
    let effects = app.update(key(KeyCode::Enter));
    assert!(app.picker.is_none());
    assert_eq!(
        requests(&effects),
        [Request::SetModel {
            session: "project".into(),
            model: "anthropic/opus".into(),
        }]
    );
    assert_eq!(
        default_saves(&effects),
        [(
            USER_FILE.into(),
            "anthropic/opus".into(),
            "/ws/project".into()
        )]
    );
}

#[test]
fn the_tick_on_the_current_model_only_saves() {
    let mut app = app();
    app.update(ctrl('o'));
    // The default is opus; the session has sonnet.
    app.update(models_answer(default_opus()));
    assert_eq!(chosen(&app), "anthropic/sonnet");
    app.update(ch(' '));
    let effects = app.update(key(KeyCode::Enter));
    assert!(requests(&effects).is_empty());
    assert_eq!(
        default_saves(&effects),
        [(
            USER_FILE.into(),
            "anthropic/sonnet".into(),
            "/ws/project".into()
        )]
    );
}

#[test]
fn without_the_tick_or_with_esc_nothing_is_saved() {
    let mut app = picking();
    app.update(key(KeyCode::Down));
    assert!(default_saves(&app.update(key(KeyCode::Enter))).is_empty());
    let mut app = picking();
    app.update(key(KeyCode::Down));
    app.update(ch(' '));
    assert!(default_saves(&app.update(key(KeyCode::Esc))).is_empty());
    assert!(app.picker.is_none());
    // The tick starts unticked when the picker opens again.
    app.update(ctrl('o'));
    app.update(models_answer(model_list()));
    assert!(!app.picker.as_ref().unwrap().make_default);
}

#[test]
fn a_saved_default_is_reported_and_the_marks_asked_again() {
    let mut app = app();
    let saved = SavedDefault {
        model: ModelRef {
            provider: "local".into(),
            model: "qwen".into(),
        },
        path: USER_FILE.into(),
        from: crate::config::DefaultFrom::User,
        effective: ModelRef {
            provider: "local".into(),
            model: "qwen".into(),
        },
    };
    let effects = app.update(Input::DefaultSaved(Ok(saved.clone())));
    assert_eq!(requests(&effects), [models_request()]);
    assert_eq!(
        notices(&app),
        [format!("default model: local/qwen (saved in {USER_FILE})")]
    );
    let mut app = self::app();
    app.update(Input::DefaultSaved(Ok(SavedDefault {
        from: crate::config::DefaultFrom::Env,
        effective: ModelRef {
            provider: "anthropic".into(),
            model: "sonnet".into(),
        },
        ..saved
    })));
    assert_eq!(
        notices(&app)[1],
        "⚠ CODEAGENT_MODEL=anthropic/sonnet still sets the default of new sessions"
    );
    let mut app = self::app();
    app.update(Input::DefaultSaved(Err("cannot write x".into())));
    assert_eq!(
        kinds(&app),
        [Kind::Error(
            "default model not saved: cannot write x".into()
        )]
    );
}

#[test]
fn slash_model_with_default() {
    let mut app = app();
    let effects = command(&mut app, "/model qwen --default");
    assert_eq!(
        requests(&effects),
        [Request::SetModel {
            session: "project".into(),
            model: "qwen".into(),
        }]
    );
    // Without a models answer yet, the TUI's own user file.
    let saves = default_saves(&effects);
    assert_eq!(saves.len(), 1);
    assert_eq!(saves[0].1, "qwen");
    // `/model --default`: the session's model, without a switch.
    let mut app = picking();
    app.update(key(KeyCode::Esc));
    let effects = command(&mut app, "/model --default");
    assert!(requests(&effects).is_empty());
    assert_eq!(
        default_saves(&effects),
        [(
            USER_FILE.into(),
            "anthropic/sonnet".into(),
            "/ws/project".into()
        )]
    );
}

#[test]
fn frame_picker_with_the_default_ticked() {
    let mut app = picking();
    app.update(key(KeyCode::Down));
    app.update(ch(' '));
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}

#[test]
fn frame_picker_when_codeagent_model_sets_the_default() {
    let mut app = app();
    app.update(ctrl('o'));
    app.update(models_answer_from(default_opus(), DefaultFrom::Env));
    app.update(ch(' '));
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}

#[test]
fn enter_sets_the_selected_model_and_closes() {
    let mut app = picking();
    app.update(key(KeyCode::Down));
    let effects = app.update(key(KeyCode::Enter));
    assert_eq!(
        requests(&effects),
        [Request::SetModel {
            session: "project".into(),
            model: "anthropic/opus".into(),
        }]
    );
    assert!(app.picker.is_none());
}

#[test]
fn enter_on_the_current_model_only_closes() {
    let mut app = picking();
    let effects = app.update(key(KeyCode::Enter));
    assert!(requests(&effects).is_empty());
    assert!(app.picker.is_none());
}

#[test]
fn enter_with_no_match_does_nothing() {
    let mut app = picking();
    typed(&mut app, "zzz");
    let effects = app.update(key(KeyCode::Enter));
    assert!(requests(&effects).is_empty());
    assert!(app.picker.is_some());
}

#[test]
fn esc_closes_and_cancels_nothing() {
    let mut app = picking();
    app.session.state = State::Working;
    let effects = app.update(key(KeyCode::Esc));
    assert!(requests(&effects).is_empty());
    assert!(app.picker.is_none());
}

#[test]
fn a_model_can_be_chosen_during_a_turn() {
    let mut app = picking();
    app.session.state = State::Working;
    app.update(key(KeyCode::Down));
    assert_eq!(requests(&app.update(key(KeyCode::Enter))).len(), 1);
}

#[test]
fn the_picker_does_not_open_while_a_prompt_is_open() {
    let mut app = with_prompt(idle_session());
    let effects = app.update(ctrl('o'));
    assert!(requests(&effects).is_empty());
    assert!(app.picker.is_none());
    // An answer that was on its way does not open it either.
    app.update(models_answer(model_list()));
    assert!(app.picker.is_none());
}

#[test]
fn a_question_closes_the_picker_and_keeps_the_keys_from_answering_it() {
    let mut app = picking();
    app.update(live(SessionEvent::Question(Box::new(limit()))));
    assert!(app.prompt.is_some());
    assert!(app.picker.is_none());
    // The answer of a request made just before the question does not open it.
    let mut app = self::app();
    app.update(ctrl('o'));
    app.update(live(SessionEvent::Question(Box::new(limit()))));
    app.update(models_answer(model_list()));
    assert!(app.picker.is_none());
}

#[test]
fn the_end_of_the_session_closes_the_picker() {
    let mut app = picking();
    app.update(live(SessionEvent::Ended {
        reason: "killed".into(),
    }));
    assert!(app.picker.is_none());
}

#[test]
fn frame_picker_with_marks() {
    let mut app = app_with(vec![usage(40_000, 1_000, 41_000)]);
    app.update(ctrl('o'));
    app.update(models_answer(model_list()));
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}

#[test]
fn frame_picker_ascii() {
    let mut app = App::new(
        settings(false, true),
        attachment(vec![usage(40_000, 1_000, 41_000)]),
        Instant::now(),
    );
    app.update(ctrl('o'));
    app.update(models_answer(model_list()));
    let text = screen(&mut app, 80, 24);
    assert!(text.is_ascii(), "{text}");
    insta::assert_snapshot!(text);
}

#[test]
fn frame_picker_with_no_match() {
    let mut app = picking();
    typed(&mut app, "zzz");
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}

#[test]
fn frame_picker_with_no_models() {
    let mut app = app();
    app.update(ctrl('o'));
    app.update(models_answer(Vec::new()));
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}

#[test]
fn frame_picker_keeps_the_selected_row_in_a_small_box() {
    let mut app = app();
    app.update(ctrl('o'));
    let many: Vec<ModelInfo> = (0..30)
        .map(|i| model("p", &format!("m{i}"), &format!("id-{i}"), 1000))
        .collect();
    app.update(models_answer(many));
    for _ in 0..29 {
        app.update(key(KeyCode::Down));
    }
    let text = screen(&mut app, 60, 12);
    assert!(text.contains("m29"), "{text}");
    assert!(text.contains("Esc cancel"), "{text}");
    assert!(text.contains("Filter:"), "{text}");
}
