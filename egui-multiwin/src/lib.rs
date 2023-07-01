//! This crate allows for a user to create a program that can have multiple windows open at the same time.

#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]

pub use {egui, egui_glow, glutin, winit};
pub mod multi_window;
pub mod tracked_window;
