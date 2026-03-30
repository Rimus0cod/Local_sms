use std::sync::Arc;
use std::sync::OnceLock;

use quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig};
use quinn::{ClientConfig, ServerConfig};
use rcgen::{CertifiedKey, generate_simple_self_signed};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

use crate::config::TransportEndpointConfig;
use crate::error::TransportError;

static RUSTLS_PROVIDER: OnceLock<()> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct TransportIdentity {
    pub server_name: String,
    pub certificate_der: Vec<u8>,
    pub private_key_der: Vec<u8>,
}

impl TransportIdentity {
    pub fn generate(server_name: impl Into<String>) -> Result<Self, TransportError> {
        let server_name = server_name.into();
        let CertifiedKey { cert, signing_key } =
            generate_simple_self_signed(vec![server_name.clone()])
                .map_err(|error| TransportError::CertificateGeneration(error.to_string()))?;

        Ok(Self {
            server_name,
            certificate_der: cert.der().to_vec(),
            private_key_der: signing_key.serialize_der(),
        })
    }

    pub fn certificate(&self) -> CertificateDer<'static> {
        CertificateDer::from(self.certificate_der.clone())
    }

    pub fn private_key(&self) -> PrivateKeyDer<'static> {
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(self.private_key_der.clone()))
    }
}

pub fn make_server_config(
    identity: &TransportIdentity,
    endpoint: &TransportEndpointConfig,
) -> Result<ServerConfig, TransportError> {
    ensure_crypto_provider();

    let mut server_crypto = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![identity.certificate()], identity.private_key())
        .map_err(|error| TransportError::Rustls(error.to_string()))?;
    server_crypto.alpn_protocols = vec![endpoint.alpn_protocol.clone()];

    let mut server_config = ServerConfig::with_crypto(Arc::new(
        QuicServerConfig::try_from(server_crypto)
            .map_err(|error| TransportError::Rustls(error.to_string()))?,
    ));
    server_config.transport = endpoint.quinn_transport_config()?;
    Ok(server_config)
}

pub fn make_client_config(
    trusted_server_certificate: &[u8],
    endpoint: &TransportEndpointConfig,
) -> Result<ClientConfig, TransportError> {
    ensure_crypto_provider();

    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(CertificateDer::from(trusted_server_certificate.to_vec()))
        .map_err(|error| TransportError::Rustls(error.to_string()))?;

    let mut client_crypto = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    client_crypto.alpn_protocols = vec![endpoint.alpn_protocol.clone()];

    let mut client_config = ClientConfig::new(Arc::new(
        QuicClientConfig::try_from(client_crypto)
            .map_err(|error| TransportError::Rustls(error.to_string()))?,
    ));
    client_config.transport_config(endpoint.quinn_transport_config()?);
    Ok(client_config)
}

fn ensure_crypto_provider() {
    RUSTLS_PROVIDER.get_or_init(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}
