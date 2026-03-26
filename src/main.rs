
use std::sync::Arc;

use clap::Parser;
use tokio::sync::RwLock;

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
    #[clap(short, long)]
    interactive: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::try_parse().expect("Failed to parse arguments");

    tokio::fs::create_dir_all(node::LOG_BASE_DIR).await.expect("Failed to create log directory");

    // Load topology file
    let topology: Topology = toml::from_str(&std::fs::read_to_string(&args.topology).expect("Failed to read topology file"))
        .expect("Failed to parse topology");

    let simulation = simulation::Simulation::from_topology(topology).await.expect("Failed to create simulation from topology");

    api::start_server(simulation).await;

    // let namespace_names: HashSet<String> = HashSet::from_iter(vec!["ns1".to_string(), "ns2".to_string()]);
    // let namespaces = NetNamespace::create_namespaces(namespace_names).await.expect("Failed to create namespaces");

    // let ns1 = namespaces[0].clone();
    // let ns2 = namespaces[1].clone();

    // ns1.bring_up_loopback().expect("Failed to bring up loopback in ns1");
    // ns2.bring_up_loopback().expect("Failed to bring up loopback in ns2");

    // let device1 = Arc::new(NetdevsimDevice::new(ns1.clone(), 1, 1).expect("Failed to create netdevsim device"));
    // let device2 = Arc::new(NetdevsimDevice::new(ns2.clone(), 2, 1).expect("Failed to create netdevsim device"));

    // device1.set_ip_address("10.10.0.1/24").expect("Failed to set IP address on device 1");
    // device2.set_ip_address("10.10.0.2/24").expect("Failed to set IP address on device 2");
    // device1.bring_link_up().expect("Failed to bring device 1 up");
    // device2.bring_link_up().expect("Failed to bring device 2 up");

    // let link = LinkedDevices::link(device1.clone(), device2.clone()).expect("Failed to link devices");

    // link.device1.set_delay(0, 10000000).expect("Failed to add delay to device 1");
    // link.device2.set_delay(0, 10000000).expect("Failed to add delay to device 2");

    // println!("Qdisc info from ns1: {}", ns1.run_command_in_namespace("tc", &["qdisc", "show"]).expect("Failed to show qdisc in ns1"));

    // println!("Devices linked successfully. Device 1: {}, Device 2: {}", link.device1.name, link.device2.name);
    // println!("IP info from ns1: {}", ns1.run_command_in_namespace("ip", &["addr", "show"]).expect("Failed to get IP info from ns1"));
    // println!("IP info from ns2: {}", ns2.run_command_in_namespace("ip", &["addr", "show"]).expect("Failed to get IP info from ns2"));

    // // Ping device2 from device1
    // // let t = ns1.run_command_in_namespace("ping", &["-I", "eth0", "-c", "4", "10.10.0.2"]).expect("Failed to ping device2 :(");
    // // println!("Ping output: {}", t);

    // println!("Ping output: {}", ns1.run_command_in_namespace("ping", &["-I", "eth0", "-c", "4", "10.10.0.2"]).expect("Failed to ping device2"));

    // let mut tshark_device1 = ns1.spawn_command_in_namespace("tshark", &["-i", "eth0", "-w", "./device1.pcap"]).expect("Failed to spawn tshark on device 1");
    // let mut tshark_device2 = ns2.spawn_command_in_namespace("tshark", &["-i", "eth0", "-w", "./device2.pcap"]).expect("Failed to spawn tshark on device 2");

    // let mut ptp4l_ns1 = ns1.spawn_command_in_namespace("ptp4l", &["-H", "-m", "-l", "6", "-2", "-i", "eth0"]).expect("Failed to spawn ptp4l");
    // let mut ptp4l_ns2 = ns2.spawn_command_in_namespace("ptp4l", &["-H", "-m", "-l", "6", "-2", "-i", "eth0", "--clientOnly=1"]).expect("Failed to spawn ptp4l");
    // sleep(Duration::from_secs(30));

    // // if !Command::new("phc_ctl")
    // //     .arg("/dev/ptp2")
    // //     .arg("adj")
    // //     .arg("0.025")
    // //     .status()
    // //     .expect("Failed to execute phc_ctl command").success() {
    // //     println!("Failed to adjust PTP clock");
    // // } else {
    // //     println!("NOTICE:     PTP clock adjusted successfully");
    // // }

    // link.device1.set_delay(0, 0).expect("Failed to clear delay on device1");
    // link.device2.set_delay(0, 0).expect("Failed to clear delay on device2");
    // println!("NOTICE:     Cleared delay on both devices");

    // sleep(Duration::from_secs(30));
    
    // ptp4l_ns1.kill().expect("Failed to kill ptp4l");
    // ptp4l_ns2.kill().expect("Failed to kill ptp4l");
    // tshark_device1.kill().expect("Failed to kill tshark on device 1");
    // tshark_device2.kill().expect("Failed to kill tshark on device 2");
}
