use crate::error::{Error, Result};
use crate::rpc::types::{RpcCommand, RpcEvent, RpcResponse};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Internal wrapper for commands with correlation IDs
struct CommandWithId {
    id: String,
    command: RpcCommand,
    response_tx: Option<oneshot::Sender<RpcResponse>>,
}

/// RPC client for communicating with Oh My Pi
pub struct RpcClient {
    /// Command sender for RPC commands
    command_tx: mpsc::UnboundedSender<CommandWithId>,
    /// Event receiver for RPC events
    event_rx: mpsc::UnboundedReceiver<RpcEvent>,
    /// Pending response channel sender
    pending_response_tx: mpsc::UnboundedSender<(String, oneshot::Sender<RpcResponse>)>,
    /// Handle for the reader task
    _reader_handle: JoinHandle<Result<()>>,
    /// Handle for the subprocess
    _subprocess_handle: JoinHandle<Result<()>>,
}

impl RpcClient {
    /// Create a new RPC client by spawning the OMP subprocess
    pub async fn new() -> Result<Self> {
        info!("Starting Oh My Pi RPC subprocess");

        let mut child = tokio::process::Command::new("omp")
            .args(&["--mode", "rpc"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| Error::OmpSubprocess(format!("Failed to spawn omp: {}", e)))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::OmpSubprocess("Failed to get stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::OmpSubprocess("Failed to get stdout".to_string()))?;

        let (command_tx, mut command_rx) = mpsc::unbounded_channel::<CommandWithId>();
        let (event_tx, event_rx) = mpsc::unbounded_channel::<RpcEvent>();
        let (pending_response_tx, mut pending_response_rx) = mpsc::unbounded_channel::<(String, oneshot::Sender<RpcResponse>)>();
        // Create a channel for routing responses from reader to pending response manager
        let (response_router_tx, mut response_router_rx) = mpsc::unbounded_channel::<(String, RpcResponse)>();


        // Clone the sender for use in the writer task
        let pending_response_tx_clone = pending_response_tx.clone();

        // Spawn writer task: reads from command_rx and writes to stdin

        let mut stdin_writer = tokio::io::BufWriter::new(stdin);
        let _writer_handle = tokio::spawn(async move {
            while let Some(cmd_with_id) = command_rx.recv().await {
                // Add correlation ID to the command
                let mut cmd_json = serde_json::to_value(&cmd_with_id.command)
                    .map_err(|e| Error::Serde(e))?;
                
                // Set the correlation ID
                if let Some(obj) = cmd_json.as_object_mut() {
                    obj.insert("id".to_string(), serde_json::json!(cmd_with_id.id));
                }
                
                let json = serde_json::to_string(&cmd_json)
                    .map_err(|e| Error::Serde(e))?;
                debug!("Sending RPC command: {}", json);
                stdin_writer
                    .write_all(json.as_bytes())
                    .await
                    .map_err(|e| Error::Io(e))?;
                stdin_writer
                    .write_all(b"\n")
                    .await
                    .map_err(|e| Error::Io(e))?;
                stdin_writer
                    .flush()
                    .await
                    .map_err(|e| Error::Io(e))?;
                
                // If there's a response channel, register it
                if let Some(response_tx) = cmd_with_id.response_tx {
                    if pending_response_tx_clone.send((cmd_with_id.id, response_tx)).is_err() {
                        error!("Failed to register pending response - channel closed");
                    }
                }
            }
            Ok::<(), Error>(())
        });

        // Spawn reader task: reads from stdout and sends events/responses
        let mut stdout_reader = BufReader::new(stdout).lines();
        let reader_handle = tokio::spawn(async move {
            loop {
                match stdout_reader.next_line().await {
                    Ok(Some(line)) => {
                        debug!("Received RPC line: {}", line);
                        
                        // Try to parse as response first
                        if let Ok(response) = serde_json::from_str::<RpcResponse>(&line) {
                            debug!("Received RPC response: {:?}", response);
                            
                            // Extract correlation ID from response
                            let id = match &response {
                                RpcResponse::Response { id, .. } => id.clone(),
                            };
                            
                            // Send response to pending response manager
                            if let Some(id_str) = id {
                                if response_router_tx.send((id_str, response)).is_err() {
                                    error!("Failed to route response - channel closed");
                                }
                            } else {
                                debug!("Response has no correlation ID");
                            }
                            continue;
                        }

                        // Try to parse as event

                        
                        // Try to parse as event
                        if let Ok(event) = serde_json::from_str::<RpcEvent>(&line) {
                            debug!("Received RPC event: {:?}", event);
                            if event_tx.send(event).is_err() {
                                error!("Failed to send RPC event - channel closed");
                                break;
                            }
                            continue;
                        }
                        
                        warn!("Received unrecognized RPC output: {}", line);
                    }
                    Ok(None) => {
                        info!("OMP stdout closed");
                        break;
                    }
                    Err(e) => {
                        error!("Error reading from OMP stdout: {}", e);
                        break;
                    }
                }
            }
            Ok::<(), Error>(())
        });

        // Spawn pending response manager task
        let mut pending_responses = std::collections::HashMap::new();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Register new pending response
                    Some((id, tx)) = pending_response_rx.recv() => {
                        pending_responses.insert(id, tx);
                    }
                    // Route response to waiting channel
                    Some((id, response)) = response_router_rx.recv() => {
                        if let Some(tx) = pending_responses.remove(&id) {
                            let _ = tx.send(response);
                        } else {
                            debug!("Received response for unknown correlation ID: {}", id);
                        }
                    }
                    else => break,
                }
            }
            Ok::<(), Error>(())
        });


        // Spawn subprocess monitor

        let subprocess_handle = tokio::spawn(async move {
            match child.wait().await {
                Ok(status) => {
                    if status.success() {
                        info!("OMP subprocess exited successfully");
                    } else {
                        error!("OMP subprocess exited with error: {:?}", status);
                    }
                }
                Err(e) => {
                    error!("Error waiting for OMP subprocess: {}", e);
                }
            }
            Ok(())
        });

        Ok(Self {
            command_tx,
            event_rx,
            pending_response_tx,
            _reader_handle: reader_handle,
            _subprocess_handle: subprocess_handle,
        })
    }

    /// Send a command to Oh My Pi without waiting for a response
    /// Returns the correlation ID for the command
    pub fn send_command(&self, command: RpcCommand) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let cmd_with_id = CommandWithId {
            id: id.clone(),
            command,
            response_tx: None,
        };
        self.command_tx
            .send(cmd_with_id)
            .map_err(|_| Error::RpcError("Failed to send command - channel closed".to_string()))?;
        Ok(id)
    }

    /// Send a prompt to Oh My Pi without waiting for a response
    /// Send a prompt to Oh My Pi without waiting for a response
    /// Returns the correlation ID for the command
    pub fn prompt(&self, message: String) -> Result<String> {
        let command = RpcCommand::prompt(message);
        self.send_command(command)
    }

    /// Send a command and wait for its response
    pub async fn send_and_wait(&mut self, command: RpcCommand) -> Result<RpcResponse> {
        let id = Uuid::new_v4().to_string();
        let (response_tx, response_rx) = oneshot::channel();
        
        let cmd_with_id = CommandWithId {
            id: id.clone(),
            command,
            response_tx: Some(response_tx),
        };
        
        self.command_tx
            .send(cmd_with_id)
            .map_err(|_| Error::RpcError("Failed to send command - channel closed".to_string()))?;
        
        // Wait for the response with a timeout
        tokio::time::timeout(
            tokio::time::Duration::from_secs(30),
            response_rx
        )
        .await
        .map_err(|_| Error::Timeout(format!("No response received for command {}", id)))?
        .map_err(|_| Error::RpcError("Response channel closed".to_string()))
    }

    /// Try to receive an event from Oh My Pi (non-blocking)
    pub fn try_recv_event(&mut self) -> Option<RpcEvent> {
        self.event_rx.try_recv().ok()
    }

    /// Receive an event from Oh My Pi (async)
    pub async fn recv_event(&mut self) -> Option<RpcEvent> {
        self.event_rx.recv().await
    }

    /// Create a stream receiver for events
    pub fn event_stream(&mut self) -> mpsc::UnboundedReceiver<RpcEvent> {
        std::mem::replace(&mut self.event_rx, mpsc::unbounded_channel().1)
    }
}

impl Drop for RpcClient {
    fn drop(&mut self) {
        debug!("Dropping RPC client");
    }
}
