use std::num::NonZeroU32;
use std::{mem, sync::Arc};

use crate::multi_window::NewWindowRequest;
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
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoopWindowTarget},
};

pub struct ContextHolder<T> {
    context: T,
    window: winit::window::Window,
    ws: glutin::surface::Surface<WindowSurface>,
    display: glutin::display::Display,
}

impl<T> ContextHolder<T> {
    fn new(
        context: T,
        window: winit::window::Window,
        ws: glutin::surface::Surface<WindowSurface>,
        display: glutin::display::Display,
    ) -> Self {
        Self {
            context,
            window,
            ws,
            display,
        }
    }
}

impl ContextHolder<PossiblyCurrentContext> {
    fn swap_buffers(&self) -> glutin::error::Result<()> {
        #[cfg(target_os="linux")]
        {
            let _e = self.ws.set_swap_interval(&self.context, glutin::surface::SwapInterval::DontWait);
        }
        self.ws.swap_buffers(&self.context)
    }

    fn resize(&self, size: winit::dpi::PhysicalSize<u32>) {
        let w = size.width;
        let h = size.height;
        self.ws.resize(
            &self.context,
            NonZeroU32::new(w).unwrap(),
            NonZeroU32::new(h).unwrap(),
        )
    }

    fn make_current(&self) -> glutin::error::Result<()> {
        self.context.make_current(&self.ws)
    }

    fn get_proc_address(&self, s: &str) -> *const std::ffi::c_void {
        let cs: *const std::ffi::c_char = s.as_ptr().cast();
        let cst = unsafe { std::ffi::CStr::from_ptr(cs) };
        self.display.get_proc_address(cst)
    }
}

impl ContextHolder<NotCurrentContext> {
    fn make_current(self) -> Result<ContextHolder<PossiblyCurrentContext>, glutin::error::Error> {
        let c = self.context.make_current(&self.ws).unwrap();
        let s = ContextHolder::<PossiblyCurrentContext> {
            context: c,
            window: self.window,
            ws: self.ws,
            display: self.display,
        };
        Ok(s)
    }
}

pub struct RedrawResponse<T> {
    pub quit: bool,
    pub new_windows: Vec<NewWindowRequest<T>>,
}

/// A window being tracked by a `MultiWindow`. All tracked windows will be forwarded all events
/// received on the `MultiWindow`'s event loop.
pub trait TrackedWindow<T> {

    /// Returns true if the window is a root window. Root windows will close all other windows when closed
    fn is_root(&self) -> bool {
        false
    }

    /// Sets whether or not the window is a root window.
    fn set_root(&mut self, _root: bool) {}

    /// Runs the redraw for the window. Return true to close the window.
    fn redraw(&mut self, c: &mut T, egui: &mut EguiGlow) -> RedrawResponse<T>;

    fn opengl_before(
        &mut self,
        _c: &mut T,
        _gl_window: &mut ContextHolder<PossiblyCurrentContext>,
    ) {
    }
    fn opengl_after(
        &mut self,
        _c: &mut T,
        _gl_window: &mut ContextHolder<PossiblyCurrentContext>,
    ) {
    }
}

