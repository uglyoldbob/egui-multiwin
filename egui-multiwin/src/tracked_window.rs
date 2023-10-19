//! This module covers definition and functionality for an individual window.

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::{mem, sync::Arc};

use crate::multi_window::{DefaultCustomEvent, EventTrait, NewWindowRequest};

use egui::NumExt;
use egui_glow::glow;
use egui_glow::EguiGlow;
use glutin::context::{NotCurrentContext, PossiblyCurrentContext};
use glutin::prelude::{GlConfig, GlDisplay};
use glutin::prelude::{
    NotCurrentGlContextSurfaceAccessor, PossiblyCurrentContextGlSurfaceAccessor,
};
use glutin::surface::GlSurface;
use glutin::surface::SurfaceAttributesBuilder;
use glutin::surface::WindowSurface;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use thiserror::Error;
use winit::window::WindowId;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoopWindowTarget},
};

/// A holder of context and related items
pub struct ContextHolder<T> {
    /// The context being held
    context: T,
    /// The window
    window: winit::window::Window,
    /// The window surface
    ws: glutin::surface::Surface<WindowSurface>,
    /// The display
    display: glutin::display::Display,
    /// The options for the display
    options: TrackedWindowOptions,
}

impl<T> ContextHolder<T> {
    /// Create a new context holder
    fn new(
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
        }
    }
}

impl ContextHolder<PossiblyCurrentContext> {
    /// Call swap_buffers. linux targets have vsync specifically disabled because it causes problems with hidden windows.
    fn swap_buffers(&self) -> glutin::error::Result<()> {
        if self.options.vsync {
            let _e = self.ws.set_swap_interval(
                &self.context,
                glutin::surface::SwapInterval::Wait(NonZeroU32::new(1).unwrap()),
            );
        } else {
            let _e = self
                .ws
                .set_swap_interval(&self.context, glutin::surface::SwapInterval::DontWait);
        }
        self.ws.swap_buffers(&self.context)
    }

    /// Resize the window to the specified size. The size cannot be zero in either dimension.
    fn resize(&self, size: winit::dpi::PhysicalSize<u32>) {
        let w = size.width;
        let h = size.height;
        self.ws.resize(
            &self.context,
            NonZeroU32::new(w.at_least(1)).unwrap(),
            NonZeroU32::new(h.at_least(1)).unwrap(),
        )
    }

    /// Make a possibly current context current
    fn make_current(&self) -> glutin::error::Result<()> {
        self.context.make_current(&self.ws)
    }

    /// convenience function to call get_proc_address on the display of this struct
    fn get_proc_address(&self, s: &str) -> *const std::ffi::c_void {
        let cs: *const std::ffi::c_char = s.as_ptr().cast();
        let cst = unsafe { std::ffi::CStr::from_ptr(cs) };
        self.display.get_proc_address(cst)
    }
}

impl ContextHolder<NotCurrentContext> {
    /// Transforms a not current context into a possibly current context
    fn make_current(self) -> Result<ContextHolder<PossiblyCurrentContext>, glutin::error::Error> {
        let c = self.context.make_current(&self.ws).unwrap();
        let s = ContextHolder::<PossiblyCurrentContext> {
            context: c,
            window: self.window,
            ws: self.ws,
            display: self.display,
            options: self.options,
        };
        Ok(s)
    }
}

/// The return value of the redraw function of trait `TrackedWindow<T>`
pub struct RedrawResponse<T, U = DefaultCustomEvent> {
    /// Should the window exit?
    pub quit: bool,
    /// A list of windows that the window desires to have created.
    pub new_windows: Vec<NewWindowRequest<T, U>>,
}

/// A window being tracked by a `MultiWindow`. All tracked windows will be forwarded all events
/// received on the `MultiWindow`'s event loop.
pub trait TrackedWindow<T, U = DefaultCustomEvent> {
    /// Returns true if the window is a root window. Root windows will close all other windows when closed. Windows are not root windows by default.
    /// It is completely valid to have more than one root window open at the same time. The program will exit when all root windows are closed.
    fn is_root(&self) -> bool {
        false
    }

    /// Returns true when the window is allowed to close. Default is windows are always allowed to close. Override to change this behavior.
    fn can_quit(&mut self, _c: &mut T) -> bool {
        true
    }

    /// Sets whether or not the window is a root window. Does nothing by default
    fn set_root(&mut self, _root: bool) {}

