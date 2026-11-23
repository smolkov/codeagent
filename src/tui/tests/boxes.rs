//! The lsp box (RFC 0019), the skills box (RFC 0023), boxes at the bottom (RFC 0024).

#![allow(clippy::wildcard_imports)]

use super::*;

// ------------------------------------------------------- lsp (RFC 0019)

pub(super) fn lsp_server(name: &str, root: &str, state: &str) -> crate::protocol::LspServerInfo {
    crate::protocol::LspServerInfo {
        name: name.to_owned(),
        root: std::path::PathBuf::from(root),
        state: state.to_owned(),
        percent: None,
        reason: None,
        sessions: vec!["project".to_owned()],
        stderr: Vec::new(),
    }
}

pub(super) fn lsp_answer(request: Request, servers: Vec<crate::protocol::LspServerInfo>) -> Input {
    Input::Response {
        request,
        result: Ok(Payload::Lsp { servers }),
    }
}

#[test]
fn lsp_box_lists_restarts_and_stops_servers() {
    let mut app = app();
    let effects = command(&mut app, "/lsp");
    assert_eq!(requests(&effects), [Request::LspStatus]);
    assert!(screen(&mut app, 100, 24).contains("asking the server"));

    let mut failed = lsp_server("gopls", "/elsewhere/go", "failed");
    failed.reason = Some("no go.mod".into());
    failed.stderr = vec!["starting".into(), "error: no go.mod".into()];
    let mut indexing = lsp_server("rust-analyzer", "/ws/project", "indexing");
    indexing.percent = Some(42);
    app.update(lsp_answer(Request::LspStatus, vec![indexing, failed]));
    let text = screen(&mut app, 100, 24);
    assert!(
        text.contains("rust-analyzer  .              indexing 42 %"),
        "{text}"
    );
    assert!(text.contains("failed: no go.mod"), "{text}");
    assert!(text.contains("r restart"), "{text}");

    // Down selects gopls and shows its last lines.
    app.update(key(KeyCode::Down));
    let text = screen(&mut app, 100, 24);
    assert!(text.contains("Last lines of gopls:"), "{text}");
    assert!(text.contains("error: no go.mod"), "{text}");
    let effects = app.update(ch('r'));
    assert_eq!(
        requests(&effects),
        [Request::LspRestart {
            name: "gopls".into(),
            root: "/elsewhere/go".into()
        }]
    );
    // A failed restart shows its error and keeps the list.
    app.update(Input::Response {
        request: Request::LspRestart {
            name: "gopls".into(),
            root: "/elsewhere/go".into(),
        },
        result: Err("gopls: no go.mod".into()),
    });
    assert!(screen(&mut app, 100, 24).contains("gopls: no go.mod"));
    let effects = app.update(ch('k'));
    assert_eq!(
        requests(&effects),
        [Request::LspStop {
            name: "gopls".into(),
            root: "/elsewhere/go".into()
        }]
    );
    app.update(key(KeyCode::Esc));
    assert!(app.lsp_box.is_none());
}

#[test]
fn lsp_box_without_servers_and_a_failed_first_answer() {
    let mut app = app();
    command(&mut app, "/lsp");
    app.update(lsp_answer(Request::LspStatus, Vec::new()));
    assert!(screen(&mut app, 100, 24).contains("No language server has run"));
    // r and k do nothing without a server.
    assert!(requests(&app.update(ch('r'))).is_empty());
    app.update(key(KeyCode::Esc));

    command(&mut app, "/lsp");
    app.update(Input::Response {
        request: Request::LspStatus,
        result: Err("unknown command".into()),
    });
    assert!(app.lsp_box.is_none());
}

pub(super) fn lsp_start_question() -> Question {
    let mut question = permission();
    question.summary = "start language server rust-analyzer in ~/project".to_owned();
    question.choices = crate::agent::LSP_START_CHOICES.to_vec();
    if let QuestionKind::Permission(p) = &mut question.kind {
        p.call.tool = Tool::Lsp;
        p.details = Details::LspStart {
            server: "rust-analyzer".into(),
            command: vec!["rust-analyzer".into()],
            root: "~/project".into(),
            env: Vec::new(),
        };
        p.proposals = Vec::new();
    }
    question
}

#[test]
fn the_start_question_names_the_command_and_has_no_always() {
    let mut app = app();
    app.update(live(SessionEvent::ToolCall(call(
        "c1",
        "lsp",
        CallState::Waiting,
    ))));
    app.update(live(SessionEvent::Question(Box::new(lsp_start_question()))));
    let text = screen(&mut app, 100, 24);
    assert!(
        text.contains("Start language server rust-analyzer in ~/project?"),
        "{text}"
    );
    assert!(text.contains("command: rust-analyzer"), "{text}");
    assert!(
        text.contains("It may run build scripts and macros"),
        "{text}"
    );
    assert!(
        text.contains("[y] yes  [n] no  [e] explain  [Esc] cancel"),
        "{text}"
    );
    assert!(!text.contains("[a]"), "{text}");
    insta::assert_snapshot!(screen(&mut app, 80, 16));
}

