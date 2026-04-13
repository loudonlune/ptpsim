
use std::{sync::Arc, time::Instant, vec};

use serde::Serialize;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;
use tokio::time::Interval;
use tokio::{sync::RwLock, task::JoinSet};

use crate::sim_logger::Message;
use crate::{Args, netdevsim::LinkedDevices, netns::NetNamespace, node::PTPNode, topology::Topology};

pub struct Simulation {
    epoch: Instant,
    sim_logger_handle: JoinHandle<()>,
    sim_logger_channel: Sender<Message>,
    nodes: Vec<PTPNode>,
    devlinks: Vec<LinkedDevices>,
}

/**
 * Routine to poll the PHC at a specific time for a PTP node.
 * This is spawned each time the PHC is polled by the phc_polling_routine.
 */
async fn poll_phc(name: String, namespace: Arc<NetNamespace>, phc_index: u32, epoch: Instant, channel: Sender<Message>) -> Result<(), String> {
    let phc_device = format!("/dev/ptp{}", phc_index);
    let args = [&phc_device, "get"];

    let fut = namespace.run_command_in_namespace("phc_ctl", &args);
    let rela_time = Instant::now().duration_since(epoch).as_secs_f64();
    // Format: phc_ctl[46233.054]: clock time is 15747.599839000 or Wed Dec 31 23:22:27 1969
    let phc_time = fut.await
        .map_err(|e| format!("Failed to run phc_ctl: {}", e))?
        .trim()
        .split_whitespace()
        .nth(4)
        .ok_or_else(|| "Failed to parse phc_ctl output".into())
        .and_then(|s| s.parse::<f64>().map_err(|e| format!("Failed to parse PHC time: {}", e)))?;

    // Placeholder implementation - replace with actual PHC polling logic
    channel.send(Message {
        message_type: "PhcPollingResult".to_string(),
        node: name,
        relative_timestamp: rela_time,
        data: crate::sim_logger::MessageData::PhcPollingResult { phc_time },
    }).await.map_err(|e| format!("Failed to send message to channel: {}", e))?;

    Ok(())
}

/**
 * Routine for periodically polling all of the simulated PHCs.
 * This runs for the duration of the simulation.
 */
async fn phc_polling_routine(simulation: Arc<RwLock<Simulation>>, shutdown_signal: Arc<RwLock<bool>>, polling_frequency_seconds: f64) {
    let mut phc_poll_interval: Interval = tokio::time::interval(std::time::Duration::from_secs_f64(polling_frequency_seconds));

    loop {
        phc_poll_interval.tick().await;

        let mut join_set = JoinSet::new();

        {
            let sim = simulation.read().await;
            for node in sim.nodes.iter() {
                join_set.spawn(poll_phc(node.name().to_string(), node.namespace().clone(), node.phc_index(), sim.epoch, sim.sim_logger_channel.clone()));
            }

            while let Some(res) = join_set.join_next().await {
                match res {
                    Ok(inner_result) => {
                        if let Err(e) = inner_result {
                            eprintln!("[PHC Polling] Error in polling PHC: {}", e);
                        }
                    },
                    Err(e) => eprintln!("[PHC Polling] Task join error: {}", e),
                }
            }
        }

        // Check for shutdown signal
        if *shutdown_signal.read().await {
            break;
        }
    }
}

impl Simulation {
    pub fn get_node_mut(&mut self, name: &str) -> Option<&mut PTPNode> {
        self.nodes.iter_mut().find(|n| n.name() == name)
    }

    pub fn get_node(&self, name: &str) -> Option<&PTPNode> {
        self.nodes.iter().find(|n| n.name() == name)
    }

    pub fn start_phc_polling(handle: Arc<RwLock<Simulation>>) -> (Arc<RwLock<bool>>, tokio::task::JoinHandle<()>) {
        let shutdown_signal = Arc::new(RwLock::new(false));

        let fut = tokio::spawn(phc_polling_routine(handle, shutdown_signal.clone(), 1.0)); // Poll every 1 second
        (shutdown_signal, fut)
    }

