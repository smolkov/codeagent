//! Images in the input (RFC 0034).

#![allow(clippy::wildcard_imports)]

// Images in the input (RFC 0034).

use super::*;
use crate::image::fixtures;
use crate::tui::app::dropped_image;
use crate::tui::clipboard::Clip;

pub(super) fn pasted(app: &mut App, bytes: Vec<u8>) {
    let effects = app.update(ctrl('v'));
    assert!(effects.contains(&Effect::ReadClipboard), "{effects:?}");
    app.update(Input::Clipboard(Ok(Clip::Image(bytes))));
}

pub(super) fn sent_images(effects: &[Effect]) -> Vec<(u32, Vec<u8>)> {
    requests(effects)
        .into_iter()
        .flat_map(|r| match r {
            Request::Send { images, .. } => images,
            _ => Vec::new(),
        })
        .map(|u| (u.number, crate::image::base64_decode(&u.data).unwrap()))
        .collect()
}

#[test]
fn ctrl_v_pastes_an_image_and_text_goes_around_it() {
    let mut app = app();
    typed(&mut app, "compare ");
    pasted(&mut app, fixtures::png(1366, 306));
    assert_eq!(app.message.as_deref(), Some("image #1: PNG 1366×306 33 B"));
    typed(&mut app, " with ");
    // Alt-V does the same.
    let effects = app.update(press(KeyCode::Char('v'), KeyModifiers::ALT));
    assert!(effects.contains(&Effect::ReadClipboard));
    app.update(Input::Clipboard(Ok(Clip::Image(fixtures::gif(20, 10)))));
    assert_eq!(app.editor.text(), "compare [Image #1] with [Image #2]");
    insta::assert_snapshot!(screen(&mut app, 60, 12));
    let effects = app.update(key(KeyCode::Enter));
    assert_eq!(
        sent_images(&effects),
        [(1, fixtures::png(1366, 306)), (2, fixtures::gif(20, 10))]
    );
    assert!(app.editor.is_empty());
    assert!(app.editor.images().is_empty());
}

#[test]
fn a_deleted_placeholder_drops_its_image() {
    let mut app = app();
    pasted(&mut app, fixtures::png(1, 1));
    pasted(&mut app, fixtures::png(2, 2));
    app.update(key(KeyCode::Backspace));
    typed(&mut app, "x");
    let effects = app.update(key(KeyCode::Enter));
    assert_eq!(sent_images(&effects), [(1, fixtures::png(1, 1))]);
}

#[test]
fn ctrl_v_with_text_errors_and_nothing() {
    let mut app = app();
    app.update(ctrl('v'));
    app.update(Input::Clipboard(Ok(Clip::Text("some text".into()))));
    assert_eq!(app.editor.text(), "some text");
    app.update(Input::Clipboard(Err(
        "no access to the local clipboard over SSH; use /image PATH".into(),
    )));
    assert_eq!(
        app.message.as_deref(),
        Some("Ctrl-V: no access to the local clipboard over SSH; use /image PATH")
    );
    app.update(Input::Clipboard(Ok(Clip::Empty)));
    assert_eq!(app.message.as_deref(), Some("the clipboard is empty"));
    app.update(Input::Clipboard(Ok(Clip::Image(b"not an image".to_vec()))));
    assert_eq!(
        app.message.as_deref(),
        Some("not an image (PNG, JPEG, GIF or WebP)")
    );
    assert_eq!(app.editor.text(), "some text");
}

#[test]
fn a_dropped_file_becomes_an_image_and_other_text_stays_text() {
    let mut app = app();
    let effects = app.update(Input::Paste("'/home/me/shot one.png'".into()));
    assert_eq!(
        effects,
        [Effect::ReadImage {
            path: "/home/me/shot one.png".into(),
            base: app.session.path.clone(),
            text: Some("'/home/me/shot one.png'".into()),
        }]
    );
    app.update(Input::ImageFile {
        path: "/home/me/shot one.png".into(),
        result: Ok(fixtures::png(3, 3)),
        text: Some("'/home/me/shot one.png'".into()),
    });
    assert_eq!(app.editor.text(), "[Image #1]");
    // A path that turns out not to be an image is pasted as text.
    app.update(Input::ImageFile {
        path: "/tmp/fake.png".into(),
        result: Ok(b"text".to_vec()),
        text: Some("/tmp/fake.png".into()),
    });
    assert_eq!(app.editor.text(), "[Image #1]/tmp/fake.png");
    // Ordinary text never reads a file.
    let effects = app.update(Input::Paste("/tmp/notes.txt".into()));
    assert_eq!(effects, [Effect::Redraw]);
}