#[test]
fn an_lsp_call_line_shows_its_count() {
    let mut app = app();
    let mut event = call("c1", "lsp", CallState::Running);
    event.summary = Some("references parse src/parse.rs:3".into());
    app.update(live(SessionEvent::ToolCall(event.clone())));
    app.update(live(SessionEvent::ToolCall(done(
        event,
        "3 references to parse:\nsrc/a.rs:1:1: a\nsrc/b.rs:2:2: b\nsrc/c.rs:3:3: c",
    ))));
    let text = screen(&mut app, 80, 24);
    assert!(
        text.contains("lsp references parse src/parse.rs:3"),
        "{text}"
    );
    assert!(text.contains("3 references"), "{text}");
}

// The skills box (RFC 0023).

pub(super) fn skill_info(dir: &str, name: &str, description: &str) -> crate::protocol::SkillInfo {
    crate::protocol::SkillInfo {
        name: name.to_owned(),
        description: description.to_owned(),
        dir: format!("{dir}/{name}").into(),
        source: crate::skills::Source::User,
        shadowed_by: None,
    }
}

pub(super) fn skills_found() -> crate::protocol::SkillsInfo {
    crate::protocol::SkillsInfo {
        skills: vec![
            skill_info(
                "/home/u/ws/team-skills",
                "release-check",
                "Steps before tagging a release of a Rust crate. Use when the user asks to prepare or check a release.",
            ),
            skill_info(
                "/home/u/ws/team-skills",
                "api-style",
                "How our HTTP handlers are written and tested.",
            ),
            skill_info(
                "/ws/project/.claude/skills",
                "migrations",
                "Writing and checking database migrations.",
            ),
            skill_info(
                "/home/u/.claude/skills",
                "pdf",
                "Read, fill and merge PDF files.",
            ),
            crate::protocol::SkillInfo {
                shadowed_by: Some("/home/u/ws/team-skills/release-check".into()),
                ..skill_info(
                    "/home/u/.claude/skills",
                    "release-check",
                    "Old release steps.",
                )
            },
        ],
        dirs: vec![
            "/home/u/ws/team-skills".into(),
            "/ws/project/.claude/skills".into(),
            "/home/u/.claude/skills".into(),
        ],
        warnings: vec!["~/.agents/skills/old/SKILL.md: no description".into()],
        active: Vec::new(),
        home: Some("/home/u".into()),
    }
}

pub(super) fn active_skill(name: &str, dir: &str) -> crate::protocol::ActiveSkill {
    crate::protocol::ActiveSkill {
        name: name.to_owned(),
        dir: dir.into(),
        tokens: 400,
        changed_on_disk: false,
        missing: false,
    }
}

pub(super) fn skills_request() -> Request {
    Request::Skills {
        workspace: "/ws/project".into(),
        session: Some("project".into()),
    }
}

pub(super) fn skills_answer(info: crate::protocol::SkillsInfo) -> Input {
    Input::Response {
        request: skills_request(),
        result: Ok(Payload::Skills(Box::new(info))),
    }
}

pub(super) fn skill_on(name: &str) -> SessionEvent {
    SessionEvent::SkillsChanged {
        added: vec![crate::skills::Active {
            name: name.to_owned(),
            dir: format!("/home/u/ws/team-skills/{name}").into(),
            text: "# text".into(),
        }],
        removed: Vec::new(),
    }
}

/// An app with the skills box open on the usual list.
pub(super) fn choosing_skills(info: crate::protocol::SkillsInfo) -> App {
    let mut app = app();
    assert_eq!(requests(&command(&mut app, "/skills")), [skills_request()]);
    assert!(app.skills_box.is_none(), "it opens with the answer");
    app.update(skills_answer(info));
    assert!(app.skills_box.is_some());
    app
}

pub(super) fn set_skills_of(effects: &[Effect]) -> Vec<Vec<String>> {
    requests(effects)
        .into_iter()
        .filter_map(|r| match r {
            Request::SetSkills { names, .. } => Some(names),
            _ => None,
        })
        .collect()
}

#[test]
fn space_ticks_and_enter_applies() {
    let mut app = choosing_skills(skills_found());
    assert_eq!(
        app.skills_box.as_ref().unwrap().chosen(),
        Some("release-check")
    );
    app.update(ch(' '));
    app.update(key(KeyCode::Down));
    app.update(key(KeyCode::Down));
    app.update(ch(' '));
    let effects = app.update(key(KeyCode::Enter));
    assert_eq!(
        set_skills_of(&effects),
        [vec!["release-check".to_owned(), "migrations".to_owned()]]
    );
    assert!(app.skills_box.is_none());
}

