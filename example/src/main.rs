use egui_multiwin::multi_window::MultiWindow;

mod windows;

const COMPUTER_MODERN_FONT: &[u8] = include_bytes!("./cmunbtl.ttf");

use windows::{
    popup_window,
    root::{self},
};

pub struct AppCommon {
    clicks: u32,
}

impl egui_multiwin::multi_window::CommonEventHandler<AppCommon, u32> for AppCommon {
    fn process_event(
        &mut self,
        event: u32,
    ) -> Vec<egui_multiwin::multi_window::NewWindowRequest<AppCommon>> {
        let mut windows_to_create = vec![];
        println!("Received an event {}", event);
        match event {
            42 => windows_to_create.push(windows::popup_window::PopupWindow::request(
                "event popup".to_string(),
            )),
            43 => windows_to_create.push(windows::root::RootWindow::request()),
            _ => {}
        }
        windows_to_create
    }
}

fn main() {
    let mut event_loop = egui_multiwin::winit::event_loop::EventLoopBuilder::with_user_event();
    #[cfg(target_os = "linux")]
    egui_multiwin::winit::platform::x11::EventLoopBuilderExtX11::with_x11(&mut event_loop);
    let event_loop = event_loop.build();
    let proxy = event_loop.create_proxy();
    if let Err(e) = proxy.send_event(42) {
        println!("Failed to send event loop message: {:?}", e);
    }
    let mut multi_window: MultiWindow<AppCommon, u32> = MultiWindow::new();
    multi_window.add_font(
        "computermodern".to_string(),
        egui_multiwin::egui::FontData::from_static(COMPUTER_MODERN_FONT),
    );
    let root_window = root::RootWindow::request();
    let root_window2 = popup_window::PopupWindow::request("initial popup".to_string());

    let ac = AppCommon { clicks: 0 };

    let _e = multi_window.add(root_window, &event_loop);
    let _e = multi_window.add(root_window2, &event_loop);
    multi_window.run(event_loop, ac);
}
