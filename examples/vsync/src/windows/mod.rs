use egui_multiwin::enum_dispatch::enum_dispatch;

use crate::egui_multiwin_dynamic::tracked_window::{RedrawResponse, TrackedWindow};
use egui_multiwin::egui_glow::EguiGlow;
use std::sync::Arc;

pub mod popup_window;
pub mod root;
pub mod transparent_window;

#[enum_dispatch(TrackedWindow)]
pub enum MyWindows {
    Root(root::RootWindow),
    Popup(popup_window::PopupWindow),
    Transparent(transparent_window::PopupWindow),
}
