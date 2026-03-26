use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeConfig {
    pub name: String,
    pub num_ports: usize,
    pub tshark: bool,
    pub ptp4l_args: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Devlink {
    pub dev1: String,
    pub port1: usize,
    pub dev2: String,
    pub port2: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Topology {
    pub nodes: Vec<NodeConfig>,
    pub devlinks: Vec<Devlink>,
}


