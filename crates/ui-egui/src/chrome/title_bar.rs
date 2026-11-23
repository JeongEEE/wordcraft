//! The title bar: AutoSave, the Quick Access Toolbar, the document title, search, community and account.

use std::cell::RefCell;

use craft::behavior::IconId;
use craft::keymap::CommandRef;
use craft::menu::RibbonMenuRequest;
use craft::theme::ControlSize;
use craft::ui::buttons::{Button, IconButton};
use craft::ui::choice::Switch;
use craft::ui::contract::{Bind, BodyCx, Props as _, Show as _, ShowCollection as _, ShowContainer as _, TooltipSpec, Trigger};
use craft::ui::display::{Divider, Text};
use craft::ui::list::{UndoList, UndoMark, UndoRows, UndoWords, footer_text, undo_count};
use craft::ui::menu::{Menu, MenuItem};
use craft::ui::popover::Popover;
use craft::ui::primitives::IconSpec;
use craft::ui::runtime::env::Env;
use craft::ui::runtime::inspect;
use craft::ui::runtime::windows::{WindowAction, WindowHost};
use craft::ui::shell::{Title, TitleBar};
use craft::ui::style::GlyphRole;
use egui::Ui;
use serde_json::json;

use super::{QAT_MENU, qat_enabled, undo_list_rows};
use crate::WordApp;

/// Fills the shell's title slot: the title bar, and what its title and Close report.
pub(crate) fn show(ui: &mut Ui, _cx: &mut BodyCx<'_>, app: &mut WordApp) {
    let title = if app.has_document() {
        let state = if app.session.dirty {
            tl!("Edited")
        } else if app.session.path.is_some() {
            tl!("Saved")
        } else {
            tl!("Not saved")
        };
        Title::new(format!("{} — {state}", craft::i18n::file_name(&app.title_stem(), super::interface_direction())))
            .action(tl!("Rename Document").to_string())
    } else {
        // No document open (#1129): the app's name alone.
        Title::new("Word")
    };
    // The controls at the end give way in a narrow window (#5886): the step that fits the room the leading controls
    // left in the last pass.
    let step = app.chrome.title_fit.step();
    // The bar runs the leading closure and then the trailing one, never one inside the other
    // (`TitleBar::show_titled`), so each borrow below succeeds.
    let app = RefCell::new(app);
    let (lead, trail) = (std::cell::Cell::new(None), std::cell::Cell::new(None));
    // The bar always names the document, under the OS's own title bar too (prod `chrome.rs:425-471`).
    let out = TitleBar::new().title_when_native().show_titled(
        ui,
        title,
        |ui, _| {
            if let Ok(mut a) = app.try_borrow_mut() {
                lead.set(leading(&mut a, ui));
            }
        },
        |ui, _| {
            if let Ok(mut a) = app.try_borrow_mut() {
                trail.set(trailing(&mut a, ui, step));
            }
        },
    );
    // Word fork: where AutoSave and the toolbar sat this pass, for `arrow_over_title_controls`.
    ui.ctx().data_mut(|d| d.insert_temp(leading_controls_id(), lead.get()));
    if let (Some(l), Some(t), Ok(mut a)) = (lead.get(), trail.get(), app.try_borrow_mut()) {
        // The end region sits at the bar's end edge whatever its width, so its outer edge less the leading controls'
        // is the room it has, less a gap on either side of the title's place between them.
        let span = if super::interface_direction().is_rtl() { l.min.x - t.min.x } else { t.max.x - l.max.x };
        let room = span - TITLE_FIT_GAP;
        if a.chrome.title_fit.note(step, t.width(), room) {
            ui.ctx().request_repaint();
        }
    }
    if out.title_activated
        && let Ok(mut a) = app.try_borrow_mut()
    {
        let _ = a.run("file.rename", json!({}));
    }
    if out.close_requested {
        // The bar never closes the window: the close goes to the OS as its own close button's does, so it passes
        // the unsaved-documents guard.
        let ctx = ui.ctx().clone();
        let _ = WindowHost::of(&ctx).act(ctx.viewport_id(), WindowAction::Close);
    }
}

/// Where the title bar's leading controls (AutoSave, the Quick Access Toolbar) sat in the last pass that drew the bar.
fn leading_controls_id() -> egui::Id {
    egui::Id::unique("wordcraft.title_bar.leading_controls")
}

