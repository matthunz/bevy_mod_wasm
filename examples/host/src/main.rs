use wasmtime::{Caller, Engine, Linker, Module, Store};

const GUEST_WASM: &[u8] = include_bytes!(env!("GUEST_WASM_PATH"));

fn main() -> anyhow::Result<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, GUEST_WASM)?;
    let mut linker: Linker<()> = Linker::new(&engine);

    linker.func_wrap(
        "env",
        "host_log",
        |mut caller: Caller<'_, ()>, ptr: i32, len: i32| {
            let mem = caller
                .get_export("memory")
                .and_then(|e| e.into_memory())
                .expect("guest must export 'memory'");
            let data = mem.data(&caller)[ptr as usize..(ptr + len) as usize].to_vec();
            let msg = String::from_utf8(data).expect("guest sent invalid utf-8");
            println!("{msg}");
        },
    )?;

    let instance = linker.instantiate(&mut store, &module)?;
    let guest_main = instance.get_typed_func::<(), ()>(&mut store, "main")?;
    guest_main.call(&mut store, ())?;

    Ok(())
}
