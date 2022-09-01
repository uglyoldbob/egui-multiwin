use glutin::event_loop::{ControlFlow, EventLoop};

use crate::tracked_window::{DisplayCreationError, TrackedWindowContainer, TrackedWindow};

/// Manages multiple `TrackedWindow`s by forwarding events to them.
pub struct MultiWindow<T> {
    windows: Vec<Box<TrackedWindowContainer<T>>>,
}

impl<T: 'static> MultiWindow<T> {
    /// Creates a new `MultiWindow`.
    pub fn new() -> Self {
        MultiWindow { windows: vec![]
        }
    }

    /// Adds a new `TrackedWindow` to the `MultiWindow`.
    pub fn add<TE>(
        &mut self,
        window: NewWindowRequest<T>,
        event_loop: &glutin::event_loop::EventLoopWindowTarget<TE>,
    ) -> Result<(), DisplayCreationError> {
        Ok(self.windows.push(Box::new(TrackedWindowContainer::create(
            window.window_state,
            window.builder,
            event_loop,
        )?)))
    }

    pub fn do_window_events(
        &mut self,
        c: &mut T,
        event: &glutin::event::Event<()>,
        event_loop_window_target: &glutin::event_loop::EventLoopWindowTarget<()>,
    ) -> Vec<ControlFlow> {
        let mut handled_windows = vec![];
        let mut window_control_flow = vec![];

        let mut root_window_exists = false;
        for other in &self.windows {
            if (*other).window.is_root() {
                root_window_exists = true;
            }
        }

        while let Some(mut window) = self.windows.pop() {
            if window.is_event_for_window(&event) {
                let window_control =
                    window.handle_event_outer(c, &event, event_loop_window_target, root_window_exists);
                match window_control.requested_control_flow {
                    ControlFlow::Exit => {
                        //println!("window requested exit. Instead of sending the exit for everyone, just get rid of this one.");
                        window_control_flow.push(ControlFlow::Exit);
                        continue;
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
    pub fn run(mut self, event_loop: EventLoop<()>, mut c: T) {
        event_loop.run(move |event, event_loop_window_target, flow| {
            let c = &mut c;
            //println!("handling event {:?}", event);
            let window_control_flow = self.do_window_events(c, &event, &event_loop_window_target);

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

pub struct NewWindowRequest<T> {
    pub window_state: Box<dyn TrackedWindow<Data=T>>,
    pub builder: glutin::window::WindowBuilder,
}
