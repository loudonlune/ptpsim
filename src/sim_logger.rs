
use serde::Serialize;
use tokio::{io::AsyncWriteExt, sync::mpsc::Receiver};

#[derive(Serialize, Debug, Clone, PartialEq)]
pub enum MessageData {
    Ptp4lLog { path_delay: f64, offset_from_master: f64, frequency: f64, state: String },
    PhcPollingResult { phc_time: f64 },
    RealDelay { delay_sec: u32, delay_nsec: u32 },
}

#[derive(Serialize, Debug, Clone)]
pub struct Message {
    pub message_type: String,
    pub node: String,
    pub relative_timestamp: f64,
    pub data: MessageData,
}

/**
 * Asynchronous logging service that listens for messages on a channel and writes them to log files in YAML format.
 * Each message is expected to have a type, a relative timestamp, and associated data.
 */
pub async fn logging_service(mut receiver: Receiver<Message>, output_dir: String) {
    while let Some(message) = receiver.recv().await {
        // Determine the log file based on the message type
        let log_file_path = format!("{}/events.yaml", output_dir);

        // Append the message to the appropriate log file in YAML format
        let yaml_message = serde_yaml_bw::to_string(&message).expect("Failed to serialize message to YAML");
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)
            .await
            .expect("Failed to open log file");

        file.write("---\n".as_bytes()).await.expect("Failed to write document separator to file");
        file.write_all(yaml_message.as_bytes()).await.expect("Failed to write log message");
    }
}
