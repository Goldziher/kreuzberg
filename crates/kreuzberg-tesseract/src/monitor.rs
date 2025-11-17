use std::os::raw::{c_int, c_void};
use std::sync::{Arc, Mutex};

pub struct TessMonitor {
    handle: Arc<Mutex<*mut c_void>>,
}

unsafe impl Send for TessMonitor {}
unsafe impl Sync for TessMonitor {}

impl TessMonitor {
    /// Creates a new instance of the TessMonitor.
    ///
    /// # Returns
    ///
    /// Returns the new instance of the TessMonitor.
    pub fn new() -> Self {
        let handle = unsafe { TessMonitorCreate() };
        TessMonitor {
            handle: Arc::new(Mutex::new(handle)),
        }
    }

    /// Sets the deadline for the monitor.
    ///
    /// # Arguments
    ///
    /// * `deadline` - Deadline in milliseconds.
    pub fn set_deadline(&self, deadline: i32) {
        let handle = self.handle.lock().unwrap();
        unsafe { TessMonitorSetDeadlineMSecs(*handle, deadline) };
    }

    /// Gets the progress of the monitor.
    ///
    /// # Returns
    ///
    /// Returns the progress as an `i32`.
    pub fn get_progress(&self) -> i32 {
        let handle = self.handle.lock().unwrap();
        unsafe { TessMonitorGetProgress(*handle) }
    }
}

impl Drop for TessMonitor {
    fn drop(&mut self) {
        let handle = self.handle.lock().unwrap();
        unsafe { TessMonitorDelete(*handle) };
    }
}

unsafe extern "C" {
    pub fn TessMonitorCreate() -> *mut c_void;
    pub fn TessMonitorDelete(monitor: *mut c_void);
    pub fn TessMonitorSetDeadlineMSecs(monitor: *mut c_void, deadline: c_int);
    pub fn TessMonitorGetProgress(monitor: *mut c_void) -> c_int;
}
