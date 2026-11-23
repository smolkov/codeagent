//! The command list and argument completion.

#![allow(clippy::wildcard_imports)]

use super::*;

// The command list.

pub(super) fn listed(app: &App) -> Vec<&'static str> {
    app.listed().iter().map(|c| c.name).collect()
}

pub(super) fn selected_command(app: &App) -> Option<&'static str> {
    let row = app.command_list?;
    app.listed().get(row).map(|c| c.name)
}

pub(super) fn working(app: &mut App) {
    app.update(live(SessionEvent::State {
        state: State::Working,
    }));
}

#[test]
fn command_list_opens_on_a_slash_in_the_empty_input() {
    let mut app = app();
    assert!(app.command_list.is_none());
    typed(&mut app, "/");
    assert_eq!(app.command_list, Some(0));
    assert_eq!(listed(&app).len(), COMMANDS.len());
}

#[test]
fn command_list_does_not_open_otherwise() {
    // After other text.
    let mut app = app();
    typed(&mut app, "a/");
    assert!(app.command_list.is_none());
    // From a paste.
    let mut app = self::app();
    app.update(Input::Paste("/model".into()));
    assert!(app.command_list.is_none());
    // From a message of the history.
    let mut app = app_with(vec![SessionEvent::UserMessage {
        text: "/etc/passwd".into(),
        images: Vec::new(),
    }]);
    app.update(key(KeyCode::Up));
    assert_eq!(app.editor.text(), "/etc/passwd");
    assert!(app.command_list.is_none());
    // With focus on the conversation.
    let mut app = self::app();
    app.update(key(KeyCode::Tab));
    typed(&mut app, "/");
    assert!(app.command_list.is_none());
}

#[test]
fn command_list_closes_when_the_text_is_no_search() {
    for (keys, text) in [("//", "//"), ("/model ", "/model ")] {
        let mut app = app();
        typed(&mut app, keys);
        assert!(app.command_list.is_none(), "{keys}");
        assert_eq!(app.editor.text(), text);
    }
    // A new line.
    let mut app = app();
    typed(&mut app, "/m");
    app.update(press(KeyCode::Enter, KeyModifiers::ALT));
    assert!(app.command_list.is_none());
    // `Backspace` on the `/`.
    let mut app = self::app();
    typed(&mut app, "/");
    app.update(key(KeyCode::Backspace));
    assert!(app.command_list.is_none());
    assert!(app.editor.is_empty());
    // `Ctrl-C` clears the input.
    let mut app = self::app();
    typed(&mut app, "/m");
    app.update(ctrl('c'));
    assert!(app.command_list.is_none());
    assert!(app.editor.is_empty());
}

#[test]
fn a_message_written_with_two_slashes_is_still_sent() {
    let mut app = app();
    assert_eq!(
        requests(&command(&mut app, "//text")),
        [Request::Send {
            session: "project".into(),
            text: "/text".into(),
            images: Vec::new(),
        }]
    );
}

#[test]
fn esc_only_closes_the_command_list() {
    let mut app = app();
    working(&mut app);
    typed(&mut app, "/m");
    // The first closes the list; the text stays, the turn goes on.
    assert!(requests(&app.update(key(KeyCode::Esc))).is_empty());
    assert!(app.command_list.is_none());
    assert_eq!(app.editor.text(), "/m");
    // The second cancels the turn.
    assert_eq!(
        requests(&app.update(key(KeyCode::Esc))),
        [Request::Cancel {
            session: "project".into()
        }]
    );
}

#[test]
fn a_closed_command_list_opens_again_only_from_an_empty_input() {
    let mut app = app();
    typed(&mut app, "/m");
    app.update(key(KeyCode::Esc));
    typed(&mut app, "o");
    assert!(app.command_list.is_none());
    app.update(key(KeyCode::Backspace));
    assert!(app.command_list.is_none());
    app.update(key(KeyCode::Backspace));
    app.update(key(KeyCode::Backspace));
    assert!(app.editor.is_empty());
    typed(&mut app, "/");
    assert_eq!(app.command_list, Some(0));
}

