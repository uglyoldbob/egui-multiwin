//! This defines the MultiWindow struct. This is the main struct used in the main function of a user application.

use std::{collections::HashMap, sync::Mutex};

use winit::window::WindowId;

lazy_static::lazy_static! {
    static ref WINDOW_REQUEST_ID: Mutex<u32> = Mutex::new(0u32);
    /// The table that is used to obtain window ids
    pub static ref WINDOW_TABLE: Mutex<HashMap<u32, Option<WindowId>>> = Mutex::new(HashMap::new());
}

/// Creates a new id for a window request that the user program can do things with
pub fn new_id() -> u32 {
    let mut l = WINDOW_REQUEST_ID.lock().unwrap();
    let mut table = WINDOW_TABLE.lock().unwrap();
    loop {
        *l = l.wrapping_add(1);
        if !table.contains_key(&l) {
            table.insert(*l, None);
            break;
        }
    }
    *l
}

/// Retrieve a window id
pub fn get_window_id(id: u32) -> Option<WindowId> {
    let table = WINDOW_TABLE.lock().unwrap();
    if let Some(id) = table.get(&id) {
        *id
    } else {
        None
    }
}

/// Create the dynamic tracked_window module for a egui_multiwin application. Takes three arguments. First argument is the type name of the common data structure for your application.
/// Second argument is the type for custom events (or egui_multiwin::NoEvent if that functionality is not desired). Third argument is the enum of all windows. It needs to be enum_dispatch.
#[macro_export]
macro_rules! tracked_window {
    ($common:ty,$event:ty, $window:ty) => {
        pub mod tracked_window {
            //! This module covers definition and functionality for an individual window.

            use std::collections::HashMap;
            use std::{mem, sync::Arc};

            use super::multi_window::NewWindowRequest;

            use egui_multiwin::egui;
            use egui_multiwin::egui_glow::EguiGlow;
            use egui_multiwin::egui_glow::{self, glow};
            use egui_multiwin::glutin::context::{NotCurrentContext, PossiblyCurrentContext};
            use egui_multiwin::glutin::prelude::{GlConfig, GlDisplay};
            use egui_multiwin::glutin::surface::SurfaceAttributesBuilder;
            use egui_multiwin::glutin::surface::WindowSurface;
            use egui_multiwin::raw_window_handle;
            use egui_multiwin::raw_window_handle_5;
            use egui_multiwin::raw_window_handle_5::{HasRawDisplayHandle, HasRawWindowHandle};
            use egui_multiwin::tracked_window::{ContextHolder, TrackedWindowOptions};
            use egui_multiwin::winit::window::WindowId;
            use egui_multiwin::winit::{
                event::Event,
                event_loop::{ControlFlow, EventLoopWindowTarget},
            };
            use egui_multiwin::{arboard, glutin, winit};

            use $window;

            /// The return value of the redraw function of trait `TrackedWindow`
            pub struct RedrawResponse {
                /// Should the window exit?
                pub quit: bool,
                /// A list of windows that the window desires to have created.
                pub new_windows: Vec<NewWindowRequest>,
            }

            /// A window being tracked by a `MultiWindow`. All tracked windows will be forwarded all events
            /// received on the `MultiWindow`'s event loop.
            #[egui_multiwin::enum_dispatch::enum_dispatch]
            pub trait TrackedWindow: core::hash::Hash {
                /// Returns the viewport id of the window.
                fn get_viewport_id(&self) -> egui_multiwin::egui::viewport::ViewportId {
                    egui_multiwin::egui::viewport::ViewportId::from_hash_of(self)
                }

                /// Returns true if the window is a root window. Root windows will close all other windows when closed. Windows are not root windows by default.
                /// It is completely valid to have more than one root window open at the same time. The program will exit when all root windows are closed.
                fn is_root(&self) -> bool {
                    false
                }

                /// Returns true when the window is allowed to close. Default is windows are always allowed to close. Override to change this behavior.
                fn can_quit(&mut self, _c: &mut $common) -> bool {
                    true
                }

                /// Sets whether or not the window is a root window. Does nothing by default
                fn set_root(&mut self, _root: bool) {}

                /// Handles a custom event sent specifically to this window.
                fn custom_event(
                    &mut self,
                    _event: &$event,
                    _c: &mut $common,
                    _egui: &mut EguiGlow,
                    _window: &egui_multiwin::winit::window::Window,
                    _clipboard: &mut egui_multiwin::arboard::Clipboard,
                ) -> RedrawResponse {
                    RedrawResponse {
                        quit: false,
                        new_windows: vec![],
                    }
                }

                /// Runs the redraw for the window. See RedrawResponse for the return value.
                fn redraw(
                    &mut self,
                    c: &mut $common,
                    egui: &mut EguiGlow,
                    window: &egui_multiwin::winit::window::Window,
                    clipboard: &mut egui_multiwin::arboard::Clipboard,
                ) -> RedrawResponse;
                /// Allows opengl rendering to be done underneath all of the egui stuff of the window
                /// # Safety
                ///
                /// opengl functions are unsafe. This function would require calling opengl functions.
                unsafe fn opengl_before(
                    &mut self,
                    _c: &mut $common,
                    _gl: &Arc<egui_multiwin::egui_glow::painter::Context>,
                ) {
                }
                /// Allows opengl rendering to be done on top of all of the egui stuff of the window
                /// # Safety
                ///
                /// opengl functions are unsafe. This function would require calling opengl functions.
                unsafe fn opengl_after(
                    &mut self,
                    _c: &mut $common,
                    _gl: &Arc<egui_multiwin::egui_glow::painter::Context>,
                ) {
                }
            }

            /// Handles one event from the event loop. Returns true if the window needs to be kept alive,
            /// otherwise it will be closed. Window events should be checked to ensure that their ID is one
            /// that the TrackedWindow is interested in.
            fn handle_event(
                s: &mut $window,
                event: &egui_multiwin::winit::event::Event<$event>,
                c: &mut $common,
                egui: &mut EguiGlow,
                root_window_exists: bool,
                gl_window: &mut egui_multiwin::tracked_window::ContextHolder<
                    PossiblyCurrentContext,
                >,
                clipboard: &mut egui_multiwin::arboard::Clipboard,
            ) -> TrackedWindowControl {
                // Child window's requested control flow.
                let mut control_flow = ControlFlow::Wait; // Unless this changes, we're fine waiting until the next event comes in.

                let mut redraw = || {
                    let input = egui.egui_winit.take_egui_input(&gl_window.window);
                    let ppp = egui.egui_ctx.pixels_per_point();
                    egui.egui_ctx.begin_frame(input);

                    let rr = s.redraw(c, egui, &gl_window.window, clipboard);

                    let full_output = egui.egui_ctx.end_frame();

                    let viewport_id = s.get_viewport_id();
                    let repaint_delay = full_output
                        .viewport_output
                        .get(&viewport_id)
                        .unwrap()
                        .repaint_delay;
                    if rr.quit {
                    } else if repaint_delay.is_zero() {
                        gl_window.window.request_redraw();
                        control_flow = egui_multiwin::winit::event_loop::ControlFlow::Poll;
                    } else if repaint_delay.as_millis() > 0 && repaint_delay.as_millis() < 10000 {
                        control_flow = egui_multiwin::winit::event_loop::ControlFlow::WaitUntil(
                            std::time::Instant::now() + repaint_delay,
                        );
                    } else {
                        control_flow = egui_multiwin::winit::event_loop::ControlFlow::Wait;
                    };

                    {
                        let color = egui_multiwin::egui::Rgba::from_white_alpha(0.0);
                        unsafe {
                            use glow::HasContext as _;
                            egui.painter
                                .gl()
                                .clear_color(color[0], color[1], color[2], color[3]);
                            egui.painter.gl().clear(glow::COLOR_BUFFER_BIT);
                        }

                        // draw things behind egui here
                        unsafe { s.opengl_before(c, egui.painter.gl()) };

                        let prim = egui.egui_ctx.tessellate(full_output.shapes, ppp);
                        egui.painter.paint_and_update_textures(
                            gl_window.window.inner_size().into(),
                            ppp,
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
                    egui_multiwin::winit::event::Event::UserEvent(ue) => {
                        Some(s.custom_event(ue, c, egui, &gl_window.window, clipboard))
                    }

                    egui_multiwin::winit::event::Event::WindowEvent { event, .. } => {
                        if let egui_multiwin::winit::event::WindowEvent::Resized(physical_size) =
                            event
                        {
                            gl_window.resize(*physical_size);
                        }

                        if let egui_multiwin::winit::event::WindowEvent::CloseRequested = event {}

                        let resp = egui.on_window_event(&gl_window.window, event);
                        if resp.repaint {
                            gl_window.window.request_redraw();
                        }

                        None
                    }
                    egui_multiwin::winit::event::Event::LoopExiting => {
                        egui.destroy();
                        None
                    }

                    _ => None,
                };

                if !root_window_exists && !s.is_root() {}

                TrackedWindowControl {
                    requested_control_flow: control_flow,
                    windows_to_create: if let Some(a) = response {
                        a.new_windows
                    } else {
                        Vec::new()
                    },
                }
            }

            /// The main container for a window. Contains all required data for operating and maintaining a window.
            /// `T` is the type that represents the common app data, `U` is the type representing the message type
            pub struct TrackedWindowContainer {
                /// The context for the window
                pub gl_window: IndeterminateWindowedContext,
                /// The egui instance for this window, each window has a separate egui instance.
                pub egui: Option<EguiGlow>,
                /// The actual window
                pub window: $window,
                /// The optional shader version for the window
                pub shader: Option<egui_multiwin::egui_glow::ShaderVersion>,
                /// Nothing, indicates that the type U is to be treated as if it exists.
                _phantom: std::marker::PhantomData<$event>,
            }

            impl TrackedWindowContainer {
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
                    window: $window,
                    window_builder: egui_multiwin::winit::window::WindowBuilder,
                    event_loop: &egui_multiwin::winit::event_loop::EventLoopWindowTarget<TE>,
                    options: &TrackedWindowOptions,
                ) -> Result<TrackedWindowContainer, DisplayCreationError> {
                    let rdh = event_loop.raw_display_handle();
                    let winitwindow = window_builder.build(event_loop).unwrap();
                    let rwh = winitwindow.raw_window_handle();
                    #[cfg(target_os = "windows")]
                    let pref = glutin::display::DisplayApiPreference::Wgl(Some(wh));
                    #[cfg(target_os = "linux")]
                    let pref = egui_multiwin::glutin::display::DisplayApiPreference::Egl;
                    #[cfg(target_os = "macos")]
                    let pref = glutin::display::DisplayApiPreference::Cgl;
                    let display = unsafe { glutin::display::Display::new(rdh, pref) };
                    if let Ok(display) = display {
                        let configt = glutin::config::ConfigTemplateBuilder::default().build();
                        let mut configs: Vec<glutin::config::Config> =
                            unsafe { display.find_configs(configt) }.unwrap().collect();
                        configs.sort_by(|a, b| a.num_samples().cmp(&b.num_samples()));
                        // Try all configurations until one works
                        for config in configs {
                            let sab: SurfaceAttributesBuilder<WindowSurface> =
                                egui_multiwin::glutin::surface::SurfaceAttributesBuilder::default();
                            let sa = sab.build(
                                rwh,
                                std::num::NonZeroU32::new(winitwindow.inner_size().width).unwrap(),
                                std::num::NonZeroU32::new(winitwindow.inner_size().height).unwrap(),
                            );
                            let ws = unsafe { display.create_window_surface(&config, &sa) };
                            if let Ok(ws) = ws {
                                let attr =
                                    egui_multiwin::glutin::context::ContextAttributesBuilder::new()
                                        .build(Some(rwh));

                                let gl_window =
                                    unsafe { display.create_context(&config, &attr) }.unwrap();

                                return Ok(TrackedWindowContainer {
                                    window,
                                    gl_window: IndeterminateWindowedContext::NotCurrent(
                                        egui_multiwin::tracked_window::ContextHolder::new(
                                            gl_window,
                                            winitwindow,
                                            ws,
                                            display,
                                            *options,
                                        ),
                                    ),
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
                pub fn is_event_for_window(&self, event: &winit::event::Event<$event>) -> bool {
                    // Check if the window ID matches, if not then this window can pass on the event.
                    match (event, &self.gl_window) {
                        (
                            Event::UserEvent(ev),
                            IndeterminateWindowedContext::PossiblyCurrent(gl_window),
                        ) => {
                            if let Some(wid) = ev.window_id() {
                                wid == gl_window.window.id()
                            } else {
                                false
                            }
                        }
                        (
                            Event::UserEvent(ev),
                            IndeterminateWindowedContext::NotCurrent(gl_window),
                        ) => {
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
                    c: &mut $common,
                    event: &winit::event::Event<$event>,
                    el: &EventLoopWindowTarget<$event>,
                    root_window_exists: bool,
                    fontmap: &HashMap<String, egui::FontData>,
                    clipboard: &mut arboard::Clipboard,
                ) -> TrackedWindowControl {
                    // Activate this gl_window so we can use it.
                    // We cannot activate it without full ownership, so temporarily move the gl_window into the current scope.
                    // It *must* be returned at the end.
                    let gl_window =
                        mem::replace(&mut self.gl_window, IndeterminateWindowedContext::None);
                    let mut gl_window = match gl_window {
                        IndeterminateWindowedContext::PossiblyCurrent(w) => {
                            let _e = w.make_current();
                            w
                        }
                        IndeterminateWindowedContext::NotCurrent(w) => w.make_current().unwrap(),
                        IndeterminateWindowedContext::None => {
                            panic!("there's no window context???")
                        }
                    };

                    // Now that the window is active, create a context if it is missing.
                    match self.egui.as_ref() {
                        None => {
                            let gl = Arc::new(unsafe {
                                glow::Context::from_loader_function(|s| {
                                    gl_window.get_proc_address(s)
                                })
                            });

                            unsafe {
                                use glow::HasContext as _;
                                gl.enable(glow::FRAMEBUFFER_SRGB);
                            }

                            let egui = egui_glow::EguiGlow::new(el, gl, self.shader, Some(1.0));
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
                                &mut self.window,
                                event,
                                c,
                                egui,
                                root_window_exists,
                                &mut gl_window,
                                clipboard,
                            );
                            /*
                            if let todo!() = result.requested_control_flow {
                                if self.window.can_quit(c) {
                                    // This window wants to go away. Close it.
                                    egui.destroy();
                                }
                            }; */
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

            /// The eventual return struct of the `TrackedWindow` trait update function. Used internally for window management.
            pub struct TrackedWindowControl {
                /// Indicates how the window desires to respond to future events
                pub requested_control_flow: ControlFlow,
                /// A list of windows to be created
                pub windows_to_create: Vec<NewWindowRequest>,
            }

            #[derive(egui_multiwin::thiserror::Error, Debug)]
            /// Enumerates the kinds of errors that display creation can have.
            pub enum DisplayCreationError {}
        }
    };
}

/// This macro creates a dynamic definition of the multi_window module. It has the same arguments as the [`tracked_window`](macro.tracked_window.html) macro.
#[macro_export]
macro_rules! multi_window {
    ($common:ty,$event:ty, $window:ty) => {
        pub mod multi_window {
            //! This defines the MultiWindow struct. This is the main struct used in the main function of a user application.

            use std::collections::HashMap;

            use egui_multiwin::{
                tracked_window::TrackedWindowOptions,
                winit::{
                    self,
                    event_loop::{ControlFlow, EventLoop},
                },
            };

            use super::tracked_window::{
                DisplayCreationError, TrackedWindow, TrackedWindowContainer,
            };

            /// The main struct of the crate. Manages multiple `TrackedWindow`s by forwarding events to them.
            /// `T` represents the common data struct for the user program. `U` is the type representing custom events.
            pub struct MultiWindow {
                /// The windows for the application.
                windows: Vec<TrackedWindowContainer>,
                /// A list of fonts to install on every egui instance
                fonts: HashMap<String, egui_multiwin::egui::FontData>,
                /// The clipboard
                clipboard: egui_multiwin::arboard::Clipboard,
            }

            impl Default for MultiWindow {
                fn default() -> Self {
                    Self::new()
                }
            }

            impl MultiWindow {
                /// Creates a new `MultiWindow`.
                pub fn new() -> Self {
                    MultiWindow {
                        windows: vec![],
                        fonts: HashMap::new(),
                        clipboard: egui_multiwin::arboard::Clipboard::new().unwrap(),
                    }
                }

                /// A simpler way to start up a user application. The provided closure should initialize the root window, add any fonts desired, store the proxy if it is needed, and return the common app struct.
                pub fn start(
                    t: impl FnOnce(
                        &mut Self,
                        &EventLoop<$event>,
                        egui_multiwin::winit::event_loop::EventLoopProxy<$event>,
                    ) -> $common,
                ) {
                    let mut event_loop =
                        egui_multiwin::winit::event_loop::EventLoopBuilder::with_user_event();
                    let event_loop = event_loop.build().unwrap();
                    let proxy = event_loop.create_proxy();
                    let mut multi_window = Self::new();

                    let ac = t(&mut multi_window, &event_loop, proxy);

                    multi_window.run(event_loop, ac);
                }

                /// Add a font that is applied to every window. Be sure to call this before calling [add](crate::multi_window::MultiWindow::add)
                /// multi_window is an instance of [MultiWindow](crate::multi_window::MultiWindow), DATA is a static `&[u8]` - most like defined with a `include_bytes!()` macro
                /// ```
                /// use egui_multiwin::multi_window::NewWindowRequest;
                /// struct Custom {}
                ///
                /// impl egui_multiwin::multi_window::CommonEventHandler for Custom {
                ///     fn process_event(&mut self, _event: egui_multiwin::multi_window::DefaultCustomEvent)  -> Vec<NewWindowRequest>{
                ///         vec!()
                ///     }
                /// }
                ///
                /// let mut multi_window: egui_multiwin::multi_window::MultiWindow = egui_multiwin::multi_window::MultiWindow::new();
                /// let DATA = include_bytes!("cmunbtl.ttf");
                /// multi_window.add_font("my_font".to_string(), egui_multiwin::egui::FontData::from_static(DATA));
                /// ```
                pub fn add_font(&mut self, name: String, fd: egui_multiwin::egui::FontData) {
                    self.fonts.insert(name, fd);
                }

                /// Adds a new `TrackedWindow` to the `MultiWindow`. If custom fonts are desired, call [add_font](crate::multi_window::MultiWindow::add_font) first.
                pub fn add<TE>(
                    &mut self,
                    window: NewWindowRequest,
                    _c: &mut $common,
                    event_loop: &egui_multiwin::winit::event_loop::EventLoopWindowTarget<TE>,
                ) -> Result<(), DisplayCreationError> {
                    let twc = TrackedWindowContainer::create::<TE>(
                        window.window_state,
                        window.builder,
                        event_loop,
                        &window.options,
                    )?;
                    let w = twc.get_window_id();
                    let mut table = egui_multiwin::multi_window::WINDOW_TABLE.lock().unwrap();
                    if let Some(id) = table.get_mut(&window.id) {
                        *id = w;
                    }
                    self.windows.push(twc);
                    Ok(())
                }

                /// Process the given event for the applicable window(s)
                pub fn do_window_events(
                    &mut self,
                    c: &mut $common,
                    event: &winit::event::Event<$event>,
                    event_loop_window_target: &winit::event_loop::EventLoopWindowTarget<$event>,
                ) -> Vec<ControlFlow> {
                    let mut handled_windows = vec![];
                    let mut window_control_flow = vec![];

                    let mut root_window_exists = false;
                    for other in &self.windows {
                        if other.window.is_root() {
                            root_window_exists = true;
                        }
                    }

                    while let Some(mut window) = self.windows.pop() {
                        if window.is_event_for_window(event) {
                            let window_control = window.handle_event_outer(
                                c,
                                event,
                                event_loop_window_target,
                                root_window_exists,
                                &self.fonts,
                                &mut self.clipboard,
                            );
                            match window_control.requested_control_flow {
                                /*
                                ControlFlow::Exit => {
                                    //println!("window requested exit. Instead of sending the exit for everyone, just get rid of this one.");
                                    if window.window.can_quit(c) {
                                        window_control_flow.push(ControlFlow::Exit);
                                        continue;
                                    } else {
                                        window_control_flow.push(ControlFlow::Wait);
                                    }
                                    // *flow = ControlFlow::Exit
                                }
                                */
                                requested_flow => {
                                    window_control_flow.push(requested_flow);
                                }
                            }

                            for new_window_request in window_control.windows_to_create {
                                let _e = self.add(new_window_request, c, event_loop_window_target);
                            }
                        }
                        handled_windows.push(window);
                    }

                    // Move them back.
                    handled_windows.reverse();
                    self.windows.append(&mut handled_windows);

                    window_control_flow
                }

                /// Runs the event loop until all `TrackedWindow`s are closed.
                pub fn run(mut self, event_loop: EventLoop<$event>, mut c: $common) {
                    event_loop.run(move |event, event_loop_window_target| {
                        let c = &mut c;
                        //println!("handling event {:?}", event);
                        let window_try = if let winit::event::Event::UserEvent(uevent) = &event {
                            uevent.window_id().is_some()
                        } else {
                            true
                        };
                        let window_control_flow = if window_try {
                            self.do_window_events(c, &event, event_loop_window_target)
                        } else {
                            if let winit::event::Event::UserEvent(uevent) = event {
                                for w in c.process_event(uevent) {
                                    let _e = self.add(w, c, event_loop_window_target);
                                }
                            }
                            vec![ControlFlow::Poll]
                        };

                        let mut flow = event_loop_window_target.control_flow();
                        // If any window requested polling, we should poll.
                        // Precedence: Poll > WaitUntil(smallest) > Wait.
                        flow = ControlFlow::Wait;
                        for flow_request in window_control_flow {
                            match flow_request {
                                ControlFlow::Poll => {
                                    flow = ControlFlow::Poll;
                                }
                                ControlFlow::Wait => (), // do nothing, if untouched it will be wait
                                ControlFlow::WaitUntil(when_new) => {
                                    if let ControlFlow::Poll = flow {
                                        continue; // Polling takes precedence, so ignore this.
                                    }

                                    // The current flow is already WaitUntil. If this one is sooner, use it instead.
                                    if let ControlFlow::WaitUntil(when_current) = flow {
                                        if when_new < when_current {
                                            flow = ControlFlow::WaitUntil(when_new);
                                        }
                                    } else {
                                        // The current flow is lower precedence, so replace it with this.
                                        flow = ControlFlow::WaitUntil(when_new);
                                    }
                                }
                            }
                        }
                        event_loop_window_target.set_control_flow(flow);
                    });
                }
            }

            /// A struct defining how a new window is to be created.
            pub struct NewWindowRequest {
                /// The actual struct containing window data. The struct must implement the `TrackedWindow` trait.
                pub window_state: $window,
                /// Specifies how to build the window with a WindowBuilder
                pub builder: egui_multiwin::winit::window::WindowBuilder,
                /// Other options for the window.
                pub options: TrackedWindowOptions,
                /// An id to allow a user program to translate window requests into actual window ids.
                pub id: u32,
            }
        }
    };
}
