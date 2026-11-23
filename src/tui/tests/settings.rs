//! Settings applied live, the theme list and the settings box (RFC 0006), syntax highlighting (RFC 0017).

#![allow(clippy::wildcard_imports)]

use super::*;

// Settings applied live (RFC 0006).

pub(super) fn tui_with(f: impl FnOnce(&mut crate::config::Tui)) -> crate::config::Tui {
    let mut tui = app().settings.tui.clone();
    f(&mut tui);
    tui
}

#[test]
fn applied_settings_change_the_conversation() {
    let long = "x\n".repeat(30);
    let mut app = app_with(vec![
        SessionEvent::ToolCall(call("c1", "run_command", CallState::Running)),
        SessionEvent::ToolCall(done(call("c1", "run_command", CallState::Running), &long)),
    ]);
    let before = screen(&mut app, 80, 40);
    assert!(before.contains("▸ run"), "{before}");
    let effects = app.apply(tui_with(|t| {
        t.tool_preview = crate::config::ToolPreview::All;
        t.tool_output_lines = 5;
        t.symbols = crate::config::Symbols::Ascii;
        t.scroll_lines = 7;
    }));
    assert!(effects.is_empty());
    let after = screen(&mut app, 80, 40);
    assert!(after.contains("v run"), "{after}");
    assert!(after.contains("25 more lines"), "{after}");
    assert!(after.is_ascii(), "{after}");
    assert_eq!(app.settings.scroll_lines, 7);
}

#[test]
fn mouse_and_frame_rate_go_to_the_shell() {
    let mut app = app();
    assert_eq!(app.frame_time(), Duration::from_secs(1) / 30);
    let effects = app.apply(tui_with(|t| {
        t.mouse = false;
        t.max_fps = 10;
    }));
    assert_eq!(effects, [Effect::Mouse(false)]);
    assert_eq!(app.frame_time(), Duration::from_millis(100));
    // The same again: nothing to do.
    assert!(app.apply(app.settings.tui.clone()).is_empty());
}

#[test]
fn timestamps_take_the_local_offset() {
    let mut app = app();
    app.apply(tui_with(|t| t.timestamps = true));
    assert_eq!(app.settings.timestamps, Some(3600));
    app.apply(tui_with(|t| t.timestamps = false));
    assert_eq!(app.settings.timestamps, None);
}

#[test]
fn a_reload_applies_the_file() {
    let mut app = app();
    let effects = app.update(Input::Server(ServerMessage::Notice {
        notice: Notice::ConfigReloaded,
    }));
    assert!(effects.contains(&Effect::ReloadConfig));
    let reloaded = crate::tui::app::Reloaded {
        tui: tui_with(|t| t.theme = "nord".into()),
        themes: app.settings.theming.themes.clone(),
        warnings: vec![warning("x")],
        file: crate::tui::app::FileTui::default(),
    };
    app.update(Input::Reloaded(Box::new(Ok(reloaded))));
    assert_eq!(app.settings.tui.theme, "nord");
    assert_eq!(app.settings.theming.warnings, [warning("x")]);
    assert_eq!(
        app.settings.look.styles.background,
        Some(Color::Indexed(237))
    );
    // A file that does not load: the settings stay, and the status says why.
    app.update(Input::Reloaded(Box::new(Err(
        "user.toml:3: tui.max_fps: 500 is not in 1–120".into(),
    ))));
    assert_eq!(app.settings.tui.theme, "nord");
    let text = screen(&mut app, 120, 24);
    assert!(text.contains("configuration: user.toml:3"), "{text}");
}

#[test]
fn a_theme_with_a_background_paints_the_screen() {
    let mut app = app();
    app.apply(tui_with(|t| t.theme = "atom-one-dark".into()));
    let terminal = screen_of(&mut app, 80, 24);
    let buffer = terminal.backend().buffer();
    // #282c34 as the nearest of 256 colours, everywhere.
    let bg = app.settings.look.styles.background.unwrap();
    assert!(
        buffer
            .content
            .iter()
            .all(|c| c.bg == bg || c.bg != Color::Reset)
    );
    assert!(buffer.content.iter().all(|c| c.bg != Color::Reset));
    assert!(buffer.content.iter().all(|c| c.fg != Color::Reset));
}

