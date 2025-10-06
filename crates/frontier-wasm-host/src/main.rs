use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, ValueHint};
use tracing_subscriber::EnvFilter;
use winit::event_loop::EventLoop;

use frontier_wasm_host::{app::App, ComponentSource};

const EMBEDDED_COUNTER_LABEL: &str = "embedded counter demo";
const EMBEDDED_COUNTER_COMPONENT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/counter-component.wasm"
));

#[derive(Parser, Debug)]
#[command(author, version, about = "Frontier WASM canvas prototype host")]
struct Args {
    #[arg(
        long,
        value_name = "WASM_COMPONENT",
        value_hint = ValueHint::FilePath,
        help = "Path to the guest component (.wasm). Omit to use the embedded counter demo."
    )]
    component: Option<PathBuf>,
}

fn main() -> Result<()> {
    let Args { component } = Args::parse();

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .compact()
        .init();

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    let component_source = if let Some(path) = component {
        ComponentSource::from_path(path)
    } else {
        tracing::info!("No --component provided; using embedded counter demo component.");
        ComponentSource::embedded(EMBEDDED_COUNTER_LABEL, EMBEDDED_COUNTER_COMPONENT)
    };

    let mut app = App::new(component_source);
    event_loop.run_app(&mut app)?;
    Ok(())
}