/// Handles one event from the event loop. Returns true if the window needs to be kept alive,
/// otherwise it will be closed. Window events should be checked to ensure that their ID is one
/// that the TrackedWindow is interested in.
fn handle_event<COMMON, U>(
    s: &mut dyn TrackedWindow<COMMON>,
    event: &winit::event::Event<U>,
    c: &mut COMMON,
    egui: &mut EguiGlow,
    root_window_exists: bool,
    gl_window: &mut ContextHolder<PossiblyCurrentContext>,
) -> TrackedWindowControl<COMMON> {
    // Child window's requested control flow.
    let mut control_flow = ControlFlow::Wait; // Unless this changes, we're fine waiting until the next event comes in.

    let mut redraw = || {
        let input = egui.egui_winit.take_egui_input(&gl_window.window);
        let ppp = input.pixels_per_point;
        egui.egui_ctx.begin_frame(input);

        let rr = s.redraw(c, egui);

        let full_output = egui.egui_ctx.end_frame();

        if rr.quit {
            control_flow = winit::event_loop::ControlFlow::Exit;
        } else if full_output.repaint_after.is_zero() {
            gl_window.window.request_redraw();
            control_flow = winit::event_loop::ControlFlow::Poll;
        } else {
            control_flow = winit::event_loop::ControlFlow::Wait;
        };

        {
            let color = egui::Rgba::from_rgb(0.1, 0.3, 0.2);
            unsafe {
                use glow::HasContext as _;
                egui.painter
                    .gl()
                    .clear_color(color[0], color[1], color[2], color[3]);
                egui.painter.gl().clear(glow::COLOR_BUFFER_BIT);
            }

            // draw things behind egui here
            s.opengl_before(c, gl_window);

            let prim = egui.egui_ctx.tessellate(full_output.shapes);
            egui.painter.paint_and_update_textures(
                gl_window.window.inner_size().into(),
                ppp.unwrap_or(1.0),
                &prim[..],
                &full_output.textures_delta,
            );

            // draw things on top of egui here
            s.opengl_after(c, gl_window);

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

pub struct TrackedWindowOptions {
    pub vsync: bool,
    pub shader: Option<egui_glow::ShaderVersion>,
}

pub struct TrackedWindowContainer<T, U> {
    pub gl_window: IndeterminateWindowedContext,
    pub egui: Option<EguiGlow>,
    pub window: Box<dyn TrackedWindow<T>>,
    pub shader: Option<egui_glow::ShaderVersion>,
    _phantom: std::marker::PhantomData<U>,
}

impl<T, U> TrackedWindowContainer<T, U> {
    pub fn create<TE>(
        window: Box<dyn TrackedWindow<T>>,
        window_builder: winit::window::WindowBuilder,
        event_loop: &winit::event_loop::EventLoopWindowTarget<TE>,
        options: &TrackedWindowOptions,
    ) -> Result<TrackedWindowContainer<T, U>, DisplayCreationError> {
        let rdh = event_loop.raw_display_handle();
        let winitwindow = window_builder.build(&event_loop).unwrap();
        let rwh = winitwindow.raw_window_handle();
        #[cfg(target_os="windows")]
        let pref = glutin::display::DisplayApiPreference::Wgl(Some(rwh));
        #[cfg(target_os="linux")]
        let pref = glutin::display::DisplayApiPreference::Egl;
        #[cfg(target_os="macos")]
        let pref = glutin::display::DisplayApiPreference::Cgl;
        let display = unsafe { glutin::display::Display::new(rdh, pref) };
            if let Ok(display) = display {
            let configt = glutin::config::ConfigTemplateBuilder::default().build();
            let config = unsafe { display.find_configs(configt) }
                .unwrap()
                .reduce(|config, acc| {
                    if config.num_samples() > acc.num_samples() {
                        config
                    } else {
                        acc
                    }
                });
            if let Some(config) = config {
                let sab: SurfaceAttributesBuilder<WindowSurface> =
                    glutin::surface::SurfaceAttributesBuilder::default();
                let sa = sab.build(
                    rwh,
                    std::num::NonZeroU32::new(winitwindow.inner_size().width).unwrap(),
                    std::num::NonZeroU32::new(winitwindow.inner_size().height).unwrap(),
                );
                let ws = unsafe { display.create_window_surface(&config, &sa).unwrap() };

                let attr = glutin::context::ContextAttributesBuilder::new().build(Some(rwh));

                let gl_window = unsafe { display.create_context(&config, &attr).unwrap() };

                return Ok(TrackedWindowContainer {window,
                    gl_window: IndeterminateWindowedContext::NotCurrent(ContextHolder::new(gl_window, winitwindow, ws, display)),
                    egui: None,
                    shader: options.shader,
                    _phantom: std::marker::PhantomData,
                });
            }
        }
        panic!("No window created");
    }

    pub fn is_event_for_window(&self, event: &winit::event::Event<U>) -> bool {
        // Check if the window ID matches, if not then this window can pass on the event.
        match (event, &self.gl_window) {
            (Event::UserEvent(_), _) => {
                false
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

    pub fn handle_event_outer(
        &mut self,
        c: &mut T,
        event: &winit::event::Event<U>,
        el: &EventLoopWindowTarget<U>,
        root_window_exists: bool,
    ) -> TrackedWindowControl<T> {
        // Activate this gl_window so we can use it.
        // We cannot activate it without full ownership, so temporarily move the gl_window into the current scope.
        // It *must* be returned at the end.
        let gl_window = mem::replace(&mut self.gl_window, IndeterminateWindowedContext::None);
        let mut gl_window = match gl_window {
            IndeterminateWindowedContext::PossiblyCurrent(w) => {
                w.make_current().unwrap();
                w
            },
            IndeterminateWindowedContext::NotCurrent(w) => { w.make_current().unwrap() },
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

                let egui = egui_glow::EguiGlow::new(&el, gl.clone(), self.shader);
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
                );
                if let ControlFlow::Exit = result.requested_control_flow {
                    // This window wants to go away. Close it.
                    egui.destroy();
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

pub enum IndeterminateWindowedContext {
    PossiblyCurrent(ContextHolder<PossiblyCurrentContext>),
    NotCurrent(ContextHolder<NotCurrentContext>),
    None,
}

pub struct TrackedWindowControl<T> {
    pub requested_control_flow: ControlFlow,
    pub windows_to_create: Vec<NewWindowRequest<T>>,
}

#[derive(Error, Debug)]
pub enum DisplayCreationError {}
