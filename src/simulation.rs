use std::{collections::HashMap, sync::Arc};

use crate::{netdevsim::LinkedDevices, netns::NetNamespace, node::PTPNode, topology::Topology};

pub struct Simulation {
    nodes: Vec<PTPNode>,
    devlinks: Vec<LinkedDevices>,
}

impl Simulation {
    pub fn get_node_mut(&mut self, name: &str) -> Option<&mut PTPNode> {
        self.nodes.iter_mut().find(|n| n.name() == name)
    }

    pub fn get_node(&self, name: &str) -> Option<&PTPNode> {
        self.nodes.iter().find(|n| n.name() == name)
    }

    pub async fn add_node(&mut self, name: &str, num_ports: u8, ptp4l_args: &[&str], tshark: bool) -> Result<(), String> {
        if self.nodes.iter().any(|n| n.name() == name) {
            return Err(format!("Node with name {} already exists", name));
        }

        let last_id = self.nodes.iter().map(|n| n.num_ports() as u32).sum();
        let ns = Arc::new(NetNamespace::create_namespace(name).await?);
        ns.bring_up_loopback().await?;

        let mut node = PTPNode::new(ns.clone(), last_id, num_ports, ptp4l_args).await;

        if tshark {
            let output_file = format!("{}/tshark_{}.pcap", crate::node::LOG_BASE_DIR, name);
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

    pub async fn from_topology(topology: Topology) -> Result<Simulation, String> {
        let mut simulation = Simulation { nodes: vec![], devlinks: vec![] };

        for node_config in topology.nodes {
            simulation.add_node(&node_config.name, node_config.num_ports as u8, &node_config.ptp4l_args.iter().map(|s| s.as_str()).collect::<Vec<&str>>(), node_config.tshark).await?;
        }

        for link in topology.devlinks {
            simulation.add_link(&link.dev1, link.port1 as usize, &link.dev2, link.port2 as usize).await?;
        }

        Ok(simulation)
    }

    pub async fn shutdown(self) {
        // Drop nodes and devlinks to clean up resources
        for devlink in self.devlinks {
            devlink.unlink().await.expect("Failed to unlink");
        }

        for node in self.nodes {
            node.shutdown().await.expect("Failed to shutdown node");
        }
    }
}
