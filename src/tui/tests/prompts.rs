//! Permission prompts and questions from the model.

#![allow(clippy::wildcard_imports)]

use super::*;

// Prompts.

#[test]
fn a_permission_question_opens_the_prompt_and_takes_the_keys() {
    let mut app = asked();
    assert!(app.prompt.is_some());
    let text = screen(&mut app, 80, 24);
    assert!(text.contains("▾ edit"), "{text}");
    assert!(text.contains("+for i in 0..lines.len() {"), "{text}");
    // The short name in the summary; the rule text keeps the real one.
    assert!(
        text.contains("edit: src/parse.rs (line 1)  asks: default for edit_file"),
        "{text}"
    );
    assert!(!text.contains("edit_file: "), "{text}");
    // A key that answers nothing goes nowhere.
    app.update(ch('x'));
    assert!(app.editor.is_empty());
    let effects = app.update(ch('y'));
    assert_eq!(answer_of(&effects), Answer::Allow);
    // No second answer while the first is on its way.
    assert!(requests(&app.update(ch('n'))).is_empty());
}

#[test]
fn the_choices_answer() {
    for (input, expected) in [
        (ch('y'), Answer::Allow),
        (ch('n'), Answer::Deny),
        (key(KeyCode::Esc), Answer::Cancel),
    ] {
        let mut app = asked();
        assert_eq!(answer_of(&app.update(input)), expected);
    }
}

#[test]
fn a_and_tab_send_the_widened_proposal() {
    let mut app = asked();
    assert!(requests(&app.update(ch('a'))).is_empty());
    assert!(screen(&mut app, 80, 24).contains("edit_file(src/parse.rs)"));
    app.update(key(KeyCode::Tab));
    assert!(screen(&mut app, 80, 24).contains("edit_file(src/**)"));
    let effects = app.update(key(KeyCode::Enter));
    assert_eq!(
        answer_of(&effects),
        Answer::AllowAlways {
            rules: vec!["edit_file(src/**)".into()]
        }
    );
}

#[test]
fn esc_goes_back_from_always() {
    let mut app = asked();
    app.update(ch('a'));
    assert!(requests(&app.update(key(KeyCode::Esc))).is_empty());
    assert_eq!(app.prompt.as_ref().unwrap().mode, Mode::Choices);
}

#[test]
fn e_sends_a_message_to_the_model() {
    let mut app = asked();
    app.update(ch('e'));
    assert!(requests(&app.update(key(KeyCode::Enter))).is_empty());
    typed(&mut app, "use b.rs");
    app.update(Input::Paste("!\nnow".into()));
    assert!(screen(&mut app, 80, 24).contains("Message to the model: use b.rs! now"));
    assert_eq!(
        answer_of(&app.update(key(KeyCode::Enter))),
        Answer::DenyMessage {
            text: "use b.rs! now".into()
        }
    );
}

#[test]
fn esc_goes_back_from_the_message() {
    let mut app = asked();
    app.update(ch('e'));
    typed(&mut app, "x");
    assert!(requests(&app.update(key(KeyCode::Esc))).is_empty());
    assert_eq!(app.prompt.as_ref().unwrap().mode, Mode::Choices);
}

#[test]
fn a_rejected_answer_keeps_the_prompt() {
    let mut app = asked();
    let effects = app.update(ch('y'));
    let request = requests(&effects).remove(0);
    app.update(Input::Response {
        request,
        result: Err("not allowed".into()),
    });
    assert!(screen(&mut app, 80, 24).contains("not accepted: not allowed"));
    assert_eq!(answer_of(&app.update(ch('n'))), Answer::Deny);
}

#[test]
fn answered_elsewhere_closes_the_prompt_and_says_who() {
    let mut app = asked();
    app.update(live(SessionEvent::QuestionClosed {
        id: "q1".into(),
        closed: Closed::Answered {
            answer: Answer::Allow,
            by: "tui pts/4".into(),
        },
    }));
    assert!(app.prompt.is_none());
    assert_eq!(notices(&app), ["answered by tui pts/4: allow"]);
}

#[test]
fn own_answers_add_no_notice() {
    let mut app = asked();
    app.update(ch('y'));
    app.update(live(SessionEvent::QuestionClosed {
        id: "q1".into(),
        closed: Closed::Answered {
            answer: Answer::Allow,
            by: "tui pts/3".into(),
        },
    }));
    assert!(app.prompt.is_none());
    assert!(notices(&app).is_empty());
}

