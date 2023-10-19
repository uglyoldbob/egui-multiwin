//! This defines the MultiWindow struct. This is the main struct used in the main function of a user application.

use std::{collections::HashMap, sync::Mutex};

use winit::{
    event_loop::{ControlFlow, EventLoop},
    window::WindowId,
};

use crate::tracked_window::{
    DisplayCreationError, TrackedWindow, TrackedWindowContainer, TrackedWindowOptions,
};

/// The default provided struct for custom events. This is used when custom events are not desired in the user program.
pub struct DefaultCustomEvent {}

impl EventTrait for DefaultCustomEvent {
    fn window_id(&self) -> Option<WindowId> {
        None
    }
}

/// This trait allows for non-window specific events to be sent to the event loop.
/// It allows for non-gui threads or code to interact with the gui through the common struct
pub trait CommonEventHandler<T, U: EventTrait = DefaultCustomEvent> {
    /// Process non-window specific events for the application
    fn process_event(&mut self, _event: U) -> Vec<NewWindowRequest<T, U>> {
        vec![]
    }
}

/// This trait is to be implemented on custom window events
pub trait EventTrait {
    /// Returns a Some when the event is for a particular window, returns None when the event is not for a particular window
    fn window_id(&self) -> Option<WindowId>;
}

/// The main struct of the crate. Manages multiple `TrackedWindow`s by forwarding events to them.
/// `T` represents the common data struct for the user program. `U` is the type representing custom events.
pub struct MultiWindow<T, U: EventTrait = DefaultCustomEvent> {
    /// The windows for the application.
    windows: Vec<TrackedWindowContainer<T, U>>,
    /// A list of fonts to install on every egui instance
    fonts: HashMap<String, egui::FontData>,
    /// The clipboard
    clipboard: arboard::Clipboard,
}