pub(super) fn warning(name: &str) -> crate::config::ThemeWarning {
    crate::config::ThemeWarning {
        name: name.into(),
        message: format!("themes/{name}.toml:1: base: broken"),
    }
}

// The theme list and the settings box (RFC 0006).

pub(super) fn saves(effects: &[Effect]) -> Vec<Vec<(&'static str, Option<Setting>)>> {
    effects
        .iter()
        .filter_map(|e| match e {
            Effect::Save { changes, .. } => Some(changes.clone()),
            _ => None,
        })
        .collect()
}

pub(super) fn theme_saved(name: &str) -> Vec<Vec<(&'static str, Option<Setting>)>> {
    vec![vec![("theme", Some(Setting::Text(name.into())))]]
}

#[test]
fn theme_list_previews_and_esc_goes_back() {
    let mut app = app();
    command(&mut app, "/theme");
    let list = app.theme_list.as_ref().expect("list");
    assert_eq!(list.preview(), Some("terminal"));
    app.update(key(KeyCode::Down));
    assert_eq!(app.settings.tui.theme, "dark");
    assert_eq!(app.settings.look.styles.text.fg, Some(Color::Indexed(252)));
    typed(&mut app, "nord");
    assert_eq!(app.settings.tui.theme, "nord");
    let effects = app.update(key(KeyCode::Esc));
    assert!(saves(&effects).is_empty());
    assert!(app.theme_list.is_none());
    assert_eq!(app.settings.tui.theme, "terminal");
    assert_eq!(app.settings.look.styles.background, None);
}

#[test]
fn enter_takes_the_theme_and_saves_it_as_the_default() {
    let mut app = app();
    command(&mut app, "/theme");
    typed(&mut app, "gruv");
    let effects = app.update(key(KeyCode::Enter));
    assert_eq!(saves(&effects), theme_saved("gruvbox-dark"));
    let Some(Effect::Save { path, .. }) = effects.iter().find(|e| matches!(e, Effect::Save { .. }))
    else {
        panic!("{effects:?}");
    };
    assert_eq!(
        path,
        std::path::Path::new("/home/u/.config/codeagent/config.toml")
    );
    assert!(app.theme_list.is_none());
    assert_eq!(app.settings.tui.theme, "gruvbox-dark");
    app.update(Input::Saved(Ok(None)));
    assert!(screen(&mut app, 100, 24).contains("theme gruvbox-dark saved as the default"));
    assert_eq!(app.settings.theming.file.tui.theme, "gruvbox-dark");
    // A failed save is shown; the theme stays for this run.
    let mut app = self::app();
    command(&mut app, "/theme dark");
    app.update(Input::Saved(Err(
        "the file changed since it was read; open it again".into(),
    )));
    assert!(kinds(&app).contains(&Kind::Error(
        "not saved: the file changed since it was read; open it again".into()
    )));
    assert_eq!(app.settings.tui.theme, "dark");
}

#[test]
fn theme_by_name() {
    let mut app = app();
    assert_eq!(
        saves(&command(&mut app, "/theme nord")),
        theme_saved("nord")
    );
    assert!(app.editor.is_empty());
    let effects = command(&mut app, "/theme neon");
    assert!(saves(&effects).is_empty());
    assert_eq!(notices(&app), ["unknown theme neon; /theme lists them"]);
    assert_eq!(app.editor.text(), "/theme neon");
}

