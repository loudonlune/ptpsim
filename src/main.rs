
use std::sync::Arc;

use clap::Parser;
use tokio::sync::RwLock;

use crate::{simulation::Simulation, topology::Topology};

mod api;
mod netns;
mod netdevsim;
mod node;
mod topology;
mod simulation;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "topology.toml", help = "Path to the topology file in TOML format")]
    topology: String,
    #[arg(short, long, default_value = "ptpsim_node_logs", help = "Directory to store node logs")]
    output_dir: String,
    #[arg(short, long, help = "Run in interactive mode")]
    interactive: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    tokio::fs::create_dir_all(&args.output_dir).await.expect("Failed to create log directory");

    // Load topology file
    let topology: Topology = toml::from_str(&std::fs::read_to_string(&args.topology).expect("Failed to read topology file"))
        .expect("Failed to parse topology");

    let simulation = Arc::new(RwLock::new(Simulation::from_topology(topology, args).await.expect("Failed to create simulation from topology")));

    api::run_api_server(simulation).await;
}
