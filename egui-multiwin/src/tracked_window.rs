//! This module covers definition and functionality for an individual window.

use std::num::NonZeroU32;

use egui::NumExt;
use glutin::context::{NotCurrentContext, PossiblyCurrentContext};
use glutin::prelude::GlDisplay;
use glutin::prelude::{NotCurrentGlContext, PossiblyCurrentGlContext};
use glutin::surface::GlSurface;
use glutin::surface::WindowSurface;
use thiserror::Error;
use winit::event_loop::ControlFlow;

/// A holder of context and related items
pub struct ContextHolder<T> {
    /// The context being held
    context: T,
    /// The window
    pub window: winit::window::Window,
    /// The window surface
    ws: glutin::surface::Surface<WindowSurface>,
    /// The display
    display: glutin::display::Display,
    /// The options for the display
    options: TrackedWindowOptions,
    /// The last control flow of the window
    pub control_flow: Option<ControlFlow>,
}

impl<T> ContextHolder<T> {
    /// Create a new context holder
    pub fn new(
        context: T,
        window: winit::window::Window,
        ws: glutin::surface::Surface<WindowSurface>,
        display: glutin::display::Display,
        options: TrackedWindowOptions,
    ) -> Self {
        Self {
            context,
            window,
            ws,
            display,
            options,
            control_flow: Some(ControlFlow::Poll),
        }
    }
}
impl<T> ContextHolder<T> {
    /// Get the window handle
    pub fn window(&self) -> &winit::window::Window {
        &self.window
    }
}

impl ContextHolder<PossiblyCurrentContext> {
    /// Call swap_buffers. linux targets have vsync specifically disabled because it causes problems with hidden windows.
    pub fn swap_buffers(&self) -> glutin::error::Result<()> {
        if self.options.vsync {
            let _e = self.ws.set_swap_interval(
                &self.context,
                glutin::surface::SwapInterval::Wait(NonZeroU32::MIN),
            );
        } else {
            let _e = self
                .ws
                .set_swap_interval(&self.context, glutin::surface::SwapInterval::DontWait);
        }
        self.ws.swap_buffers(&self.context)
    }

    /// Resize the window to the specified size. The size cannot be zero in either dimension.
    pub fn resize(&self, size: winit::dpi::PhysicalSize<u32>) {
        let w = size.width;
        let h = size.height;
        self.ws.resize(
            &self.context,
            NonZeroU32::new(w.at_least(1)).unwrap(),
            NonZeroU32::new(h.at_least(1)).unwrap(),
        )
    }

    /// Make a possibly current context current
    pub fn make_current(&self) -> glutin::error::Result<()> {
        self.context.make_current(&self.ws)
    }

    /// convenience function to call get_proc_address on the display of this struct
    pub fn get_proc_address(&self, s: &str) -> *const std::ffi::c_void {
        let cs: *const std::ffi::c_char = s.as_ptr().cast();
        let cst = unsafe { std::ffi::CStr::from_ptr(cs) };
        self.display.get_proc_address(cst)
    }
}

impl ContextHolder<NotCurrentContext> {
    /// Transforms a not current context into a possibly current context
    pub fn make_current(
        self,
    ) -> Result<ContextHolder<PossiblyCurrentContext>, glutin::error::Error> {
        let c = self.context.make_current(&self.ws).unwrap();
        let s = ContextHolder::<PossiblyCurrentContext> {
            context: c,
            window: self.window,
            ws: self.ws,
            display: self.display,
            options: self.options,
            control_flow: self.control_flow,
        };
        Ok(s)
    }
}

/// The options for a window.
#[derive(Copy, Clone)]
pub struct TrackedWindowOptions {
    /// Should the window be vsynced. Check github issues to see if this property actually does what it is supposed to.
    pub vsync: bool,
    /// Optionally sets the shader version for the window.
    pub shader: Option<egui_glow::ShaderVersion>,
}

#[derive(Error, Debug)]
/// Enumerates the kinds of errors that display creation can have.
pub enum DisplayCreationError {}