#[test]
fn a_broken_theme_file_cannot_be_taken() {
    let mut app = app();
    app.settings.theming.warnings = vec![warning("bad")];
    command(&mut app, "/theme");
    typed(&mut app, "bad");
    // The last theme that could be shown stays: `b` found gruvbox-dark.
    assert_eq!(
        app.theme_list.as_ref().unwrap().chosen().unwrap().name,
        "bad"
    );
    assert_eq!(app.settings.tui.theme, "gruvbox-dark");
    assert!(saves(&app.update(key(KeyCode::Enter))).is_empty());
    assert!(app.theme_list.is_some());
    let text = screen(&mut app, 100, 30);
    assert!(text.contains("themes/bad.toml:1: base: broken"), "{text}");
    // `/theme bad` neither.
    let mut app = self::app();
    app.settings.theming.warnings = vec![warning("bad")];
    assert!(saves(&command(&mut app, "/theme bad")).is_empty());
}

#[test]
fn theme_names_in_the_argument_list() {
    let mut app = app();
    typed(&mut app, "/theme so");
    assert_eq!(values(&app), ["solarized-light", "solarized-dark"]);
}

#[test]
fn codeagent_theme_wins_until_a_theme_is_taken() {
    let mut app = app();
    app.settings.theming.theme_env = Some("light".into());
    let reloaded = || crate::tui::app::Reloaded {
        tui: tui_with(|t| t.theme = "light".into()),
        themes: crate::config::themes::file_themes(&[]).0,
        warnings: Vec::new(),
        file: crate::tui::app::FileTui::default(),
    };
    app.update(Input::Reloaded(Box::new(Ok(reloaded()))));
    assert_eq!(app.settings.tui.theme, "light");
    command(&mut app, "/theme nord");
    app.update(Input::Reloaded(Box::new(Ok(reloaded()))));
    assert_eq!(app.settings.tui.theme, "nord");
}

/// The settings box on a file that sets `mouse = false`.
pub(super) fn settings_open() -> App {
    let mut app = app();
    app.settings.theming.file.tui.mouse = false;
    app.settings.theming.file.keys = vec!["mouse".into()];
    app.apply(tui_with(|t| t.mouse = false));
    command(&mut app, "/settings");
    assert!(app.settings_box.is_some());
    app
}

pub(super) fn select_row(app: &mut App, key_name: &str) {
    for _ in 0..20 {
        app.update(key(KeyCode::Up));
    }
    for _ in 0..20 {
        if app.settings_box.as_ref().unwrap().row().unwrap().key == key_name {
            return;
        }
        app.update(key(KeyCode::Down));
    }
    panic!("no row {key_name}");
}

#[test]
fn settings_preview_and_save() {
    let mut app = settings_open();
    select_row(&mut app, "mouse");
    let effects = app.update(key(KeyCode::Enter));
    assert!(effects.contains(&Effect::Mouse(true)));
    select_row(&mut app, "tool_preview");
    app.update(key(KeyCode::Right));
    assert_eq!(
        app.settings.tui.tool_preview,
        crate::config::ToolPreview::All
    );
    let effects = app.update(ch('s'));
    assert_eq!(
        saves(&effects),
        [vec![
            ("mouse", Some(Setting::Bool(true))),
            ("tool_preview", Some(Setting::Text("all".into())))
        ]]
    );
    app.update(Input::Saved(Ok(None)));
    let settings = app.settings_box.as_ref().expect("still open");
    assert!(settings.changes().is_empty());
    assert!(screen(&mut app, 100, 30).contains("saved"));
    assert_eq!(
        app.settings.theming.file.tui.tool_preview,
        crate::config::ToolPreview::All
    );
    assert!(
        app.settings
            .theming
            .file
            .keys
            .contains(&"tool_preview".to_owned())
    );
    // Nothing changed now: Esc closes at once.
    app.update(key(KeyCode::Esc));
    assert!(app.settings_box.is_none());
}

#[test]
fn settings_discarded_go_back_to_the_file() {
    let mut app = settings_open();
    select_row(&mut app, "mouse");
    app.update(key(KeyCode::Enter));
    app.update(key(KeyCode::Esc));
    assert!(screen(&mut app, 100, 30).contains("save changes? [s] save  [n] discard"));
    let effects = app.update(ch('n'));
    assert!(app.settings_box.is_none());
    assert!(effects.contains(&Effect::Mouse(false)));
    assert!(!app.settings.tui.mouse);
}

