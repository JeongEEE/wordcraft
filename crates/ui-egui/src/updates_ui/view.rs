//! Presentation only: a compact background status card, the restart prompt, the update window
//! and a short notice once an update has installed.
use super::{Phase, Runtime, VERSION};
use crate::i18n::fmt;
use crate::theme::{self, Tokens};
use egui::{Align, Align2, RichText, Sense, Stroke, vec2};
use std::time::Duration;

/// How long the "updated" notice stays, in seconds; it fades out over the last quarter second.
const NOTICE_SECONDS: f64 = 4.0;

#[derive(Default)]
pub(super) struct Response {
    pub restart: bool,
    pub releases: bool,
    pub modal: bool,
}

fn card_frame(ctx: &egui::Context) -> egui::Frame {
    let t = Tokens::get(ctx);
    egui::Frame::popup(&ctx.global_style()).fill(t.panel).stroke(Stroke::new(1.0, t.border)).corner_radius(12).inner_margin(16)
}

fn badge(ui: &mut egui::Ui, size: f32) {
    let t = Tokens::get(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    ui.painter().rect_filled(rect, 10.0, t.checked);
    let r = rect.shrink(size * 0.25);
    let tick = [egui::pos2(r.left(), r.center().y), egui::pos2(r.left() + r.width() * 0.4, r.bottom()), egui::pos2(r.right(), r.top())];
    ui.painter().add(egui::Shape::line(tick.to_vec(), Stroke::new(size * 0.09, t.accent)));
}

/// The cards' contents. The old `rtl::root` mirror went with the framework port; the cards keep the
/// interface's left-to-right order.
fn root<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    add(ui)
}

fn progress(ui: &mut egui::Ui, phase: &Phase) {
    let t = Tokens::get(ui.ctx());
    let (title, fraction, subtitle) = match phase {
        Phase::Checking => (tl!("Checking for updates"), None, tl!("Looking for a new version").to_string()),
        Phase::Downloading { reconnecting: true, .. } => (tl!("Downloading the update"), None, tl!("Trying to connect again").to_string()),
        Phase::Downloading { total: 0, .. } => (tl!("Preparing the update download"), None, tl!("You can keep working").to_string()),
        Phase::Downloading { done, total, .. } if done >= total => {
            (tl!("Verifying the update"), None, tl!("Checking the downloaded file").to_string())
        }
        Phase::Downloading { done, total, .. } => (
            tl!("Downloading the update"),
            Some((*done as f64 / *total as f64).clamp(0.0, 1.0) as f32),
            format!("{:.1} / {:.1} MB", *done as f64 / 1_000_000.0, *total as f64 / 1_000_000.0),
        ),
        _ => return,
    };
    ui.horizontal(|ui| {
        ui.add(egui::Spinner::new().size(16.0).color(t.accent));
        ui.label(RichText::new(title).strong().color(t.text));
        if let Some(fraction) = fraction {
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                // Never 100 % while the file is still being checked.
                ui.label(RichText::new(format!("{}%", (fraction * 100.0).min(99.0) as u32)).color(t.accent));
            });
        }
    });
    ui.add_space(6.0);
    ui.label(RichText::new(subtitle).small().color(t.text_dim));
    if let Some(fraction) = fraction {
        ui.add_space(8.0);
        ui.add(egui::ProgressBar::new(fraction).desired_height(4.0).fill(t.accent));
    }
}

/// Bottom right, above the status bar: the check or download under way, or the button that
/// brings the restart prompt back.
fn corner(rt: &mut Runtime, ctx: &egui::Context) {
    let ready = matches!(rt.phase, Phase::Ready(_));
    egui::Area::new(egui::Id::unique("word.update.status"))
        .anchor(Align2::RIGHT_BOTTOM, vec2(-18.0, -44.0))
        .order(egui::Order::Foreground)
        .sense(Sense::hover())
        .interactable(ready)
        .show(ctx, |ui| {
            card_frame(ctx).show(ui, |ui| {
                root(ui, |ui| {
                    if ready {
                        let t = Tokens::get(ctx);
                        ui.horizontal(|ui| {
                            badge(ui, 28.0);
                            if ui.button(RichText::new(tl!("Update ready")).color(t.text)).on_hover_text(tl!("Open the restart options")).clicked() {
                                rt.visible = true;
                            }
                        });
                    } else {
                        ui.set_width(270.0);
                        progress(ui, &rt.phase);
                    }
                })
            });
        });
}

/// The notice after a successful update, top right, for [`NOTICE_SECONDS`].
fn completed(rt: &mut Runtime, ctx: &egui::Context) {
    let Some((version, shown_at)) = &mut rt.completed else { return };
    let now = ctx.input(|i| i.time);
    let elapsed = now - *shown_at.get_or_insert(now);
    if elapsed >= NOTICE_SECONDS {
        rt.completed = None;
        return;
    }
    let text = fmt(tl!("Word was updated to {version}."), &[("version", version)]);
    let opacity = ((NOTICE_SECONDS - elapsed) / 0.25).clamp(0.0, 1.0) as f32;
    egui::Area::new(egui::Id::unique("word.update.completed"))
        .anchor(Align2::RIGHT_TOP, vec2(-18.0, 48.0))
        .order(egui::Order::Tooltip)
        .sense(Sense::hover())
        .interactable(false)
        .show(ctx, |ui| {
            ui.set_opacity(opacity);
            card_frame(ctx).show(ui, |ui| {
                ui.set_max_width(320.0);
                root(ui, |ui| {
                    ui.horizontal(|ui| {
                        badge(ui, 28.0);
                        ui.label(RichText::new(text).color(Tokens::get(ctx).text));
                    });
                })
            });
        });
    ctx.request_repaint_after(Duration::from_millis(if elapsed >= NOTICE_SECONDS - 0.25 { 16 } else { 100 }));
}

