use egui_multiwin::multi_window::MultiWindow;

mod windows;

use windows::{
    popup_window,
    root::{self},
};

pub struct AppCommon {
    clicks: u32,
}

fn main() {
    let event_loop = glutin::event_loop::EventLoopBuilder::with_user_event().build();
    let mut multi_window = MultiWindow::new();
    let root_window = root::RootWindow::new();
    let root_window2 = popup_window::PopupWindow::new("initial popup".to_string());

    let ac = AppCommon {
        clicks: 0,
    };

    let _e = multi_window.add(root_window, &event_loop);
    let _e = multi_window.add(root_window2, &event_loop);
    multi_window.run(event_loop, ac);
}
