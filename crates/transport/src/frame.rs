use quinn::{RecvStream, SendStream};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::TransportError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportFrame {
    Ping,
    Pong,
    Handshake(Vec<u8>),
    Payload(Vec<u8>),
    Ack { sequence: u64 },
}

impl TransportFrame {
    pub fn payload(bytes: impl Into<Vec<u8>>) -> Self {
        Self::Payload(bytes.into())
    }
}

pub async fn write_frame(
    send: &mut SendStream,
    frame: &TransportFrame,
    max_frame_size: usize,
) -> Result<(), TransportError> {
    let encoded = bincode::serialize(frame)
        .map_err(|error| TransportError::FrameEncoding(error.to_string()))?;
    if encoded.len() > max_frame_size {
        return Err(TransportError::FrameTooLarge(encoded.len()));
    }

    send.write_u32(encoded.len() as u32)
        .await
        .map_err(|error| TransportError::Io(error.to_string()))?;
    send.write_all(&encoded)
        .await
        .map_err(|error| TransportError::Io(error.to_string()))?;
    send.finish()
        .map_err(|error| TransportError::Io(error.to_string()))?;
    Ok(())
}

pub async fn read_frame(
    recv: &mut RecvStream,
    max_frame_size: usize,
) -> Result<TransportFrame, TransportError> {
    let len = recv
        .read_u32()
        .await
        .map_err(|error| TransportError::Io(error.to_string()))? as usize;
    if len > max_frame_size {
        return Err(TransportError::FrameTooLarge(len));
    }

    let mut buffer = vec![0_u8; len];
    recv.read_exact(&mut buffer)
        .await
        .map_err(|error| TransportError::Io(error.to_string()))?;
    bincode::deserialize(&buffer).map_err(|error| TransportError::FrameDecoding(error.to_string()))
}