#[test]
fn the_theme_row_opens_the_theme_list() {
    let mut app = settings_open();
    app.update(key(KeyCode::Enter));
    assert!(app.theme_list.as_ref().unwrap().for_settings);
    typed(&mut app, "nord");
    // Enter only sets the row; `s` in the box saves it.
    let effects = app.update(key(KeyCode::Enter));
    assert!(saves(&effects).is_empty());
    assert!(app.theme_list.is_none());
    assert_eq!(app.settings.tui.theme, "nord");
    assert_eq!(app.settings_box.as_ref().unwrap().current.theme, "nord");
    assert_eq!(saves(&app.update(ch('s'))), theme_saved("nord"));
}

#[test]
fn a_question_closes_the_settings_and_undoes_the_preview() {
    let mut app = settings_open();
    select_row(&mut app, "mouse");
    app.update(key(KeyCode::Enter));
    let effects = app.update(live(SessionEvent::Question(Box::new(permission()))));
    assert!(app.settings_box.is_none());
    assert!(effects.contains(&Effect::Mouse(false)));
    let mut app = self::app();
    command(&mut app, "/theme");
    app.update(key(KeyCode::Down));
    app.update(live(SessionEvent::Question(Box::new(permission()))));
    assert!(app.theme_list.is_none());
    assert_eq!(app.settings.tui.theme, "terminal");
}

#[test]
fn a_failed_save_shows_in_the_box() {
    let mut app = settings_open();
    select_row(&mut app, "timestamps");
    app.update(key(KeyCode::Enter));
    app.update(ch('s'));
    app.update(Input::Saved(Err("tui.theme: unknown theme `x`".into())));
    let settings = app.settings_box.as_ref().unwrap();
    assert_eq!(
        settings.message.as_deref(),
        Some("tui.theme: unknown theme `x`")
    );
    assert!(!settings.changes().is_empty());
}

#[test]
fn frame_settings() {
    let mut app = settings_open();
    select_row(&mut app, "mouse");
    app.update(key(KeyCode::Enter));
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}

#[test]
fn frame_settings_ascii_editing_a_number() {
    let mut app = App::new(
        settings(false, true),
        attachment(Vec::new()),
        Instant::now(),
    );
    command(&mut app, "/settings");
    select_row(&mut app, "max_fps");
    app.update(key(KeyCode::Enter));
    let text = screen(&mut app, 80, 24);
    assert!(text.is_ascii(), "{text}");
    insta::assert_snapshot!(text);
}

#[test]
fn frame_settings_with_codeagent_theme() {
    let mut app = settings_open();
    app.settings.theming.theme_env = Some("nord".into());
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}

#[test]
fn frame_theme_list() {
    let mut app = app();
    app.settings.theming.warnings = vec![warning("bad")];
    command(&mut app, "/theme");
    app.update(key(KeyCode::Down));
    insta::assert_snapshot!(screen(&mut app, 80, 40));
}

#[test]
fn frame_theme_list_ascii() {
    let mut app = App::new(
        settings(false, true),
        attachment(Vec::new()),
        Instant::now(),
    );
    command(&mut app, "/theme");
    let text = screen(&mut app, 80, 40);
    assert!(text.is_ascii(), "{text}");
    insta::assert_snapshot!(text);
}

/// One frame per built-in and bundled theme: the styles of the cells, not
/// only the text, so that a palette change shows.
#[test]
fn frame_of_each_theme() {
    for theme in [
        "dark",
        "light",
        "atom-one-dark",
        "nord",
        "gruvbox-dark",
        "solarized-light",
        "tokyo-night",
        "one-light",
    ] {
        let mut app = app_with(vec![
            SessionEvent::UserMessage {
                text: "hi".into(),
                images: Vec::new(),
            },
            SessionEvent::TextDone {
                text: "**done** `x`".into(),
            },
            SessionEvent::ToolCall(done(call("c1", "read_file", CallState::Running), "1 line")),
            SessionEvent::Error {
                message: "oops".into(),
            },
        ]);
        app.apply(tui_with(|t| t.theme = theme.into()));
        let terminal = screen_of(&mut app, 40, 12);
        insta::assert_debug_snapshot!(format!("theme_{theme}"), terminal.backend().buffer());
    }
}

