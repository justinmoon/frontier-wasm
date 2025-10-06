use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;
use winit::event_loop::EventLoop;

use frontier_wasm_host::app::App;

#[derive(Parser, Debug)]
#[command(author, version, about = "Frontier WASM canvas prototype host")]
struct Args {
    #[arg(
        long,
        value_name = "WASM_COMPONENT",
        help = "Path to the guest component (.wasm)"
    )]
    component: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .compact()
        .init();

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    let mut app = App::new(args.component);
    event_loop.run_app(&mut app)?;
    Ok(())
}
