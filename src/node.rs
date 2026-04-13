use std::{sync::Arc, time::Instant};

use regex::Regex;
use tokio::{io::{AsyncRead, BufReader}, process::Child, sync::{RwLock, mpsc::Sender}};
use tokio::io::AsyncBufReadExt;

use crate::{netdevsim::{NetdevsimDevice, NetdevsimPort}, netns::NetNamespace, sim_logger::Message};

async fn ptp4l_message_send_routine(stdout: impl AsyncRead + Unpin, channel: Sender<Message>, node_name: String, epoch: Instant) {
    let mut stdout_reader = BufReader::new(stdout);
    
    // Regex to extract master offset, state value, frequency adjustment, and path delay from ptp4l log lines
    // Example: ptp4l[3488.724]: master offset       -749 s2 freq     -38 path delay   1001829
    let ptp4l_line_regex = Regex::new(r"ptp4l\[(\d+\.\d+)\]: master offset\s+(-?\d+)\s+([a-z0-9]+)\s+freq\s+[+]{0,1}(-?\d+)\s+path delay\s+(\d+)").expect("Failed to compile regex");

    loop {
        let mut buffer = String::new();
        match stdout_reader.read_line(&mut buffer).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                // Use regex to parse the line and extract relevant information
                if let Some(captures) = ptp4l_line_regex.captures(&buffer) {
                    let offset_from_master = captures.get(2).unwrap().as_str().parse::<f64>().unwrap_or(0.0);
                    let state = captures.get(3).unwrap().as_str().to_string();
                    let frequency = captures.get(4).unwrap().as_str().parse::<f64>().unwrap_or(0.0);
                    let path_delay = captures.get(5).unwrap().as_str().parse::<f64>().unwrap_or(0.0);
                    let relative_timestamp = Instant::now().duration_since(epoch).as_secs_f64();

                    channel.send(Message {
                        message_type: "Ptp4lLog".to_string(),
                        node: node_name.clone(),
                        relative_timestamp: relative_timestamp,
                        data: crate::sim_logger::MessageData::Ptp4lLog { path_delay, offset_from_master, frequency, state },
                    }).await.expect("Failed to send message to channel");
                }
            }
            Err(e) => {
                eprintln!("Error reading from stdout: {}", e);
                break;
            }
        }
    }
}

pub struct PTPNode {
    output_dir: String,
    logging_channel: Sender<Message>,
    epoch: Instant,
    ns: Arc<NetNamespace>,
    device: Arc<NetdevsimDevice>,
    set_delays: Vec<(u32, u32)>,
    ptp4l_process: Arc<RwLock<Child>>,
    tshark_process: Option<Child>,
}

impl PTPNode {
    pub fn device(&self) -> Arc<NetdevsimDevice> {
        self.device.clone()
    }

    pub fn namespace(&self) -> Arc<NetNamespace> {
        self.ns.clone()
    }

    pub fn phc_index(&self) -> u32 {
        self.device.phc_index
    }

    pub fn name(&self) -> &str {
        &self.ns.name
    }

    pub fn output_dir(&self) -> &str {
        &self.output_dir
    }

    pub async fn log_current_delay(&self, port_index: u8) {
        let current_delay = self.set_delays.get(port_index as usize).cloned().expect("Port index out of range");
        let relative_timestamp = Instant::now().duration_since(self.epoch).as_secs_f64();
        self.logging_channel.send(Message {
            message_type: "RealDelay".to_string(),
            node: self.name().to_string(),
            relative_timestamp,
            data: crate::sim_logger::MessageData::RealDelay { delay_sec: current_delay.0, delay_nsec: current_delay.1 },
        }).await.expect("Failed to send message to channel");
    }

