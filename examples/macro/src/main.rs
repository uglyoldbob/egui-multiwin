#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]

//! Another example program. Shows how request_repaint can be used.

use std::collections::HashSet;

/// Macro generated code
pub mod egui_multiwin_dynamic {
    egui_multiwin::tracked_window!(
        crate::AppCommon,
        crate::CustomEvent,
        crate::windows::MyWindows
    );
    egui_multiwin::multi_window!(
        crate::AppCommon,
        crate::CustomEvent,
        crate::windows::MyWindows
    );
}

use egui_multiwin::winit::{event_loop::EventLoopProxy, window::WindowId};

mod windows;

/// The custom font to use for the example
const COMPUTER_MODERN_FONT: &[u8] = include_bytes!("./cmunbtl.ttf");

use windows::{
    popup_window,
    root::{self},
};

/// The common data structure for the program
pub struct AppCommon {
    /// Number of times a button has been clicked
    clicks: u32,
    /// The id of the root window
    root_window: u32,
    /// The id of all popup windows
    popup_windows: HashSet<u32>,
    /// How messages can be sent to other windows
    sender: EventLoopProxy<CustomEvent>,
}

#[derive(Debug)]
/// The custom event
pub struct CustomEvent {
    /// The optional window id for the targeted window
    window: Option<WindowId>,
    /// The messaget to deliver
    message: u32,
}

impl CustomEvent {
    /// Get the window id from the event
    fn window_id(&self) -> Option<WindowId> {
        self.window
    }
}

impl AppCommon {
    /// Process events
    fn process_event(
        &mut self,
        event: CustomEvent,
    ) -> Vec<crate::egui_multiwin_dynamic::multi_window::NewWindowRequest> {
        let mut windows = vec![];
        match event.message {
            42 => {
                let r = popup_window::PopupWindow::request("initial popup".to_string());
                self.popup_windows.insert(r.id);
                windows.push(r);
            }
            _ => {
                println!("Recieved unhandled message {}", event.message);
            }
        }
        windows
    }
}

fn main() {
    crate::egui_multiwin_dynamic::multi_window::MultiWindow::start(
        |multi_window, event_loop, proxy| {
            multi_window.add_font(
                "computermodern".to_string(),
                egui_multiwin::egui::FontData::from_static(COMPUTER_MODERN_FONT),
            );
            let root_window = root::RootWindow::request();
            let root_window2 = popup_window::PopupWindow::request("initial popup".to_string());

            let mut ac = AppCommon {
                clicks: 0,
                root_window: root_window.id,
                popup_windows: HashSet::new(),
                sender: proxy,
            };

            ac.popup_windows.insert(root_window2.id);
            if let Err(e) = multi_window.add(root_window, &mut ac, event_loop) {
                println!("Failed to create main window {:?}", e);
            }
            if let Err(e) = multi_window.add(root_window2, &mut ac, event_loop) {
                println!("Failed to create popup window {:?}", e);
            }
            ac
        },
    )
    .unwrap();
}