#[test]
fn another_question_closing_changes_nothing() {
    let mut app = asked();
    app.update(live(SessionEvent::QuestionClosed {
        id: "q9".into(),
        closed: Closed::Cancelled,
    }));
    assert!(app.prompt.is_some());
}

#[test]
fn the_call_collapses_after_the_question() {
    let mut app = asked();
    app.update(live(SessionEvent::QuestionClosed {
        id: "q1".into(),
        closed: Closed::Cancelled,
    }));
    app.update(live(SessionEvent::ToolCall(done(
        call("c1", "edit_file", CallState::Running),
        "edited",
    ))));
    let text = screen(&mut app, 80, 24);
    assert!(text.contains("▸ edit"), "{text}");
    assert!(!text.contains("+for i"), "{text}");
}

#[test]
fn a_long_diff_is_shown_in_full_from_its_start_and_can_be_scrolled() {
    let mut question = permission();
    let QuestionKind::Permission(p) = &mut question.kind else {
        panic!("not a permission");
    };
    let diff: String = (0..60).map(|i| format!("+line {i}\n")).collect();
    let Details::EditFile { diff: text, .. } = &mut p.details else {
        panic!("not an edit");
    };
    text.text = diff;
    let mut app = app();
    notices_in(&mut app, 5);
    app.update(live(SessionEvent::ToolCall(call(
        "c1",
        "edit_file",
        CallState::Waiting,
    ))));
    app.update(live(SessionEvent::Question(Box::new(question))));
    let text = screen(&mut app, 80, 24);
    // The summary line is in view, and no "more lines": nothing is cut.
    assert!(text.contains("▾ edit"), "{text}");
    assert!(!text.contains("more lines"), "{text}");
    assert!(!app.follow);
    // Scrolling works while the prompt is open, and is no answer.
    let effects = app.update(key(KeyCode::PageDown));
    assert!(requests(&effects).is_empty());
    assert!(app.prompt.is_some());
    app.update(press(KeyCode::End, KeyModifiers::CONTROL));
    assert!(screen(&mut app, 80, 24).contains("+line 59"));
}

#[test]
fn following_resumes_when_a_long_question_closes() {
    let mut question = permission();
    if let QuestionKind::Permission(p) = &mut question.kind
        && let Details::EditFile { diff, .. } = &mut p.details
    {
        diff.text = (0..60).map(|i| format!("+line {i}\n")).collect();
    }
    let mut app = app();
    app.update(live(SessionEvent::ToolCall(call(
        "c1",
        "edit_file",
        CallState::Waiting,
    ))));
    app.update(live(SessionEvent::Question(Box::new(question))));
    screen(&mut app, 80, 24);
    assert!(!app.follow);
    app.update(live(SessionEvent::QuestionClosed {
        id: "q1".into(),
        closed: Closed::Cancelled,
    }));
    assert!(app.follow);
}

#[test]
fn ctrl_d_leaves_a_question_for_another_client() {
    let mut app = asked();
    let effects = app.update(ctrl('d'));
    assert_eq!(exit(&effects), detached("project"));
    assert!(requests(&effects).is_empty());
}

#[test]
fn the_limit_prompt() {
    for (input, expected) in [
        (ch('y'), Answer::Continue),
        (ch('n'), Answer::Stop),
        (key(KeyCode::Esc), Answer::Stop),
    ] {
        let mut app = app();
        app.update(live(SessionEvent::Question(Box::new(limit()))));
        let text = screen(&mut app, 100, 24);
        assert!(
            text.contains("Turn limit reached (50 requests, 120 tool calls)"),
            "{text}"
        );
        assert_eq!(answer_of(&app.update(input)), expected);
    }
}

#[test]
fn an_open_question_opens_the_prompt_on_attach() {
    let mut session = info();
    session.state = State::Waiting;
    session.question = Some(permission());
    let events = vec![
        SessionEvent::ToolCall(call("c1", "edit_file", CallState::Waiting)),
        SessionEvent::Question(Box::new(permission())),
    ];
    let attachment = Attachment {
        session: session.clone(),
        sessions: vec![session],
        backlog: events.into_iter().map(replayed).collect(),
    };
    let mut app = App::new(settings(true, true), attachment, Instant::now());
    assert_eq!(app.prompt.as_ref().unwrap().id(), "q1");
    assert!(screen(&mut app, 80, 24).contains("+for i in"));
}

