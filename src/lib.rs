pub mod control {
    use kiibohd_sys::*;
    use std::ffi::*;
    use std::os::raw::*;

    use lazy_static::lazy_static;
    use log::{debug, error, info, trace};
    use std::collections::HashMap;
    use std::sync::Mutex;

    type Callback = dyn Fn(&[u8]) -> i32 + Send;

    lazy_static! {
        static ref CALLBACKS: Mutex<HashMap<String, Box<Callback>>> = Mutex::new(HashMap::new());
    }

    unsafe extern "C" fn callback(cmd: *const c_char, args: *const c_char) -> i32 {
        let cmd = CStr::from_ptr(cmd).to_str().unwrap();
        let args = CStr::from_ptr(args).to_bytes_with_nul();
        exec(cmd, args)
    }

    pub fn init() {
        unsafe {
            let _ = env_logger::try_init();

            Host_register_callback(callback as *mut c_void);
            Host_callback_test();

            info!("Host_init");
            Host_init();
        }
    }

    pub fn add_cmd<T, F>(name: T, f: F)
    where
        T: Into<String>,
        F: Fn(&[u8]) -> Option<i32> + Send + 'static,
    {
        let mut dict = CALLBACKS.lock().unwrap();
        dict.insert(
            name.into(),
            Box::new(move |args| {
                let ret = f(args);
                ret.unwrap_or(1) // Success in C code
            }),
        );
    }

    pub fn exec(cmd: &str, args: &[u8]) -> i32 {
        let dict = CALLBACKS.lock().unwrap();
        match dict.get(cmd) {
            Some(callback) => {
                trace!("Exec: {} {:?}", cmd, args);
                callback(args)
            }
            None => {
                error!("Unhandled callback: {}", cmd);
                0
            }
        }
    }

    pub fn process(number_of_loops: usize) {
        for i in 0..number_of_loops {
            debug!("Host Process ({})", i);
            unsafe {
                Host_process();
            }
        }
    }
}

pub mod output {
    use kiibohd_sys::*;

    pub fn serial_available(_args: &[u8]) -> Option<i32> {
        Some(0)
    }

    pub fn serial_read(_args: &[u8]) -> Option<i32> {
        None
    }

    pub fn serial_write(args: &[u8]) -> Option<i32> {
        print!("{}", std::str::from_utf8(args).unwrap_or(""));
        None
    }

    pub fn keyboard_send(_args: &[u8]) -> Option<i32> {
        unsafe {
            //println!("Size: {}", USBKeys_BitfieldSize);
            //println!("Protocol: {}", USBKeys_Protocol);
            println!("{:?}", USBKeys_primary);

            // Indicate we are done with the buffer
            USBKeys_primary.changed = 0;
        }
        None
    }

    pub fn mouse_send(_args: &[u8]) -> Option<i32> {
        unsafe {
            println!("{:?}", USBMouse_primary);

            // Indicate we are done with the buffer
            USBMouse_primary.changed = 0;
        }
        None
    }
}

pub mod data {
    pub fn usb_keyboard() {}
    pub fn trigger_list_buffer() {}
    pub fn pending_trigger_list() {}
    pub fn pending_result_list() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_test() {
        control::add_cmd("serial_write", output::serial_write);
        control::add_cmd("keyboard_send", output::keyboard_send);
        control::add_cmd("mouse_send", output::mouse_send);
        control::add_cmd("serial_read", output::serial_read);
        control::add_cmd("serial_available", output::serial_available);
        control::add_cmd("layerState", output::serial_available);
        control::init();
        control::process(3);
    }
}
