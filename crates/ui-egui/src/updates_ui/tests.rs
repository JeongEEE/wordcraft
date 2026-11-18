//! The update UI: restart through the Save Changes prompts, the files it opens again, and what
//! the card, prompt and notices show. No test checks the network: a check is started only with
//! the self-update refused, and the worker's messages are fed by hand.
#![cfg(not(target_arch = "wasm32"))]

use super::{Message, Phase, Runtime, begin, begin_restart, fail, outcome_notice, receive, tick};
use crate::{Services, WordApp};
use craft_update::http::Transfer;
use egui_kittest::kittest::Queryable;
use serde_json::json;
use std::sync::mpsc::channel;
use wordcraft_engine::Session;

/// A folder of its own per test, removed afterwards.
struct Folder(std::path::PathBuf);

impl Folder {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("word-update-reopen-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn path(&self, name: &str) -> String {
        self.0.join(name).to_str().unwrap().to_string()
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn app() -> WordApp {
    WordApp::new(Session::new(wordcraft_doc::Document::new()), Services::default())
}

fn body(a: &WordApp) -> String {
    a.session.doc.plain_text(wordcraft_doc::StoryRef::Body)
}

/// A document saved at `path`, with no changes since.
fn saved_app(path: &str) -> WordApp {
    let mut a = app();
    a.run("text.insert", json!({"text": "보존할 내용"})).unwrap();
    a.run("file.save", json!({"path": path})).unwrap();
    assert!(!a.session.dirty);
    a
}

fn remembered(a: &WordApp) -> Vec<&str> {
    a.updates_rt.reopen.iter().filter_map(|(_, p)| p.as_deref()).collect()
}

fn answer(a: &mut WordApp, answer: &str) {
    a.run("ui.saveChanges", json!({"answer": answer})).unwrap();
}

fn ready(a: &mut WordApp) {
    a.updates_rt.phase = Phase::Ready("0.18.9".into());
    a.updates_rt.visible = true;
}

/// Only the update UI, drawn alone, with the app's fonts (Hangul shows as boxes without the CJK faces).
fn update_harness(a: WordApp) -> egui_kittest::Harness<'static, WordApp> {
    let h = egui_kittest::Harness::builder()
        .with_size(egui::vec2(800.0, 480.0))
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx| {
            ctx.set_fonts(crate::theme::font_definitions(false, false));
        })
        .build_ui_state(|ui, a| super::show(a, ui.ctx()), a);
    h.ctx.set_missing_glyph_policy(egui::MissingGlyphPolicy::Tofu);
    h
}

/// The whole window, as the app draws it.
fn app_harness(a: WordApp) -> egui_kittest::Harness<'static, WordApp> {
    egui_kittest::Harness::builder().with_size(egui::vec2(1440.0, 900.0)).with_step_dt(1.0 / 60.0).build_ui_state(
        |ui, a: &mut WordApp| {
            let ctx = ui.ctx().clone();
            a.logic(&ctx);
            a.ui(ui);
        },
        a,
    )
}

#[test]
fn restart_remembers_every_file_and_opens_the_active_one_last() {
    let folder = Folder::new();
    let first = folder.path("한글 보고서.docx");
    let second = folder.path("두 번째.docx");
    let mut a = saved_app(&first);
    let first_id = a.session.document_id();
    a.run("file.new", json!({})).unwrap();
    a.run("file.save", json!({"path": second})).unwrap();
    a.show_document(first_id).unwrap();
    begin_restart(&mut a);
    assert!(a.quit_requested, "nothing unsaved: the app quits at once");
    assert!(a.updates_rt.restart_requested);
    assert_eq!(remembered(&a), vec![second.as_str(), first.as_str()]);
}

#[test]
fn saving_during_the_restart_keeps_the_file_to_open_again() {
    let folder = Folder::new();
    let path = folder.path("보고서.docx");
    let mut a = saved_app(&path);
    a.run("text.insert", json!({"text": "수정한 내용 "})).unwrap();
    begin_restart(&mut a);
    assert!(!a.quit_requested, "the unsaved change is asked about first");
    assert!(matches!(a.dialog, Some(crate::dialogs::Dialog::SaveChanges { .. })));
    answer(&mut a, "save");
    assert!(a.quit_requested);
    assert!(a.updates_rt.restart_requested, "the restart goes on");
    assert_eq!(remembered(&a), vec![path.as_str()]);
    let on_disk = wordcraft_engine::io::open_path(std::path::Path::new(&path)).unwrap().plain_text(wordcraft_doc::StoryRef::Body);
    assert!(on_disk.contains("수정한 내용"), "{on_disk:?}");
}

