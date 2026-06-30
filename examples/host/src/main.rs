use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_mod_wasm::WasmPlugin;
use bevy_mod_wasm_example_core::Score;

fn main() {
    let mut wasm = WasmPlugin::new();
    wasm.add_module(env!("GUEST_WASM_PATH"))
        .add_resource::<Score>();

    App::new()
        .add_plugins(LogPlugin::default())
        .insert_resource(Score { value: 42 })
        .add_plugins(wasm)
        .run();
}
