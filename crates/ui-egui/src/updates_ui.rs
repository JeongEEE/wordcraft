//! Software updates. The desktop app looks for a newer fork release (signed `WordUpdate.json` on
//! JeongEEE/wordcraft, through `wordcraft-update`) at start, from Help ▸ Check for Updates and
//! every six hours, downloads and verifies it in the background, and installs it only after an
//! explicit restart that first goes through the normal Save Changes prompts. The files open at
//! the restart open again afterwards. craft-update's own updater is not used: it would look at the
//! upstream releases and install with its own detection.

use crate::WordApp;
#[cfg(not(target_arch = "wasm32"))]
use craft_update::{Release, Stop, http::Transfer};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{
    Arc,
    mpsc::{Receiver, TryRecvError, channel},
};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
#[cfg(not(target_arch = "wasm32"))]
use wordcraft_update::{download::Prepared, target::Target};

#[cfg(not(target_arch = "wasm32"))]
mod view;

#[cfg(not(target_arch = "wasm32"))]
const VERSION: &str = env!("CARGO_PKG_VERSION");
#[cfg(not(target_arch = "wasm32"))]
const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
enum Phase {
    #[default]
    Idle,
    Checking,
    Current,
    Downloading {
        done: u64,
        total: u64,
        reconnecting: bool,
    },
    Ready(String),
    /// A newer release this copy can't install itself (no supported install found).
    Manual {
        version: String,
        reason: String,
    },
    Failed(String),
}

#[cfg(not(target_arch = "wasm32"))]
enum Message {
    Progress(Transfer),
    Manual(String, String),
    Done(Result<Option<Prepared>, String>),
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
pub(crate) struct Runtime {
    /// The desktop app turns the check at start on; tests leave it off.
    startup: bool,
    /// Help ▸ Check for Updates asked, to be started on the next frame.
    manual: bool,
    /// The first check ran (and read the last update's outcome).
    started: bool,
    /// The update window or the restart prompt is shown (else only the corner card).
    visible: bool,
    /// The running check was asked for by hand: say so when Word is current.
    requested: bool,
    next_check: Option<Instant>,
    pending: Option<Receiver<Message>>,
    stop: Arc<Stop>,
    phase: Phase,
    ready: Option<Prepared>,
    /// The version an update just installed and when its notice first showed.
    completed: Option<(String, Option<f64>)>,
    /// Why the last update didn't install, until the window saying so is closed.
    install_failed: Option<String>,
    /// [`WordApp::disable_self_update`]'s reason: no check runs, Help says why.
    refused: Option<String>,
    /// The restart prompt drew last frame: the document takes no keys ([`WordApp::modal_open`]).
    pub(crate) modal: bool,
    /// Restart to Update was chosen and the quit is under way.
    restart_requested: bool,
    /// The documents to open again, by document id, the active one last; `None` for one with no file.
    reopen: Vec<(u64, Option<String>)>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for Runtime {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Default)]
pub(crate) struct Runtime {
    pub(crate) manual_window_open: bool,
    pub(crate) status_message: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
impl WordApp {
    /// Whether the app asks for a newer release when it starts (off until the desktop app turns it on).
    pub fn set_startup_update_check(&mut self, on: bool) {
        self.updates_rt.startup = on;
    }

    /// Turns the self-update off for the rest of this process: no update is offered, and Help ▸
    /// Check for Updates says why. For a process that still carries another program's AppImage
    /// variables, which the update would take for this copy's file and replace (#5653).
    pub fn disable_self_update(&mut self) {
        let rt = &mut self.updates_rt;
        rt.startup = false;
        rt.refused = Some(
            tl!("Updates are off in this window: WordCraft was started with another program's AppImage settings. Start WordCraft from its own menu entry to update it.")
                .to_string(),
        );
    }

