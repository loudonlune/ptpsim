use std::sync::Arc;
use tokio::{fs::File, io::AsyncWriteExt, process::Command};

use crate::netns::NetNamespace;

pub struct NetdevsimPort {
    pub namespace: Arc<NetNamespace>,
    pub ifindex: u32,
    pub name: String,
}

impl NetdevsimPort {
    pub async fn set_delay(&self, delay_sec: u32, delay_nsec: u32) -> Result<(), String> {        
        // Write to /sys/bus/netdevsim/set_delay to add delay to the netdevsim device
        // Format: <namespace_fd>:<ifindex> <delay_sec> <delay_nsec>
        let sysfs_path = "/sys/bus/netdevsim/set_delay";
        let mut file = File::create(sysfs_path).await.map_err(|e| format!("Failed to open {}: {}", sysfs_path, e)).unwrap();
        file.write(format!("{}:{} {} {}", 
            self.namespace.file_descriptor,
            self.ifindex,
            delay_sec,
            delay_nsec).as_bytes())
            .await
            .map_err(|e| format!("Failed to write to {}: {}", sysfs_path, e))?;

        return Ok(());
    }

    pub async fn set_ip_address(&self, ip_address: &str) -> Result<(), String> {
        // Use `ip addr add` to set the IP address of the device
        let status = Command::new("ip")
            .args(&["-n", &self.namespace.name])
            .arg("addr")
            .arg("add")
            .arg(ip_address)
            .arg("dev")
            .arg(&self.name)
            .status()
            .await
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        if !status.success() {
            Err(format!("Failed to set IP address {} on device {}: Exit status {}", ip_address, &self.name, status.to_string()))
        } else {
            Ok(())
        }
    }

    pub async fn bring_link_up(&self) -> Result<(), String> {
        // Use `ip link set <device_name> up` to bring the device up
        let status = Command::new("ip")
            .args(&["-n", &self.namespace.name])
            .arg("link")
            .arg("set")
            .arg(&self.name)
            .arg("up")
            .status()
            .await
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        if !status.success() {
            return Err(format!("Failed to bring link {} up: Exit status {}", &self.name, status.to_string()));
        }

        Ok(())
    }

    pub async fn bring_link_down(&self) -> Result<(), String> {
        // Use `ip link set <device_name> down` to bring the device down
        let status = Command::new("ip")
            .args(&["-n", &self.namespace.name])
            .arg("link")
            .arg("set")
            .arg(&self.name)
            .arg("down")
            .status()
            .await
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        if !status.success() {
            return Err(format!("Failed to bring link {} down: Exit status {}", &self.name, status.to_string()));
        }

        Ok(())
    }
}

pub struct NetdevsimDevice {
    pub ident: u32,
    pub phc_index: u32,
    pub ports: Vec<Arc<NetdevsimPort>>,
    pub namespace: Arc<NetNamespace>,
}