/// The shell's save: written, checked by loading, refused when invalid.
#[test]
fn the_shell_saves_into_a_real_user_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let text = "# mine\ndefault_model = \"a/m\"\n\n[providers.a]\napi = \"anthropic\"\n\n[providers.a.models.m]\nid = \"m\"\ncontext_window = 1000\n";
    std::fs::write(&path, text).unwrap();
    std::fs::create_dir(dir.path().join("themes")).unwrap();
    std::fs::write(dir.path().join("themes/mine.toml"), "base = \"light\"\n").unwrap();
    let options = crate::config::LoadOptions {
        config_file: Some(path.clone()),
        workspace: dir.path().to_path_buf(),
        env: crate::config::Environment::default(),
    };
    let stamp = crate::config::edit::stamp(&path);
    let theme = |name: &str| vec![("theme", Some(Setting::Text(name.into())))];
    let new = save(&options, &path, stamp, &theme("mine")).unwrap();
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.starts_with("# mine\n"), "{written}");
    assert!(written.ends_with("[tui]\ntheme = \"mine\"\n"), "{written}");
    // A theme the loader does not know is refused, and nothing is written.
    let e = save(&options, &path, new, &theme("neon")).unwrap_err();
    assert!(e.contains("unknown theme `neon`"), "{e}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), written);
}

#[test]
fn opening_reads_the_theme_files_again() {
    let mut app = app();
    assert!(command(&mut app, "/theme").contains(&Effect::ReloadConfig));
    app.update(key(KeyCode::Down));
    let mut themes = app.settings.theming.themes.clone();
    themes.insert(
        "fresh".into(),
        crate::config::Theme {
            base: crate::config::BuiltinTheme::Light,
            colours: Vec::new(),
            code: None,
            source: crate::config::ThemeSource::File("themes/fresh.toml".into()),
        },
    );
    app.update(Input::Reloaded(Box::new(Ok(crate::tui::app::Reloaded {
        tui: app.settings.theming.file.tui.clone(),
        themes,
        warnings: Vec::new(),
        file: crate::tui::app::FileTui::default(),
    }))));
    let list = app.theme_list.as_ref().unwrap();
    assert!(list.visible().iter().any(|e| e.name == "fresh"));
    // The selection and its preview stay.
    assert_eq!(list.preview(), Some("dark"));
    assert_eq!(app.settings.tui.theme, "dark");
    assert!(command(&mut self::app(), "/settings").contains(&Effect::ReloadConfig));
}

/// Two calls in one answer of the model: the first denied with a message,
/// then the second asks at once. Its prompt must take keys.
#[test]
fn after_a_message_the_next_question_takes_keys() {
    let mut app = asked();
    app.update(ch('e'));
    typed(&mut app, "use b.rs");
    let effects = app.update(key(KeyCode::Enter));
    let request = requests(&effects).remove(0);
    let first = permission();
    app.update(live(SessionEvent::QuestionClosed {
        id: first.id.clone(),
        closed: Closed::Answered {
            answer: Answer::DenyMessage {
                text: "use b.rs".into(),
            },
            by: "tui pts/3".into(),
        },
    }));
    app.update(live(SessionEvent::ToolCall(call(
        "c2",
        "edit_file",
        CallState::Waiting,
    ))));
    let mut second = permission();
    second.id = "q2".into();
    if let QuestionKind::Permission(p) = &mut second.kind {
        p.call.id = "c2".into();
    }
    app.update(live(SessionEvent::Question(Box::new(second))));
    // The answer to the first question comes back only now.
    app.update(Input::Response {
        request,
        result: Ok(Payload::Done),
    });
    let effects = app.update(ch('y'));
    assert_eq!(
        requests(&effects),
        [Request::Answer {
            session: "project".into(),
            question: "q2".into(),
            answer: Answer::Allow,
        }]
    );
}

