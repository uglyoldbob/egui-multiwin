use egui_multiwin::egui::FontId;
use egui_multiwin::egui_glow::EguiGlow;
use egui_multiwin::{
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};

use crate::AppCommon;

use super::popup_window::PopupWindow;

pub struct RootWindow {
    pub button_press_count: u32,
    pub num_popups_created: u32,
    stuff: String,
}

impl RootWindow {
    pub fn request() -> NewWindowRequest<AppCommon> {
        NewWindowRequest {
            window_state: Box::new(RootWindow {
                button_press_count: 0,
                num_popups_created: 0,
                stuff: "".to_string(),
            }),
            builder: egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(true)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 800.0,
                    height: 600.0,
                })
                .with_title("egui-multiwin root window"),
            options: egui_multiwin::tracked_window::TrackedWindowOptions {
                vsync: false,
                shader: None,
            },
            id: egui_multiwin::multi_window::new_id(),
        }
    }
}

impl TrackedWindow<AppCommon> for RootWindow {
    fn is_root(&self) -> bool {
        true
    }

    fn set_root(&mut self, _root: bool) {}

    fn redraw(
        &mut self,
        c: &mut AppCommon,
        egui: &mut EguiGlow,
        _window: &egui_multiwin::winit::window::Window,
        clipboard: &mut egui_multiwin::arboard::Clipboard,
    ) -> RedrawResponse<AppCommon> {
        let mut quit = false;

        let mut windows_to_create = vec![];

        egui_multiwin::egui::SidePanel::left("my_side_panel").show(&egui.egui_ctx, |ui| {
            ui.heading("Hello World!");
            if ui.button("New popup").clicked() {
                windows_to_create.push(PopupWindow::request(format!(
                    "popup window #{}",
                    self.num_popups_created
                )));
                self.num_popups_created += 1;
            }
            if ui.button("Quit").clicked() {
                quit = true;
            }
        });
        egui_multiwin::egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            ui.heading(format!("number {}", c.clicks));
            let t = egui_multiwin::egui::widget_text::RichText::new("Example custom font text");
            let t = t.font(FontId {
                size: 12.0,
                family: egui_multiwin::egui::FontFamily::Name("computermodern".into()),
            });
            ui.label(format!("Text from clipboard is {}", self.stuff));
            if ui.button("Click to get clipboard contents").clicked() {
                if let Ok(s) = clipboard.get_text() {
                    self.stuff = s;
                }
            }
            if ui.button("Click to put text onto clipboard").clicked() {
                clipboard.set_text("This is text from the egui-multiwin demo");
            }
            ui.label(t);
        });
        RedrawResponse {
            quit,
            new_windows: windows_to_create,
        }
    }
}