#[test]
fn a_question_in_the_log_that_is_no_longer_open_opens_nothing() {
    let mut app = app_with(vec![
        SessionEvent::ToolCall(call("c1", "edit_file", CallState::Waiting)),
        SessionEvent::Question(Box::new(permission())),
    ]);
    assert!(app.prompt.is_none());
    // The replayed question still gives the call its diff.
    select_last(&mut app);
    assert!(screen(&mut app, 80, 24).contains("+for i in"));
}

// Questions from the model.

pub(super) fn ask_question(two: bool) -> Question {
    let mut questions = vec![json!({
        "question": "Which database should the cache use?", "header": "Database",
        "options": [
            {"label": "SQLite (recommended)", "description": "One file next to the binary, no server."},
            {"label": "Postgres", "description": "Shared with the API; needs the server running."}
        ]
    })];
    if two {
        questions.push(json!({
            "question": "Which features should the first version have?", "header": "Features",
            "options": [{"label": "Search"}, {"label": "Export"}, {"label": "Import"}],
            "multiple": true
        }));
    }
    let arguments = json!({
        "questions": questions,
        "context": "Both work with the current code; the choice decides the **migration**."
    });
    let asked = crate::tools::ask_user::check(&arguments).unwrap();
    Question {
        id: "q3".to_owned(),
        session: SessionRef::default(),
        opened: 0,
        turn: TurnInfo {
            number: 1,
            requests: 1,
            tool_calls: 1,
        },
        summary: crate::tools::ask_user::summary(&asked.questions),
        choices: crate::agent::ASK_USER_CHOICES.to_vec(),
        kind: QuestionKind::AskUser(Box::new(crate::agent::AskUser {
            call: crate::agent::AskCall {
                tool: "ask_user".into(),
                id: "c9".into(),
                arguments,
            },
            questions: asked.questions,
            context: asked.context,
            text: None,
        })),
    }
}

pub(super) fn ask(mut app: App, two: bool) -> App {
    let mut waiting = call("c9", "ask_user", CallState::Waiting);
    waiting.summary = Some(
        if two {
            "Database, Features"
        } else {
            "Database"
        }
        .into(),
    );
    waiting.arguments = json!({});
    app.update(live(SessionEvent::ToolCall(waiting)));
    app.update(live(SessionEvent::Question(Box::new(ask_question(two)))));
    app
}

pub(super) fn user_answer(selected: &[usize], text: Option<&str>) -> crate::tools::UserAnswer {
    crate::tools::UserAnswer {
        selected: selected.to_vec(),
        text: text.map(str::to_owned),
    }
}

#[test]
fn one_question_enter_sends_the_selected_option() {
    let mut app = ask(super::app(), false);
    assert!(app.prompt.as_ref().is_some_and(|p| p.is_ask()));
    assert!(requests(&app.update(key(KeyCode::Down))).is_empty());
    assert_eq!(
        answer_of(&app.update(key(KeyCode::Enter))),
        Answer::Reply {
            answers: vec![user_answer(&[1], None)]
        }
    );
    // A digit selects; then Enter sends it.
    let mut app = ask(super::app(), false);
    assert!(requests(&app.update(ch('1'))).is_empty());
    assert_eq!(
        answer_of(&app.update(key(KeyCode::Enter))),
        Answer::Reply {
            answers: vec![user_answer(&[0], None)]
        }
    );
}

#[test]
fn a_typed_answer() {
    let mut app = ask(super::app(), false);
    app.update(ch('3'));
    typed(&mut app, "Redis");
    assert_eq!(
        answer_of(&app.update(key(KeyCode::Enter))),
        Answer::Reply {
            answers: vec![user_answer(&[], Some("Redis"))]
        }
    );
    // Ctrl-D while typing does not detach.
    let mut app = ask(super::app(), false);
    app.update(ch('3'));
    typed(&mut app, "ab");
    app.update(key(KeyCode::Left));
    assert!(exit(&app.update(ctrl('d'))).is_none());
    assert_eq!(
        answer_of(&app.update(key(KeyCode::Enter))),
        Answer::Reply {
            answers: vec![user_answer(&[], Some("ab"))]
        }
    );
}

