use std::collections::HashSet;

use egui_multiwin::{
    multi_window::{CommonEventHandler, EventTrait, MultiWindow},
    winit::{event_loop::EventLoopProxy, window::WindowId},
};

mod windows;

const COMPUTER_MODERN_FONT: &[u8] = include_bytes!("./cmunbtl.ttf");

use windows::{
    popup_window,
    root::{self},
};

pub struct AppCommon {
    clicks: u32,
    root_window: u32,
    popup_windows: HashSet<u32>,
    sender: EventLoopProxy<CustomEvent>,
}

#[derive(Debug)]
pub struct CustomEvent {
    window: Option<WindowId>,
    message: u32,
}

impl EventTrait for CustomEvent {
    fn window_id(&self) -> Option<WindowId> {
        self.window
    }
}

impl CommonEventHandler<AppCommon, CustomEvent> for AppCommon {
    fn process_event(
        &mut self,
        event: CustomEvent,
    ) -> Vec<egui_multiwin::multi_window::NewWindowRequest<AppCommon, CustomEvent>> {
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
    egui_multiwin::multi_window::MultiWindow::start(|multi_window, event_loop, proxy| {
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
        match multi_window.add(root_window, &mut ac, &event_loop) {
            Err(e) => {
                println!("Failed to create main window {:?}", e);
            }
            _ => {}
        }
        match multi_window.add(root_window2, &mut ac, &event_loop) {
            Err(e) => {
                println!("Failed to create popup window {:?}", e);
            }
            _ => {}
        }
        ac
    });
}
