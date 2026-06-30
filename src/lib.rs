use bevy::prelude::*;
use std::path::PathBuf;
use wasmtime::{Caller, Engine, Linker, Module, Store};

#[derive(Default)]
pub struct WasmPlugin {
    modules: Vec<PathBuf>,
}

impl WasmPlugin {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_module(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.modules.push(path.into());
        self
    }
}

#[derive(Resource)]
struct WasmRuntime {
    store: Store<()>,
    linker: Linker<()>,
    modules: Vec<(PathBuf, Module)>,
}

impl WasmRuntime {
    fn new(paths: &[PathBuf]) -> anyhow::Result<Self> {
        let engine = Engine::default();
        let store = Store::new(&engine, ());
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
                info!("{msg}");
            },
        )?;

        let modules = paths
            .iter()
            .map(|path| Ok((path.clone(), Module::from_file(&engine, path)?)))
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(Self {
            store,
            linker,
            modules,
        })
    }

    fn run(&mut self) {
        let WasmRuntime {
            store,
            linker,
            modules,
        } = self;
        for (path, module) in modules.iter() {
            if let Err(error) = run_module(store, linker, module) {
                error!("failed to run wasm module {}: {error:#}", path.display());
            }
        }
    }
}

impl Plugin for WasmPlugin {
    fn build(&self, app: &mut App) {
        match WasmRuntime::new(&self.modules) {
            Ok(runtime) => {
                app.insert_resource(runtime)
                    .add_systems(Startup, run_modules);
            }
            Err(error) => error!("failed to set up wasm runtime: {error:#}"),
        }
    }
}

fn run_modules(mut runtime: ResMut<WasmRuntime>) {
    runtime.run();
}

fn run_module(store: &mut Store<()>, linker: &Linker<()>, module: &Module) -> anyhow::Result<()> {
    let instance = linker.instantiate(&mut *store, module)?;
    let guest_main = instance.get_typed_func::<(), ()>(&mut *store, "main")?;
    guest_main.call(&mut *store, ())?;

    Ok(())
}
