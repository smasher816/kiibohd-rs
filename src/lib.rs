pub mod control {
    use kiibohd_sys::*;
    use std::ffi::*;
    use std::os::raw::*;

    use lazy_static::lazy_static;
    use log::{debug, info, error};
    use std::collections::HashMap;
    use std::sync::Mutex;

    type Callback = dyn Fn(&str) + Send;

    lazy_static! {
        static ref CALLBACKS: Mutex<HashMap<String, Box<Callback>>> = Mutex::new(HashMap::new());
    }

    unsafe extern "C" fn callback(cmd: *const c_char, args: *const c_char) {
        let cmd = CStr::from_ptr(cmd).to_str().unwrap();
        let args = CStr::from_ptr(args).to_str().unwrap();
        exec(cmd, args);
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
        F: Fn(&str) + Send + 'static,
    {
        let mut dict = CALLBACKS.lock().unwrap();
        dict.insert(name.into(), Box::new(f));
    }

    pub fn exec(cmd: &str, args: &str) {
        let dict = CALLBACKS.lock().unwrap();
        match dict.get(cmd) {
            Some(callback) => {
                debug!("Exec: {} {}", cmd, args);
                callback(args);
            }
            None => error!("Unhandled callback: {}", cmd),
        }
    }
}

pub mod output {
    pub fn serial_write(text: &str) {
        print!("{}", text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_test() {
        control::add_cmd("serial_write", output::serial_write);
        control::init();
    }
}
