#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]

//! Shows a very simple example with minimal code

/// Macro generated code
pub mod egui_multiwin_dynamic {
    egui_multiwin::tracked_window!(crate::AppCommon, crate::CustomEvent, crate::MyWindows);
    egui_multiwin::multi_window!(crate::AppCommon, crate::CustomEvent, crate::MyWindows);
}

/// The windows for the program
#[enum_dispatch(TrackedWindow)]
pub enum MyWindows {
    /// A popup window
    Popup(PopupWindow),
}

use egui_multiwin::arboard;
use egui_multiwin::egui_glow::EguiGlow;
use egui_multiwin::enum_dispatch::enum_dispatch;
use egui_multiwin_dynamic::multi_window::NewWindowRequest;
use egui_multiwin_dynamic::tracked_window::RedrawResponse;
use egui_multiwin_dynamic::tracked_window::TrackedWindow;
use std::sync::Arc;

/// Data common to all windows
pub struct AppCommon {
    /// Number of times a button has been clicked
    clicks: u32,
}

/// Custom event type passed to windows
#[derive(Debug)]
pub struct CustomEvent {
    /// The optional window id
    window: Option<egui_multiwin::winit::window::WindowId>,
    /// The message
    message: u32,
}

impl CustomEvent {
    /// Return the window id
    fn window_id(&self) -> Option<egui_multiwin::winit::window::WindowId> {
        self.window
    }
}

/// The popup window
pub struct PopupWindow {}

impl PopupWindow {
    /// Create a request to create a window
    pub fn request() -> NewWindowRequest {
        NewWindowRequest::new(
            MyWindows::Popup(PopupWindow {}),
            egui_multiwin::winit::window::WindowBuilder::new()
                .with_resizable(false)
                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
                    width: 400.0,
                    height: 200.0,
                })
                .with_title("A window"),
            egui_multiwin::tracked_window::TrackedWindowOptions {
                vsync: false,
                shader: None,
            },
            egui_multiwin::multi_window::new_id(),
        )
    }
}

impl TrackedWindow for PopupWindow {
    fn is_root(&self) -> bool {
        true
    }

    fn redraw(
        &mut self,
        c: &mut AppCommon,
        egui: &mut EguiGlow,
        _window: &egui_multiwin::winit::window::Window,
        _clipboard: &mut arboard::Clipboard,
    ) -> RedrawResponse {
        let quit = false;
        egui_multiwin::egui::CentralPanel::default().show(&egui.egui_ctx, |ui| {
            ui.heading(format!("number {}", c.clicks));
        });
        RedrawResponse {
            quit,
            new_windows: Vec::new(),
        }
    }
}

impl AppCommon {
    /// Process events
    fn process_event(&mut self, event: CustomEvent) -> Vec<NewWindowRequest> {
        let mut windows_to_create = vec![];
        println!("Received an event {:?}", event);
        if event.message == 42 {
            windows_to_create.push(PopupWindow::request());
        }
        windows_to_create
    }
}

fn main() {
    egui_multiwin_dynamic::multi_window::MultiWindow::start(|multi_window, event_loop, _proxy| {
        let root_window = PopupWindow::request();

        let mut ac = AppCommon { clicks: 0 };

        if let Err(e) = multi_window.add(root_window, &mut ac, event_loop) {
            println!("Failed to create main window {:?}", e);
        }
        ac
    })
    .unwrap();
}
