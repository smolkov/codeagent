//! Clicks on a tool call (RFC 0022) and tool groups (RFC 0043).

#![allow(clippy::wildcard_imports)]

use super::*;

// Clicks on a tool call (RFC 0022).

/// One click at `row`, after the double-click time of the last one.
pub(super) fn click_on(app: &mut App, row: u16) {
    app.now += Duration::from_secs(1);
    app.update(mouse(MouseEventKind::Down(MouseButton::Left), 10, row));
    app.update(mouse(MouseEventKind::Up(MouseButton::Left), 10, row));
}

/// Two clicks at `row` within the double-click time.
pub(super) fn double_click_on(app: &mut App, row: u16) {
    click_on(app, row);
    app.now += Duration::from_millis(100);
    app.update(mouse(MouseEventKind::Down(MouseButton::Left), 10, row));
    app.update(mouse(MouseEventKind::Up(MouseButton::Left), 10, row));
}

pub(super) fn view_of(app: &App, id: &str) -> Expansion {
    let entry = app.conversation.entry_of_call(id).unwrap();
    app.conversation.view(entry).unwrap()
}

#[test]
fn a_click_opens_the_short_view_and_the_next_closes_it() {
    let mut app = app_with(command_run("r1", 40));
    let row = row_of(&mut app, "▸ run");
    assert_eq!(view_of(&app, "r1"), Expansion::Collapsed);
    click_on(&mut app, row);
    assert_eq!(view_of(&app, "r1"), Expansion::Preview);
    let text = screen(&mut app, 80, 30);
    assert!(text.contains("line 40"), "{text}");
    assert!(!text.contains("line 20 "), "{text}");
    click_on(&mut app, row);
    assert_eq!(view_of(&app, "r1"), Expansion::Collapsed);
    // A call without a preview shows its details, cut.
    let mut app = app_with(vec![SessionEvent::ToolCall(done(
        call("c1", "read_file", CallState::Running),
        "one\ntwo",
    ))]);
    let row = row_of(&mut app, "▸ read");
    click_on(&mut app, row);
    assert_eq!(view_of(&app, "c1"), Expansion::Expanded);
    click_on(&mut app, row);
    assert_eq!(view_of(&app, "c1"), Expansion::Collapsed);
}

#[test]
fn a_call_with_a_preview_closes_on_the_first_click() {
    let mut app = previewing(command_run("r1", 40));
    assert_eq!(view_of(&app, "r1"), Expansion::Preview);
    let row = row_of(&mut app, "▾ run");
    click_on(&mut app, row);
    assert_eq!(view_of(&app, "r1"), Expansion::Collapsed);
}

#[test]
fn a_double_click_opens_the_whole_result_and_closes_it() {
    // From closed.
    let mut app = app_with(command_run("r1", 40));
    let row = row_of(&mut app, "▸ run");
    double_click_on(&mut app, row);
    assert_eq!(view_of(&app, "r1"), Expansion::Full);
    let text = screen(&mut app, 80, 60);
    assert!(text.contains("line 1 "), "{text}");
    assert!(app.selection.is_none());
    // From full: closed.
    double_click_on(&mut app, row);
    assert_eq!(view_of(&app, "r1"), Expansion::Collapsed);
    // From short.
    let mut app = previewing(command_run("r1", 40));
    let row = row_of(&mut app, "▾ run");
    double_click_on(&mut app, row);
    assert_eq!(view_of(&app, "r1"), Expansion::Full);
}

#[test]
fn a_slow_second_click_is_a_single_click() {
    let mut app = app_with(command_run("r1", 40));
    let row = row_of(&mut app, "▸ run");
    click_on(&mut app, row);
    click_on(&mut app, row);
    assert_eq!(view_of(&app, "r1"), Expansion::Collapsed);
}

#[test]
fn a_triple_click_on_a_summary_opens_the_whole_result_then_closes() {
    let mut app = app_with(command_run("r1", 40));
    let row = row_of(&mut app, "▸ run");
    double_click_on(&mut app, row);
    app.now += Duration::from_millis(100);
    let effects = app.update(mouse(MouseEventKind::Down(MouseButton::Left), 10, row));
    app.update(mouse(MouseEventKind::Up(MouseButton::Left), 10, row));
    assert_eq!(view_of(&app, "r1"), Expansion::Collapsed);
    assert!(copies(&effects).is_empty());
}

#[test]
fn a_double_click_on_text_still_selects_a_word() {
    let mut app = app_with(vec![SessionEvent::TextDone {
        text: "hello world".into(),
    }]);
    let row = row_of(&mut app, "hello world");
    app.update(mouse(MouseEventKind::Down(MouseButton::Left), 3, row));
    app.update(mouse(MouseEventKind::Up(MouseButton::Left), 3, row));
    app.now += Duration::from_millis(100);
    let effects = app.update(mouse(MouseEventKind::Down(MouseButton::Left), 3, row));
    assert_eq!(copies(&effects), ["hello"]);
}

