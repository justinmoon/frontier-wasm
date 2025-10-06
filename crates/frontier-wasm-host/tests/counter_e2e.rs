use std::path::PathBuf;
use std::process::Command;

use frontier_wasm_host::{ComponentRuntime, LogicalSize};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn counter_component_artifact() -> PathBuf {
    workspace_root().join("target/wasm32-wasip1/debug/counter_component.wasm")
}

fn build_counter_component() {
    let status = Command::new("cargo")
        .current_dir(workspace_root())
        .args(["component", "build", "-p", "counter-component"])
        .status()
        .expect("failed to spawn cargo component build");
    assert!(status.success(), "cargo component build failed");
    assert!(
        counter_component_artifact().exists(),
        "component artifact missing after build"
    );
}

#[test]
fn counter_component_lifecycle() {
    build_counter_component();

    let component_path = counter_component_artifact();
    let mut runtime = ComponentRuntime::new(component_path).expect("instantiate runtime");

    let init = runtime
        .call_init(LogicalSize {
            width: 800.0,
            height: 600.0,
            scale_factor: 1.0,
        })
        .expect("call init");
    assert!(init.requested_redraw, "guest should request redraw on init");

    let frame = runtime.call_frame(16.0).expect("call frame");
    assert!(
        frame.frame.commands.len() > 0,
        "frame should contain drawing commands"
    );
}
