use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use serde_json::Value;

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
                    Err(_) => break,
                };

                let message = match RpcMessage::parse_line(&line) {
                    Ok(message) => message,
                    Err(_) => continue,
                };

                match message {
                    RpcMessage::Response(response) => {
                        let sender = inner
                            .pending
                            .lock()
                            .ok()
                            .and_then(|mut pending| pending.remove(&response.id));
                        if let Some(sender) = sender {
                            if let Some(error) = response.error {
                                let _ = sender.send(Err(IpcError::RpcError {
                                    code: error.code,
                                    message: error.message,
                                }));
                            } else {
                                let value = response.result.unwrap_or(Value::Null);
                                let _ = sender.send(Ok(value));
                            }
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
                            let _ = connection.send_response(response);
                        });
                    },
                    RpcMessage::Notification(notification) => {
                        let handler = Arc::clone(&handler);
                        thread::spawn(move || {
                            let _ = handler
                                .handle_notification(&notification.method, notification.params);
                        });
                    },
                }
            }

            if let Ok(mut pending) = inner.pending.lock() {
                for (_, sender) in pending.drain() {
                    let _ = sender.send(Err(IpcError::Disconnected));
                }
            }
        });
    }
}