    pub async fn add_node(&mut self, name: &str, num_ports: u8, ptp4l_args: &[&str], tshark: bool, output_dir: &str) -> Result<(), String> {
        if self.nodes.iter().any(|n| n.name() == name) {
            return Err(format!("Node with name {} already exists", name));
        }

        let last_id = self.nodes.iter().map(|n| n.num_ports() as u32).sum();
        let ns = Arc::new(NetNamespace::create_namespace(name).await?);
        ns.bring_up_loopback().await?;

        let mut node = PTPNode::new(ns.clone(), self.sim_logger_channel.clone(), last_id, num_ports, ptp4l_args, output_dir, self.epoch).await;

        if tshark {
            let output_file = format!("{}/tshark_{}.pcap", output_dir, name);
            node.start_tshark(&output_file, &[]).await;
        }

        self.nodes.push(node);
        Ok(())
    }

    pub async fn add_link(&mut self, node1_name: &str, port1: usize, node2_name: &str, port2: usize) -> Result<(), String> {
        let node1 = self.get_node(node1_name).ok_or_else(|| format!("Node {} not found", node1_name))?;
        let node2 = self.get_node(node2_name).ok_or_else(|| format!("Node {} not found", node2_name))?;

        let dev1 = node1.port(port1).ok_or_else(|| format!("Port {} not found on node {}", port1, node1_name))?;
        let dev2 = node2.port(port2).ok_or_else(|| format!("Port {} not found on node {}", port2, node2_name))?;

        if self.devlinks.iter().any(|link| link.matches(&dev1, &dev2)) {
            return Err(format!("Link between {}:{} and {}:{} already exists", node1_name, port1, node2_name, port2));
        }

        self.devlinks.push(LinkedDevices::link(dev1, dev2).await?);
        Ok(())
    }

    pub async fn remove_link(&mut self, node1_name: &str, port1: usize, node2_name: &str, port2: usize) -> Result<(), String> {
        let node1 = self.get_node(node1_name).ok_or_else(|| format!("Node {} not found", node1_name))?;
        let node2 = self.get_node(node2_name).ok_or_else(|| format!("Node {} not found", node2_name))?;

        let dev1 = node1.port(port1).ok_or_else(|| format!("Port {} not found on node {}", port1, node1_name))?;
        let dev2 = node2.port(port2).ok_or_else(|| format!("Port {} not found on node {}", port2, node2_name))?;

        if let Some(index) = self.devlinks.iter().position(|link| link.matches(&dev1, &dev2)) {
            self.devlinks.remove(index).unlink().await?;
            Ok(())
        } else {
            Err(format!("Link between {}:{} and {}:{} not found", node1_name, port1, node2_name, port2))
        }
    }

    pub async fn from_topology(topology: Topology, args: Args) -> Result<Simulation, String> {
        let (send, recv) = tokio::sync::mpsc::channel(128);
        let sim_logger_handle = tokio::task::spawn(crate::sim_logger::logging_service(recv, args.output_dir.clone()));        
        let mut simulation = Simulation { epoch: Instant::now(), nodes: vec![], devlinks: vec![], sim_logger_channel: send, sim_logger_handle: sim_logger_handle };

        for node_config in topology.nodes {
            simulation.add_node(&node_config.name, node_config.num_ports as u8, &node_config.ptp4l_args.iter().map(|s| s.as_str()).collect::<Vec<&str>>(), node_config.tshark, &args.output_dir).await?;
        }

        for link in topology.devlinks {
            simulation.add_link(&link.dev1, link.port1 as usize, &link.dev2, link.port2 as usize).await?;
        }

        Ok(simulation)
    }

    pub async fn shutdown(self) {
        self.sim_logger_handle.abort();

        // Drop nodes and devlinks to clean up resources
        for devlink in self.devlinks {
            devlink.unlink().await.expect("Failed to unlink");
        }

        for node in self.nodes {
            node.shutdown().await.expect("Failed to shutdown node");
        }
    }
}
