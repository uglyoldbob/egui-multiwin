#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]

//! An example of how to use custom events

use std::collections::HashSet;

use egui_multiwin::winit::{event_loop::EventLoopProxy, window::WindowId};

use egui_multiwin_dynamic::multi_window::{MultiWindow, NewWindowRequest};

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

mod windows;

/// The custom font to use for the example
const COMPUTER_MODERN_FONT: &[u8] = include_bytes!("./cmunbtl.ttf");

use windows::{
    popup_window,
    root::{self},
};

/// Data common to all windows
pub struct AppCommon {
    /// Number of times a button has been clicked
    clicks: u32,
    /// Id for the root window
    root_window: u32,
    /// The id of all popup windows
    popup_windows: HashSet<u32>,
    /// How messages are sent to other windows
    sender: EventLoopProxy<CustomEvent>,
}

/// The custom event that is passed to windows in the program
#[derive(Debug)]
pub struct CustomEvent {
    /// The target window
    window: Option<WindowId>,
    /// The message
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
    fn process_event(&mut self, event: CustomEvent) -> Vec<NewWindowRequest> {
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
    let mut event_loop = egui_multiwin::winit::event_loop::EventLoopBuilder::with_user_event();
    let event_loop = event_loop.build().unwrap();
    let proxy = event_loop.create_proxy();
    if let Err(e) = proxy.send_event(CustomEvent {
        window: None,
        message: 41,
    }) {
        println!("Error sending non-window specific event: {:?}", e);
    }
    let mut multi_window: MultiWindow = MultiWindow::new();
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
    if let Err(e) = multi_window.add(root_window, &mut ac, &event_loop) {
        println!("Failed to create main window {:?}", e);
    }
    if let Err(e) = multi_window.add(root_window2, &mut ac, &event_loop) {
        println!("Failed to create popup window {:?}", e);
    }
    multi_window.run(event_loop, ac).unwrap();
}