#[test]
fn up_and_down_select_in_the_command_list() {
    let mut app = app_with(vec![SessionEvent::UserMessage {
        text: "earlier".into(),
        images: Vec::new(),
    }]);
    typed(&mut app, "/");
    app.update(key(KeyCode::Up));
    assert_eq!(app.command_list, Some(0));
    assert_eq!(app.editor.text(), "/");
    app.update(key(KeyCode::Down));
    assert_eq!(selected_command(&app), Some("clear"));
    for _ in 0..COMMANDS.len() * 2 {
        app.update(key(KeyCode::Down));
    }
    assert_eq!(selected_command(&app), Some("detach"));
    assert_eq!(app.editor.text(), "/");
}

#[test]
fn a_new_search_selects_the_best_match() {
    let mut app = app();
    typed(&mut app, "/");
    app.update(key(KeyCode::Down));
    app.update(key(KeyCode::Down));
    typed(&mut app, "m");
    assert_eq!(selected_command(&app), Some("model"));
    assert_eq!(
        listed(&app),
        ["model", "permissions", "rename", "theme", "commit", "image"]
    );
    // Moving the cursor is no new search.
    app.update(key(KeyCode::Down));
    app.update(key(KeyCode::Left));
    assert_eq!(selected_command(&app), Some("permissions"));
    // A paste is.
    app.update(key(KeyCode::Right));
    app.update(Input::Paste("o".into()));
    assert_eq!(selected_command(&app), Some("model"));
}

#[test]
fn tab_completes_the_selected_command() {
    let mut app = app();
    typed(&mut app, "/mo");
    app.update(key(KeyCode::Tab));
    assert_eq!(app.editor.text(), "/model ");
    assert!(app.command_list.is_none());
    assert_eq!(app.focus, Focus::Input);
    // Then it is typed as before.
    typed(&mut app, "local/qwen");
    assert_eq!(
        requests(&app.update(key(KeyCode::Enter))),
        [Request::SetModel {
            session: "project".into(),
            model: "local/qwen".into()
        }]
    );
    // No space after a command without arguments.
    let mut app = self::app();
    typed(&mut app, "/he");
    app.update(key(KeyCode::Tab));
    assert_eq!(app.editor.text(), "/help");
    // With no match, nothing happens, and focus stays.
    let mut app = self::app();
    typed(&mut app, "/zzz");
    app.update(key(KeyCode::Tab));
    assert_eq!(app.editor.text(), "/zzz");
    assert!(app.command_list.is_some());
    assert_eq!(app.focus, Focus::Input);
}

#[test]
fn enter_runs_the_selected_command() {
    let mut app = app();
    command(&mut app, "/he");
    assert_eq!(app.dialog, Some(Dialog::Help));
    assert!(app.editor.is_empty());
    assert!(app.command_list.is_none());
    // `q` finds `/detach` by its other name.
    let mut app = self::app();
    assert_eq!(exit(&command(&mut app, "/q")), detached("project"));
    // A selection counts, not the text.
    let mut app = self::app();
    typed(&mut app, "/m");
    app.update(key(KeyCode::Down));
    app.update(key(KeyCode::Down));
    assert_eq!(selected_command(&app), Some("rename"));
    app.update(key(KeyCode::Enter));
    assert_eq!(app.editor.text(), "/rename ");
}

#[test]
fn an_unbuilt_command_says_so_from_the_list() {
    let mut app = app();
    command(&mut app, "/comm");
    assert_eq!(notices(&app), ["`/commit` comes with a later version"]);
    assert!(app.editor.is_empty());
}

