use egui_glow::EguiGlow;
use egui_multiwin::{
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};

use crate::AppCommon;

pub struct PopupWindow {
    pub input: String,
}

impl PopupWindow {
    pub fn new(label: String) -> NewWindowRequest<AppCommon> {
        NewWindowRequest {
            window_state: Box::new(PopupWindow {
                input: label.clone(),
            }),
            builder: glutin::window::WindowBuilder::new()
                .with_resizable(false)
                .with_inner_size(glutin::dpi::LogicalSize {
                    width: 400.0,
                    height: 200.0,
                })
                .with_title(label),
        }
    }
}

impl TrackedWindow for PopupWindow {
    type Data = AppCommon;

    fn redraw(&mut self, c: &mut Self::Data, egui: &mut EguiGlow) -> RedrawResponse<Self::Data> {
        let mut quit = false;

        egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            if ui.button("Increment").clicked() {
                c.clicks += 1;
            }
            let response = ui.add(egui::TextEdit::singleline(&mut self.input));
            if response.changed() {
                // …
            }
            if response.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
                // …
            }
            if ui.button("Quit").clicked() {
                quit = true;
            }
        });
        RedrawResponse {
            quit: quit,
            new_windows: Vec::new(),
        }
    }
}
