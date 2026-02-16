use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::Value;
use tracing::{error, warn};

#[cfg(not(test))]
const REQUEST_TIMEOUT_MS: u64 = 5_000;
#[cfg(test)]
const REQUEST_TIMEOUT_MS: u64 = 120;

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

        match rx.recv_timeout(Duration::from_millis(REQUEST_TIMEOUT_MS)) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Ok(mut pending) = self.inner.pending.lock() {
                    pending.remove(&id);
                }
                Err(IpcError::Timeout {
                    timeout_ms: REQUEST_TIMEOUT_MS,
                })
            },
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(IpcError::ResponseChannelClosed),
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
                                    warn!(
                                        id = response.id,
                                        "Failed to deliver RPC success response"
                                    );
                                }
                            }
                        } else {
                            warn!(
                                id = response.id,
                                "Received RPC response with no pending request"
                            );
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
                            if let Err(err) = handler
                                .handle_notification(&notification.method, notification.params)
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
                            warn!(
                                id = id,
                                "Failed to notify pending RPC request of disconnect"
                            );
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

    struct CrashHandler;

    impl RpcHandler for CrashHandler {
        fn handle_request(&self, _method: &str, _params: Value) -> Result<Value, IpcError> {
            panic!("intentional test panic");
        }
    }

    fn make_loopback_connection(handler: Arc<dyn RpcHandler>) -> RpcConnection {
        let (host_to_worker_tx, host_to_worker_rx) = mpsc::channel::<Vec<u8>>();
        let (worker_to_host_tx, worker_to_host_rx) = mpsc::channel::<Vec<u8>>();

        thread::spawn(move || {
            while let Ok(msg) = host_to_worker_rx.recv() {
                let text = String::from_utf8_lossy(&msg).to_string();
                if let Ok(RpcMessage::Request(request)) = RpcMessage::parse_line(&text) {
                    let response = RpcResponse::success(request.id, Value::Null);
                    let encoded = serde_json::to_vec(&RpcMessage::Response(response))
                        .expect("encode response");
                    let mut encoded_with_nl = encoded;
                    encoded_with_nl.push(b'\n');
                    worker_to_host_tx
                        .send(encoded_with_nl)
                        .expect("send response");
                }
            }
        });

        struct ChannelReader {
            rx: mpsc::Receiver<Vec<u8>>,
            buffer: Vec<u8>,
            cursor: usize,
        }

        impl std::io::Read for ChannelReader {
            fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
                if self.cursor >= self.buffer.len() {
                    self.buffer = self.rx.recv().map_err(|_| {
                        std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "channel closed")
                    })?;
                    self.cursor = 0;
                }

                let remaining = self.buffer.len().saturating_sub(self.cursor);
                let n = remaining.min(out.len());
                out[..n].copy_from_slice(&self.buffer[self.cursor..self.cursor + n]);
                self.cursor += n;
                Ok(n)
            }
        }

        impl std::io::BufRead for ChannelReader {
            fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
                if self.cursor >= self.buffer.len() {
                    self.buffer = self.rx.recv().map_err(|_| {
                        std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "channel closed")
                    })?;
                    self.cursor = 0;
                }
                Ok(&self.buffer[self.cursor..])
            }

            fn consume(&mut self, amt: usize) {
                self.cursor = (self.cursor + amt).min(self.buffer.len());
            }
        }

        struct ChannelWriter {
            tx: mpsc::Sender<Vec<u8>>,
            pending: Vec<u8>,
        }

        impl std::io::Write for ChannelWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.pending.extend_from_slice(buf);
                Ok(buf.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                if !self.pending.is_empty() {
                    self.tx
                        .send(std::mem::take(&mut self.pending))
                        .map_err(|_| {
                            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "channel closed")
                        })?;
                }
                Ok(())
            }
        }

        let reader = ChannelReader {
            rx: worker_to_host_rx,
            buffer: Vec::new(),
            cursor: 0,
        };
        let writer = ChannelWriter {
            tx: host_to_worker_tx,
            pending: Vec::new(),
        };

        RpcConnection::new(reader, writer, handler)
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
    fn test_send_request_times_out_when_no_response() {
        let (_tx, rx) = mpsc::channel::<Vec<u8>>();
        let writer = Cursor::new(Vec::<u8>::new());
        let handler: Arc<dyn RpcHandler> = Arc::new(NoopHandler);

        struct BlockingReader {
            rx: mpsc::Receiver<Vec<u8>>,
            buffer: Vec<u8>,
            cursor: usize,
        }

        impl std::io::Read for BlockingReader {
            fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
                if self.cursor >= self.buffer.len() {
                    self.buffer = self.rx.recv().map_err(|_| {
                        std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "channel closed")
                    })?;
                    self.cursor = 0;
                }

                let remaining = self.buffer.len().saturating_sub(self.cursor);
                let n = remaining.min(out.len());
                out[..n].copy_from_slice(&self.buffer[self.cursor..self.cursor + n]);
                self.cursor += n;
                Ok(n)
            }
        }

        impl std::io::BufRead for BlockingReader {
            fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
                if self.cursor >= self.buffer.len() {
                    self.buffer = self.rx.recv().map_err(|_| {
                        std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "channel closed")
                    })?;
                    self.cursor = 0;
                }
                Ok(&self.buffer[self.cursor..])
            }

            fn consume(&mut self, amt: usize) {
                self.cursor = (self.cursor + amt).min(self.buffer.len());
            }
        }

        let reader = BlockingReader {
            rx,
            buffer: Vec::new(),
            cursor: 0,
        };

        let connection = RpcConnection::new(reader, writer, handler);
        let result = connection.send_request("test.request", Value::Null);

        assert!(matches!(result, Err(IpcError::Timeout { .. })));
    }

    #[test]
    fn test_send_request_succeeds_with_loopback_response() {
        let handler: Arc<dyn RpcHandler> = Arc::new(NoopHandler);
        let connection = make_loopback_connection(handler);

        let response = connection
            .send_request("test.loopback", serde_json::json!({"ok": true}))
            .expect("loopback request should receive response");

        assert_eq!(response, Value::Null);
    }

    #[test]
    fn test_handler_panic_returns_disconnect_or_timeout() {
        let (request_tx, request_rx) = mpsc::channel::<Vec<u8>>();
        let writer = Cursor::new(Vec::<u8>::new());
        let handler: Arc<dyn RpcHandler> = Arc::new(CrashHandler);

        struct ChannelReader {
            rx: mpsc::Receiver<Vec<u8>>,
            buffer: Vec<u8>,
            cursor: usize,
        }

        impl std::io::Read for ChannelReader {
            fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
                if self.cursor >= self.buffer.len() {
                    self.buffer = self.rx.recv().map_err(|_| {
                        std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "channel closed")
                    })?;
                    self.cursor = 0;
                }
                let remaining = self.buffer.len().saturating_sub(self.cursor);
                let n = remaining.min(out.len());
                out[..n].copy_from_slice(&self.buffer[self.cursor..self.cursor + n]);
                self.cursor += n;
                Ok(n)
            }
        }

        impl std::io::BufRead for ChannelReader {
            fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
                if self.cursor >= self.buffer.len() {
                    self.buffer = self.rx.recv().map_err(|_| {
                        std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "channel closed")
                    })?;
                    self.cursor = 0;
                }
                Ok(&self.buffer[self.cursor..])
            }

            fn consume(&mut self, amt: usize) {
                self.cursor = (self.cursor + amt).min(self.buffer.len());
            }
        }

        let reader = ChannelReader {
            rx: request_rx,
            buffer: Vec::new(),
            cursor: 0,
        };

        let connection = RpcConnection::new(reader, writer, handler);

        let req = RpcRequest::new(99, "panic", Value::Null);
        let encoded = serde_json::to_vec(&RpcMessage::Request(req)).expect("encode request");
        let mut line = encoded;
        line.push(b'\n');
        request_tx.send(line).expect("send request line");
        drop(request_tx);

        let result = connection.send_request("host.wait", Value::Null);
        assert!(matches!(
            result,
            Err(IpcError::Disconnected)
                | Err(IpcError::ResponseChannelClosed)
                | Err(IpcError::Timeout { .. })
        ));
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
