use std::collections::HashSet;
use std::os::fd::{FromRawFd, IntoRawFd};
use std::process::Stdio;

use tokio::process::{Child, Command};
use tokio::task::JoinSet;
use std::fs::File;
use std::sync::Arc;

pub struct NetNamespace {
    pub file_descriptor: i32,
    pub name: String,
}

impl Drop for NetNamespace {
    fn drop(&mut self) {
        // Close the file descriptor when the NetNamespace is dropped
        unsafe { File::from_raw_fd(self.file_descriptor); }

        self.remove_netns().unwrap_or_else(|e| {
            eprintln!("Failed to remove namespace {}: {}", self.name, e);
        });
    }
}

impl NetNamespace {
    fn remove_netns(&self) -> Result<(), String> {
        // Remove the network namespace using the `ip netns delete` command
        let status = std::process::Command::new("ip")
            .arg("netns")
            .arg("delete")
            .arg(&self.name)
            .status()
            .map_err(|e| format!("Failed to delete namespace {}: {}", &self.name, e))?;

        if !status.success() {
            return Err(format!("Failed to delete namespace {}: Exit status {}", &self.name, status.to_string()));
        }

        Ok(())
    }

    pub async fn create_namespace(name: &str) -> Result<NetNamespace, String> {
        // Create a network namespace using the `ip netns add` command
        Command::new("ip")
            .arg("netns")
            .arg("add")
            .arg(name)
            .status()
            .await
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        // Get file descriptor by opening /var/run/netns/<name>
        let fd = std::fs::File::open(format!("/var/run/netns/{}", name))
            .map_err(|e| format!("Failed to open namespace file: {}", e))?
            .into_raw_fd();

        Ok(NetNamespace {
            file_descriptor: fd,
            name: name.to_string(),
        })
    }

    pub async fn bring_up_loopback(&self) -> Result<(), String> {
        let status = Command::new("ip")
            .args(&["-n", &self.name])
            .arg("link")
            .arg("set")
            .arg("lo")
            .arg("up")
            .status()
            .await
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        if !status.success() {
            return Err(format!("Failed to bring loopback up in namespace {}: Exit status {}", &self.name, status.to_string()));
        } 

        Ok(())
    }

    pub async fn spawn_command_in_namespace_piped(&self, command: &str, args: &[&str]) -> Result<Child, String> {
        // Use `ip netns exec` to spawn the command in the specified namespace
        let child = Command::new("ip")
            .arg("netns")
            .arg("exec")
            .arg(&self.name)
            .arg(command)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn command: {}", e))?;

        Ok(child)
    }

    pub async fn spawn_command_in_namespace(&self, command: &str, args: &[&str]) -> Result<Child, String> {
        // Use `ip netns exec` to spawn the command in the specified namespace
        let child = Command::new("ip")
            .arg("netns")
            .arg("exec")
            .arg(&self.name)
            .arg(command)
            .args(args)
            .spawn()
            .map_err(|e| format!("Failed to spawn command: {}", e))?;

        Ok(child)
    }

    pub async fn run_command_in_namespace(&self, command: &str, args: &[&str]) -> Result<String, String> {
        // Use `ip netns exec` to run the command in the specified namespace
        let cmd = Command::new("ip")
            .arg("netns")
            .arg("exec")
            .arg(&self.name)
            .arg(command)
            .args(args)
            .output()
            .await
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        let status = cmd.status;
        let output: String = String::from_utf8_lossy(&cmd.stdout).to_string();

        if !status.success() {
            eprintln!("stderr: {}", String::from_utf8_lossy(&cmd.stderr));
            eprintln!("stdout: {}", output);

            return Err(format!("Command exited with status: {}", status));
        }

        Ok(output)
    }

    pub async fn create_namespaces(namespaces: HashSet<String>) -> Result<Vec<Arc<NetNamespace>>, String> {
        let mut tasks = JoinSet::new();
        for name in namespaces {
            tasks.spawn(async move {
                NetNamespace::create_namespace(&name).await
            });
        }
        let results = tasks.join_all().await;
        
        results.into_iter().map(|res| {
            match res {
                Ok(ns) => Ok(Arc::new(ns)),
                Err(e) => Err(format!("Task failed: {}", e)),
            }
        }).collect()
    }
}
