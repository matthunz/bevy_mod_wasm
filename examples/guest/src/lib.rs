use bevy_mod_wasm::{World, log};
use bevy_mod_wasm_example_core::Score;

#[bevy_mod_wasm::main]
fn main() {
    let mut world = World::new();
    let score = world.get_resource::<Score>().unwrap();
    log(&score.value.to_string());
}
