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

    pub fn capability_callback(_args: &[u8]) -> Option<i32> {
        unsafe {
            println!("{:?}", resultCapabilityCallbackData);
        }
        None
    }

    pub fn set_output_debug(debug_mode: usize) {
        unsafe {
            Output_DebugMode = debug_mode as u8;
        }
    }
}

pub mod input {
    use kiibohd_sys::*;

    pub fn trigger(key: u8, typ: u8, state: u8) {
        unsafe {
            Scan_setTriggerCode(key, typ, state);
        }
    }

    pub fn press(key: u8, typ: u8) {
        unsafe {
            Scan_addScanCode(key, typ);
        }
    }

    pub fn release(key: u8, typ: u8) {
        unsafe {
            Scan_removeScanCode(key, typ);
        }
    }

    pub fn apply_layer(state: u8, layer: u16, layer_state: bool) {
        unsafe {
            let trigger = std::ptr::null_mut();
            let state_type = TriggerType_TriggerType_Switch1;
            Layer_layerStateSet(trigger, state, state_type as u8, layer, layer_state as u8);
        }
    }

    pub fn lock_layer(layer: u16) {
        unsafe {
            let trigger = std::ptr::null_mut();
            let state = ScheduleState_ScheduleType_P;
            let state_type = TriggerType_TriggerType_Switch1;
            let layer_state = LayerStateType_LayerStateType_Lock;
            Layer_layerStateSet(trigger, state as u8, state_type as u8, layer, layer_state as u8);
        }
    }

    pub fn clear_layers() {
        unsafe {
            Layer_clearLayers();
        }
    }

    pub fn get_layer_state() {
        unsafe {
            //LayerNum_host
            dbg!(LayerState);
            //macroLayerIndexStackSize
            dbg!(macroLayerIndexStack);
        }
    }

    pub fn set_kbd_protocol(nkro: bool) {
        unsafe {
            USBKeys_Protocol_New = nkro as u8;
            USBKeys_Protocol_Change = 1;
        }
    }

    pub fn set_macro_debug(debug_mode: usize) {
        unsafe {
            macroDebugMode = debug_mode as u8;
        }
    }

    pub fn set_cap_debug(debug_enabled: bool) {
        unsafe {
            capDebugMode = debug_enabled as u8;
        }
    }
    pub fn set_vote_debug(debug_enabled: bool) {
        unsafe {
            voteDebugMode = debug_enabled as u8;
        }
    }

    pub fn set_layer_debug(debug_enabled: bool) {
        unsafe {
            layerDebugMode = debug_enabled as u8;
        }
    }

    pub fn set_trigger_debug(debug_enabled: bool) {
        unsafe {
            triggerPendingDebugMode = debug_enabled as u8;
        }
    }

    pub fn layer_callback(_args: &[u8]) -> Option<i32> {
        get_layer_state();
        None
    }

    pub fn animation_display_buffers() -> Vec<Vec<u32>> {
        unsafe {
            let num_buffers = Pixel_Buffers_HostLen;
            let pixelbufs = std::slice::from_raw_parts(&Pixel_Buffers as *const PixelBuf, num_buffers as usize);

            let mut outputbufs = vec![];
            for buf in pixelbufs.iter() {
                let data: Vec<u32> = match buf.width {
                    8 => {
                        let s = std::slice::from_raw_parts(buf.data as *const u8, buf.size as usize);
                        s.into_iter().map(|x| *x as u32).collect()
                    },
                    16 => {
                        let s = std::slice::from_raw_parts(buf.data as *const u16, buf.size as usize);
                        s.into_iter().map(|x| *x as u32).collect()
                    }
                    32 => std::slice::from_raw_parts(buf.data as *const u32, buf.size as usize).to_vec(),
                    _ => panic!("Unsupported pixel width {}", buf.width),
                };
                outputbufs.push(data);
            }

            println!("{:?}", outputbufs);
            outputbufs
        }
    }

    pub fn rect_disp() {
        unsafe {
            Pixel_dispBuffer();
        }
    }

    pub fn add_animation(index: usize) {
        unsafe {
            let mut elt = AnimationStackElement {
                trigger: std::ptr::null_mut(),
                index: index as u16,
                pos: 0,
                subpos: 0,
                loops: 1,
                framedelay: 0,
                frameoption: PixelFrameOption_PixelFrameOption_None,
                ffunc: PixelFrameFunction_PixelFrameFunction_Interpolation, //Off,
                pfunc: PixelPixelFunction_PixelPixelFunction_PointInterpolation, //Off,
                replace: AnimationReplaceType_AnimationReplaceType_ClearActive, //None,
                state: AnimationPlayState_AnimationPlayState_AutoStart,
            };
            Pixel_addAnimation(&mut elt, CapabilityState_CapabilityState_None);
        }
    }

    pub fn animation_stack_info() -> AnimationStack {
        unsafe {
            //Pixel_AnimationStack_HostSize
            Pixel_AnimationStack
        }
    }
}

pub mod data {
    use kiibohd_sys::*;

    pub fn usb_keyboard() -> USBKeys {
        unsafe {
            USBKeys_primary
        }
    }

    pub fn trigger_list_buffer() -> Vec<TriggerEvent> {
        unsafe {
            macroTriggerEventBuffer.to_vec()
        }
    }

    pub fn pending_trigger_list() -> Vec<u16> {
        unsafe {
            // bindgen treats this as a 0 length array, so we have to cast this ourselves
            std::slice::from_raw_parts(&macroTriggerMacroPendingList as *const u16, macroTriggerMacroPendingListSize as usize).to_vec()
        }
    }

    pub fn pending_result_list() -> ResultsPending {
        unsafe {
            macroResultMacroPendingList
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init() {
        control::add_cmd("serial_write", output::serial_write);
        control::add_cmd("keyboard_send", output::keyboard_send);
        control::add_cmd("mouse_send", output::mouse_send);
        control::add_cmd("serial_read", output::serial_read);
        control::add_cmd("serial_available", output::serial_available);
        control::add_cmd("layerState", input::layer_callback);
        control::add_cmd("capabilityCallback", output::capability_callback);
        /*control::add_cmd("rawio_available", output::rawio_available);
        control::add_cmd("rawio_rx", output::rawio_available);
        control::add_cmd("rawio_tx", output::rawio_available);*/
        control::init();
    }

    #[test]
    fn output_test() {
        init();
        input::set_macro_debug(2);
        input::set_vote_debug(true);
        input::set_layer_debug(true);
        input::set_trigger_debug(true);
        input::set_trigger_debug(true);
        output::set_output_debug(2);

        control::process(1);
        input::press(0x01, 0);
        control::process(1);
        println!("TPending {:?}", data::pending_trigger_list());
    }

    #[test]
    fn animation_test() {
        init();
        input::add_animation(13); //rainbow
        dbg!(input::animation_stack_info());

        control::process(1);
        input::rect_disp();

        control::process(1);
        input::rect_disp();

        input::animation_display_buffers();
    }
}