impl NetdevsimDevice {
    pub async fn new(in_namespace: Arc<NetNamespace>, id: u32, ports: u8, queues: u8) -> Result<NetdevsimDevice, String> {
        // Write to /sys/bus/netdevsim/new_device to create a new netdevsim device
        let sysfs_path = "/sys/bus/netdevsim/new_device";

        Command::new("ip")
            .arg("netns")
            .arg("exec")
            .arg(&in_namespace.name)
            .arg("sh")
            .arg("-c")
            .arg(format!("echo {} {} {} > {}", id, ports, queues, sysfs_path))
            .status()
            .await
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        // Wait for the device to come up
        //sleep(Duration::from_millis(1000));

        // Get device names from listing the devices in /sys/bus/netdevsim/devices/netdevsim{id}/net
        let path = format!("/sys/bus/netdevsim/devices/netdevsim{id}/net");
        let device_names: Vec<String> = in_namespace.run_command_in_namespace("ls", &[&path])
            .await
            .map_err(|e| format!("Failed to list devices in {}: {}", path, e))?
            .lines()
            .map(|s| s.to_string())
            .collect();

        let mut devports = Vec::new();
        for device_name in device_names {
            let ifindex_path = format!("/sys/bus/netdevsim/devices/netdevsim{id}/net/{device_name}/ifindex");
            let ifindex = in_namespace.run_command_in_namespace("cat", &[&ifindex_path])
                .await
                .map_err(|e| format!("Failed to read ifindex from {}: {}", ifindex_path, e))?
                .trim()
                .parse::<u32>()
                .map_err(|e| format!("Failed to parse ifindex: {}", e))?;
        
            devports.push(Arc::new(NetdevsimPort {
                namespace: in_namespace.clone(),
                ifindex: ifindex,
                name: device_name,
            }));
        }

        let phc_index: String = {
            let output = in_namespace.run_command_in_namespace("ethtool", &["-T", &devports[0].name])
            .await
            .expect("Failed to run ethtool");

            let mut phc_index = None;

            println!("ethtool output:\n{}", output);

            for line in output.lines() {
                if line.trim().starts_with("Hardware timestamp provider index:") {
                    phc_index = Some(line.trim().split(':').nth(1).unwrap().trim().to_string());
                }
            }

            if let Some(index) = phc_index {
                index
            } else {
                return Err("Failed to find PTP Hardware Clock index from ethtool output".to_string());
            }
        };

        let phc_index: u32 = phc_index.parse().map_err(|e| format!("Failed to parse PHC index: {}", e))?;

        Ok(NetdevsimDevice {
            ident: id,
            phc_index: phc_index,
            ports: devports,
            namespace: in_namespace,
        })
    }

    pub async fn remove_device(&self) -> Result<(), String> {
        // Write to /sys/bus/netdevsim/del_device to remove the netdevsim device
        let sysfs_path = "/sys/bus/netdevsim/del_device";

        Command::new("ip")
            .arg("netns")
            .arg("exec")
            .arg(&self.namespace.name)
            .arg("sh")
            .arg("-c")
            .arg(format!("echo {} > {}", self.ident, sysfs_path))
            .status()
            .await
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        Ok(())
    }
}

pub struct LinkedDevices {
    pub device1: Arc<NetdevsimPort>,
    pub device2: Arc<NetdevsimPort>,
}

impl LinkedDevices {
    pub fn matches(&self, dev1: &Arc<NetdevsimPort>, dev2: &Arc<NetdevsimPort>) -> bool {
        (Arc::ptr_eq(&self.device1, dev1) && Arc::ptr_eq(&self.device2, dev2)) ||
        (Arc::ptr_eq(&self.device1, dev2) && Arc::ptr_eq(&self.device2, dev1))
    }

    pub async fn link(nsim1: Arc<NetdevsimPort>, nsim2: Arc<NetdevsimPort>) -> Result<LinkedDevices, String> {
        // Write to /sys/bus/netdevsim/link_devices to link the two netdevsim devices
        let sysfs_path = "/sys/bus/netdevsim/link_device";

        let mut file = File::create(sysfs_path).await.map_err(|e| format!("Failed to open {}: {}", sysfs_path, e)).unwrap();
        file.write(format!("{}:{} {}:{}", 
            nsim1.namespace.file_descriptor,
            nsim1.ifindex,
            nsim2.namespace.file_descriptor,
            nsim2.ifindex).as_bytes())
            .await
            .map_err(|e| format!("Failed to write to {}: {}", sysfs_path, e))?;

        std::mem::drop(file); // Close the file to ensure the write is flushed

        Ok(LinkedDevices {
            device1: nsim1,
            device2: nsim2,
        })
    }

    pub async fn unlink(self) -> Result<(), String> {
        let sysfs_path = "/sys/bus/netdevsim/unlink_device";
        let mut file = File::create(sysfs_path).await.map_err(|e| format!("Failed to open {}: {}", sysfs_path, e)).unwrap();
        file.write(format!("{}:{} {}:{}", 
            self.device1.namespace.file_descriptor,
            self.device1.ifindex,
            self.device2.namespace.file_descriptor,
            self.device2.ifindex).as_bytes())
            .await
            .map_err(|e| format!("Failed to write to {}: {}", sysfs_path, e)).unwrap();

        std::mem::drop(file);
        Ok(())
    }
}
