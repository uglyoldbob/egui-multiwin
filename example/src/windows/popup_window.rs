use egui_multiwin::egui_glow::EguiGlow;
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
            builder: egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(false)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 400.0,
                    height: 200.0,
                })
                .with_title(label),
            options: egui_multiwin::tracked_window::TrackedWindowOptions {
                vsync: false,
                shader: None,
            },
        }
    }
}

impl TrackedWindow<AppCommon> for PopupWindow {

    fn redraw(&mut self, c: &mut AppCommon, egui: &mut EguiGlow, window: &egui_multiwin::winit::window::Window) -> RedrawResponse<AppCommon> {
        let mut quit = false;

        egui_multiwin::egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            if ui.button("Increment").clicked() {
                c.clicks += 1;
                window.set_title(&format!("Title update {}", c.clicks));
            }
            let response = ui.add(egui_multiwin::egui::TextEdit::singleline(&mut self.input));
            if response.changed() {
                // …
            }
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui_multiwin::egui::Key::Enter))
            {
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