#[test]
fn several_questions_tick_tab_and_review() {
    let mut app = ask(super::app(), true);
    // First question: Postgres, then on to the second.
    app.update(key(KeyCode::Down));
    assert!(requests(&app.update(key(KeyCode::Enter))).is_empty());
    // Tick Search and Import, add a text.
    app.update(ch(' '));
    app.update(ch('3'));
    app.update(ch('4'));
    typed(&mut app, "keep the old format");
    app.update(key(KeyCode::Enter));
    let text = screen(&mut app, 80, 24);
    assert!(text.contains("Question ─ review"), "{text}");
    assert!(text.contains("Postgres"), "{text}");
    assert!(
        text.contains("Search, Import; \"keep the old format\""),
        "{text}"
    );
    // Back to the first question with Shift-Tab, which keeps the choice.
    app.update(key(KeyCode::BackTab));
    app.update(key(KeyCode::BackTab));
    let text = screen(&mut app, 80, 24);
    assert!(text.contains("1/2 Database"), "{text}");
    app.update(key(KeyCode::Tab));
    app.update(key(KeyCode::Tab));
    assert_eq!(
        answer_of(&app.update(key(KeyCode::Enter))),
        Answer::Reply {
            answers: vec![
                user_answer(&[1], None),
                user_answer(&[0, 2], Some("keep the old format"))
            ]
        }
    );
}

#[test]
fn the_review_sends_only_complete_answers() {
    let mut app = ask(super::app(), true);
    app.update(key(KeyCode::Tab));
    app.update(key(KeyCode::Tab));
    // Nothing chosen: Enter goes to the first question without an answer.
    assert!(requests(&app.update(key(KeyCode::Enter))).is_empty());
    assert_eq!(app.prompt.as_ref().unwrap().ask.as_ref().unwrap().page, 0);
}

#[test]
fn esc_declines_and_asks_first_when_something_is_chosen() {
    let mut app = ask(super::app(), true);
    assert_eq!(answer_of(&app.update(key(KeyCode::Esc))), Answer::Decline);

    let mut app = ask(super::app(), true);
    app.update(key(KeyCode::Enter));
    assert!(requests(&app.update(key(KeyCode::Esc))).is_empty());
    let text = screen(&mut app, 80, 24);
    assert!(text.contains("Skip the questions"), "{text}");
    assert!(requests(&app.update(ch('n'))).is_empty());
    app.update(key(KeyCode::Esc));
    assert_eq!(answer_of(&app.update(ch('y'))), Answer::Decline);

    let mut app = ask(super::app(), true);
    app.update(key(KeyCode::Enter));
    assert_eq!(answer_of(&app.update(ctrl('c'))), Answer::Cancel);
}

#[test]
fn the_question_box_closes_when_another_client_answers() {
    let mut app = ask(super::app(), false);
    app.update(live(SessionEvent::QuestionClosed {
        id: "q3".into(),
        closed: Closed::Answered {
            answer: Answer::Decline,
            by: "codeagent chat".into(),
        },
    }));
    assert!(app.prompt.is_none());
    assert_eq!(notices(&app), ["answered by codeagent chat: decline"]);
    assert!(!screen(&mut app, 80, 24).contains("Question ─"));
}

#[test]
fn a_rejected_reply_shows_in_the_box() {
    let mut app = ask(super::app(), false);
    let effects = app.update(key(KeyCode::Enter));
    let request = requests(&effects).remove(0);
    app.update(Input::Response {
        request,
        result: Err("answer 1: option 0 does not exist".into()),
    });
    assert!(screen(&mut app, 80, 24).contains("not accepted: answer 1: option 0 does not exist"));
    assert!(!requests(&app.update(key(KeyCode::Enter))).is_empty());
}

#[test]
fn frame_question_from_the_model() {
    let mut app = ask(idle_session(), false);
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}

#[test]
fn frame_question_from_the_model_multiple() {
    let mut app = ask(idle_session(), true);
    app.update(key(KeyCode::Enter));
    app.update(ch(' '));
    app.update(ch('3'));
    insta::assert_snapshot!(screen(&mut app, 80, 24));
}

#[test]
fn frame_question_from_the_model_narrow_and_ascii() {
    let mut app = ask(idle_session(), true);
    insta::assert_snapshot!("question_from_the_model_narrow", screen(&mut app, 40, 12));
    let app = App::new(settings(false, true), attachment(vec![]), Instant::now());
    let mut app = ask(app, true);
    let text = screen(&mut app, 80, 24);
    assert!(text.is_ascii(), "{text}");
    insta::assert_snapshot!("question_from_the_model_ascii", text);
}

// Questions at the bottom (RFC 0022).

