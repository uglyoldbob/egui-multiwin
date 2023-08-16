//! [![Rust Windows](https://github.com/uglyoldbob/egui-multiwin/actions/workflows/windows_build.yml/badge.svg)](https://github.com/uglyoldbob/egui-multiwin/actions/workflows/windows_build.yml)
//! [![Rust MacOS](https://github.com/uglyoldbob/egui-multiwin/actions/workflows/macos_build.yml/badge.svg)](https://github.com/uglyoldbob/egui-multiwin/actions/workflows/macos_build.yml)
//! [![Rust Linux](https://github.com/uglyoldbob/egui-multiwin/actions/workflows/linux_build.yml/badge.svg)](https://github.com/uglyoldbob/egui-multiwin/actions/workflows/linux_build.yml)
//!
//! This crate is based on the work by vivlim (<https://github.com/vivlim>) and repository located (<https://github.com/vivlim/egui-glow-multiwin>).
//! Vivlim's example repository combines the work at <https://github.com/shivshank/mini_gl_fb/blob/master/examples/multi_window.rs> and egui to form a
//! nice package. This crate makes some modifications to make it useful as an external crate by defining a few traits for users to implement on their
//! custom structs.
//!
//! There is an example (<https://github.com/uglyoldbob/egui-multiwin/tree/master/example>) that shows how to use this crate in your project. It is named multiwin-demo and is in the examples folder.
//!
//! The main struct for this crate is [MultiWindow<T,U>](crate::multi_window::MultiWindow<T,U>)
//!
//! Generally you will create a struct for data that is common to all windows, implement the [ComonEventHandler<T,U>](crate::multi_window::CommonEventHandler<T,U>) trait on it.
//! `T` is the name of your struct, and `U` and the name of the message you want to pass as a non-window specific event.
//!
//! ```
//! use egui_multiwin::multi_window::NewWindowRequest;
//! use egui_multiwin::tracked_window::RedrawResponse;
//! use egui_multiwin::tracked_window::TrackedWindow;
//! use egui_glow::EguiGlow;
//!
//! pub struct AppCommon {
//!     clicks: u32,
//! }
//!
//! pub struct PopupWindow { }
//!
//! impl PopupWindow {
//!     pub fn request() -> NewWindowRequest<AppCommon> {
//!        NewWindowRequest {
//!            window_state: Box::new(PopupWindow {
//!            }),
//!            builder: egui_multiwin::winit::window::WindowBuilder::new()
//!                .with_resizable(false)
//!                .with_inner_size(egui_multiwin::winit::dpi::LogicalSize {
//!                    width: 400.0,
//!                    height: 200.0,
//!                })
//!                .with_title("A window"),
//!            options: egui_multiwin::tracked_window::TrackedWindowOptions {
//!                vsync: false,
//!                shader: None,
//!            },
//!        }
//!    }
//! }
//!
//! impl TrackedWindow<AppCommon> for PopupWindow {
//!     fn is_root(&self) -> bool {
//!         true
//!     }
//!
//!     fn redraw(
//!         &mut self,
//!         c: &mut AppCommon,
//!         egui: &mut EguiGlow,
//!         window: &egui_multiwin::winit::window::Window,
//!         ) -> RedrawResponse<AppCommon> {
//!         let quit = false;
//!         todo!();
//!         RedrawResponse {
//!             quit,
//!             new_windows: Vec::new(),
//!         }
//!     }
//! }
//!
//! impl egui_multiwin::multi_window::CommonEventHandler<AppCommon, u32> for AppCommon {
//!
//!     fn process_event(&mut self, event: u32) -> Vec<egui_multiwin::multi_window::NewWindowRequest<AppCommon>> {
//!         let mut windows_to_create = vec![];
//!         println!("Received an event {}", event);
//!         match event {
//!             42 => windows_to_create.push(PopupWindow::request()),
//!             _ => {}
//!         }
//!         windows_to_create
//!     }
//! }
//! ```
//!
//! Check github issues to see if wayland (linux) still has a problem with the clipboard. That issue should give a temporary solution to a segfault that
//! occurs after closing a window in your program.
//!
//! In your main event, create an event loop, create an event loop proxy (if desired). The event loop proxy can be cloned and sent to other threads,
//! allowing custom logic to send events that can create windows and modify the common state of the application as required. Create a multiwindow instance,
//! then create window requests to make initial windows, and add them to the multiwindow with the add function. Create an instance of your common data
//! structure, and finally call run of your multiwindow instance.

#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]

pub use {egui, egui_glow, glutin, winit};
pub mod multi_window;
pub mod tracked_window;
