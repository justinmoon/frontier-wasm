use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};

use crate::component;
use crate::component::exports::vello::canvas::app as guest_app;
use crate::host::{FrameOutput, HostCtx, Phase};
use crate::model::{KeyEvent, LogicalSize, Modifiers, PointerEvent, PointerKind};

struct StoreState {
    host: HostCtx,
    table: ResourceTable,
    wasi: WasiCtx,
}

impl StoreState {
    fn new() -> Result<Self> {
        let wasi = WasiCtxBuilder::new().inherit_stdio().build();
        Ok(Self {
            host: HostCtx::new(),
            table: ResourceTable::new(),
            wasi,
        })
    }
}

impl WasiView for StoreState {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

#[derive(Clone)]
pub enum ComponentSource {
    Path(PathBuf),
    Embedded {
        bytes: Arc<[u8]>,
        label: &'static str,
    },
}

impl ComponentSource {
    pub fn from_path<P: Into<PathBuf>>(path: P) -> Self {
        Self::Path(path.into())
    }

    pub fn embedded(label: &'static str, bytes: &'static [u8]) -> Self {
        Self::Embedded {
            bytes: Arc::from(bytes),
            label,
        }
    }
}

pub struct ComponentRuntime {
    source: ComponentSource,
    engine: Engine,
    component: Component,
    store: Store<StoreState>,
    bindings: component::CanvasApp,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CallResult {
    pub requested_redraw: bool,
}

#[derive(Debug)]
pub struct FrameResult {
    pub requested_redraw: bool,
    pub frame: FrameOutput,
}

impl ComponentRuntime {
    pub fn new(source: ComponentSource) -> Result<Self> {
        let engine = Self::build_engine()?;
        let component = Self::load_component(&engine, &source)?;
        let (store, bindings) = Self::instantiate(&engine, &component)?;

        Ok(Self {
            source,
            engine,
            component,
            store,
            bindings,
        })
    }

    pub fn reload(&mut self) -> Result<()> {
        self.component = Self::load_component(&self.engine, &self.source)?;
        let (store, bindings) = Self::instantiate(&self.engine, &self.component)?;
        self.store = store;
        self.bindings = bindings;
        Ok(())
    }

    pub fn call_init(&mut self, size: LogicalSize) -> Result<CallResult> {
        self.invoke(Phase::Init, |bindings, store| {
            bindings
                .vello_canvas_app()
                .call_init(store, to_wit_logical_size(size))
        })
    }

    pub fn call_resize(&mut self, size: LogicalSize) -> Result<CallResult> {
        self.invoke(Phase::Resize, |bindings, store| {
            bindings
                .vello_canvas_app()
                .call_resize(store, to_wit_logical_size(size))
        })
    }

    pub fn call_pointer_down(&mut self, event: &PointerEvent) -> Result<CallResult> {
        self.invoke(Phase::Event, |bindings, store| {
            bindings
                .vello_canvas_app()
                .call_pointer_down(store, to_wit_pointer_event(event))
        })
    }

    pub fn call_pointer_up(&mut self, event: &PointerEvent) -> Result<CallResult> {
        self.invoke(Phase::Event, |bindings, store| {
            bindings
                .vello_canvas_app()
                .call_pointer_up(store, to_wit_pointer_event(event))
        })
    }

    pub fn call_pointer_move(&mut self, event: &PointerEvent) -> Result<CallResult> {
        self.invoke(Phase::Event, |bindings, store| {
            bindings
                .vello_canvas_app()
                .call_pointer_move(store, to_wit_pointer_event(event))
        })
    }

    pub fn call_key_down(&mut self, event: &KeyEvent) -> Result<CallResult> {
        let evt = to_wit_key_event(event);
        self.invoke(Phase::Event, move |bindings, store| {
            bindings.vello_canvas_app().call_key_down(store, &evt)
        })
    }

    pub fn call_key_up(&mut self, event: &KeyEvent) -> Result<CallResult> {
        let evt = to_wit_key_event(event);
        self.invoke(Phase::Event, move |bindings, store| {
            bindings.vello_canvas_app().call_key_up(store, &evt)
        })
    }