    /// Starts the helper of an update that is ready, once Restart to Update has been chosen and
    /// every Save Changes prompt answered, so that it installs once the process has ended. Call it
    /// once the exit is certain, before a path that ends the process without dropping the app. A
    /// normal quit installs nothing, and later calls do nothing.
    pub fn start_pending_update(&mut self) {
        let asking = matches!(self.dialog, Some(crate::dialogs::Dialog::SaveChanges { .. })) || self.file_dialog_open();
        if !std::mem::take(&mut self.updates_rt.restart_requested) || asking {
            self.updates_rt.reopen.clear();
            return;
        }
        let mut seen = std::collections::BTreeSet::new();
        let reopen =
            std::mem::take(&mut self.updates_rt.reopen).into_iter().filter_map(|(_, path)| path).filter(|path| seen.insert(path.clone())).collect();
        if let Some(ready) = self.updates_rt.ready.take()
            && let Err(reason) = ready.launch(reopen)
        {
            log::error!("Word update helper: {reason}");
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn check_and_download(stop: &Stop, send: &dyn Fn(Message)) -> Result<Option<Prepared>, String> {
    let http = craft_update::Http::new("Word/Update")?;
    let Some(release) = craft_update::check_for_update(&crate::UPDATE_APP, &http, VERSION)? else { return Ok(None) };
    let target = match Target::current() {
        Ok(target) => target,
        Err(reason) => {
            send(Message::Manual(release.version_label(), reason));
            return Ok(None);
        }
    };
    send(Message::Progress(Transfer::Bytes { done: 0, total: 0 }));
    download(&release, target, stop, send).map(Some)
}

#[cfg(not(target_arch = "wasm32"))]
fn download(release: &Release, target: Target, stop: &Stop, send: &dyn Fn(Message)) -> Result<Prepared, String> {
    let mut last = wordcraft_update::instant().checked_sub(Duration::from_secs(1)).unwrap_or_else(wordcraft_update::instant);
    Prepared::download(release, target, stop, &mut |progress| {
        if last.elapsed() >= Duration::from_millis(100) {
            send(Message::Progress(progress));
            last = wordcraft_update::instant();
        }
    })
}

/// Starts a check (and the download of what it finds) on a thread of its own. The corner card
/// it shows ends with the thread: craft-update's requests have timeouts, a panic is caught, and
/// a thread that goes away without an answer counts as a failure ([`tick`]).
#[cfg(not(target_arch = "wasm32"))]
fn begin(rt: &mut Runtime, ctx: &egui::Context, manual: bool) {
    rt.visible |= manual;
    if let Some(why) = &rt.refused {
        rt.phase = Phase::Failed(why.clone());
        return;
    }
    if rt.pending.is_some() {
        rt.requested |= manual;
        return;
    }
    if rt.ready.is_some() {
        return;
    }
    rt.requested = manual;
    rt.phase = Phase::Checking;
    rt.next_check = Some(wordcraft_update::instant() + CHECK_INTERVAL);
    rt.stop = Arc::new(Stop::default());
    let stop = rt.stop.clone();
    let (tx, rx) = channel();
    let repaint = ctx.clone();
    let spawned = std::thread::Builder::new().name("word-auto-update".into()).spawn(move || {
        let send = |message| {
            let _ = tx.send(message);
            repaint.request_repaint();
        };
        let answer = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check_and_download(&stop, &send)))
            .unwrap_or_else(|_| Err("Word's update worker stopped unexpectedly.".into()));
        send(Message::Done(answer));
    });
    match spawned {
        Ok(_) => rt.pending = Some(rx),
        Err(e) => {
            rt.phase = Phase::Failed(e.to_string());
            rt.visible = true;
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn receive(rt: &mut Runtime, message: Message) {
    match message {
        Message::Progress(Transfer::Bytes { done, total }) => {
            rt.phase = Phase::Downloading { done, total, reconnecting: false };
        }
        Message::Progress(Transfer::Reconnecting { done, total }) => {
            rt.phase = Phase::Downloading { done, total, reconnecting: true };
        }
        Message::Manual(version, reason) => {
            rt.phase = Phase::Manual { version, reason };
            rt.visible = true;
        }
        Message::Done(result) => {
            rt.pending = None;
            match result {
                Ok(Some(ready)) => {
                    rt.phase = Phase::Ready(ready.version.clone());
                    rt.ready = Some(ready);
                    rt.visible = true;
                }
                Ok(None) if matches!(rt.phase, Phase::Manual { .. }) => {}
                Ok(None) => rt.phase = if rt.requested { Phase::Current } else { Phase::Idle },
                Err(reason) => {
                    log::warn!("Word auto-update: {reason}");
                    rt.phase = Phase::Failed(reason);
                    rt.visible = true;
                }
            }
        }
    }
}

/// A quit is under way: allowed, or a Save Changes prompt of the quit (or the Save As it led
/// to) still waiting for an answer.
#[cfg(not(target_arch = "wasm32"))]
fn quit_is_pending(app: &WordApp) -> bool {
    let quit_prompt = matches!(&app.dialog, Some(crate::dialogs::Dialog::SaveChanges { then, params, .. })
        if then == "file.close" && params.get("quit").and_then(serde_json::Value::as_bool) == Some(true));
    app.quit_requested || quit_prompt || app.file_dialog_open()
}

/// What the last update came to, read once at the first check: a success shows a short notice,
/// a failure a window with the reason.
#[cfg(not(target_arch = "wasm32"))]
fn outcome_notice(rt: &mut Runtime, outcome: wordcraft_update::helper::Outcome) {
    if outcome.success {
        rt.completed = Some((outcome.version, None));
    } else {
        rt.install_failed = Some(outcome.message);
    }
}

pub(crate) fn check(app: &mut WordApp) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        app.updates_rt.manual = true;
        if let Some(ctx) = &app.ctx {
            ctx.request_repaint();
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        app.updates_rt.manual_window_open = true;
        app.updates_rt.status_message = Some(tl!("Update check is not supported in the web browser.").to_string());
    }
}

pub(crate) fn tick(app: &mut WordApp, ctx: &egui::Context) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Cancel in a Save Changes prompt (or its Save As) ends the quit: the update waits again.
        if app.updates_rt.restart_requested && !quit_is_pending(app) {
            cancel_restart(app);
        }
        let rt = &mut app.updates_rt;
        let manual = std::mem::take(&mut rt.manual);
        let startup = std::mem::take(&mut rt.startup);
        let periodic = rt.next_check.is_some_and(|at| wordcraft_update::instant() >= at) && rt.ready.is_none() && rt.pending.is_none();
        if manual || (startup && !rt.started) || periodic {
            if !rt.started {
                if let Some(outcome) = wordcraft_update::helper::take_outcome() {
                    outcome_notice(rt, outcome);
                }
                rt.started = true;
            }
            begin(rt, ctx, manual);
        }
        while let Some(rx) = &rt.pending {
            match rx.try_recv() {
                Ok(message) => receive(rt, message),
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint_after(Duration::from_millis(200));
                    break;
                }
                Err(TryRecvError::Disconnected) => {
                    receive(rt, Message::Done(Err("Word's update worker disconnected.".into())));
                    break;
                }
            }
        }
        if let Some(at) = rt.next_check {
            ctx.request_repaint_after(at.saturating_duration_since(wordcraft_update::instant()));
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (app, ctx);
}

#[cfg(not(target_arch = "wasm32"))]
fn cancel_restart(app: &mut WordApp) {
    app.updates_rt.restart_requested = false;
    app.updates_rt.reopen.clear();
}

/// The file a document opens again from after the restart: its own, made absolute. A document
/// never saved has none. No password is kept: an encrypted file asks for it again.
#[cfg(not(target_arch = "wasm32"))]
fn reopen_path(path: Option<&std::path::Path>) -> Option<String> {
    let path = path?;
    let path = if path.is_absolute() { path.to_path_buf() } else { std::env::current_dir().ok()?.join(path) };
    path.to_str().map(str::to_string)
}

/// A Save Changes answer of the restart's quit is closing the shown document: remember the file
/// it was saved to (Save As gives a new document one) before it goes.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn remember_restart_document(app: &mut WordApp) {
    if !app.updates_rt.restart_requested {
        return;
    }
    let id = app.session.document_id();
    let path = reopen_path(app.session.path.as_deref());
    if let Some((_, remembered)) = app.updates_rt.reopen.iter_mut().find(|(doc, _)| *doc == id) {
        *remembered = path;
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn request_restart(app: &mut WordApp) {
    if app.updates_rt.ready.is_none() {
        return;
    }
    begin_restart(app);
}

/// Quit as the window's close button does, asking about each document with unsaved work, and
/// remember the open files for the restart.
#[cfg(not(target_arch = "wasm32"))]
fn begin_restart(app: &mut WordApp) {
    let mut reopen: Vec<_> = app.session.documents().into_iter().map(|d| (d.id, d.active, reopen_path(d.path.as_deref()))).collect();
    // Files open in turn, the last one shown, as the prompts may leave another document shown.
    reopen.sort_by_key(|(_, active, _)| *active);
    app.updates_rt.visible = false;
    app.updates_rt.restart_requested = true;
    app.updates_rt.reopen = reopen.into_iter().map(|(id, _, path)| (id, path)).collect();
    let _ = app.ask_before_quit();
    if !quit_is_pending(app) {
        cancel_restart(app);
    }
}

/// The restart prompt waits while something in progress would sit under it: a dialog, a file
/// dialog, a Colors dialog, an input method's composition, an unwritten comment or document
/// property, or a quit.
#[cfg(not(target_arch = "wasm32"))]
fn prompt_allowed(app: &WordApp) -> bool {
    app.dialog.is_none()
        && app.colors.is_none()
        && !app.file_dialog_open()
        && app.canvas.ime_preedit.is_empty()
        && !crate::panes::has_comment_drafts(app)
        && app.info_editing.is_none()
        && !quit_is_pending(app)
}

pub(crate) fn show(app: &mut WordApp, ctx: &egui::Context) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let allowed = prompt_allowed(app);
        let response = view::show(&mut app.updates_rt, ctx, allowed);
        app.updates_rt.modal = response.modal;
        if response.modal {
            // The page, the ribbon and the shortcuts read raw events: after the prompt has had
            // them, none goes on to edit the document under it.
            ctx.input_mut(|i| {
                i.events.retain(|e| {
                    !matches!(
                        e,
                        egui::Event::Key { .. }
                            | egui::Event::Text(_)
                            | egui::Event::Ime(_)
                            | egui::Event::Copy
                            | egui::Event::Cut
                            | egui::Event::Paste(_)
                            | egui::Event::PasteImage(_)
                            | egui::Event::MiddleClickPaste { .. }
                    )
                });
            });
        }
        if response.releases {
            ctx.open_url(egui::OpenUrl::new_tab(crate::UPDATE_APP.releases_url()));
        }
        if response.restart {
            request_restart(app);
        }
    }

    #[cfg(target_arch = "wasm32")]
    if app.updates_rt.manual_window_open {
        let mut open = true;
        egui::Window::new(tl!("Check for Updates"))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                crate::rtl::root(ui, |ui| {
                    if let Some(msg) = &app.updates_rt.status_message {
                        ui.label(msg);
                    }
                })
            });
        if !open {
            app.updates_rt.manual_window_open = false;
            app.updates_rt.status_message = None;
        }
    }
}

#[cfg(test)]
mod tests;
