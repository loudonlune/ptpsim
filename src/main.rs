
use clap::Parser;

use crate::topology::Topology;

mod api;
mod netns;
mod netdevsim;
mod node;
mod topology;
mod simulation;

#[derive(clap::Parser)]
struct Args {
    #[clap(short, long, default_value = "topology.toml")]
    topology: String,
    #[clap(short, long, default_value = "ptpsim_node_logs")]
    output_dir: String,
    #[clap(short, long)]
    interactive: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::try_parse().expect("Failed to parse arguments");

    tokio::fs::create_dir_all(&args.output_dir).await.expect("Failed to create log directory");

    // Load topology file
    let topology: Topology = toml::from_str(&std::fs::read_to_string(&args.topology).expect("Failed to read topology file"))
        .expect("Failed to parse topology");

    let simulation = simulation::Simulation::from_topology(topology, args).await.expect("Failed to create simulation from topology");

    api::start_server(simulation).await;
}