#[test]
fn a_question_takes_the_place_of_the_input_and_keeps_its_text() {
    let mut app = idle_session();
    typed(&mut app, "half a thought");
    let mut app = ask(app, false);
    let text = screen(&mut app, 80, 24);
    assert!(!text.contains("half a thought"), "{text}");
    assert!(text.contains("── Question ─ Database ──"), "{text}");
    // The rows of the input and its rules: the question ends above the footer.
    let rows: Vec<&str> = text.lines().collect();
    assert!(rows[20].starts_with("Up Down choose"), "{text}");
    assert!(rows[21].starts_with("────"), "{text}");
    // Answered elsewhere: the input comes back as it was.
    app.update(live(SessionEvent::QuestionClosed {
        id: "q3".into(),
        closed: Closed::Answered {
            answer: Answer::Decline,
            by: "tui pts/4".into(),
        },
    }));
    assert!(app.prompt.is_none());
    assert_eq!(app.editor.text(), "half a thought");
    assert!(screen(&mut app, 80, 24).contains("> half a thought"));
}

#[test]
fn the_input_comes_back_after_an_answer_a_skip_and_a_cancel() {
    for keys in [&[KeyCode::Enter][..], &[KeyCode::Esc][..]] {
        let mut app = idle_session();
        typed(&mut app, "kept");
        let mut app = ask(app, false);
        for code in keys {
            app.update(key(*code));
        }
        // The answer goes to the server; the question closes when it is
        // accepted.
        app.update(live(SessionEvent::QuestionClosed {
            id: "q3".into(),
            closed: Closed::Answered {
                answer: Answer::Decline,
                by: "tui pts/3".into(),
            },
        }));
        assert_eq!(app.editor.text(), "kept");
    }
    let mut app = idle_session();
    typed(&mut app, "kept");
    let mut app = ask(app, false);
    app.update(ctrl('c'));
    assert_eq!(app.editor.text(), "kept");
}

#[test]
fn frame_a_tall_question_keeps_three_rows_of_conversation() {
    let mut app = ask(idle_session(), true);
    app.update(key(KeyCode::Enter));
    let text = screen(&mut app, 80, 14);
    let rows: Vec<&str> = text.lines().collect();
    // 3 rows of conversation, then the titled rule.
    assert!(rows[3].starts_with("── Question"), "{text}");
    // The keys line stays the last row of the question.
    assert!(rows[10].starts_with("Space tick"), "{text}");
    insta::assert_snapshot!(text);
}

#[test]
fn frame_a_question_on_the_smallest_screen() {
    let mut app = ask(idle_session(), false);
    let text = screen(&mut app, 40, 10);
    let rows: Vec<&str> = text.lines().collect();
    assert!(rows[0].starts_with("── Question"), "{text}");
    insta::assert_snapshot!(text);
}

#[test]
fn the_wheel_scrolls_the_question_under_it_and_the_conversation_above() {
    let mut app = ask(idle_session(), true);
    app.update(key(KeyCode::Enter));
    screen(&mut app, 80, 14);
    let wheel = |app: &mut App, kind, row| {
        app.update(mouse(kind, 10, row));
        screen(app, 80, 14)
    };
    let before = app.prompt.as_ref().unwrap().ask.as_ref().unwrap().top;
    wheel(&mut app, MouseEventKind::ScrollDown, 6);
    let after = app.prompt.as_ref().unwrap().ask.as_ref().unwrap().top;
    assert!(after > before, "{before} {after}");
    // Never further than the question's end.
    for _ in 0..20 {
        wheel(&mut app, MouseEventKind::ScrollDown, 6);
    }
    let most = app.prompt.as_ref().unwrap().ask.as_ref().unwrap().top;
    wheel(&mut app, MouseEventKind::ScrollUp, 6);
    assert!(app.prompt.as_ref().unwrap().ask.as_ref().unwrap().top < most);
    // Over the conversation, the conversation scrolls, not the question.
    let asked = app.prompt.as_ref().unwrap().ask.as_ref().unwrap().top;
    assert!(app.follow);
    wheel(&mut app, MouseEventKind::ScrollUp, 1);
    assert!(!app.follow);
    assert_eq!(
        app.prompt.as_ref().unwrap().ask.as_ref().unwrap().top,
        asked
    );
}

#[test]
fn the_picker_opens_over_a_question_and_the_question_comes_back() {
    let mut app = ask(idle_session(), false);
    app.update(ctrl('s'));
    app.update(sessions_answer(three()));
    assert!(app.session_list.is_some());
    let text = screen(&mut app, 80, 24);
    assert!(text.contains("Sessions") || text.contains("docs"), "{text}");
    app.update(key(KeyCode::Esc));
    assert!(app.session_list.is_none());
    let text = screen(&mut app, 80, 24);
    assert!(text.contains("── Question ─ Database"), "{text}");
}
