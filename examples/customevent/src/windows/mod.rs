//! The code for all of the actual windows of the program

use egui_multiwin::enum_dispatch::enum_dispatch;

use crate::egui_multiwin_dynamic::tracked_window::{RedrawResponse, TrackedWindow};
use egui_multiwin::egui_glow::EguiGlow;
use std::sync::Arc;

pub mod popup_window;
pub mod root;

/// The windows that exist for the program
#[enum_dispatch(TrackedWindow)]
pub enum MyWindows {
    /// The main window
    Root(root::RootWindow),
    /// A popup window
    Popup(popup_window::PopupWindow),
}
