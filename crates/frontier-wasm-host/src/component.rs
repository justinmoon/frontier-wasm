#![allow(clippy::all)]

wasmtime::component::bindgen!({
    path: "../../wit/vello",
    world: "canvas-app",
});
