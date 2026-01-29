#![allow(clippy::missing_errors_doc)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::significant_drop_tightening)]
#![allow(clippy::use_self)]

mod connection;
mod error;
pub mod messages;
pub mod methods;
mod protocol;

pub use connection::{RpcConnection, RpcHandler};
pub use error::IpcError;
pub use messages::*;
pub use protocol::{
    RpcErrorData, RpcMessage, RpcNotification, RpcRequest, RpcResponse, RPC_VERSION,
};