/// Word fork: the pointer over the macOS traffic lights, and over AutoSave and the toolbar beside them, is the arrow, as
/// AppKit shows it over its own buttons and a Mac title bar's controls. They sit over the bar's move band, whose grab
/// cursor would otherwise show on them. Called after the window is drawn, so this request comes after the band's and
/// wins the tie. Elsewhere the OS draws the frame and its buttons, and the bar has no move band.
pub(crate) fn arrow_over_title_controls(ctx: &egui::Context) {
    use craft::behavior::{Cursor, CursorIcon, CursorPriority};
    // Taken, so a pass that doesn't draw the bar leaves no stale rect behind.
    let controls = ctx.data_mut(|d| d.remove_temp::<Option<egui::Rect>>(leading_controls_id())).flatten();
    let craft::window::policy::Frame::Inset(inset) = WindowHost::of(ctx).frame() else {
        return;
    };
    // The inset is in window points; the interface zoom scales egui's points against them.
    let lights = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(inset.leading, 2.0 * inset.centre_from_top) / ctx.zoom_factor());
    if ctx.pointer_hover_pos().is_some_and(|at| lights.contains(at) || controls.is_some_and(|c| c.contains(at))) {
        craft::ui::runtime::cursor::request(ctx, Cursor::System(CursorIcon::Default), CursorPriority::Part);
    }
}

/// The room kept between the leading controls and the ones at the end, beyond what they take: the bar's gaps on
/// either side of the title's place.
const TITLE_FIT_GAP: f32 = 16.0;

/// AutoSave and the Quick Access Toolbar, at the start of the bar; returns the rect they took.
fn leading(app: &mut WordApp, ui: &mut Ui) -> Option<egui::Rect> {
    let switch = autosave(app, ui);
    // Hidden (#1945): neither the buttons nor the … menu; the title centres as before.
    let toolbar = if app.ui.quick_access_toolbar { quick_access(app, ui) } else { None };
    super::union([Some(switch), toolbar].into_iter().flatten())
}