impl<T: 'static + CommonEventHandler<T, U>, U: EventTrait + 'static> Default for MultiWindow<T, U> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static + CommonEventHandler<T, U>, U: EventTrait + 'static> MultiWindow<T, U> {
    /// Creates a new `MultiWindow`.
    pub fn new() -> Self {
        MultiWindow {
            windows: vec![],
            fonts: HashMap::new(),
            clipboard: arboard::Clipboard::new().unwrap(),
        }
    }

    /// A simpler way to start up a user application. The provided closure should initialize the root window, add any fonts desired, store the proxy if it is needed, and return the common app struct.
    pub fn start(
        t: impl FnOnce(&mut Self, &EventLoop<U>, winit::event_loop::EventLoopProxy<U>) -> T,
    ) {
        let mut event_loop = winit::event_loop::EventLoopBuilder::with_user_event();
        let event_loop = event_loop.build();
        let proxy = event_loop.create_proxy();
        let mut multi_window = Self::new();

        let ac = t(&mut multi_window, &event_loop, proxy);

        multi_window.run(event_loop, ac);
    }

    /// Add a font that is applied to every window. Be sure to call this before calling [add](crate::multi_window::MultiWindow::add)
    /// multi_window is an instance of [MultiWindow<T,U>](crate::multi_window::MultiWindow<T,U>), DATA is a static `&[u8]` - most like defined with a `include_bytes!()` macro
    /// ```
    /// use egui_multiwin::multi_window::NewWindowRequest;
    /// struct Custom {}
    /// 
    /// impl egui_multiwin::multi_window::CommonEventHandler<Custom> for Custom {
    ///     fn process_event(&mut self, _event: egui_multiwin::multi_window::DefaultCustomEvent)  -> Vec<NewWindowRequest<Custom>>{
    ///         vec!()
    ///     }
    /// }
    ///
    /// let mut multi_window: egui_multiwin::multi_window::MultiWindow<Custom> = egui_multiwin::multi_window::MultiWindow::new();
    /// let DATA = include_bytes!("cmunbtl.ttf");
    /// multi_window.add_font("my_font".to_string(), egui_multiwin::egui::FontData::from_static(DATA));
    /// ```
    pub fn add_font(&mut self, name: String, fd: egui::FontData) {
        self.fonts.insert(name, fd);
    }

    /// Adds a new `TrackedWindow` to the `MultiWindow`. If custom fonts are desired, call [add_font](crate::multi_window::MultiWindow::add_font) first.
    pub fn add<TE>(
        &mut self,
        window: NewWindowRequest<T, U>,
        c: &mut T,
        event_loop: &winit::event_loop::EventLoopWindowTarget<TE>,
    ) -> Result<(), DisplayCreationError> {
        let twc = TrackedWindowContainer::create::<TE>(
            window.window_state,
            window.builder,
            event_loop,
            &window.options,
        )?;
        let w = twc.get_window_id();
        let mut table = WINDOW_TABLE.lock().unwrap();
        if let Some(id) = table.get_mut(&window.id) {
            *id = w;
        }
        self.windows.push(twc);
        Ok(())
    }

    /// Process the given event for the applicable window(s)
    pub fn do_window_events(
        &mut self,
        c: &mut T,
        event: &winit::event::Event<U>,
        event_loop_window_target: &winit::event_loop::EventLoopWindowTarget<U>,
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
                    ControlFlow::Exit => {
                        //println!("window requested exit. Instead of sending the exit for everyone, just get rid of this one.");
                        if window.window.can_quit(c) {
                            window_control_flow.push(ControlFlow::Exit);
                            continue;
                        } else {
                            window_control_flow.push(ControlFlow::Wait);
                        }
                        //*flow = ControlFlow::Exit
                    }
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
    pub fn run(mut self, event_loop: EventLoop<U>, mut c: T) {
        event_loop.run(move |event, event_loop_window_target, flow| {
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

            // If any window requested polling, we should poll.
            // Precedence: Poll > WaitUntil(smallest) > Wait.
            if let ControlFlow::Exit = *flow {
            } else {
                *flow = ControlFlow::Wait;
                for flow_request in window_control_flow {
                    match flow_request {
                        ControlFlow::Poll => {
                            *flow = ControlFlow::Poll;
                        }
                        ControlFlow::Wait => (), // do nothing, if untouched it will be wait
                        ControlFlow::WaitUntil(when_new) => {
                            if let ControlFlow::Poll = *flow {
                                continue; // Polling takes precedence, so ignore this.
                            }

                            // The current flow is already WaitUntil. If this one is sooner, use it instead.
                            if let ControlFlow::WaitUntil(when_current) = *flow {
                                if when_new < when_current {
                                    *flow = ControlFlow::WaitUntil(when_new);
                                }
                            } else {
                                // The current flow is lower precedence, so replace it with this.
                                *flow = ControlFlow::WaitUntil(when_new);
                            }
                        }
                        ControlFlow::Exit => (), // handle differently, only exit if all windows are gone?? what do about a closed root window
                        ControlFlow::ExitWithCode(_n) => (),
                    }
                }
            }

            if self.windows.is_empty() {
                //println!("no more windows running, exiting event loop.");
                *flow = ControlFlow::Exit;
            }
        });
    }
}

/// A struct defining how a new window is to be created.
pub struct NewWindowRequest<T, U = DefaultCustomEvent> {
    /// The actual struct containing window data. The struct must implement the `TrackedWindow<T>` trait.
    pub window_state: Box<dyn TrackedWindow<T, U>>,
    /// Specifies how to build the window with a WindowBuilder
    pub builder: winit::window::WindowBuilder,
    /// Other options for the window.
    pub options: TrackedWindowOptions,
    /// An id to allow a user program to translate window requests into actual window ids.
    pub id: u32,
}

lazy_static::lazy_static! {
    static ref WINDOW_REQUEST_ID: Mutex<u32> = Mutex::new(0u32);
    static ref WINDOW_TABLE: Mutex<HashMap<u32, Option<WindowId>>> = Mutex::new(HashMap::new());
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
    let val = *l;
    val
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
