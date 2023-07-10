//! This defines the MultiWindow struct. This is the main struct used in the main function of a user application.

use std::collections::HashMap;

use winit::event_loop::{ControlFlow, EventLoop};

use crate::tracked_window::{
    DisplayCreationError, TrackedWindow, TrackedWindowContainer, TrackedWindowOptions,
};

/// This trait allows for non-window specific events to be sent to the event loop.
/// It allows for non-gui threads or code to interact with the gui through the common struct
pub trait CommonEventHandler<T, U> {
    /// Process non-window specific events for the application
    fn process_event(&mut self, event: U) -> Vec<NewWindowRequest<T>>;
}

/// The main struct of the crate. Manages multiple `TrackedWindow`s by forwarding events to them.
pub struct MultiWindow<T, U> {
    /// The windows for the application.
    windows: Vec<TrackedWindowContainer<T, U>>,
    /// A list of fonts to install on every egui instance
    fonts: HashMap<String, egui::FontData>,
}

impl<T: 'static + CommonEventHandler<T, U>, U: 'static> Default for MultiWindow<T, U> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static + CommonEventHandler<T, U>, U: 'static> MultiWindow<T, U> {
    /// Creates a new `MultiWindow`.
    pub fn new() -> Self {
        MultiWindow {
            windows: vec![],
            fonts: HashMap::new(),
        }
    }

    /// Add a font that is applied to every window. Be sure to call this before calling [add](crate::multi_window::MultiWindow::add)
    /// multi_window is an instance of [MultiWindow<T,U>](crate::multi_window::MultiWindow<T,U>), DATA is a static `&[u8]` - most like defined with a `include_bytes!()` macro
    /// ```
    /// multi_window.add_font("my_font".to_string(), egui_multiwin::egui::FontData::from_static(DATA));
    /// ```
    pub fn add_font(&mut self, name: String, fd: egui::FontData) {
        self.fonts.insert(name, fd);
    }

    /// Adds a new `TrackedWindow` to the `MultiWindow`. If custom fonts are desired, call [add_font](crate::multi_window::MultiWindow::add_font) first.
    pub fn add<TE>(
        &mut self,
        window: NewWindowRequest<T>,
        event_loop: &winit::event_loop::EventLoopWindowTarget<TE>,
    ) -> Result<(), DisplayCreationError> {
        self.windows.push(TrackedWindowContainer::create::<TE>(
            window.window_state,
            window.builder,
            event_loop,
            &window.options,
        )?);
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
                    let _e = self.add(new_window_request, event_loop_window_target);
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
            let window_control_flow = if let winit::event::Event::UserEvent(event) = event {
                for w in c.process_event(event) {
                    let _e = self.add(w, event_loop_window_target);
                }
                vec![ControlFlow::Poll]
            } else {
                self.do_window_events(c, &event, event_loop_window_target)
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
pub struct NewWindowRequest<T> {
    /// The actual struct containing window data. The struct must implement the `TrackedWindow<T>` trait.
    pub window_state: Box<dyn TrackedWindow<T>>,
    /// Specifies how to build the window with a WindowBuilder
    pub builder: winit::window::WindowBuilder,
    /// Other options for the window.
    pub options: TrackedWindowOptions,
}