/// The AutoSave switch: off and with the reason as its tooltip when AutoSave can't cover the document (#196,
/// #176); a click then offers Save As where that helps. Returns the rect it took.
fn autosave(app: &mut WordApp, ui: &mut Ui) -> egui::Rect {
    let block = app.autosave_block();
    let on = app.autosaves() && block.is_none();
    let tip = block.as_ref().map_or_else(|| tl!("AutoSave saves every change to the file (needs a saved document)").to_string(), |b| b.reason());
    // The small switch, as the old title bar drew it. Word fork: centred on the bar's height, as the toolbar's icons and
    // the macOS traffic lights are. The bar's row sits a labelled control on its line from the top, so the switch is
    // drawn in a band moved down by half the room it leaves in the bar (`size.bar.title`; the room the row offers can be
    // taller), from the height it took in the last pass.
    let room = ui.available_rect_before_wrap();
    let band = room.with_max_y(room.min.y + room.height().min(Env::of(ui).tokens.size.bar.title));
    let height_id = ui.scope_id().with("wordcraft.autosave.height");
    let top = ui.data(|d| d.get_temp::<f32>(height_id)).map_or(0.0, |h| ((band.height() - h) / 2.0).max(0.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(band.with_min_y(band.min.y + top)).layout(*ui.layout()));
    let out = Switch::new(Bind::given(on)).label(tl!("AutoSave").to_string()).size(ControlSize::Sm).tooltip(TooltipSpec::new(tip)).show(&mut child);
    // The switch and its label; a measuring pass draws nothing and takes no height.
    let taken = child.min_rect().height();
    if taken > 0.0 && ui.data(|d| d.get_temp::<f32>(height_id)) != Some(taken) {
        ui.data_mut(|d| d.insert_temp(height_id, taken));
        ui.ctx().request_repaint();
    }
    ui.allocate_rect(egui::Rect::from_x_y_ranges(child.min_rect().x_range(), band.y_range()), egui::Sense::hover());
    if out.response.clicked() {
        match block {
            None => {
                let _ = app.run("file.autosave", json!({}));
            }
            // AutoSave comes on once Save As has saved (maybe on a later frame, #94).
            Some(b) if b.save_as_helps() => {
                let _ = app.save_as(crate::file_dialogs::AfterSave::AutoSave);
            }
            Some(b) => app.status(b.reason()),
        }
    }
    out.response.rect
}

/// The user's Quick Access Toolbar commands, then the menu button that customizes it (#376); returns the rect they
/// took.
fn quick_access(app: &mut WordApp, ui: &mut Ui) -> Option<egui::Rect> {
    let ids: Vec<String> = app.session.ribbon.qat().into_iter().map(str::to_string).collect();
    let ctx = ui.ctx().clone();
    let mut rects = Vec::new();
    // The buttons are an inspected region, so the keytips badge them.
    inspect::record(ui, crate::keytips::qat_region(), |ui| {
        for id in &ids {
            let enabled = qat_enabled(&ctx, app, id);
            // Tool glyphs: the old toolbar drew its icons at the standard icon side, not a small glyph.
            let mut button =
                IconButton::command(crate::icons::for_command(id), CommandRef::new(id.as_str())).glyphs(GlyphRole::Tool).enabled(enabled);
            if id == "edit.undo" {
                button = button.tooltip(TooltipSpec::new(crate::i18n::prefixed("Undo", app.session.undo_label().unwrap_or(""))));
            }
            let out = button.show(ui);
            rects.push(out.response.rect);
            // The ribbon's command menu: on or off the toolbar, and the customize rows.
            let request = RibbonMenuRequest::new(Some(CommandRef::new(id.as_str())), out.response.rect.left_bottom());
            Menu::context(tl!("Quick Access Toolbar").to_string(), &out.response).show(ui, |ui, _| crate::ribbon::command_menu(ui, app, &request));
            if id == "edit.undo" {
                // The list is the document's steps, not the Markdown Source pane's typing.
                rects.push(undo_arrow(app, ui, enabled && app.session.md_source.is_none()));
            }
        }
    });
    rects.push(qat_menu(app, ui));
    super::union(rects)
}

/// How many steps the undo list shows before it scrolls.
const UNDO_ROWS_SHOWN: u16 = 8;

/// The small arrow beside Undo: a list of steps to undo several at once, with a footer that names how many a pick
/// undoes, or Cancel while none is marked (#1903). Returns the arrow's rect.
fn undo_arrow(app: &mut WordApp, ui: &mut Ui, enabled: bool) -> egui::Rect {
    // The small filled triangle of the old toolbar.
    let out = IconButton::new(IconSpec::glyph(&crate::ribbon::common::MENU_ARROW)).label(tl!("Undo History").to_string()).enabled(enabled).show(ui);
    let labels = undo_list_rows(app);
    let mut open = app.chrome.undo;
    if out.response.clicked() {
        open = !open;
    }
    let mut mark = app.chrome.undo_mark;
    let mut picked = None;
    Popover::new(tl!("Undo History").to_string(), Bind::bound(&mut open)).opened_by(&out.response).show(ui, |ui, cx| {
        let rows = UndoRows::new(&labels);
        // A collection owns its scroll: the popover's body scrolls, so the list caps its rows and scrolls the rest.
        let events = UndoList::new(tl!("Undo History").to_string())
            .max_rows(UNDO_ROWS_SHOWN)
            .selection(Bind::bound(&mut mark))
            .show_items(ui, &rows, &mut |key, _| rows.row(*key))
            .events;
        if let Some(count) = undo_count(&events) {
            picked = Some(count);
            cx.request_close();
        }
        Divider::horizontal().show(ui);
        let env = Env::of(ui);
        let words = env.words::<UndoWords>();
        match footer_text(&words, env.format(), &mark) {
            Some(text) => {
                Text::new(text).show(ui);
            }
            None => {
                if Button::new(words.cancel.clone()).show(ui).clicked() {
                    cx.request_close();
                }
            }
        }
    });
    app.chrome.undo = open;
    app.chrome.undo_mark = if open { mark } else { UndoMark::default() };
    if let Some(count) = picked {
        let _ = app.run("edit.undoTo", json!({"count": count}));
    }
    out.response.rect
}

/// The menu button (…): common commands to show or hide, More Commands…, and Hide. Returns the button's rect.
fn qat_menu(app: &mut WordApp, ui: &mut Ui) -> egui::Rect {
    let out =
        IconButton::new(IconId::new("wordcraft.more")).label(tl!("Customize Quick Access Toolbar").to_string()).glyphs(GlyphRole::Tool).show(ui);
    let shown: Vec<(&'static str, String, bool)> = QAT_MENU
        .iter()
        .filter_map(|id| {
            let label = app.session.registry.get(id).map(|s| tl!(s.label).to_string())?;
            Some((*id, label, app.session.ribbon.qat().contains(id)))
        })
        .collect();
    // No `open` binding: with one the framework leaves the opening to the app, without it the opener opens the menu.
    Menu::new(tl!("Customize Quick Access Toolbar").to_string()).opened_by(&out.response, Trigger::Click).show(ui, |ui, _| {
        MenuItem::heading(tl!("Customize Quick Access Toolbar").to_string()).show(ui);
        for (id, label, on) in &shown {
            let key = if *on { "qatRemove" } else { "qatAdd" };
            MenuItem::command(CommandRef::with_params("tools.customizeRibbon", json!({ key: *id }))).label(label.clone()).check(*on).show(ui);
        }
        Divider::horizontal().show(ui);
        MenuItem::command(CommandRef::with_params("ui.dialog", json!({"name": "customizeQuickAccess"})))
            .label(tl!("More Commands…").to_string())
            .show(ui);
        // Last, as in Word (#1945); the ribbon's right-click menu brings the toolbar back.
        Divider::horizontal().show(ui);
        MenuItem::command(CommandRef::with_params("ui.quickAccessToolbar", json!({"value": false})))
            .label(tl!("Hide Quick Access Toolbar").to_string())
            .show(ui);
    });
    out.response.rect
}

/// Search at the end of the bar; returns the rect it took. In a narrow window it gives way by `step` (#5886, as the old
/// bar's fit): at 2 the search shows its magnifier alone. Word fork: no community (Discord) button and no account
/// circle here; the name and initials are set in File › Options › General.
fn trailing(app: &mut WordApp, ui: &mut Ui, step: usize) -> Option<egui::Rect> {
    let rects = vec![super::search::show(app, ui, step >= 2)];
    super::union(rects)
}