#[test]
fn shadowed_rows_are_passed_over() {
    let mut app = choosing_skills(skills_found());
    for _ in 0..10 {
        app.update(key(KeyCode::Down));
    }
    assert_eq!(app.skills_box.as_ref().unwrap().chosen(), Some("pdf"));
}

#[test]
fn esc_and_an_unchanged_enter_send_nothing() {
    let mut app = choosing_skills(skills_found());
    app.update(ch(' '));
    assert!(requests(&app.update(key(KeyCode::Esc))).is_empty());
    assert!(app.skills_box.is_none());
    let mut info = skills_found();
    info.active = vec![active_skill("pdf", "/home/u/.claude/skills/pdf")];
    let mut app = choosing_skills(info);
    assert!(requests(&app.update(key(KeyCode::Enter))).is_empty());
}

#[test]
fn the_filter_looks_at_names_and_descriptions() {
    let mut app = choosing_skills(skills_found());
    typed(&mut app, "HTTP");
    let open = app.skills_box.as_ref().unwrap();
    assert_eq!(open.chosen(), Some("api-style"));
    app.update(ch(' '));
    let effects = app.update(key(KeyCode::Enter));
    assert_eq!(set_skills_of(&effects), [vec!["api-style".to_owned()]]);
}

#[test]
fn an_active_skill_not_found_any_more_can_be_turned_off() {
    let mut info = skills_found();
    info.active = vec![crate::protocol::ActiveSkill {
        missing: true,
        ..active_skill("gone", "/home/u/old/gone")
    }];
    let mut app = choosing_skills(info);
    assert_eq!(app.skills_box.as_ref().unwrap().chosen(), Some("gone"));
    app.update(ch(' '));
    let effects = app.update(key(KeyCode::Enter));
    assert_eq!(set_skills_of(&effects), [Vec::<String>::new()]);
}

#[test]
fn slash_skills_name_toggles_one() {
    let mut app = app_with(vec![skill_on("pdf")]);
    assert_eq!(app.skills, ["pdf"]);
    assert_eq!(notices(&app), ["skills: + pdf"]);
    let effects = command(&mut app, "/skills release-check");
    assert_eq!(
        set_skills_of(&effects),
        [vec!["pdf".to_owned(), "release-check".to_owned()]]
    );
    let effects = command(&mut app, "/skills pdf");
    assert_eq!(set_skills_of(&effects), [Vec::<String>::new()]);
    // The server refuses an unknown name and lists the found ones.
    app.update(Input::Response {
        request: Request::SetSkills {
            session: "project".into(),
            names: vec!["pdf".into(), "nope".into()],
        },
        result: Err("unknown skill `nope`; found: pdf, release-check".into()),
    });
    assert!(
        kinds(&app).iter().any(|k| matches!(
            k,
            Kind::Error(text) if text == "unknown skill `nope`; found: pdf, release-check"
        )),
        "{:?}",
        kinds(&app)
    );
    app.update(live(SessionEvent::SkillsChanged {
        added: Vec::new(),
        removed: vec!["pdf".into()],
    }));
    assert!(app.skills.is_empty());
    assert_eq!(notices(&app).last().unwrap(), "skills: \u{2212} pdf");
}

#[test]
fn the_argument_list_offers_the_skills() {
    let mut app = app_with(vec![skill_on("pdf")]);
    typed(&mut app, "/skil");
    // Tab completes `/skills `, opens the values and asks for them.
    let asked = requests(&app.update(key(KeyCode::Tab)));
    assert_eq!(app.editor.text(), "/skills ");
    assert_eq!(asked, [skills_request()]);
    assert!(values(&app).is_empty());
    app.update(skills_answer(skills_found()));
    assert!(app.skills_box.is_none(), "only the list asked");
    let values = values(&app);
    assert_eq!(values, ["release-check", "api-style", "migrations", "pdf"]);
    let pdf = app
        .arguments()
        .into_iter()
        .find(|v| v.value == "pdf")
        .unwrap();
    assert!(pdf.description.starts_with("active"), "{}", pdf.description);
}

#[test]
fn a_question_closes_the_skills_box() {
    let mut app = choosing_skills(skills_found());
    app.update(live(SessionEvent::Question(Box::new(limit()))));
    assert!(app.skills_box.is_none());
    // An answer on its way does not open it under the question.
    let mut app = self::app();
    command(&mut app, "/skills");
    app.update(live(SessionEvent::Question(Box::new(limit()))));
    app.update(skills_answer(skills_found()));
    assert!(app.skills_box.is_none());
}

#[test]
fn frame_skills_box() {
    let mut info = skills_found();
    info.active = vec![active_skill(
        "release-check",
        "/home/u/ws/team-skills/release-check",
    )];
    let mut app = choosing_skills(info);
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}

