use bevy::prelude::*;
use serde::Serialize;
use std::any::{TypeId, type_name};
use std::collections::HashMap;
use std::path::PathBuf;
use std::ptr;
use std::sync::Mutex;
use wasmtime::{Caller, Engine, Linker, Module, Store};

type SerializeFn = Box<dyn Fn(&World) -> Option<String> + Send + Sync>;

struct ResourceEntry {
    type_id: TypeId,
    serialize: SerializeFn,
}

#[derive(Default)]
pub struct WasmPlugin {
    modules: Vec<PathBuf>,
    resources: Mutex<HashMap<String, ResourceEntry>>,
}

impl WasmPlugin {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_module(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.modules.push(path.into());
        self
    }

    pub fn add_resource<T: Resource + Serialize>(&mut self) -> &mut Self {
        let entry = ResourceEntry {
            type_id: TypeId::of::<T>(),
            serialize: Box::new(|world: &World| {
                world
                    .get_resource::<T>()
                    .and_then(|value| serde_json::to_string(value).ok())
            }),
        };
        self.resources
            .get_mut()
            .expect("resource registry mutex poisoned")
            .insert(type_name::<T>().to_string(), entry);
        self
    }
}

struct HostState {
    world: *mut World,
    resources: HashMap<String, ResourceEntry>,
    id_to_name: HashMap<u64, String>,
}

unsafe impl Send for HostState {}
unsafe impl Sync for HostState {}

#[derive(Resource)]
struct WasmRuntime {
    store: Store<HostState>,
    linker: Linker<HostState>,
    modules: Vec<(PathBuf, Module)>,
}

impl WasmRuntime {
    fn new(paths: &[PathBuf], resources: HashMap<String, ResourceEntry>) -> anyhow::Result<Self> {
        let engine = Engine::default();
        let state = HostState {
            world: ptr::null_mut(),
            resources,
            id_to_name: HashMap::new(),
        };
        let store = Store::new(&engine, state);
        let mut linker: Linker<HostState> = Linker::new(&engine);

        linker.func_wrap(
            "env",
            "host_log",
            |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| {
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .expect("guest must export 'memory'");
                let data = mem.data(&caller)[ptr as usize..(ptr + len) as usize].to_vec();
                let msg = String::from_utf8(data).expect("guest sent invalid utf-8");
                info!("{msg}");
            },
        )?;

        linker.func_wrap(
            "env",
            "get_resource_id",
            |mut caller: Caller<'_, HostState>, name_ptr: u32, name_len: u32| -> i64 {
                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .expect("guest must export 'memory'");
                let bytes =
                    mem.data(&caller)[name_ptr as usize..(name_ptr + name_len) as usize].to_vec();
                let Ok(name) = String::from_utf8(bytes) else {
                    return -1;
                };

                let state = caller.data();
                let Some(entry) = state.resources.get(&name) else {
                    return -1;
                };
                let type_id = entry.type_id;
                if state.world.is_null() {
                    return -1;
                }
                let world: &World = unsafe { &*state.world };
                let Some(component_id) = world.components().get_id(type_id) else {
                    return -1;
                };
                let id = component_id.index() as u64;

                caller.data_mut().id_to_name.insert(id, name);
                id as i64
            },
        )?;

        linker.func_wrap(
            "env",
            "get_resource",
            |mut caller: Caller<'_, HostState>, id: u64| -> u64 {
                let bytes = {
                    let state = caller.data();
                    let Some(name) = state.id_to_name.get(&id) else {
                        return 0;
                    };
                    if state.world.is_null() {
                        return 0;
                    }
                    let Some(entry) = state.resources.get(name) else {
                        return 0;
                    };
                    let world: &World = unsafe { &*state.world };
                    match (entry.serialize)(world) {
                        Some(json) => json.into_bytes(),
                        None => return 0,
                    }
                };

                let alloc = caller
                    .get_export("alloc")
                    .and_then(|e| e.into_func())
                    .expect("guest must export 'alloc'")
                    .typed::<u32, u32>(&caller)
                    .expect("guest 'alloc' has unexpected signature");
                let ptr = alloc
                    .call(&mut caller, bytes.len() as u32)
                    .expect("guest 'alloc' trapped");

                let mem = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .expect("guest must export 'memory'");
                mem.write(&mut caller, ptr as usize, &bytes)
                    .expect("failed to write resource into guest memory");

                ((ptr as u64) << 32) | bytes.len() as u64
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

    fn run(&mut self, world: &mut World) {
        let WasmRuntime {
            store,
            linker,
            modules,
        } = self;
        store.data_mut().world = world as *mut World;

        for (path, module) in modules.iter() {
            if let Err(error) = run_module(store, linker, module) {
                error!("failed to run wasm module {}: {error:#}", path.display());
            }
        }

        store.data_mut().world = ptr::null_mut();
    }
}

impl Plugin for WasmPlugin {
    fn build(&self, app: &mut App) {
        let resources = std::mem::take(
            &mut *self
                .resources
                .lock()
                .expect("resource registry mutex poisoned"),
        );
        match WasmRuntime::new(&self.modules, resources) {
            Ok(runtime) => {
                app.insert_resource(runtime)
                    .add_systems(Startup, run_modules);
            }
            Err(error) => error!("failed to set up wasm runtime: {error:#}"),
        }
    }
}

fn run_modules(world: &mut World) {
    world.resource_scope(|world, mut runtime: Mut<WasmRuntime>| {
        runtime.run(world);
    });
}

fn run_module(
    store: &mut Store<HostState>,
    linker: &Linker<HostState>,
    module: &Module,
) -> anyhow::Result<()> {
    let instance = linker.instantiate(&mut *store, module)?;
    let guest_main = instance.get_typed_func::<(), ()>(&mut *store, "main")?;
    guest_main.call(&mut *store, ())?;

    Ok(())
}
