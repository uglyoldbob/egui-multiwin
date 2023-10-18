use egui_multiwin::multi_window::{CommonEventHandler, MultiWindow};

mod windows;

const COMPUTER_MODERN_FONT: &[u8] = include_bytes!("./cmunbtl.ttf");

use windows::{
    popup_window,
    root::{self},
};

pub struct AppCommon {
    clicks: u32,
}

impl CommonEventHandler<AppCommon> for AppCommon {}

fn main() {
    let mut event_loop = egui_multiwin::winit::event_loop::EventLoopBuilder::with_user_event();
    let event_loop = event_loop.build();
    let mut multi_window: MultiWindow<AppCommon> = MultiWindow::new();
    multi_window.add_font(
        "computermodern".to_string(),
        egui_multiwin::egui::FontData::from_static(COMPUTER_MODERN_FONT),
    );
    let root_window = root::RootWindow::request();
    let root_window2 = popup_window::PopupWindow::request("initial popup".to_string());

    let mut ac = AppCommon { clicks: 0 };

    let _e = multi_window.add(root_window, &mut ac, &event_loop);
    let _e = multi_window.add(root_window2, &mut ac, &event_loop);
    multi_window.run(event_loop, ac);
}