    pub fn call_frame(&mut self, dt_ms: f32) -> Result<FrameResult> {
        let phase = Phase::Frame;
        {
            let data = self.store.data_mut();
            data.host.enter_phase(phase);
        }

        let call_result = self
            .bindings
            .vello_canvas_app()
            .call_frame(&mut self.store, dt_ms);

        let (frame, requested) = {
            let data = self.store.data_mut();
            let requested = data.host.take_redraw_request();
            let frame = data.host.take_frame_output();
            data.host.exit_phase();
            (frame, requested)
        };

        call_result.context("guest frame call failed")?;

        Ok(FrameResult {
            requested_redraw: requested,
            frame,
        })
    }

    pub fn recent_logs(&self) -> Vec<String> {
        self.store.data().host.recent_logs_snapshot()
    }

    fn invoke<F>(&mut self, phase: Phase, f: F) -> Result<CallResult>
    where
        F: FnOnce(&component::CanvasApp, &mut Store<StoreState>) -> wasmtime::Result<()>,
    {
        {
            let data = self.store.data_mut();
            data.host.enter_phase(phase);
        }

        let result = f(&self.bindings, &mut self.store);

        let requested = {
            let data = self.store.data_mut();
            let requested = data.host.take_redraw_request();
            data.host.exit_phase();
            requested
        };

        result.context("guest call failed")?;

        Ok(CallResult {
            requested_redraw: requested,
        })
    }

    fn build_engine() -> Result<Engine> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        Engine::new(&config).context("failed to initialise Wasmtime engine")
    }

    fn load_component(engine: &Engine, source: &ComponentSource) -> Result<Component> {
        match source {
            ComponentSource::Path(path) => Component::from_file(engine, path)
                .with_context(|| format!("failed to load component from {}", path.display())),
            ComponentSource::Embedded { bytes, label } => {
                Component::from_binary(engine, bytes.as_ref())
                    .with_context(|| format!("failed to load {label} component"))
            }
        }
    }

    fn instantiate(
        engine: &Engine,
        component: &Component,
    ) -> Result<(Store<StoreState>, component::CanvasApp)> {
        let mut linker = Linker::new(engine);
        wasmtime_wasi::add_to_linker_sync(&mut linker).context("failed to add WASI to linker")?;
        component::vello::canvas::host::add_to_linker(&mut linker, |state: &mut StoreState| {
            &mut state.host
        })
        .context("failed to add host bindings to linker")?;

        let store_state = StoreState::new()?;
        let mut store = Store::new(engine, store_state);
        let bindings = component::CanvasApp::instantiate(&mut store, component, &linker)
            .context("failed to instantiate component")?;
        Ok((store, bindings))
    }
}

fn to_wit_logical_size(size: LogicalSize) -> guest_app::LogicalSize {
    guest_app::LogicalSize {
        width: size.width,
        height: size.height,
        scale_factor: size.scale_factor,
    }
}

fn to_wit_pointer_event(event: &PointerEvent) -> guest_app::PointerEvent {
    guest_app::PointerEvent {
        kind: match event.kind {
            PointerKind::Mouse => guest_app::PointerKind::Mouse,
            PointerKind::Touch => guest_app::PointerKind::Touch,
            PointerKind::Pen => guest_app::PointerKind::Pen,
        },
        position: component::vello::canvas::math::Vec2 {
            x: event.position[0],
            y: event.position[1],
        },
        buttons: guest_app::PointerButton {
            primary: event.buttons.primary,
            secondary: event.buttons.secondary,
        },
        modifiers: to_wit_modifiers(event.modifiers),
        pointer_id: event.pointer_id,
    }
}

fn to_wit_key_event(event: &KeyEvent) -> guest_app::KeyEvent {
    guest_app::KeyEvent {
        key: event.key.clone(),
        code: event.code.clone(),
        modifiers: to_wit_modifiers(event.modifiers),
        is_repeat: event.is_repeat,
    }
}

fn to_wit_modifiers(mods: Modifiers) -> guest_app::Modifiers {
    guest_app::Modifiers {
        shift: mods.shift,
        ctrl: mods.ctrl,
        alt: mods.alt,
        meta: mods.meta,
    }
}