/// A session waiting on a question can be left for another one: the
/// question stays open on the server, and nothing is answered.
#[test]
fn a_waiting_session_can_be_left() {
    let mut app = asked();
    app.update(sessions_answer(three()));
    let effects = app.update(ctrl('n'));
    assert_eq!(switches(&effects), ["docs"]);
    assert!(
        !requests(&effects)
            .iter()
            .any(|r| matches!(r, Request::Answer { .. }))
    );
    app.update(attached_to(2, "docs", Vec::new()));
    assert!(app.prompt.is_none());
    assert_eq!(switches(&app.update(ctrl('p'))), ["project"]);
}

/// `Ctrl-S` opens the session list over the prompt; its keys go to the
/// list, not to the question.
#[test]
fn the_session_list_opens_over_a_prompt() {
    let mut app = asked();
    let effects = app.update(ctrl('s'));
    assert_eq!(requests(&effects), [Request::List]);
    app.update(sessions_answer(three()));
    assert!(app.session_list.is_some());
    assert!(app.prompt.is_some());
    // The arrow moves in the list and answers nothing.
    let effects = app.update(key(KeyCode::Down));
    assert!(requests(&effects).is_empty());
    let effects = app.update(key(KeyCode::Enter));
    assert_eq!(switches(&effects), ["docs"]);
    assert!(
        !requests(&effects)
            .iter()
            .any(|r| matches!(r, Request::Answer { .. }))
    );
    // `Esc` closes the list, and the prompt takes keys again.
    let mut app = asked();
    app.update(ctrl('s'));
    app.update(sessions_answer(three()));
    app.update(key(KeyCode::Esc));
    assert!(app.session_list.is_none());
    assert_eq!(
        requests(&app.update(ch('y'))),
        [Request::Answer {
            session: "project".into(),
            question: permission().id,
            answer: Answer::Allow,
        }]
    );
}

// Syntax highlighting (RFC 0017).

pub(super) fn highlighted(events: Vec<SessionEvent>) -> App {
    use crate::tui::highlight::CodeTheme;
    use crate::tui::style::Colouring;
    let mut settings = settings(true, true);
    settings.look = settings
        .look
        .highlighted(CodeTheme::Ansi, Colouring::Ansi256);
    settings.tui.tool_preview = crate::config::ToolPreview::Changes;
    App::new(settings, attachment(events), Instant::now())
}

/// The style of the first cell of `text` on the screen.
pub(super) fn cell_style(app: &mut App, text: &str) -> ratatui::style::Style {
    let terminal = screen_of(app, 80, 30);
    let buffer = terminal.backend().buffer();
    let area = *buffer.area();
    for y in 0..area.height {
        let cells: Vec<&str> = (0..area.width).map(|x| buffer[(x, y)].symbol()).collect();
        let row: String = cells.concat();
        if let Some(at) = row.find(text) {
            // Cells before it, counted by their symbols.
            let mut seen = 0;
            for (x, symbol) in cells.iter().enumerate() {
                if seen == at {
                    return buffer[(u16::try_from(x).unwrap(), y)].style();
                }
                seen += symbol.len();
            }
        }
    }
    panic!("{text} not on the screen");
}

pub(super) fn edit_of(path: &str, diff: &str) -> Vec<SessionEvent> {
    let mut running = call("e1", "edit_file", CallState::Running);
    let mut details = edit_details();
    if let Details::EditFile {
        path: p, diff: d, ..
    } = &mut details
    {
        p.resolved = path.to_owned();
        d.text = format!("--- {path}\n+++ {path}\n{diff}");
    }
    running.details = Some(Box::new(details));
    vec![
        SessionEvent::ToolCall(running),
        SessionEvent::ToolCall(done(call("e1", "edit_file", CallState::Running), "edited")),
    ]
}