// Grouped tool calls (RFC 0043).

/// A finished command `cmd` with its call id.
pub(super) fn ran(id: &str, cmd: &str, state: CallState) -> SessionEvent {
    let mut event = done(
        call(id, "run_command", CallState::Running),
        "ok\nexit code 0",
    );
    event.summary = Some(cmd.to_owned());
    event.arguments = json!({ "command": cmd });
    event.state = state;
    if state == CallState::Error {
        event.result = Some(CallResult {
            text: "exit code 1".into(),
            is_error: true,
        });
    }
    SessionEvent::ToolCall(event)
}

pub(super) fn five_commands() -> Vec<SessionEvent> {
    let mut events = vec![SessionEvent::UserMessage {
        text: "check".into(),
        images: Vec::new(),
    }];
    for i in 1..=5 {
        events.push(ran(
            &format!("r{i}"),
            &format!("cargo test t{i}"),
            CallState::Done,
        ));
    }
    events
}

pub(super) fn grouped(events: Vec<SessionEvent>) -> App {
    let mut settings = settings(true, true);
    settings.tui.tool_preview = crate::config::ToolPreview::Changes;
    App::new(settings, attachment(events), Instant::now())
}

#[test]
fn frame_tool_group_closed_and_open() {
    let mut app = grouped(five_commands());
    let closed = screen(&mut app, 80, 12);
    assert!(closed.contains("▸ Ran 5 commands"), "{closed}");
    assert!(!closed.contains("cargo test t1"), "{closed}");
    insta::assert_snapshot!("frame_tool_group_closed", closed);
    let row = row_of(&mut app, "Ran 5 commands");
    click_on(&mut app, row);
    let open = screen(&mut app, 80, 40);
    assert!(open.contains("▾ Ran 5 commands"), "{open}");
    insta::assert_snapshot!("frame_tool_group_open", open);
    click_on(&mut app, row);
    assert!(screen(&mut app, 80, 12).contains("▸ Ran 5 commands"));
}

#[test]
fn a_group_needs_enough_calls_of_one_tool_in_a_row() {
    // One call alone is no group.
    let mut app = grouped(vec![ran("r1", "ls", CallState::Done)]);
    assert!(!screen(&mut app, 80, 12).contains("Ran 1"));
    // Text between calls splits them; another tool starts its own group.
    let read =
        |id: &str| SessionEvent::ToolCall(done(call(id, "read_file", CallState::Running), "a"));
    let mut app = grouped(vec![
        ran("r1", "ls", CallState::Done),
        ran("r2", "pwd", CallState::Done),
        SessionEvent::TextDone {
            text: "Now the files.".into(),
        },
        ran("r3", "ls", CallState::Done),
        read("f1"),
        read("f2"),
    ]);
    let text = screen(&mut app, 80, 20);
    assert!(text.contains("Ran 2 commands"), "{text}");
    assert!(text.contains("▾ run ls"), "{text}");
    assert!(text.contains("Read 2 files"), "{text}");
    // `group_tools_min` raises the bar.
    let mut settings = settings(true, true);
    settings.tui.group_tools_min = 3;
    let mut app = App::new(
        settings,
        attachment(vec![
            ran("r1", "ls", CallState::Done),
            ran("r2", "pwd", CallState::Done),
        ]),
        Instant::now(),
    );
    assert!(!screen(&mut app, 80, 12).contains("Ran 2"));
}

#[test]
fn edits_running_and_waiting_calls_stay_outside() {
    let mut events = vec![
        ran("r1", "ls", CallState::Done),
        ran("r2", "pwd", CallState::Done),
    ];
    events.extend(edited().into_iter().skip(1));
    events.push(ran("r3", "ls", CallState::Done));
    let mut app = grouped(events);
    let text = screen(&mut app, 80, 30);
    assert!(text.contains("Ran 2 commands"), "{text}");
    assert!(text.contains("edit src/parse.rs"), "{text}");
    assert!(text.contains("run ls"), "{text}");

    // A running call shows on its own and joins when it finishes.
    let mut app = grouped(vec![ran("r1", "ls", CallState::Done)]);
    let mut running = call("r2", "run_command", CallState::Running);
    running.summary = Some("cargo build".into());
    running.arguments = json!({"command": "cargo build"});
    app.update(live(SessionEvent::ToolCall(running)));
    let text = screen(&mut app, 80, 20);
    assert!(!text.contains("Ran 2"), "{text}");
    app.update(live(ran("r2", "cargo build", CallState::Done)));
    let text = screen(&mut app, 80, 20);
    assert!(text.contains("Ran 2 commands"), "{text}");

    // A call that waits for permission is no part of a group.
    let mut app = grouped(vec![
        ran("r1", "ls", CallState::Done),
        ran("r2", "pwd", CallState::Done),
        SessionEvent::ToolCall(call("r3", "run_command", CallState::Waiting)),
    ]);
    let text = screen(&mut app, 80, 20);
    assert!(text.contains("Ran 2 commands"), "{text}");
    assert!(text.contains("? ask"), "{text}");
}

