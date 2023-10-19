//! This defines the MultiWindow struct. This is the main struct used in the main function of a user application.

use std::{collections::HashMap, sync::Mutex};

use winit::
    window::WindowId
;

/// This trait is to be implemented on custom window events
pub trait EventTrait {
    /// Returns a Some when the event is for a particular window, returns None when the event is not for a particular window
    fn window_id(&self) -> Option<WindowId>;
}

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