/// Restart to Update or Later. Esc and a click outside are Later: the corner button brings it back.
fn restart_prompt(rt: &mut Runtime, ctx: &egui::Context, version: &str, response: &mut Response) {
    let t = Tokens::get(ctx);
    let mut later = false;
    let modal = egui::Modal::new(egui::Id::unique("word.update.ready")).frame(card_frame(ctx).inner_margin(24)).show(ctx, |ui| {
        ui.set_width(376.0);
        root(ui, |ui| {
            ui.horizontal(|ui| {
                badge(ui, 48.0);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(tl!("Update ready to install")).font(theme::semibold(20.0)).color(t.text));
                    ui.label(RichText::new(format!("Word {VERSION}  →  {version}")).color(t.text_dim));
                });
            });
            ui.add_space(18.0);
            ui.label(RichText::new(tl!("The new version is ready.\nRestart Word now?")).color(t.text));
            ui.add_space(12.0);
            egui::Frame::NONE.fill(t.checked).corner_radius(8).inner_margin(12).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new(tl!("Documents with unsaved changes are asked about first,\nand the open files open again after the restart."))
                        .small()
                        .color(t.text),
                );
            });
            ui.add_space(20.0);
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                let restart = egui::Button::new(RichText::new(tl!("Restart to Update")).strong().color(t.on_accent)).fill(t.accent).corner_radius(7);
                response.restart = ui.add_sized(vec2(178.0, 36.0), restart).clicked();
                later = ui.add_sized(vec2(84.0, 36.0), egui::Button::new(tl!("Later")).corner_radius(7)).clicked();
            });
        })
    });
    response.modal = true;
    // Escape reaches the window as the framework's Cancel intent, not as a key event the modal would see.
    let cancel = craft::ui::runtime::keys::with_window_intents(ctx, false, |window| window.take(craft::ui::runtime::keys::Intent::Cancel));
    if later || cancel || modal.should_close() {
        rt.visible = false;
    }
}

/// Help ▸ Check for Updates, a release this copy can't install, or a failure.
fn update_window(rt: &mut Runtime, ctx: &egui::Context, response: &mut Response) {
    let mut open = true;
    egui::Window::new(tl!("Word Update")).id(egui::Id::unique("word.update.window")).collapsible(false).resizable(false).open(&mut open).show(
        ctx,
        |ui| {
            ui.set_max_width(440.0);
            root(ui, |ui| {
                match &rt.phase {
                    Phase::Idle => {
                        ui.label(tl!("You can check for updates."));
                    }
                    Phase::Checking | Phase::Downloading { .. } => progress(ui, &rt.phase),
                    Phase::Current => {
                        ui.label(fmt(tl!("Word {version} is up to date."), &[("version", VERSION)]));
                    }
                    Phase::Manual { version, reason } => {
                        ui.label(fmt(tl!("Word {version} is available."), &[("version", version)]));
                        ui.label(reason);
                        response.releases = ui.button(tl!("Get the installer from the releases page")).clicked();
                    }
                    Phase::Failed(reason) => {
                        ui.label(tl!("Couldn't prepare the update."));
                        ui.label(reason);
                        rt.manual |= ui.button(tl!("Try Again")).clicked();
                        response.releases = ui.button(tl!("Download from the releases page")).clicked();
                    }
                    Phase::Ready(_) => {}
                }
                if ui.button(tl!("Close")).clicked() {
                    rt.visible = false;
                }
            })
        },
    );
    rt.visible &= open;
}

/// Why the last update didn't install, until closed.
fn install_failed(rt: &mut Runtime, ctx: &egui::Context) {
    let Some(reason) = &rt.install_failed else { return };
    let mut open = true;
    let mut close = false;
    egui::Window::new(tl!("Word Update")).id(egui::Id::unique("word.update.failed")).collapsible(false).resizable(false).open(&mut open).show(
        ctx,
        |ui| {
            ui.set_max_width(440.0);
            root(ui, |ui| {
                ui.label(fmt(tl!("Couldn't install the update. {reason}"), &[("reason", reason)]));
                close = ui.button(tl!("Close")).clicked();
            })
        },
    );
    if close || !open {
        rt.install_failed = None;
    }
}

pub(super) fn show(rt: &mut Runtime, ctx: &egui::Context, prompt_allowed: bool) -> Response {
    completed(rt, ctx);
    install_failed(rt, ctx);
    let mut response = Response::default();
    if !rt.visible || !prompt_allowed {
        if matches!(rt.phase, Phase::Checking | Phase::Downloading { .. } | Phase::Ready(_)) {
            corner(rt, ctx);
        }
        return response;
    }
    if let Phase::Ready(version) = &rt.phase {
        let version = version.clone();
        restart_prompt(rt, ctx, &version, &mut response);
        return response;
    }
    update_window(rt, ctx, &mut response);
    response
}