#[test]
fn saving_a_new_document_reopens_it_from_where_save_as_put_it() {
    let folder = Folder::new();
    let path = folder.path("새 보고서.docx");
    let picked = path.clone();
    let mut a = WordApp::new(
        Session::new(wordcraft_doc::Document::new()),
        Services { pick_save: Some(Box::new(move |_| Some(picked.clone()))), ..Default::default() },
    );
    a.run("text.insert", json!({"text": "42"})).unwrap();
    begin_restart(&mut a);
    assert_eq!(remembered(&a), Vec::<&str>::new(), "a new document has no file yet");
    answer(&mut a, "save");
    assert!(a.quit_requested);
    assert_eq!(remembered(&a), vec![path.as_str()]);
    assert!(std::path::Path::new(&path).is_file());
}

#[test]
fn dont_save_reopens_only_files_and_cancel_ends_the_restart() {
    let folder = Folder::new();
    let path = folder.path("원본.docx");
    let mut a = saved_app(&path);
    a.run("text.insert", json!({"text": "버릴 변경 "})).unwrap();
    a.run("file.new", json!({})).unwrap();
    a.run("text.insert", json!({"text": "저장 안 할 새 문서"})).unwrap();
    begin_restart(&mut a);
    answer(&mut a, "dontSave");
    answer(&mut a, "dontSave");
    assert!(a.quit_requested);
    assert_eq!(remembered(&a), vec![path.as_str()], "the unsaved new document isn't reopened");

    let mut a = saved_app(&path);
    a.run("text.insert", json!({"text": "작업 유지 "})).unwrap();
    begin_restart(&mut a);
    assert_eq!(remembered(&a), vec![path.as_str()]);
    answer(&mut a, "cancel");
    tick(&mut a, &egui::Context::default());
    assert!(!a.updates_rt.restart_requested, "Cancel ends the restart");
    assert!(remembered(&a).is_empty());
    assert!(!a.quit_requested);
    assert!(a.session.dirty && body(&a).contains("작업 유지"), "the work stays");
    // A later normal quit installs nothing.
    a.start_pending_update();
    assert!(!a.updates_rt.restart_requested);
}

#[test]
fn a_quit_abandoned_without_a_prompt_ends_the_restart() {
    let mut a = app();
    a.updates_rt.restart_requested = true;
    a.updates_rt.reopen = vec![(1, Some("/tmp/a.docx".into()))];
    tick(&mut a, &egui::Context::default());
    assert!(!a.updates_rt.restart_requested);
    assert!(a.updates_rt.reopen.is_empty());
}

#[test]
fn the_ready_prompt_can_be_put_off_without_quitting_or_changing_the_document() {
    let mut a = app();
    a.run("text.insert", json!({"text": "keep me"})).unwrap();
    ready(&mut a);
    let mut h = update_harness(a);
    h.run_steps(3);
    assert!(h.query_by_label("Restart to Update").is_some());
    h.get_by_label("Later").click();
    h.run_steps(3);
    assert!(!h.state().updates_rt.visible);
    assert!(!h.state().updates_rt.restart_requested);
    assert!(!h.state().quit_requested);
    assert!(h.state().session.dirty);
    h.get_by_label("Update ready").click();
    h.run_steps(2);
    assert!(h.query_by_label("Restart to Update").is_some(), "the corner button brings it back");
    assert!(!h.state().updates_rt.restart_requested);
}

/// In Korean the prompt reads from the fork's own table.
#[test]
fn the_ready_prompt_is_korean_in_a_korean_interface() {
    let mut a = app();
    ready(&mut a);
    let mut h = update_harness(a);
    crate::i18n::set_current(crate::i18n::fixture_lang("ko", "\tHome\t홈\n"));
    h.run_steps(3);
    let korean = h.query_by_label("재시작하여 업데이트").is_some() && h.query_by_label("나중에").is_some();
    crate::i18n::set_current(crate::i18n::Lang::EN);
    assert!(korean);
}

#[test]
fn a_background_download_shows_progress_without_opening_a_window() {
    let mut a = app();
    receive(&mut a.updates_rt, Message::Progress(Transfer::Bytes { done: 25, total: 100 }));
    let mut h = update_harness(a);
    h.run_steps(3);
    assert!(h.query_by_label("Downloading the update").is_some());
    assert!(h.query_by_label("25%").is_some());
    assert!(h.query_by_label("Word Update").is_none());
    assert!(!h.state().updates_rt.visible);
}

