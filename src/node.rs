use std::{sync::Arc};

use tokio::{fs::File, io::{AsyncRead, AsyncWriteExt, BufReader}, process::Child, sync::RwLock};
use tokio::io::AsyncBufReadExt;

use crate::{netdevsim::{NetdevsimDevice, NetdevsimPort}, netns::NetNamespace};

pub static LOG_BASE_DIR: &str = "ptpsim_node_logs";

async fn file_log_update_routine(stdout: impl AsyncRead + Unpin, mut log_file: File) {
    let mut stdout_reader = BufReader::new(stdout);

    loop {
        let mut buffer = String::new();
        match stdout_reader.read_line(&mut buffer).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                log_file.write_all(buffer.as_bytes()).await.expect("Failed to write to log file");
                log_file.flush().await.expect("Failed to flush log file");
            }
            Err(e) => {
                eprintln!("Error reading from stdout: {}", e);
                break;
            }
        }
    }
}

pub struct PTPNode {
    ns: Arc<NetNamespace>,
    device: Arc<NetdevsimDevice>,
    ptp4l_process: Arc<RwLock<Child>>,
    tshark_process: Option<Child>,
}

impl PTPNode {
    pub fn device(&self) -> Arc<NetdevsimDevice> {
        self.device.clone()
    }

    pub fn name(&self) -> &str {
        &self.ns.name
    }

    pub async fn new(ns: Arc<NetNamespace>, last_id: u32, num_ports: u8, ptp4l_args: &[&str], output_dir: &str) -> Self {
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
        let log_file = File::create(format!("{}/ptp4l_{}.log", output_dir, ns.name))
            .await
            .expect("Failed to create log file");
        let log_file_stderr = File::create(format!("{}/ptp4l_{}_stderr.log", output_dir, ns.name))
            .await
            .expect("Failed to create stderr log file");

        let maybe_stdout = ptp4l_process.stdout.take();
        let maybe_stderr = ptp4l_process.stderr.take();
        tokio::spawn(async move {
            let stdout = maybe_stdout.expect("Failed to take stdout");
            let stderr = maybe_stderr.expect("Failed to take stderr");
            file_log_update_routine(stdout, log_file).await;
            file_log_update_routine(stderr, log_file_stderr).await;
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

        PTPNode {
            ns,
            device,
            ptp4l_process,
            tshark_process: None,
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
