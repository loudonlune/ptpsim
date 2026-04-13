
// Axum endpoints for controlling the simulation

use std::sync::{Arc};
use tokio::sync::RwLock;

use axum::{Json, Router, extract::{Path, State}};
use serde::{Deserialize, Serialize};

use crate::simulation::Simulation;

#[derive(Serialize, Deserialize)]
struct Timespec {
    sec: u32,
    nsec: u32,
}

#[derive(Clone)]
struct APIState {
    sim: Arc<RwLock<Simulation>>,
}

async fn new_link_handler(_state: State<APIState>, _path: Path<(String, u8, String, u8)>) -> String {
    // Create a new link between the specified nodes and ports
    // This will involve finding the corresponding PTPNodes and NetdevsimPorts, then linking them together

    let mut state = _state.sim.write().await;
    let node1_name = _path.0.0.as_str();
    let port1_index = _path.0.1;
    let node2_name = _path.0.2.as_str();
    let port2_index = _path.0.3;

    state.add_link(node1_name, port1_index as usize, node2_name, port2_index as usize).await
        .map(|_| format!("Linked node {} port {} to node {} port {}", node1_name, port1_index, node2_name, port2_index))
        .unwrap_or_else(|e| format!("Failed to link node {} port {} to node {} port {}: {}", node1_name, port1_index, node2_name, port2_index, e))
}

async fn remove_link_handler(_state: State<APIState>, _path: Path<(String, u8, String, u8)>) -> String {
    // Remove the link between the specified nodes and ports
    // This will involve finding the corresponding PTPNodes and NetdevsimPorts, then unlinking them

    let mut state = _state.sim.write().await;
    let node1_name = _path.0.0.as_str();
    let port1_index = _path.0.1;
    let node2_name = _path.0.2.as_str();
    let port2_index = _path.0.3;

    state.remove_link(node1_name, port1_index as usize, node2_name, port2_index as usize).await
        .map(|_| format!("Unlinked node {} port {} from node {} port {}", node1_name, port1_index, node2_name, port2_index))
        .unwrap_or_else(|e| format!("Failed to unlink node {} port {} from node {} port {}: {}", node1_name, port1_index, node2_name, port2_index, e))
}

async fn set_delay_handler(_state: State<APIState>, _path: Path<(String, u8)>, _delay: Json<Timespec>) -> String {
    // Set delay on the specified node and port
    // This will involve finding the corresponding PTPNode and NetdevsimPort, then calling set_delay on the port

    let mut state = _state.sim.write().await;
    let node_name = _path.0.0;
    let port_index = _path.0.1;
    let delay = _delay.0;

    if let Some(node) = state.get_node_mut(&node_name) {
        node.set_delay(port_index as u8, delay.sec, delay.nsec).await
            .map(|_| format!("Set delay on node {} port {} to {} sec and {} nsec", node_name, port_index, delay.sec, delay.nsec))
            .unwrap_or_else(|e| format!("Failed to set delay on node {} port {}: {}", node_name, port_index, e))
    } else {
        return format!("Node {} not found", node_name);
    }
}

// Axum server setup and endpoints for controlling the simulation
pub async fn run_api_server(simulation: Arc<RwLock<Simulation>>) {
    let api_state = APIState { sim: simulation.clone() };
    let sim_handle = api_state.sim.clone();

    let api_app: Router = Router::new()
        .route("/delay/{node}/{port}", axum::routing::put(set_delay_handler))
        .route("/link/{node1}/{port1}/{node2}/{port2}", axum::routing::post(new_link_handler))
        .route("/link/{node1}/{port1}/{node2}/{port2}", axum::routing::delete(remove_link_handler))
        .with_state(api_state);

    let (shutdown_signal, handle) = Simulation::start_phc_polling(simulation);

    println!("Simulation is running. Press Ctrl+C to exit...");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8300").await.expect("Failed to bind to socket");

    axum::serve(listener, api_app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        })
        .await
        .expect("Failed to start API server");

    // Drop the simulation and wait for clean-up
    println!("Simulation is shutting down...");

    // Signal the PHC polling routine to shut down
    {
        let mut shutdown = shutdown_signal.write().await;
        *shutdown = true;
    }

    // Wait for the PHC polling routine to finish
    handle.await.expect("Failed to wait for PHC polling routine to finish");

    let sim = match Arc::try_unwrap(sim_handle) {
        Ok(lock) => lock.into_inner(),
        Err(_) => panic!("Simulation state still has active references"),
    };

    sim.shutdown().await;
}
