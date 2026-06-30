use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_mod_wasm::WasmPlugin;

fn main() {
    let mut wasm = WasmPlugin::new();
    wasm.add_module(env!("GUEST_WASM_PATH"));

    App::new()
        .add_plugins(LogPlugin::default())
        .add_plugins(wasm)
        .run();
}