    /// Handles a custom event sent specifically to this window.
    fn custom_event(
        &mut self,
        _event: &U,
        _c: &mut T,
        _egui: &mut EguiGlow,
        _window: &winit::window::Window,
        _clipboard: &mut arboard::Clipboard,
    ) -> RedrawResponse<T, U> {
        RedrawResponse {
            quit: false,
            new_windows: vec![],
        }
    }

    /// Runs the redraw for the window. See RedrawResponse for the return value.
    fn redraw(
        &mut self,
        c: &mut T,
        egui: &mut EguiGlow,
        window: &winit::window::Window,
        clipboard: &mut arboard::Clipboard,
    ) -> RedrawResponse<T, U>;
    /// Allows opengl rendering to be done underneath all of the egui stuff of the window
    /// # Safety
    ///
    /// opengl functions are unsafe. This function would require calling opengl functions.
    unsafe fn opengl_before(&mut self, _c: &mut T, _gl: &Arc<egui_glow::painter::Context>) {}
    /// Allows opengl rendering to be done on top of all of the egui stuff of the window
    /// # Safety
    ///
    /// opengl functions are unsafe. This function would require calling opengl functions.
    unsafe fn opengl_after(&mut self, _c: &mut T, _gl: &Arc<egui_glow::painter::Context>) {}
}

