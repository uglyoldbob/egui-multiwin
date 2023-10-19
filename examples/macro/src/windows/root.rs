use crate::egui_multiwin_dynamic::{
    multi_window::NewWindowRequest,
    tracked_window::{RedrawResponse, TrackedWindow},
};
use egui_multiwin::egui::FontId;
use egui_multiwin::egui_glow::EguiGlow;

use crate::{AppCommon, CustomEvent};

use super::popup_window::PopupWindow;

pub struct RootWindow {
    pub button_press_count: u32,
    pub num_popups_created: u32,
    prev_time: std::time::Instant,
    fps: Option<f32>,
}

impl RootWindow {
    pub fn request() -> NewWindowRequest {
        NewWindowRequest {
            window_state: super::MyWindows::Root(RootWindow {
                button_press_count: 0,
                num_popups_created: 0,
                prev_time: std::time::Instant::now(),
                fps: None,
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

impl TrackedWindow for RootWindow {
    fn is_root(&self) -> bool {
        true
    }

    fn custom_event(
        &mut self,
        event: &CustomEvent,
        _c: &mut AppCommon,
        _egui: &mut EguiGlow,
        _window: &egui_multiwin::winit::window::Window,
        _clipboard: &mut egui_multiwin::arboard::Clipboard,
    ) -> RedrawResponse {
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
        _clipboard: &mut egui_multiwin::arboard::Clipboard,
    ) -> RedrawResponse {
        let mut quit = false;

        egui.egui_ctx.request_repaint();

        let cur_time = std::time::Instant::now();
        let delta = cur_time.duration_since(self.prev_time);
        self.prev_time = cur_time;

        let new_fps = 1_000_000_000.0 / delta.as_nanos() as f32;
        if let Some(fps) = &mut self.fps {
            *fps = (*fps * 0.95) + (0.05 * new_fps);
        } else {
            self.fps = Some(new_fps);
        }

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
            ui.label(format!("The fps is {}", self.fps.unwrap()));
            ui.heading(format!("number {}", c.clicks));
            let t = egui_multiwin::egui::widget_text::RichText::new("Example custom font text");
            let t = t.font(FontId {
                size: 12.0,
                family: egui_multiwin::egui::FontFamily::Name("computermodern".into()),
            });
            ui.label(t);

            if let Some(wid) = egui_multiwin::multi_window::get_window_id(c.root_window) {
                ui.label(format!(
                    "Root window id {} has window id {:?}",
                    c.root_window, wid
                ));
                if ui.button("Send message").clicked() {
                    if let Err(e) = c.sender.send_event(CustomEvent {
                        window: Some(wid),
                        message: 40,
                    }) {
                        println!("Failed to send message to root window {:?}", e);
                    }
                }
            } else {
                ui.label(format!("Root window id {} failed", c.root_window));
            }

            for id in &c.popup_windows {
                if let Some(wid) = egui_multiwin::multi_window::get_window_id(*id) {
                    ui.label(format!("Popup window id {} has window id {:?}", id, wid));
                    if ui.button("Send message").clicked() {
                        if let Err(e) = c.sender.send_event(CustomEvent {
                            window: Some(wid),
                            message: 40,
                        }) {
                            println!("Failed to send message to popupwindow {:?}", e);
                        }
                    }
                } else {
                    ui.label(format!("Popup window id {} failed", id));
                }
            }
        });
        RedrawResponse {
            quit,
            new_windows: windows_to_create,
        }
    }
}