    pub async fn set_delay(&mut self, port_index: u8, delay_sec: u32, delay_nsec: u32) -> Result<(), String> {
        self.log_current_delay(port_index).await;
        self.set_delays[port_index as usize] = (delay_sec, delay_nsec);
        
        if let Some(port) = self.device.ports.get(port_index as usize) {
            port.set_delay(delay_sec, delay_nsec).await?;
        } else {
            return Err(format!("Port index {} out of range for node {}", port_index, self.name()))
        }

        self.log_current_delay(port_index).await;
        Ok(())
    }

    pub async fn new(ns: Arc<NetNamespace>, logging_channel: Sender<Message>, last_id: u32, num_ports: u8, ptp4l_args: &[&str], output_dir: &str, epoch: Instant) -> Self {
        let device = Arc::new(NetdevsimDevice::new(ns.clone(), last_id + 1, num_ports, 1).await.expect("Failed to create netdevsim device"));
        let mut args = vec![];

        for port in device.ports.iter() {
            port.bring_link_up().await.expect("Failed to bring link up");
        }

        args.extend_from_slice(ptp4l_args);

        for port in device.ports.iter() {
            args.push("-i");
            args.push(port.name.as_str());
        }

        let mut ptp4l_process = ns.spawn_command_in_namespace_piped("ptp4l", args.as_slice())
            .await
            .expect("Failed to spawn ptp4l");

        let maybe_stdout = ptp4l_process.stdout.take();
        let maybe_stderr = ptp4l_process.stderr.take();
        let logging_channel2 = logging_channel.clone();
        let ns2 = ns.clone();
        tokio::spawn(async move {
            let stdout = maybe_stdout.expect("Failed to take stdout");
            let stderr = maybe_stderr.expect("Failed to take stderr");
            ptp4l_message_send_routine(stdout, logging_channel2.clone(), ns2.name.clone(), epoch).await;
            ptp4l_message_send_routine(stderr, logging_channel2, ns2.name.clone(), epoch).await;
        });

        let ptp4l_process = Arc::new(RwLock::new(ptp4l_process));

        let ptp4l_process_clone = ptp4l_process.clone();
        tokio::spawn(async move {
            // Poll process

            loop {
                let mut process = ptp4l_process_clone.write().await;
                match process.try_wait() {
                    Ok(Some(status)) => {
                        eprintln!("ptp4l exited with status: {}", status);
                        break;
                    }
                    Ok(None) => {
                        // Process is still running
                    }
                    Err(e) => {
                        eprintln!("Error while checking ptp4l status: {}", e);
                        break;
                    }
                }
                drop(process); // Release the lock before sleeping
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });

        let set_delays = vec![(0, 0); num_ports as usize];

        PTPNode {
            logging_channel: logging_channel,
            set_delays,
            ns,
            device,
            ptp4l_process,
            tshark_process: None,
            epoch,
            output_dir: output_dir.to_string(),
        }
    }

    pub fn is_tshark_running(&self) -> bool {
        self.tshark_process.is_some()
    }

    pub async fn start_tshark(&mut self, output_file: &str, xargs: &[&str]) {
        let mut args = vec!["-w", output_file];
        args.extend_from_slice(xargs);
        let tshark_process = self.ns.spawn_command_in_namespace_piped("tshark", args.as_slice())
            .await
            .expect("Failed to spawn tshark");
        self.tshark_process = Some(tshark_process);
    }

    pub fn kill_tshark(&mut self) {
        if let Some(mut tshark_process) = self.tshark_process.take() {
            let _ = tshark_process.kill();
        }
    }

    pub async fn shutdown(mut self) -> Result<(), String> {
        // Kill the ptp4l process
        let _ = self.ptp4l_process.write().await.kill().await;

        // Kill the tshark process if it exists
        if let Some(mut tshark_process) = self.tshark_process.take() {
            let _ = tshark_process.kill().await;
        }

        self.device.remove_device().await?;
        drop(self.ns);

        Ok(())
    }

    pub fn num_ports(&self) -> usize {
        self.device.ports.len()
    }

    pub fn port(&self, index: usize) -> Option<Arc<NetdevsimPort>> {
        self.device.ports.get(index).cloned()
    }
}