#[test]
fn dropped_paths() {
    assert_eq!(dropped_image("/a/b.PNG"), Some("/a/b.PNG".into()));
    assert_eq!(dropped_image(" ~/s.jpg\n"), Some("~/s.jpg".into()));
    assert_eq!(
        dropped_image("file:///a/b%20c.webp"),
        Some("/a/b c.webp".into())
    );
    assert_eq!(dropped_image("/a/b\\ c.gif"), Some("/a/b c.gif".into()));
    assert_eq!(dropped_image("\"/a/b.jpeg\""), Some("/a/b.jpeg".into()));
    for text in ["a.png", "/a.png\n/b.png", "/a/b.txt", "see /a.png"] {
        assert_eq!(dropped_image(text), None, "{text}");
    }
}

#[test]
fn slash_image_reads_a_file() {
    let mut app = app();
    typed(&mut app, "/image shots/a.png");
    let effects = app.update(key(KeyCode::Enter));
    assert!(effects.contains(&Effect::ReadImage {
        path: "shots/a.png".into(),
        base: app.session.path.clone(),
        text: None,
    }));
    assert!(app.editor.is_empty());
    app.update(Input::ImageFile {
        path: "shots/a.png".into(),
        result: Err("No such file or directory (os error 2)".into()),
        text: None,
    });
    assert_eq!(
        notices(&app),
        ["shots/a.png: No such file or directory (os error 2)"]
    );
    app.update(Input::ImageFile {
        path: "shots/a.png".into(),
        result: Ok(fixtures::jpeg(8, 6)),
        text: None,
    });
    assert_eq!(app.editor.text(), "[Image #1]");
}

#[test]
fn a_refused_send_and_the_history_bring_the_images_back() {
    let mut app = app();
    typed(&mut app, "look ");
    pasted(&mut app, fixtures::png(5, 5));
    let effects = app.update(key(KeyCode::Enter));
    let request = requests(&effects).remove(0);
    app.update(Input::Response {
        request: request.clone(),
        result: Err("fake/one does not read images".into()),
    });
    assert_eq!(app.editor.text(), "look [Image #1]");
    assert_eq!(app.editor.images().len(), 1);
    // Sent again, shown, and found again in the history.
    app.update(key(KeyCode::Enter));
    let numbered = crate::protocol::Upload::of(&app_numbered(5))
        .image()
        .unwrap();
    app.update(live(SessionEvent::UserMessage {
        text: "look [Image #1]".into(),
        images: vec![numbered],
    }));
    assert_eq!(notices(&app), ["images: #1 PNG 5×5 33 B"]);
    app.update(key(KeyCode::Up));
    assert_eq!(app.editor.text(), "look [Image #1]");
    assert_eq!(app.editor.images().len(), 1);
    insta::assert_snapshot!(screen(&mut app, 60, 12));
}

pub(super) fn app_numbered(size: u32) -> crate::image::Numbered {
    crate::image::Numbered {
        number: 1,
        image: crate::image::Image::from_bytes(fixtures::png(size, size)).unwrap(),
    }
}

#[test]
fn a_question_shows_its_hidden_characters_and_answers_with_the_original() {
    let mut question = permission();
    question.summary = "edit_file: src/\u{202e}sr.esrap (line 1)".to_owned();
    let QuestionKind::Permission(p) = &mut question.kind else {
        panic!("not a permission");
    };
    p.proposals = vec![vec!["edit_file(src/\u{202e}sr.esrap)".to_owned()]];
    let mut app = app();
    app.update(live(SessionEvent::ToolCall(call(
        "c1",
        "edit_file",
        CallState::Waiting,
    ))));
    app.update(live(SessionEvent::Question(Box::new(question))));
    let text = screen(&mut app, 100, 24);
    assert!(
        text.contains("note: this question contains 2 hidden characters, shown as ⟨…⟩"),
        "{text}"
    );
    assert!(text.contains("src/⟨U+202E⟩sr.esrap"), "{text}");
    // "Always" sends the rule as the server proposed it.
    app.update(key(KeyCode::Char('a')));
    let effects = app.update(key(KeyCode::Enter));
    assert_eq!(
        answer_of(&effects),
        Answer::AllowAlways {
            rules: vec!["edit_file(src/\u{202e}sr.esrap)".to_owned()]
        }
    );
}