#[test]
fn a_group_line_counts_what_is_not_ok() {
    let mut app = grouped(vec![
        ran("r1", "ls", CallState::Done),
        ran("r2", "false", CallState::Error),
        ran("r3", "rm -rf x", CallState::Denied),
    ]);
    let text = screen(&mut app, 80, 12);
    assert!(text.contains("Ran 3 commands"), "{text}");
    assert!(text.contains("✗ 1 failed, ⊘ 1 denied"), "{text}");
    let mut app = App::new(
        settings(false, false),
        attachment(vec![
            ran("r1", "ls", CallState::Done),
            ran("r2", "false", CallState::Error),
        ]),
        Instant::now(),
    );
    let text = screen(&mut app, 80, 12);
    assert!(text.contains("> Ran 2 commands"), "{text}");
    assert!(text.contains("x 1 failed"), "{text}");
}

#[test]
fn keys_open_groups_and_skip_their_hidden_calls() {
    let mut events = five_commands();
    events.push(SessionEvent::TextDone {
        text: "Done.".into(),
    });
    let mut app = grouped(events);
    select_last(&mut app);
    assert_eq!(app.selected, Some(6));
    app.update(key(KeyCode::Up));
    // The group's first call stands for the group.
    assert_eq!(app.selected, Some(1));
    app.update(key(KeyCode::Up));
    assert_eq!(app.selected, Some(0));
    app.update(key(KeyCode::Down));
    assert_eq!(app.selected, Some(1));
    app.update(key(KeyCode::Down));
    assert_eq!(app.selected, Some(6));
    app.update(key(KeyCode::Up));
    // `o` has no call to open on a closed group.
    app.update(ch('o'));
    assert!(app.details.is_none());
    app.update(key(KeyCode::Enter));
    assert!(screen(&mut app, 80, 40).contains("▾ Ran 5 commands"));
    app.update(key(KeyCode::Down));
    assert_eq!(
        app.selected,
        Some(2),
        "an open group's calls are selectable"
    );
    app.update(key(KeyCode::Up));
    app.update(key(KeyCode::Enter));
    assert!(screen(&mut app, 80, 40).contains("▸ Ran 5 commands"));
    // `Ctrl-T` opens and closes the last turn's groups.
    app.update(ctrl('t'));
    assert!(screen(&mut app, 80, 40).contains("▾ Ran 5 commands"));
    app.update(ctrl('t'));
    assert!(screen(&mut app, 80, 40).contains("▸ Ran 5 commands"));
}

#[test]
fn a_call_in_an_open_group_keeps_its_own_click() {
    let mut app = grouped(five_commands());
    let row = row_of(&mut app, "Ran 5 commands");
    click_on(&mut app, row);
    let call_row = row_of(&mut app, "run cargo test t3");
    assert_eq!(view_of(&app, "r3"), Expansion::Preview);
    click_on(&mut app, call_row);
    assert_eq!(view_of(&app, "r3"), Expansion::Collapsed);
    assert!(screen(&mut app, 80, 40).contains("▾ Ran 5 commands"));
    // A double click on the group's line acts as one click.
    double_click_on(&mut app, row);
    assert!(screen(&mut app, 80, 40).contains("▸ Ran 5 commands"));
}

#[test]
fn y_on_a_closed_group_copies_its_commands() {
    let mut app = grouped(five_commands());
    select_last(&mut app);
    let copied = copies(&app.update(ch('y')));
    assert_eq!(
        copied,
        ["cargo test t1\ncargo test t2\ncargo test t3\ncargo test t4\ncargo test t5"]
    );
}

#[test]
fn group_tools_off_shows_every_call() {
    let mut settings = settings(true, true);
    settings.tui.group_tools = false;
    let mut app = App::new(settings, attachment(five_commands()), Instant::now());
    let text = screen(&mut app, 80, 20);
    assert!(!text.contains("Ran 5"), "{text}");
    assert!(text.contains("run cargo test t5"), "{text}");
    // Turned on from the settings, it applies at once.
    let mut tui = app.settings.tui.clone();
    tui.group_tools = true;
    app.apply(tui);
    assert!(screen(&mut app, 80, 20).contains("Ran 5 commands"));
}

#[test]
fn open_groups_are_kept_with_the_choices() {
    let mut app = grouped(five_commands());
    let row = row_of(&mut app, "Ran 5 commands");
    click_on(&mut app, row);
    let (choices, queued) = app.conversation.take_choices();
    assert!(screen(&mut app, 80, 40).contains("▸ Ran 5 commands"));
    app.conversation.set_choices(choices, queued);
    assert!(screen(&mut app, 80, 40).contains("▾ Ran 5 commands"));
}