#[test]
fn an_unknown_size_and_a_reconnect_claim_neither_zero_nor_done() {
    let mut a = app();
    receive(&mut a.updates_rt, Message::Progress(Transfer::Bytes { done: 0, total: 0 }));
    let mut h = update_harness(a);
    h.run_steps(3);
    assert!(h.query_by_label("Preparing the update download").is_some());
    assert!(h.query_by_label("0%").is_none());
    receive(&mut h.state_mut().updates_rt, Message::Progress(Transfer::Reconnecting { done: 25, total: 100 }));
    h.run_steps(2);
    assert!(h.query_by_label("Trying to connect again").is_some());
    receive(&mut h.state_mut().updates_rt, Message::Progress(Transfer::Bytes { done: 99_999, total: 100_000 }));
    h.run_steps(2);
    assert!(h.query_by_label("99%").is_some());
    receive(&mut h.state_mut().updates_rt, Message::Progress(Transfer::Bytes { done: 100, total: 100 }));
    h.run_steps(2);
    assert!(h.query_by_label("Verifying the update").is_some());
    assert!(h.query_by_label("100%").is_none());
}

/// The card ends with the check: a worker that answers nothing, or goes away, ends it too.
#[test]
fn the_card_goes_when_the_check_ends_or_its_worker_goes_away() {
    let mut a = app();
    let (tx, rx) = channel();
    a.updates_rt.pending = Some(rx);
    a.updates_rt.phase = Phase::Checking;
    tx.send(Message::Done(Ok(None))).unwrap();
    tick(&mut a, &egui::Context::default());
    assert!(matches!(a.updates_rt.phase, Phase::Idle) && a.updates_rt.pending.is_none(), "a check at start that finds nothing shows nothing");

    let (tx, rx) = channel::<Message>();
    a.updates_rt.pending = Some(rx);
    a.updates_rt.phase = Phase::Checking;
    drop(tx);
    tick(&mut a, &egui::Context::default());
    assert!(matches!(a.updates_rt.phase, Phase::Idle) && a.updates_rt.pending.is_none());
    assert!(!a.updates_rt.visible, "a check nobody asked for fails quietly");

    let (tx, rx) = channel::<Message>();
    a.updates_rt.pending = Some(rx);
    a.updates_rt.phase = Phase::Checking;
    a.updates_rt.requested = true;
    drop(tx);
    tick(&mut a, &egui::Context::default());
    assert!(matches!(a.updates_rt.phase, Phase::Failed(_)) && a.updates_rt.pending.is_none());
    assert!(a.updates_rt.visible, "a check asked for by hand says why, in a window that closes");
}

#[test]
fn a_failed_start_of_the_worker_is_quiet_unless_asked_for() {
    let mut rt = Runtime::default();
    fail(&mut rt, "no thread".into());
    assert!(matches!(rt.phase, Phase::Idle) && !rt.visible);
    rt.requested = true;
    fail(&mut rt, "no thread".into());
    assert!(matches!(rt.phase, Phase::Failed(_)) && rt.visible);
}

#[test]
fn a_successful_install_shows_a_notice_that_goes_by_itself() {
    let mut a = app();
    outcome_notice(&mut a.updates_rt, wordcraft_update::helper::Outcome { success: true, version: "0.19.0".into(), message: String::new() });
    assert!(a.updates_rt.install_failed.is_none());
    let text = crate::i18n::fmt("Word was updated to {version}.", &[("version", "0.19.0")]);
    let mut h = update_harness(a);
    h.run_steps(3);
    assert!(h.query_by_label(&text).is_some());
    assert!(!h.state().updates_rt.modal, "no prompt");
    h.input_mut().time = Some(h.ctx.input(|i| i.time) + 3.0);
    h.step();
    assert!(h.query_by_label(&text).is_some());
    h.input_mut().time = Some(h.ctx.input(|i| i.time) + 1.1);
    h.step();
    assert!(h.query_by_label(&text).is_none());
    assert!(h.state().updates_rt.completed.is_none());
}

#[test]
fn a_failed_install_says_why_until_closed() {
    let mut a = app();
    outcome_notice(&mut a.updates_rt, wordcraft_update::helper::Outcome { success: false, version: "0.19.0".into(), message: "disk full".into() });
    assert!(a.updates_rt.completed.is_none());
    let text = crate::i18n::fmt("Couldn't install the update. {reason}", &[("reason", "disk full")]);
    let mut h = update_harness(a);
    h.run_steps(3);
    assert!(h.query_by_label(&text).is_some());
    h.get_by_label("Close").click();
    h.run_steps(2);
    assert!(h.query_by_label(&text).is_none());
}

