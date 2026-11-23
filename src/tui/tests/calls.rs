//! Tool calls: details and copy (RFC 0016).

#![allow(clippy::wildcard_imports)]

use super::*;

// Tool details.

/// Puts the focus on the last entry of the conversation.
pub(super) fn select_last(app: &mut App) {
    app.update(key(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Conversation);
}

pub(super) fn long_command(lines: usize) -> App {
    let output: Vec<String> = (1..=lines).map(|i| format!("output {i}")).collect();
    let mut event = call("c1", "run_command", CallState::Running);
    event.summary = Some("cargo test".to_owned());
    event.arguments = json!({"command": "cargo test"});
    app_with(vec![SessionEvent::ToolCall(done(
        event,
        &output.join("\n"),
    ))])
}

#[test]
fn tool_calls_are_collapsed_until_selected_and_toggled() {
    let mut app = long_command(3);
    let text = screen(&mut app, 80, 24);
    assert!(text.contains("▸ run cargo test"), "{text}");
    assert!(!text.contains("output 1"));
    select_last(&mut app);
    app.update(key(KeyCode::Enter));
    let text = screen(&mut app, 80, 24);
    assert!(text.contains("▾ run"), "{text}");
    assert!(text.contains("$ cargo test") && text.contains("output 3"));
    app.update(key(KeyCode::Enter));
    assert!(!screen(&mut app, 80, 24).contains("output 1"));
}

#[test]
fn long_output_is_cut_and_enter_shows_the_whole() {
    let mut app = long_command(50);
    select_last(&mut app);
    app.update(key(KeyCode::Enter));
    let text = screen(&mut app, 80, 40);
    // The command line and 19 lines of output: 20 together.
    assert!(
        text.contains("output 19") && !text.contains("output 20"),
        "{text}"
    );
    assert!(text.contains("… 31 more lines"), "{text}");
    app.update(key(KeyCode::Enter));
    let text = screen(&mut app, 80, 80);
    assert!(
        text.contains("output 50") && !text.contains("more lines"),
        "{text}"
    );
    app.update(key(KeyCode::Enter));
    assert!(!screen(&mut app, 80, 40).contains("$ cargo test"));
}

#[test]
fn tool_preview_all_and_zero_means_no_limit() {
    let mut settings = settings(true, true);
    settings.tui.tool_preview = crate::config::ToolPreview::All;
    settings.tool_output_lines = 0;
    let output = (1..=30)
        .map(|i| format!("o{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let event = done(call("c1", "read_file", CallState::Running), &output);
    let mut app = App::new(
        settings,
        attachment(vec![SessionEvent::ToolCall(event)]),
        Instant::now(),
    );
    let text = screen(&mut app, 80, 50);
    assert!(
        text.contains("o30") && !text.contains("more lines"),
        "{text}"
    );
}

#[test]
fn up_and_down_select_entries_in_the_conversation() {
    let mut app = app_with(vec![
        SessionEvent::UserMessage {
            text: "a".into(),
            images: Vec::new(),
        },
        SessionEvent::UserMessage {
            text: "b".into(),
            images: Vec::new(),
        },
    ]);
    select_last(&mut app);
    assert_eq!(app.selected, Some(1));
    app.update(key(KeyCode::Up));
    assert_eq!(app.selected, Some(0));
    app.update(key(KeyCode::Up));
    assert_eq!(app.selected, Some(0));
    app.update(key(KeyCode::Down));
    app.update(key(KeyCode::Down));
    assert_eq!(app.selected, Some(1));
    // Keys do not type into the input from here.
    app.update(ch('x'));
    assert!(app.editor.is_empty());
    app.update(key(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Input);
}

#[test]
fn question_mark_opens_help_in_the_conversation_only() {
    let mut app = app();
    app.update(ch('?'));
    assert_eq!(app.dialog, None);
    assert_eq!(app.editor.text(), "?");
    app.update(key(KeyCode::Tab));
    app.update(ch('?'));
    assert_eq!(app.dialog, Some(Dialog::Help));
    let text = screen(&mut app, 100, 30);
    assert!(
        text.contains("Help") && text.contains("Alt-Enter"),
        "{text}"
    );
    app.update(ch('x'));
    assert_eq!(app.dialog, None);
}

#[test]
fn colours_of_a_diff() {
    let mut app = asked();
    let terminal = screen_of(&mut app, 80, 24);
    let buffer = terminal.backend().buffer();
    let colour_of = |wanted: char| {
        for y in 0..24 {
            let row: String = (0..80).map(|x| buffer[(x, y)].symbol()).collect();
            let text = row.trim_start();
            if text.starts_with(wanted) {
                let x = row.chars().position(|c| c == wanted).unwrap();
                return buffer[(u16::try_from(x).unwrap(), y)].fg;
            }
        }
        panic!("no line starts with {wanted}");
    };
    assert_eq!(colour_of('+'), Color::Green);
    assert_eq!(colour_of('-'), Color::Red);
}

#[test]
fn no_color_uses_no_colours() {
    let mut app = App::new(
        settings(true, false),
        attachment(Vec::new()),
        Instant::now(),
    );
    app.update(live(SessionEvent::ToolCall(call(
        "c1",
        "edit_file",
        CallState::Waiting,
    ))));
    app.update(live(SessionEvent::Question(Box::new(permission()))));
    app.update(live(SessionEvent::Error {
        message: "boom".into(),
    }));
    let terminal = screen_of(&mut app, 80, 24);
    let buffer = terminal.backend().buffer();
    assert!(
        buffer
            .content()
            .iter()
            .all(|c| c.fg == Color::Reset && c.bg == Color::Reset)
    );
}

// Copy and tool details (RFC 0016).

pub(super) fn previewing(events: Vec<SessionEvent>) -> App {
    let mut settings = settings(true, true);
    settings.tui.tool_preview = crate::config::ToolPreview::Changes;
    // Each call with its own preview; groups have tests of their own.
    settings.tui.group_tools = false;
    App::new(settings, attachment(events), Instant::now())
}

pub(super) fn edit_details() -> Details {
    Details::EditFile {
        path: PathDetails {
            written: "src/parse.rs".to_owned(),
            resolved: "src/parse.rs".to_owned(),
            area: "workspace".to_owned(),
        },
        diff: Text {
            text: format!("--- src/parse.rs\n+++ src/parse.rs\n{DIFF}"),
            cut: false,
        },
        first_line: 1,
        last_line: 1,
        occurrences: 1,
        size: 10,
        modified: None,
    }
}

/// An edit allowed without a question: its details come with `running`.
pub(super) fn edited() -> Vec<SessionEvent> {
    let mut running = call("e1", "edit_file", CallState::Running);
    running.details = Some(Box::new(edit_details()));
    vec![
        SessionEvent::UserMessage {
            text: "fix it".into(),
            images: Vec::new(),
        },
        SessionEvent::ToolCall(running),
        SessionEvent::ToolCall(done(
            call("e1", "edit_file", CallState::Running),
            "edited src/parse.rs: replaced 1 occurrence (line 1)",
        )),
    ]
}

pub(super) fn command_run(id: &str, lines: usize) -> Vec<SessionEvent> {
    let mut running = call(id, "run_command", CallState::Running);
    running.summary = Some("cargo test parse".into());
    running.arguments = json!({"command": "cargo test parse"});
    let output: Vec<String> = (1..=lines).map(|i| format!("line {i}")).collect();
    let mut finished = running.clone();
    finished = done(finished, &format!("{}\nexit code 0", output.join("\n")));
    vec![
        SessionEvent::ToolCall(running),
        SessionEvent::ToolCall(finished),
    ]
}

pub(super) fn copies(effects: &[Effect]) -> Vec<String> {
    effects
        .iter()
        .filter_map(|e| match e {
            Effect::Copy(text) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

pub(super) fn mouse(kind: MouseEventKind, column: u16, row: u16) -> Input {
    Input::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

/// The screen row that shows `text`.
pub(super) fn row_of(app: &mut App, text: &str) -> u16 {
    let shown = screen(app, 80, 30);
    let row = shown
        .lines()
        .position(|l| l.contains(text))
        .unwrap_or_else(|| panic!("{text} not on the screen:\n{shown}"));
    u16::try_from(row).unwrap()
}

#[test]
fn frame_tool_previews() {
    let mut events = edited();
    events.extend(command_run("r1", 40));
    let mut failed = call("f1", "read_file", CallState::Running);
    failed.state = CallState::Error;
    failed.result = Some(CallResult {
        text: "src/gone.rs: no such file".into(),
        is_error: true,
    });
    events.push(SessionEvent::ToolCall(failed));
    events.push(SessionEvent::ToolCall(done(
        call("r2", "read_file", CallState::Running),
        "1: fn main() {}",
    )));
    let mut live = call("r3", "run_command", CallState::Running);
    live.summary = Some("cargo build".into());
    live.arguments = json!({"command": "cargo build"});
    events.push(SessionEvent::ToolCall(live.clone()));
    live.output = Some("   Compiling codeagent v0.1.0\n".into());
    events.push(SessionEvent::ToolCall(live));
    let mut app = previewing(events);
    insta::assert_snapshot!(screen(&mut app, 80, 34));
}

#[test]
fn tool_preview_none_keeps_calls_on_one_line() {
    let mut app = app_with(edited());
    let text = screen(&mut app, 80, 20);
    assert!(text.contains("▸ edit"), "{text}");
    assert!(!text.contains("+for i"), "{text}");
    // `Ctrl-T` shows the previews of the last turn, and hides them again.
    app.update(ctrl('t'));
    let text = screen(&mut app, 80, 20);
    assert!(text.contains("+for i in 0..lines.len() {"), "{text}");
    app.update(ctrl('t'));
    assert!(!screen(&mut app, 80, 20).contains("+for i"));
}

#[test]
fn enter_on_a_preview_expands_and_a_collapsed_call_stays_collapsed() {
    let mut app = previewing(command_run("r1", 40));
    let text = screen(&mut app, 80, 30);
    assert!(text.contains("… 35 lines"), "{text}");
    assert!(!text.contains("line 1\n"), "{text}");
    app.update(key(KeyCode::Tab));
    app.update(key(KeyCode::Enter));
    let text = screen(&mut app, 80, 60);
    assert!(text.contains("more lines"), "{text}");
    app.update(key(KeyCode::Enter));
    app.update(key(KeyCode::Enter));
    let text = screen(&mut app, 80, 60);
    assert!(text.contains("▸ run"), "{text}");
}

#[test]
fn y_copies_the_selected_entry() {
    let mut events = vec![SessionEvent::UserMessage {
        text: "hello".into(),
        images: Vec::new(),
    }];
    events.push(SessionEvent::TextDone {
        text: "Use **this**:\n\n```sh\nls -l\n```\n".into(),
    });
    events.extend(command_run("r1", 2));
    let mut app = previewing(events);
    app.update(key(KeyCode::Tab));
    let effects = app.update(ch('y'));
    assert_eq!(
        copies(&effects),
        ["$ cargo test parse\nline 1\nline 2\nexit code 0"]
    );
    assert!(screen(&mut app, 80, 30).contains("copied 4 lines"));
    app.update(key(KeyCode::Up));
    assert_eq!(
        copies(&app.update(ch('y'))),
        ["Use **this**:\n\n```sh\nls -l\n```\n"]
    );
    assert_eq!(copies(&app.update(ch('Y'))), ["ls -l\n"]);
    app.update(key(KeyCode::Up));
    assert_eq!(copies(&app.update(ch('y'))), ["hello"]);
}

#[test]
fn a_copied_edit_applies_with_git_apply() {
    let mut app = previewing(edited());
    app.update(key(KeyCode::Tab));
    let effects = app.update(ch('y'));
    let [diff] = copies(&effects).try_into().unwrap();
    assert!(
        diff.starts_with("--- a/src/parse.rs\n+++ b/src/parse.rs\n"),
        "{diff}"
    );
    let dir = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(dir.path())
            .output()
            .unwrap()
    };
    if !git(&["init", "-q"]).status.success() {
        return;
    }
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/parse.rs"),
        "for i in 0..lines.len() - 1 {\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("p.diff"), &diff).unwrap();
    let applied = git(&["apply", "p.diff"]);
    assert!(applied.status.success(), "{applied:?}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("src/parse.rs")).unwrap(),
        "for i in 0..lines.len() {\n"
    );
}

#[test]
fn copy_commands() {
    let mut app = app_with(vec![
        SessionEvent::TextDone {
            text: "first\n\n```\ncode one\n```".into(),
        },
        SessionEvent::UserMessage {
            text: "more".into(),
            images: Vec::new(),
        },
        SessionEvent::TextDone {
            text: "second".into(),
        },
    ]);
    assert_eq!(copies(&command(&mut app, "/copy")), ["second"]);
    assert_eq!(
        copies(&command(&mut app, "/copy 2")),
        ["first\n\n```\ncode one\n```"]
    );
    let effects = command(&mut app, "/copy 3");
    assert!(copies(&effects).is_empty());
    assert!(notices(&app).contains(&"only 2 answers".to_owned()));
    let effects = command(&mut app, "/copy code");
    assert!(copies(&effects).is_empty());
    assert!(notices(&app).contains(&"the last answer has no code block".to_owned()));
    let effects = command(&mut app, "/copy x");
    assert!(copies(&effects).is_empty());
    assert_eq!(app.editor.text(), "/copy x");
    app.editor.clear();
    // `Ctrl-Y` from the input.
    assert_eq!(copies(&app.update(ctrl('y'))), ["second"]);
}

#[test]
fn copying_off_and_the_message_goes_by_itself() {
    let mut settings = settings(true, true);
    settings.tui.clipboard = crate::config::Clipboard::Off;
    let mut app = App::new(
        settings,
        attachment(vec![SessionEvent::TextDone { text: "a".into() }]),
        Instant::now(),
    );
    assert!(copies(&app.update(ctrl('y'))).is_empty());
    assert!(screen(&mut app, 80, 20).contains("copying is off"));
    app.now += Duration::from_secs(3);
    app.update(Input::Tick);
    assert!(!screen(&mut app, 80, 20).contains("copying is off"));
    app.update(Input::CopyFailed("xclip: broken pipe".into()));
    assert!(screen(&mut app, 80, 20).contains("cannot copy: xclip: broken pipe"));
}

#[test]
fn a_drag_selects_and_a_release_copies() {
    let mut app = previewing(vec![
        SessionEvent::UserMessage {
            text: "first line".into(),
            images: Vec::new(),
        },
        SessionEvent::TextDone {
            text: "an answer that the TUI wraps because it is longer than the width of the screen \
                   it is shown on"
                .into(),
        },
    ]);
    let user = row_of(&mut app, "> first line");
    let answer = row_of(&mut app, "an answer");
    assert!(app.follow);
    app.update(mouse(MouseEventKind::Down(MouseButton::Left), 2, user));
    app.update(mouse(
        MouseEventKind::Drag(MouseButton::Left),
        8,
        answer + 1,
    ));
    // The view stops following while the text is selected.
    assert!(!app.follow);
    let effects = app.update(mouse(MouseEventKind::Up(MouseButton::Left), 8, answer + 1));
    assert_eq!(
        copies(&effects),
        [
            "first line\n\nan answer that the TUI wraps because it is longer than the width of the screen it is s"
        ]
    );
    assert!(app.selection.is_some());
    // A key clears it; the view follows again.
    app.update(key(KeyCode::Esc));
    assert!(app.selection.is_none());
    assert!(app.follow);
}

#[test]
fn double_and_triple_clicks() {
    let mut app = previewing(vec![SessionEvent::TextDone {
        text: "see src/parse.rs: here, and a line long enough to wrap at the edge of the screen"
            .into(),
    }]);
    let row = row_of(&mut app, "see src");
    app.update(mouse(MouseEventKind::Down(MouseButton::Left), 6, row));
    app.update(mouse(MouseEventKind::Up(MouseButton::Left), 6, row));
    let word = app.update(mouse(MouseEventKind::Down(MouseButton::Left), 6, row));
    assert_eq!(copies(&word), ["src/parse.rs"]);
    app.update(mouse(MouseEventKind::Up(MouseButton::Left), 6, row));
    let line = app.update(mouse(MouseEventKind::Down(MouseButton::Left), 6, row));
    assert_eq!(
        copies(&line),
        ["see src/parse.rs: here, and a line long enough to wrap at the edge of the screen"]
    );
}

#[test]
fn a_click_toggles_a_call_and_the_focus_stays() {
    let mut app = app_with(edited());
    let row = row_of(&mut app, "▸ edit");
    app.update(mouse(MouseEventKind::Down(MouseButton::Left), 10, row));
    app.update(mouse(MouseEventKind::Up(MouseButton::Left), 10, row));
    assert_eq!(app.focus, Focus::Input);
    let text = screen(&mut app, 80, 30);
    assert!(text.contains("▾ edit"), "{text}");
    typed(&mut app, "x");
    assert_eq!(app.editor.text(), "x");
}

#[test]
fn a_click_on_more_lines_opens_the_details() {
    let mut app = previewing(command_run("r1", 40));
    let row = row_of(&mut app, "… 35 lines");
    app.update(mouse(MouseEventKind::Down(MouseButton::Left), 4, row));
    app.update(mouse(MouseEventKind::Up(MouseButton::Left), 4, row));
    assert!(app.details.is_some());
}

#[test]
fn frame_details_view() {
    let mut app = previewing(command_run("r1", 40));
    app.update(key(KeyCode::Tab));
    app.update(ch('o'));
    assert!(app.details.is_some());
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}

#[test]
fn details_view_search_wrap_copy_and_close() {
    let mut app = previewing(command_run("r1", 40));
    app.update(key(KeyCode::Tab));
    app.update(ch('o'));
    screen(&mut app, 80, 24);
    typed(&mut app, "/line 30");
    assert!(screen(&mut app, 80, 24).contains("/line 30"));
    app.update(key(KeyCode::Enter));
    let details = app.details.as_ref().unwrap();
    assert_eq!(details.found, Some(30));
    // Near the end, the last page is shown with the match on it.
    let text = screen(&mut app, 80, 24);
    assert!(text.contains("line 30"), "{text}");
    assert_eq!(details_top(&app), app.details.as_ref().unwrap().max_top());
    typed(&mut app, "/zzz");
    app.update(key(KeyCode::Enter));
    assert!(screen(&mut app, 80, 24).contains("not found: zzz"));
    app.update(ch('w'));
    assert!(!app.details.as_ref().unwrap().wrap);
    let effects = app.update(ch('y'));
    let [text] = copies(&effects).try_into().unwrap();
    assert!(text.starts_with("$ cargo test parse\n"), "{text}");
    assert!(text.ends_with("line 40\nexit code 0\n"), "{text}");
    app.update(key(KeyCode::Esc));
    assert!(app.details.is_none());
    // A question closes it.
    app.update(ch('o'));
    assert!(app.details.is_some());
    app.update(live(SessionEvent::ToolCall(call(
        "c1",
        "edit_file",
        CallState::Waiting,
    ))));
    app.update(live(SessionEvent::Question(Box::new(permission()))));
    assert!(app.details.is_none());
}

#[test]
fn diff_shows_the_changes_of_the_last_turn() {
    let mut app = previewing(edited());
    command(&mut app, "/diff");
    let details = app.details.as_ref().expect("open");
    assert!(details.title.contains("1 file"), "{}", details.title);
    let text = screen(&mut app, 80, 24);
    assert!(text.contains("+for i in 0..lines.len() {"), "{text}");
    app.update(key(KeyCode::Esc));
    let mut app = previewing(command_run("r1", 2));
    command(&mut app, "/diff");
    assert!(app.details.is_none());
    assert!(notices(&app).contains(&"the last turn changed no file".to_owned()));
}

pub(super) fn details_top(app: &App) -> usize {
    app.details.as_ref().unwrap().top
}
