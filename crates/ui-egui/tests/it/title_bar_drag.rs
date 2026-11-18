//! macOS: dragging the empty title bar moves the window by positioning it. `StartDrag` did
//! nothing there, so the window could not be moved at all.
#![cfg(target_os = "macos")]

use egui::{Event, Modifiers, PointerButton, Pos2, ViewportCommand, ViewportId, pos2, vec2};
use wordcraft_engine::Session;
use wordcraft_ui_egui::{Services, WordApp};

type Harness = egui_kittest::Harness<'static, WordApp>;

fn harness() -> Harness {
    let mut app = WordApp::new(Session::new(wordcraft_doc::Document::new()), Services::default());
    app.integrated_titlebar = true;
    let mut h = egui_kittest::Harness::builder().with_size(vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut WordApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        app,
    );
    h.run_steps(6);
    h
}

/// A window on the screen that goes where the app tells it, as macOS does.
struct Window {
    h: Harness,
    at: Pos2,
    moves: usize,
    commands: Vec<ViewportCommand>,
}

impl Window {
    /// One frame. The pointer is given on the screen; the app sees it relative to the window.
    fn step(&mut self, pointer: Pos2, button: Option<bool>) {
        let local = pointer - self.at.to_vec2();
        let viewport = self.h.input_mut().viewports.entry(ViewportId::ROOT).or_default();
        viewport.outer_rect = Some(egui::Rect::from_min_size(self.at, vec2(1200.0, 800.0)));
        self.h.input_mut().events.push(Event::PointerMoved(local));
        if let Some(pressed) = button {
            self.h.input_mut().events.push(Event::PointerButton { pos: local, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
        }
        self.h.step();
        let commands = self.h.output().viewport_output.get(&ViewportId::ROOT).map(|v| v.commands.clone()).unwrap_or_default();
        for c in &commands {
            if let ViewportCommand::OuterPosition(p) = c {
                self.at = *p;
                self.moves += 1;
            }
        }
        self.commands.extend(commands);
    }
}

#[test]
fn dragging_the_empty_title_bar_moves_the_window_under_the_pointer() {
    let mut w = Window { h: harness(), at: pos2(300.0, 200.0), moves: 0, commands: Vec::new() };
    // Between the quick access toolbar and the centred title: nothing there but the bar.
    let start = pos2(800.0, 210.0);
    w.step(start, None);
    w.step(start, Some(true));
    // A steady drag: the window keeps pace, so the pointer stays put relative to it.
    for i in 1..=4 {
        w.step(start + vec2(15.0, 10.0) * i as f32, None);
    }
    // The last move arrives with the release.
    w.step(start + vec2(75.0, 50.0), Some(false));
    assert!(!w.commands.contains(&ViewportCommand::StartDrag), "StartDrag does not move a macOS window: {:?}", w.commands);
    assert_eq!(w.moves, 5, "every pointer move moves the window: {:?}", w.commands);
    // The pressed point stays under the pointer: the window goes where the pointer went.
    assert_eq!(w.at, pos2(375.0, 250.0), "{:?}", w.commands);
    // Repaints without pointer input leave it there.
    w.h.step();
    w.h.step();
    assert!(
        !w.h.output()
            .viewport_output
            .get(&ViewportId::ROOT)
            .is_some_and(|v| v.commands.iter().any(|c| matches!(c, ViewportCommand::OuterPosition(_))))
    );
}
