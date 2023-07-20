use egui_multiwin::egui::FontId;
use egui_multiwin::egui_glow::EguiGlow;
use egui_multiwin::{
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};

use crate::{AppCommon, CustomEvent};

use super::popup_window::PopupWindow;

pub struct RootWindow {
    pub button_press_count: u32,
    pub num_popups_created: u32,
}

impl RootWindow {
    pub fn request() -> NewWindowRequest<AppCommon, CustomEvent> {
        NewWindowRequest {
            window_state: Box::new(RootWindow {
                button_press_count: 0,
                num_popups_created: 0,
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

impl TrackedWindow<AppCommon, CustomEvent> for RootWindow {
    fn is_root(&self) -> bool {
        true
    }

    fn custom_event(
        &mut self,
        event: &CustomEvent,
        _c: &mut AppCommon,
        _egui: &mut EguiGlow,
        _window: &egui_multiwin::winit::window::Window,
    ) -> RedrawResponse<AppCommon, CustomEvent> {
        println!("Main window received an event {}", event.message);
        RedrawResponse {
            quit: false,
            new_windows: vec![],
        }
    }

    fn set_root(&mut self, _root: bool) {}

    fn redraw(
        &mut self,
        c: &mut AppCommon,
        egui: &mut EguiGlow,
        _window: &egui_multiwin::winit::window::Window,
    ) -> RedrawResponse<AppCommon, CustomEvent> {
        let mut quit = false;

        let mut windows_to_create = vec![];

        egui_multiwin::egui::SidePanel::left("my_side_panel").show(&egui.egui_ctx, |ui| {
            ui.heading("Hello World!");
            if ui.button("New popup").clicked() {
                let r = PopupWindow::request(format!("popup window #{}", self.num_popups_created));
                c.popup_windows.insert(r.id);
                windows_to_create.push(r);
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
            ui.label(t);
            for id in &c.popup_windows {
                if let Some(wid) = egui_multiwin::multi_window::get_window_id(*id) {
                    ui.label(format!("Popup window id {} has window id {:?}", id, wid));
                    if ui.button("Send message").clicked() {
                        if let Err(e) = c.sender.send_event(CustomEvent {
                            window: Some(wid),
                            message: 41,
                        }) {
                            println!("Failed to send message to popupwindow {:?}", e);
                        }
                    }
                } else {
                    ui.label(format!("Popup window id {}", id));
                }
            }
        });
        RedrawResponse {
            quit,
            new_windows: windows_to_create,
        }
    }
}
