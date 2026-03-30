use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum TransportError {
    CertificateGeneration(String),
    Rustls(String),
    Endpoint(String),
    Connect(String),
    ConnectionClosed,
    FrameTooLarge(usize),
    FrameEncoding(String),
    FrameDecoding(String),
    Io(String),
    RetryExhausted { attempts: usize, last_error: String },
}

impl fmt::Display for TransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CertificateGeneration(error) => {
                write!(formatter, "certificate generation error: {error}")
            }
            Self::Rustls(error) => write!(formatter, "rustls error: {error}"),
            Self::Endpoint(error) => write!(formatter, "endpoint error: {error}"),
            Self::Connect(error) => write!(formatter, "connect error: {error}"),
            Self::ConnectionClosed => formatter.write_str("connection closed"),
            Self::FrameTooLarge(size) => write!(formatter, "frame too large: {size} bytes"),
            Self::FrameEncoding(error) => write!(formatter, "frame encoding error: {error}"),
            Self::FrameDecoding(error) => write!(formatter, "frame decoding error: {error}"),
            Self::Io(error) => write!(formatter, "io error: {error}"),
            Self::RetryExhausted {
                attempts,
                last_error,
            } => write!(
                formatter,
                "connection retry exhausted after {attempts} attempts: {last_error}"
            ),
        }
    }
}

impl Error for TransportError {}

impl From<std::io::Error> for TransportError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}