#[test]
fn code_blocks_of_answers_are_highlighted() {
    let mut app = highlighted(vec![SessionEvent::TextDone {
        text: "```rust\nfn main() { let s = \"hi\"; }\n```".into(),
    }]);
    assert_eq!(cell_style(&mut app, "fn main").fg, Some(Color::Magenta));
    assert_eq!(cell_style(&mut app, "hi\"").fg, Some(Color::Green));
    // Off: the `code` colour, as before.
    let mut app = app_with(vec![SessionEvent::TextDone {
        text: "```rust\nfn main() {}\n```".into(),
    }]);
    assert_eq!(cell_style(&mut app, "fn main").fg, Some(Color::Yellow));
}

#[test]
fn diffs_keep_their_markers_and_highlight_the_code() {
    let mut app = highlighted(edit_of("src/parse.rs", DIFF));
    assert_eq!(cell_style(&mut app, "+for").fg, Some(Color::Green));
    assert_eq!(
        cell_style(&mut app, "for i in 0..lines.len() {").fg,
        Some(Color::Magenta)
    );
    assert_eq!(cell_style(&mut app, "-for").fg, Some(Color::Red));
    assert_eq!(cell_style(&mut app, "@@ -1").fg, Some(Color::Blue));
    // A file of no known language: whole lines in the diff colours.
    let mut app = highlighted(edit_of("notes.zzz", DIFF));
    assert_eq!(
        cell_style(&mut app, "for i in 0..lines.len() {").fg,
        Some(Color::Green)
    );
}

#[test]
fn the_old_and_new_side_are_highlighted_apart() {
    let diff = "@@ -1,2 +1,2 @@\n-/* old\n+let a = 1;\n let b = 2;\n";
    let mut app = highlighted(edit_of("src/a.rs", diff));
    // The comment opened on the old side does not colour the new one.
    assert_eq!(cell_style(&mut app, "let a").fg, Some(Color::Cyan));
    assert_eq!(cell_style(&mut app, "/* old").fg, Some(Color::DarkGray));
}

#[test]
fn commands_are_highlighted_as_shell() {
    let mut running = call("r1", "run_command", CallState::Running);
    running.summary = Some("echo".into());
    running.arguments = json!({"command": "echo \"hello world\""});
    let finished = done(running.clone(), "hello world\nexit code 0");
    let mut app = highlighted(vec![
        SessionEvent::ToolCall(running),
        SessionEvent::ToolCall(finished),
    ]);
    assert_eq!(cell_style(&mut app, "hello world\"").fg, Some(Color::Green));
    // The output is not code: it keeps the text style.
    assert_ne!(cell_style(&mut app, "hello world ").fg, Some(Color::Green));
}

#[test]
fn a_streamed_block_ends_as_the_whole_block() {
    let block = "Here:\n\n```python\ndef f(x):\n    return \"x\" * 2\n```\n";
    let mut streamed = highlighted(Vec::new());
    for piece in block.as_bytes().chunks(5) {
        let text = String::from_utf8(piece.to_vec()).unwrap();
        streamed.update(live(SessionEvent::TextDelta { text }));
        screen_of(&mut streamed, 80, 30);
    }
    let mut whole = highlighted(vec![SessionEvent::TextDone { text: block.into() }]);
    let a = screen_of(&mut streamed, 80, 30).backend().buffer().clone();
    let b = screen_of(&mut whole, 80, 30).backend().buffer().clone();
    assert_eq!(a, b);
    assert_eq!(cell_style(&mut whole, "def").fg, Some(Color::Magenta));
}

#[test]
fn a_long_diff_lays_out_within_its_budget() {
    let lines: String = (0..crate::tui::highlight::MAX_LINES / 2)
        .map(|i| format!("-let a{i} = {i};\n+let b{i} = \"{i}\";\n"))
        .collect();
    let diff = format!("@@ -1,2500 +1,2500 @@\n{lines}");
    let mut app = highlighted(edit_of("src/big.rs", &diff));
    app.update(key(KeyCode::Tab));
    app.update(key(KeyCode::Enter));
    app.update(key(KeyCode::Enter));
    let start = Instant::now();
    screen_of(&mut app, 80, 30);
    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_secs(20), "{elapsed:?}");
}