#[test]
fn frame_skills_box_ascii() {
    let mut app = App::new(
        settings(false, true),
        attachment(Vec::new()),
        Instant::now(),
    );
    command(&mut app, "/skills");
    app.update(skills_answer(skills_found()));
    let text = screen(&mut app, 80, 24);
    assert!(text.is_ascii(), "{text}");
    insta::assert_snapshot!(text);
}

#[test]
fn frame_skills_box_without_skills() {
    let mut info = skills_found();
    info.skills.clear();
    info.warnings.clear();
    let mut app = choosing_skills(info);
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}

// Boxes at the bottom (RFC 0024).

#[test]
fn a_box_covers_the_input_and_keeps_the_conversation_above_it() {
    let mut app = idle_session();
    typed(&mut app, "half a message");
    app.update(ctrl('o'));
    app.update(models_answer(model_list()));
    let size = Rect::new(0, 0, 100, 30);
    let areas = view::areas(&app, size).unwrap();
    let panel = areas.panel.unwrap();
    assert_eq!(panel.width, 100, "the full width");
    assert_eq!(areas.input.height, 0);
    assert_eq!(areas.activity, None);
    // Directly above the footer, and the conversation ends above its rule.
    assert_eq!(panel.y + panel.height + 1, areas.status.y);
    assert_eq!(
        areas.conversation.y + areas.conversation.height + 1,
        panel.y
    );
    assert!(areas.conversation.height >= 3);
    let text = screen(&mut app, 100, 30);
    assert!(!text.contains("half a message"), "{text}");
    assert!(
        text.contains("Let me look."),
        "the last answer stays in sight: {text}"
    );
    // The input comes back as it was.
    app.update(key(KeyCode::Esc));
    assert!(app.picker.is_none());
    assert_eq!(app.editor.text(), "half a message");
    let areas = view::areas(&app, size).unwrap();
    assert_eq!(areas.panel, None);
    assert!(areas.input.height > 0);
    assert!(screen(&mut app, 100, 30).contains("half a message"));
}

#[test]
fn a_tall_box_leaves_three_rows_of_conversation() {
    let mut app = app();
    app.update(ctrl('o'));
    let many: Vec<ModelInfo> = (0..60)
        .map(|i| model("p", &format!("m{i}"), &format!("id-{i}"), 1000))
        .collect();
    app.update(models_answer(many));
    let areas = view::areas(&app, Rect::new(0, 0, 80, 24)).unwrap();
    assert_eq!(areas.conversation.height, 3);
    // Footer, the two rules and the three rows: the rest is the box.
    assert_eq!(areas.panel.unwrap().height, 24 - 2 - 2 - 3);
}

#[test]
fn a_box_over_a_question_gives_it_back() {
    let mut app = with_prompt(idle_session());
    app.update(ctrl('s'));
    app.update(sessions_answer(vec![info()]));
    assert!(app.session_list.is_some());
    let areas = view::areas(&app, Rect::new(0, 0, 80, 24)).unwrap();
    assert!(areas.panel.is_some());
    assert_eq!(areas.prompt, None);
    app.update(key(KeyCode::Esc));
    let areas = view::areas(&app, Rect::new(0, 0, 80, 24)).unwrap();
    assert_eq!(areas.panel, None);
    assert!(areas.prompt.is_some(), "the permission question is back");
}

#[test]
fn settings_take_the_whole_screen() {
    let mut app = idle_session();
    command(&mut app, "/settings");
    assert!(app.settings_box.is_some());
    let areas = view::areas(&app, Rect::new(0, 0, 80, 24)).unwrap();
    let screen_area = areas.screen.unwrap();
    assert_eq!(areas.conversation.height, 0);
    assert_eq!(areas.input.height, 0);
    assert_eq!((screen_area.y, screen_area.height), (1, 24 - 2 - 2));
    let text = screen(&mut app, 80, 24);
    assert!(!text.contains("Let me look."), "{text}");
    assert!(text.starts_with("── Settings"), "{text}");
}

#[test]
fn the_wheel_scrolls_the_status_box_under_it_and_the_conversation_above() {
    let mut app = idle_session();
    command(&mut app, "/status");
    app.update(status_answer(status_of(State::Working)));
    screen(&mut app, 80, 24);
    let panel = app.panel_area.unwrap();
    let wheel = |row: u16| {
        Input::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 5,
            row,
            modifiers: KeyModifiers::NONE,
        })
    };
    app.update(wheel(panel.y + 1));
    assert!(app.status_box.as_ref().unwrap().top > 0);
    let before = app.status_box.as_ref().unwrap().top;
    app.update(wheel(0));
    assert_eq!(app.status_box.as_ref().unwrap().top, before);
}