#[test]
fn every_name_of_the_table_is_known() {
    for command in COMMANDS {
        for name in std::iter::once(command.name).chain(command.alias) {
            let mut app = app();
            typed(&mut app, &format!("/{name}"));
            // Closed, so the text goes to the command as it stands.
            app.update(key(KeyCode::Esc));
            app.update(key(KeyCode::Enter));
            assert!(
                !notices(&app).iter().any(|n| n.starts_with("unknown")),
                "{name}"
            );
        }
    }
}

/// The command of each box. No `_` arm: a new dialog needs a command to compile.
pub(super) fn command_of(dialog: Dialog) -> &'static str {
    match dialog {
        Dialog::Help => "/help",
        Dialog::ConfirmKill => "/kill",
    }
}

#[test]
fn every_box_opens_from_the_command_list() {
    for dialog in [Dialog::Help, Dialog::ConfirmKill] {
        let mut app = app();
        command(&mut app, command_of(dialog));
        assert_eq!(app.dialog, Some(dialog));
    }
    let mut app = app();
    assert_eq!(requests(&command(&mut app, "/model")), [models_request()]);
    app.update(models_answer(model_list()));
    assert!(app.picker.is_some());
    assert!(app.editor.is_empty());
}

#[test]
fn ctrl_o_closes_the_command_list_and_keeps_the_text() {
    let mut app = app();
    typed(&mut app, "/m");
    assert_eq!(requests(&app.update(ctrl('o'))), [models_request()]);
    assert!(app.command_list.is_none());
    assert_eq!(app.editor.text(), "/m");
}

#[test]
fn a_picker_that_opens_late_closes_the_command_list() {
    let mut app = app();
    app.update(ctrl('o'));
    typed(&mut app, "/m");
    assert!(app.command_list.is_some());
    app.update(models_answer(model_list()));
    assert!(app.picker.is_some());
    assert!(app.command_list.is_none());
    assert_eq!(app.editor.text(), "/m");
}

#[test]
fn a_question_closes_the_command_list_and_keeps_the_text() {
    let mut app = app();
    typed(&mut app, "/m");
    app.update(live(SessionEvent::Question(Box::new(permission()))));
    assert!(app.command_list.is_none());
    assert!(app.prompt.is_some());
    assert_eq!(app.editor.text(), "/m");
}

#[test]
fn the_end_of_the_session_closes_the_command_list() {
    let mut app = app();
    typed(&mut app, "/m");
    app.update(live(SessionEvent::Ended {
        reason: "killed".into(),
    }));
    assert!(app.command_list.is_none());
}

#[test]
fn scrolling_works_with_the_command_list_open() {
    let mut app = idle_session();
    screen(&mut app, 80, 24);
    typed(&mut app, "/");
    app.update(key(KeyCode::PageUp));
    assert!(!app.follow);
    assert_eq!(app.command_list, Some(0));
}

#[test]
fn frame_command_list() {
    let mut app = idle_session();
    typed(&mut app, "/");
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}

#[test]
fn frame_command_list_with_a_search() {
    let mut app = idle_session();
    typed(&mut app, "/m");
    app.update(key(KeyCode::Down));
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}

#[test]
fn frame_command_list_with_no_match() {
    let mut app = idle_session();
    typed(&mut app, "/zzz");
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}

#[test]
fn frame_command_list_ascii() {
    let mut app = App::new(
        settings(false, true),
        attachment(Vec::new()),
        Instant::now(),
    );
    typed(&mut app, "/");
    let text = screen(&mut app, 80, 24);
    assert!(text.is_ascii(), "{text}");
    insta::assert_snapshot!(text);
}

#[test]
fn frame_command_list_keeps_the_selected_row_on_a_small_screen() {
    let mut app = app();
    typed(&mut app, "/");
    for _ in 0..COMMANDS.len() {
        app.update(key(KeyCode::Down));
    }
    let text = screen(&mut app, 40, 10);
    assert!(text.contains("/detach, /quit"), "{text}");
    assert!(!text.contains("Show keys"), "{text}");
    insta::assert_snapshot!(text);
}

// Argument completion.