/// The prompt doesn't cover work in progress: it waits, as the corner button, until it ends.
#[test]
fn the_restart_prompt_waits_for_a_dialog_or_a_composition() {
    for kind in ["dialog", "ime"] {
        let mut a = app();
        ready(&mut a);
        if kind == "dialog" {
            a.run("ui.dialog", json!({"name": "about"})).unwrap();
            assert!(a.dialog.is_some());
        } else {
            a.canvas.ime_preedit = "ㅎ".into();
        }
        let mut h = update_harness(a);
        h.run_steps(3);
        assert!(h.query_by_label("Restart to Update").is_none(), "{kind}: waits");
        assert!(h.query_by_label("Update ready").is_some(), "{kind}");
        h.state_mut().dialog = None;
        h.state_mut().canvas.ime_preedit.clear();
        h.run_steps(3);
        assert!(h.query_by_label("Restart to Update").is_some(), "{kind}: shown once it ends");
    }
}

#[test]
fn the_restart_prompt_keeps_keys_from_the_document_and_escape_only_puts_it_off() {
    let mut a = app();
    a.run("text.insert", json!({"text": "keep me"})).unwrap();
    ready(&mut a);
    let mut h = app_harness(a);
    h.run_steps(4);
    assert!(h.query_by_label("Restart to Update").is_some());
    assert!(h.state().modal_open());
    h.input_mut().events.extend([egui::Event::Text("oops".into()), egui::Event::Paste("oops".into())]);
    h.key_press(egui::Key::Backspace);
    h.key_press(egui::Key::Enter);
    h.run_steps(2);
    assert_eq!(body(h.state()).trim(), "keep me", "nothing reached the document");
    assert!(!h.state().quit_requested && !h.state().updates_rt.restart_requested, "Enter didn't restart");
    h.key_press(egui::Key::Escape);
    h.run_steps(3);
    assert!(!h.state().updates_rt.visible);
    assert!(h.query_by_label("Update ready").is_some());
    assert!(!h.state().quit_requested);
    assert!(!h.state().updates_rt.restart_requested);
    assert!(!h.state().modal_open());
}

#[test]
fn a_check_by_hand_during_the_one_at_start_still_says_word_is_current() {
    let mut rt = Runtime::default();
    let (_tx, rx) = channel();
    rt.pending = Some(rx);
    begin(&mut rt, &egui::Context::default(), true);
    receive(&mut rt, Message::Done(Ok(None)));
    assert!(rt.visible);
    assert!(matches!(rt.phase, Phase::Current));
}

/// With the self-update off, no check is made at start, and Help ▸ Check for Updates, asked for
/// while the ribbon draws, shows the reason from the next frame on (#5653).
#[test]
fn with_the_self_update_off_help_says_why() {
    let mut a = app();
    a.set_startup_update_check(true);
    a.disable_self_update();
    assert!(!a.updates_rt.startup, "no check at start");
    // The last update's outcome is the user's own file: this test leaves it alone.
    a.updates_rt.started = true;
    let mut h = update_harness(a);
    h.run_steps(2);
    assert!(h.query_by_label("Word Update").is_none());
    h.state_mut().run("ui.checkForUpdates", json!({})).unwrap();
    let ctx = h.ctx.clone();
    tick(h.state_mut(), &ctx);
    h.run_steps(2);
    assert!(h.state().updates_rt.pending.is_none(), "no check ran");
    assert!(matches!(&h.state().updates_rt.phase, Phase::Failed(why) if why.contains("another program's AppImage")));
    assert!(h.query_by_label("Word Update").is_some());
}

/// The prompt, the card and the notice draw over the real window in both themes.
/// `WORD_UPDATE_UI_SCREENSHOTS=<folder>` saves each as a PNG there to look at.
#[test]
fn the_update_feedback_draws_in_light_and_dark() {
    for theme in [crate::theme::Appearance::Light, crate::theme::Appearance::Dark] {
        let mut a = app();
        a.ui.theme = theme;
        a.run("text.insert", json!({"text": "Quarterly report"})).unwrap();
        ready(&mut a);
        let mut h = app_harness(a);
        h.run_steps(4);
        assert!(h.query_by_label("Restart to Update").is_some(), "{theme:?}");
        let Ok(dir) = std::env::var("WORD_UPDATE_UI_SCREENSHOTS") else { continue };
        let name = format!("{theme:?}").to_lowercase();
        h.render().unwrap().save(format!("{dir}/restart-{name}.png")).unwrap();
        h.state_mut().updates_rt.visible = false;
        receive(&mut h.state_mut().updates_rt, Message::Progress(Transfer::Bytes { done: 29_600_000, total: 80_000_000 }));
        h.run_steps(4);
        h.render().unwrap().save(format!("{dir}/progress-{name}.png")).unwrap();
        h.state_mut().updates_rt.phase = Phase::Idle;
        outcome_notice(
            &mut h.state_mut().updates_rt,
            wordcraft_update::helper::Outcome { success: true, version: "0.18.9".into(), message: String::new() },
        );
        h.run_steps(3);
        h.render().unwrap().save(format!("{dir}/completed-{name}.png")).unwrap();
    }
}
