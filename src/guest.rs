use serde::de::DeserializeOwned;
use std::any::type_name;
use std::collections::HashMap;

pub use bevy_mod_wasm_macros::main;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn host_log(ptr: *const u8, len: u32);
    fn get_resource_id(name_ptr: *const u8, name_len: u32) -> i64;
    fn get_resource(id: u64) -> u64;
}

pub fn log(msg: &str) {
    unsafe { host_log(msg.as_ptr(), msg.len() as u32) }
}

#[doc(hidden)]
pub fn __alloc(size: u32) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(size as usize);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[macro_export]
macro_rules! guest_exports {
    () => {
        #[unsafe(no_mangle)]
        pub extern "C" fn alloc(size: u32) -> *mut u8 {
            $crate::__alloc(size)
        }
    };
}

#[derive(Default)]
pub struct World {
    resources: HashMap<String, u64>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_resource<T: DeserializeOwned>(&mut self) -> Option<T> {
        let name = type_name::<T>();

        let id = match self.resources.get(name) {
            Some(id) => *id,
            None => {
                let resolved = unsafe { get_resource_id(name.as_ptr(), name.len() as u32) };
                if resolved < 0 {
                    return None;
                }
                let id = resolved as u64;
                self.resources.insert(name.to_string(), id);
                id
            }
        };

        let packed = unsafe { get_resource(id) };
        if packed == 0 {
            return None;
        }
        let ptr = (packed >> 32) as *mut u8;
        let len = (packed & 0xffff_ffff) as usize;

        let bytes = unsafe { Vec::from_raw_parts(ptr, len, len) };
        serde_json::from_slice(&bytes).ok()
    }
}
