use std::net::SocketAddr;

use quinn::{Connection, Endpoint, Incoming};

use crate::config::ReconnectPolicy;
use crate::error::TransportError;
use crate::frame::{TransportFrame, read_frame, write_frame};

#[derive(Clone)]
pub struct TransportConnection {
    connection: Connection,
    max_frame_size: usize,
}

impl TransportConnection {
    pub(crate) fn new(connection: Connection, max_frame_size: usize) -> Self {
        Self {
            connection,
            max_frame_size,
        }
    }

    pub fn remote_address(&self) -> SocketAddr {
        self.connection.remote_address()
    }

    pub async fn send_frame(&self, frame: &TransportFrame) -> Result<(), TransportError> {
        let mut send = self
            .connection
            .open_uni()
            .await
            .map_err(|error| TransportError::Connect(error.to_string()))?;
        write_frame(&mut send, frame, self.max_frame_size).await
    }

    pub async fn receive_frame(&self) -> Result<TransportFrame, TransportError> {
        let mut recv = self
            .connection
            .accept_uni()
            .await
            .map_err(|error| TransportError::Connect(error.to_string()))?;
        read_frame(&mut recv, self.max_frame_size).await
    }

    pub fn close(&self, reason: &'static str) {
        self.connection.close(0_u32.into(), reason.as_bytes());
    }
}

pub async fn accept_incoming(
    incoming: Incoming,
    max_frame_size: usize,
) -> Result<TransportConnection, TransportError> {
    let connection = incoming
        .await
        .map_err(|error| TransportError::Connect(error.to_string()))?;
    Ok(TransportConnection::new(connection, max_frame_size))
}

pub async fn connect_with_retry(
    endpoint: &Endpoint,
    remote_addr: SocketAddr,
    server_name: &str,
    policy: &ReconnectPolicy,
    max_frame_size: usize,
) -> Result<TransportConnection, TransportError> {
    let mut last_error = None;

    for attempt in 0..policy.max_attempts {
        let delay = policy.backoff_for_attempt(attempt);
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }

        match endpoint.connect(remote_addr, server_name) {
            Ok(connecting) => match connecting.await {
                Ok(connection) => return Ok(TransportConnection::new(connection, max_frame_size)),
                Err(error) => last_error = Some(error.to_string()),
            },
            Err(error) => last_error = Some(error.to_string()),
        }
    }

    Err(TransportError::RetryExhausted {
        attempts: policy.max_attempts,
        last_error: last_error.unwrap_or_else(|| "unknown connection error".to_string()),
    })
}
