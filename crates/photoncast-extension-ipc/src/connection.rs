use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use serde_json::Value;
use tracing::{error, warn};

use crate::{IpcError, RpcMessage, RpcNotification, RpcRequest, RpcResponse};

pub trait RpcHandler: Send + Sync {
    fn handle_request(&self, method: &str, params: Value) -> Result<Value, IpcError>;
    fn handle_notification(&self, _method: &str, _params: Value) -> Result<(), IpcError> {
        Ok(())
    }
}

struct RpcConnectionInner {
    writer: Mutex<Box<dyn Write + Send>>,
    pending: Mutex<HashMap<u64, mpsc::Sender<Result<Value, IpcError>>>>,
    next_id: AtomicU64,
}

#[derive(Clone)]
pub struct RpcConnection {
    inner: Arc<RpcConnectionInner>,
}

impl RpcConnection {
    pub fn new<R, W>(reader: R, writer: W, handler: Arc<dyn RpcHandler>) -> Self
    where
        R: BufRead + Send + 'static,
        W: Write + Send + 'static,
    {
        let inner = Arc::new(RpcConnectionInner {
            writer: Mutex::new(Box::new(writer)),
            pending: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        });
        let connection = Self {
            inner: Arc::clone(&inner),
        };
        connection.start_reader(reader, handler);
        connection
    }

    pub fn send_request(&self, method: &str, params: Value) -> Result<Value, IpcError> {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let request = RpcRequest::new(id, method, params);
        let (tx, rx) = mpsc::channel();

        self.inner
            .pending
            .lock()
            .map_err(|_| IpcError::Disconnected)?
            .insert(id, tx);

        self.write_message(&RpcMessage::Request(request))?;

        match rx.recv() {
            Ok(result) => result,
            Err(_) => Err(IpcError::ResponseChannelClosed),
        }
    }

    pub fn send_notification(&self, method: &str, params: Value) -> Result<(), IpcError> {
        let notification = RpcNotification::new(method, params);
        self.write_message(&RpcMessage::Notification(notification))
    }

    pub fn send_response(&self, response: RpcResponse) -> Result<(), IpcError> {
        self.write_message(&RpcMessage::Response(response))
    }

    fn write_message(&self, message: &RpcMessage) -> Result<(), IpcError> {
        let json = serde_json::to_string(message)?;
        let mut writer = self
            .inner
            .writer
            .lock()
            .map_err(|_| IpcError::Disconnected)?;
        writer.write_all(json.as_bytes())?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        Ok(())
    }

    fn start_reader<R>(&self, reader: R, handler: Arc<dyn RpcHandler>)
    where
        R: BufRead + Send + 'static,
    {
        let inner = Arc::clone(&self.inner);
        let connection = self.clone();
        thread::spawn(move || {
            for line in reader.lines() {
                let line = match line {
                    Ok(line) => line,
                    Err(err) => {
                        error!(error = %err, "RPC reader loop failed to read line");
                        break;
                    },
                };

                let message = match RpcMessage::parse_line(&line) {
                    Ok(message) => message,
                    Err(err) => {
                        warn!(
                            error = %err,
                            line_len = line.len(),
                            "Failed to parse RPC message line"
                        );
                        continue;
                    },
                };

                match message {
                    RpcMessage::Response(response) => {
                        let sender = match inner.pending.lock() {
                            Ok(mut pending) => pending.remove(&response.id),
                            Err(_) => {
                                error!("RPC pending response map lock poisoned");
                                break;
                            },
                        };

                        if let Some(sender) = sender {
                            if let Some(error_data) = response.error {
                                if sender
                                    .send(Err(IpcError::RpcError {
                                        code: error_data.code,
                                        message: error_data.message,
                                    }))
                                    .is_err()
                                {
                                    warn!(id = response.id, "Failed to deliver RPC error response");
                                }
                            } else {
                                let value = response.result.unwrap_or(Value::Null);
                                if sender.send(Ok(value)).is_err() {
                                    warn!(id = response.id, "Failed to deliver RPC success response");
                                }
                            }
                        } else {
                            warn!(id = response.id, "Received RPC response with no pending request");
                        }
                    },
                    RpcMessage::Request(request) => {
                        let handler = Arc::clone(&handler);
                        let connection = connection.clone();
                        thread::spawn(move || {
                            let result = handler.handle_request(&request.method, request.params);
                            let response = match result {
                                Ok(value) => RpcResponse::success(request.id, value),
                                Err(err) => RpcResponse::error(
                                    request.id,
                                    err.rpc_code(),
                                    err.rpc_message(),
                                ),
                            };
                            if let Err(err) = connection.send_response(response) {
                                error!(error = %err, id = request.id, "Failed to send RPC response");
                            }
                        });
                    },
                    RpcMessage::Notification(notification) => {
                        let handler = Arc::clone(&handler);
                        thread::spawn(move || {
                            if let Err(err) =
                                handler.handle_notification(&notification.method, notification.params)
                            {
                                warn!(
                                    error = %err,
                                    method = %notification.method,
                                    "Failed to handle RPC notification"
                                );
                            }
                        });
                    },
                }
            }

            match inner.pending.lock() {
                Ok(mut pending) => {
                    for (id, sender) in pending.drain() {
                        if sender.send(Err(IpcError::Disconnected)).is_err() {
                            warn!(id = id, "Failed to notify pending RPC request of disconnect");
                        }
                    }
                },
                Err(_) => {
                    error!("RPC pending response map lock poisoned during disconnect cleanup");
                },
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, Cursor};

    struct NoopHandler;

    impl RpcHandler for NoopHandler {
        fn handle_request(&self, _method: &str, _params: Value) -> Result<Value, IpcError> {
            Ok(Value::Null)
        }
    }

    #[test]
    fn test_rpc_connection_creation() {
        let reader = BufReader::new(Cursor::new(Vec::<u8>::new()));
        let writer = Cursor::new(Vec::<u8>::new());
        let handler: Arc<dyn RpcHandler> = Arc::new(NoopHandler);

        let connection = RpcConnection::new(reader, writer, handler);

        connection
            .send_notification("test.notification", serde_json::json!({"ok": true}))
            .expect("connection should be usable after creation");
    }

    #[test]
    fn test_rpc_connection_is_cloneable() {
        let reader = BufReader::new(Cursor::new(Vec::<u8>::new()));
        let writer = Cursor::new(Vec::<u8>::new());
        let handler: Arc<dyn RpcHandler> = Arc::new(NoopHandler);

        let connection = RpcConnection::new(reader, writer, handler);
        let cloned = connection.clone();

        cloned
            .send_notification("test.clone", Value::Null)
            .expect("cloned connection should be constructable and usable");
    }
}