pub(super) fn values(app: &App) -> Vec<String> {
    app.arguments().into_iter().map(|v| v.value).collect()
}

#[test]
fn attach_offers_the_other_sessions() {
    let mut app = among_three();
    // Tab in the command list completes `/attach ` and opens the values.
    typed(&mut app, "/att");
    app.update(key(KeyCode::Tab));
    assert_eq!(app.editor.text(), "/attach ");
    assert_eq!(app.argument_list, Some(0));
    assert_eq!(values(&app), ["docs", "web"]);
    typed(&mut app, "w");
    assert_eq!(values(&app), ["web"]);
    // Enter runs it with the selected value.
    app.update(key(KeyCode::Backspace));
    app.update(key(KeyCode::Down));
    assert_eq!(switches(&app.update(key(KeyCode::Enter))), ["web"]);
}

#[test]
fn tab_completes_a_value_and_esc_closes_the_list() {
    let mut app = among_three();
    typed(&mut app, "/attach d");
    assert_eq!(app.argument_list, Some(0));
    app.update(key(KeyCode::Tab));
    assert_eq!(app.editor.text(), "/attach docs");
    assert!(app.argument_list.is_none());
    // Esc closes it, and it stays closed while the text is edited.
    let mut app = among_three();
    typed(&mut app, "/attach ");
    let effects = app.update(key(KeyCode::Esc));
    assert!(app.argument_list.is_none());
    assert!(requests(&effects).is_empty());
    typed(&mut app, "x");
    assert!(app.argument_list.is_none());
    // A second word closes it too.
    let mut app = among_three();
    typed(&mut app, "/attach docs ");
    assert!(app.argument_list.is_none());
}

#[test]
fn model_offers_the_models_from_the_server() {
    let mut app = app();
    typed(&mut app, "/model");
    let effects = app.update(ch(' '));
    // Not known yet: asked for once, without opening the picker.
    assert_eq!(requests(&effects), [models_request()]);
    assert!(requests(&app.update(ch('q'))).is_empty());
    app.update(models_answer(model_list()));
    assert!(app.picker.is_none());
    assert_eq!(values(&app), ["local/qwen"]);
    app.update(key(KeyCode::Backspace));
    typed(&mut app, "claude-o");
    assert_eq!(values(&app), ["anthropic/opus"]);
    assert_eq!(
        requests(&app.update(key(KeyCode::Enter))),
        [Request::SetModel {
            session: "project".into(),
            model: "anthropic/opus".into()
        }]
    );
}

#[test]
fn the_model_list_marks_the_current_and_the_default_model() {
    let mut app = app();
    typed(&mut app, "/model ");
    app.update(models_answer(model_list()));
    let described = |app: &App| -> Vec<String> {
        app.arguments()
            .into_iter()
            .map(|v| format!("{} {}", v.value, v.description))
            .collect()
    };
    assert_eq!(
        described(&app),
        [
            "anthropic/sonnet claude-sonnet-5-5  current, default",
            "anthropic/opus claude-opus-5-5",
            "local/qwen qwen3-coder:30b",
        ]
    );
    // A new default (saved here or elsewhere) shows with the next answer.
    app.update(models_answer(default_opus()));
    assert_eq!(
        described(&app)[..2],
        [
            "anthropic/sonnet claude-sonnet-5-5  current",
            "anthropic/opus claude-opus-5-5  default",
        ]
    );
    assert!(screen(&mut app, 80, 24).contains("claude-opus-5-5  default"));
}

#[test]
fn with_no_match_enter_runs_the_text() {
    let mut app = among_three();
    typed(&mut app, "/attach zzz");
    assert!(values(&app).is_empty());
    assert!(screen(&mut app, 80, 24).contains("no match"));
    app.update(key(KeyCode::Enter));
    assert_eq!(
        notices(&app),
        ["no session named zzz; /sessions lists them"]
    );
}

#[test]
fn frame_argument_list() {
    let mut app = among_three();
    typed(&mut app, "/attach ");
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}
