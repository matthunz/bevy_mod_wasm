#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn host_log(ptr: *const u8, len: u32);
}

fn log(msg: &str) {
    unsafe { host_log(msg.as_ptr(), msg.len() as u32) }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() {
    log("Hello, world!");
}