/// Handles one event from the event loop. Returns true if the window needs to be kept alive,
/// otherwise it will be closed. Window events should be checked to ensure that their ID is one
/// that the TrackedWindow is interested in.
fn handle_event<COMMON, U: EventTrait>(
    s: &mut dyn TrackedWindow<COMMON, U>,
    event: &winit::event::Event<U>,
    c: &mut COMMON,
    egui: &mut EguiGlow,
    root_window_exists: bool,
    gl_window: &mut ContextHolder<PossiblyCurrentContext>,
    clipboard: &mut arboard::Clipboard,
) -> TrackedWindowControl<COMMON, U> {
    // Child window's requested control flow.
    let mut control_flow = ControlFlow::Wait; // Unless this changes, we're fine waiting until the next event comes in.

    let mut redraw = || {
        let input = egui.egui_winit.take_egui_input(&gl_window.window);
        let ppp = input.pixels_per_point;
        egui.egui_ctx.begin_frame(input);

        let rr = s.redraw(c, egui, &gl_window.window, clipboard);

        let full_output = egui.egui_ctx.end_frame();

        if rr.quit {
            control_flow = winit::event_loop::ControlFlow::Exit;
        } else if full_output.repaint_after.is_zero() {
            gl_window.window.request_redraw();
            control_flow = winit::event_loop::ControlFlow::Poll;
        } else if full_output.repaint_after.as_millis() > 0 && full_output.repaint_after.as_millis() < 10000 {
            control_flow = winit::event_loop::ControlFlow::WaitUntil(
                std::time::Instant::now() + full_output.repaint_after,
            );
        } else {
            control_flow = winit::event_loop::ControlFlow::Wait;
        };

        {
            let color = egui::Rgba::from_white_alpha(0.0);
            unsafe {
                use glow::HasContext as _;
                egui.painter
                    .gl()
                    .clear_color(color[0], color[1], color[2], color[3]);
                egui.painter.gl().clear(glow::COLOR_BUFFER_BIT);
            }

            // draw things behind egui here
            unsafe { s.opengl_before(c, egui.painter.gl()) };

            let prim = egui.egui_ctx.tessellate(full_output.shapes);
            egui.painter.paint_and_update_textures(
                gl_window.window.inner_size().into(),
                ppp.unwrap_or(1.0),
                &prim[..],
                &full_output.textures_delta,
            );

            // draw things on top of egui here
            unsafe { s.opengl_after(c, egui.painter.gl()) };

            gl_window.swap_buffers().unwrap();
        }
        rr
    };

    let response = match event {
        // Platform-dependent event handlers to workaround a winit bug
        // See: https://github.com/rust-windowing/winit/issues/987
        // See: https://github.com/rust-windowing/winit/issues/1619
        winit::event::Event::RedrawEventsCleared if cfg!(windows) => Some(redraw()),
        winit::event::Event::RedrawRequested(_) if !cfg!(windows) => Some(redraw()),
        winit::event::Event::UserEvent(ue) => Some(s.custom_event(ue, c, egui, &gl_window.window, clipboard)),

        winit::event::Event::WindowEvent { event, .. } => {
            if let winit::event::WindowEvent::Resized(physical_size) = event {
                gl_window.resize(*physical_size);
            }

            if let winit::event::WindowEvent::CloseRequested = event {
                control_flow = winit::event_loop::ControlFlow::Exit;
            }

            let resp = egui.on_event(event);
            if resp.repaint {
                gl_window.window.request_redraw();
            }

            None
        }
        winit::event::Event::LoopDestroyed => {
            egui.destroy();
            None
        }

        _ => None,
    };

    if !root_window_exists && !s.is_root() {
        control_flow = ControlFlow::Exit;
    }

    TrackedWindowControl {
        requested_control_flow: control_flow,
        windows_to_create: if let Some(a) = response {
            a.new_windows
        } else {
            Vec::new()
        },
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

/// The main container for a window. Contains all required data for operating and maintaining a window.
/// `T` is the type that represents the common app data, `U` is the type representing the message type
pub struct TrackedWindowContainer<T, U: EventTrait> {
    /// The context for the window
    pub gl_window: IndeterminateWindowedContext,
    /// The egui instance for this window, each window has a separate egui instance.
    pub egui: Option<EguiGlow>,
    /// The actual window
    pub window: Box<dyn TrackedWindow<T, U>>,
    /// The optional shader version for the window
    pub shader: Option<egui_glow::ShaderVersion>,
    /// Nothing, indicates that the type U is to be treated as if it exists.
    _phantom: std::marker::PhantomData<U>,
}

impl<T, U: EventTrait> TrackedWindowContainer<T, U> {
    /// Retrieve the window id for the container
    pub fn get_window_id(&self) -> Option<WindowId> {
        match &self.gl_window {
            IndeterminateWindowedContext::PossiblyCurrent(w) => Some(w.window.id()),
            IndeterminateWindowedContext::NotCurrent(w) => Some(w.window.id()),
            IndeterminateWindowedContext::None => {
                println!("Not able to get window id");
                None
            }
        }
    }

    /// Create a new window.
    pub fn create<TE>(
        window: Box<dyn TrackedWindow<T, U>>,
        window_builder: winit::window::WindowBuilder,
        event_loop: &winit::event_loop::EventLoopWindowTarget<TE>,
        options: &TrackedWindowOptions,
    ) -> Result<TrackedWindowContainer<T, U>, DisplayCreationError> {
        let rdh = event_loop.raw_display_handle();
        let winitwindow = window_builder.build(event_loop).unwrap();
        let rwh = winitwindow.raw_window_handle();
        #[cfg(target_os = "windows")]
        let pref = glutin::display::DisplayApiPreference::Wgl(Some(rwh));
        #[cfg(target_os = "linux")]
        let pref = glutin::display::DisplayApiPreference::Egl;
        #[cfg(target_os = "macos")]
        let pref = glutin::display::DisplayApiPreference::Cgl;
        let display = unsafe { glutin::display::Display::new(rdh, pref) };
        if let Ok(display) = display {
            let configt = glutin::config::ConfigTemplateBuilder::default().build();
            let mut configs : Vec<glutin::config::Config> = unsafe { display.find_configs(configt) }
                .unwrap().collect();
            configs.sort_by(|a,b| a.num_samples().cmp(&b.num_samples()));
            // Try all configurations until one works
            for config in configs {
                let sab: SurfaceAttributesBuilder<WindowSurface> =
                    glutin::surface::SurfaceAttributesBuilder::default();
                let sa = sab.build(
                    rwh,
                    std::num::NonZeroU32::new(winitwindow.inner_size().width).unwrap(),
                    std::num::NonZeroU32::new(winitwindow.inner_size().height).unwrap(),
                );
                let ws = unsafe { display.create_window_surface(&config, &sa) };
                if let Ok(ws) = ws {
                    let attr = glutin::context::ContextAttributesBuilder::new().build(Some(rwh));

                    let gl_window = unsafe { display.create_context(&config, &attr) }.unwrap();

                    return Ok(TrackedWindowContainer {
                        window,
                        gl_window: IndeterminateWindowedContext::NotCurrent(ContextHolder::new(
                            gl_window,
                            winitwindow,
                            ws,
                            display,
                            *options,
                        )),
                        egui: None,
                        shader: options.shader,
                        _phantom: std::marker::PhantomData,
                    });
                }
            }
        }
        panic!("No window created");
    }

    /// Returns true if the specified event is for this window. A UserEvent (one generated by the EventLoopProxy) is not for any window.
    pub fn is_event_for_window(&self, event: &winit::event::Event<U>) -> bool {
        // Check if the window ID matches, if not then this window can pass on the event.
        match (event, &self.gl_window) {
            (Event::UserEvent(ev), IndeterminateWindowedContext::PossiblyCurrent(gl_window)) => {
                if let Some(wid) = ev.window_id() {
                    wid == gl_window.window.id()
                } else {
                    false
                }
            }
            (Event::UserEvent(ev), IndeterminateWindowedContext::NotCurrent(gl_window)) => {
                if let Some(wid) = ev.window_id() {
                    wid == gl_window.window.id()
                } else {
                    false
                }
            }
            (
                Event::WindowEvent {
                    window_id: id,
                    event: _,
                    ..
                },
                IndeterminateWindowedContext::PossiblyCurrent(gl_window),
            ) => gl_window.window.id() == *id,
            (
                Event::WindowEvent {
                    window_id: id,
                    event: _,
                    ..
                },
                IndeterminateWindowedContext::NotCurrent(gl_window),
            ) => gl_window.window.id() == *id,
            _ => true, // we weren't able to check the window ID, maybe this window is not initialized yet. we should run it.
        }
    }

    /// The outer event handler for a window. Responsible for activating the context, creating the egui context if required, and calling handle_event.
    pub fn handle_event_outer(
        &mut self,
        c: &mut T,
        event: &winit::event::Event<U>,
        el: &EventLoopWindowTarget<U>,
        root_window_exists: bool,
        fontmap: &HashMap<String, egui::FontData>,
        clipboard: &mut arboard::Clipboard,
    ) -> TrackedWindowControl<T, U> {
        // Activate this gl_window so we can use it.
        // We cannot activate it without full ownership, so temporarily move the gl_window into the current scope.
        // It *must* be returned at the end.
        let gl_window = mem::replace(&mut self.gl_window, IndeterminateWindowedContext::None);
        let mut gl_window = match gl_window {
            IndeterminateWindowedContext::PossiblyCurrent(w) => {
                let _e = w.make_current();
                w
            }
            IndeterminateWindowedContext::NotCurrent(w) => w.make_current().unwrap(),
            IndeterminateWindowedContext::None => panic!("there's no window context???"),
        };

        // Now that the window is active, create a context if it is missing.
        match self.egui.as_ref() {
            None => {
                let gl = Arc::new(unsafe {
                    glow::Context::from_loader_function(|s| gl_window.get_proc_address(s))
                });

                unsafe {
                    use glow::HasContext as _;
                    gl.enable(glow::FRAMEBUFFER_SRGB);
                }

                let egui = egui_glow::EguiGlow::new(el, gl, self.shader);
                {
                    let mut fonts = egui::FontDefinitions::default();
                    for (name, font) in fontmap {
                        fonts.font_data.insert(name.clone(), font.clone());
                        fonts.families.insert(
                            egui::FontFamily::Name(name.to_owned().into()),
                            vec![name.to_owned()],
                        );
                    }
                    egui.egui_ctx.set_fonts(fonts)
                }
                self.egui = Some(egui);
            }
            Some(_) => (),
        };

        let result = match self.egui.as_mut() {
            Some(egui) => {
                let result = handle_event(
                    &mut *self.window,
                    event,
                    c,
                    egui,
                    root_window_exists,
                    &mut gl_window,
                    clipboard,
                );
                if let ControlFlow::Exit = result.requested_control_flow {
                    if self.window.can_quit(c) {
                        // This window wants to go away. Close it.
                        egui.destroy();
                    }
                };
                result
            }
            _ => {
                panic!("Window wasn't fully initialized");
            }
        };

        match mem::replace(
            &mut self.gl_window,
            IndeterminateWindowedContext::PossiblyCurrent(gl_window),
        ) {
            IndeterminateWindowedContext::None => (),
            _ => {
                panic!("Window had a GL context while we were borrowing it?");
            }
        }
        result
    }
}

/// Enum of the potential options for a window context
pub enum IndeterminateWindowedContext {
    /// The window context is possibly current
    PossiblyCurrent(ContextHolder<PossiblyCurrentContext>),
    /// The window context is not current
    NotCurrent(ContextHolder<NotCurrentContext>),
    /// The window context is empty
    None,
}

/// The eventual return struct of the `TrackedWindow<T, U>` trait update function. Used internally for window management.
pub struct TrackedWindowControl<T, U = DefaultCustomEvent> {
    /// Indicates how the window desires to respond to future events
    pub requested_control_flow: ControlFlow,
    /// A list of windows to be created
    pub windows_to_create: Vec<NewWindowRequest<T, U>>,
}

#[derive(Error, Debug)]
/// Enumerates the kinds of errors that display creation can have.
pub enum DisplayCreationError {}
