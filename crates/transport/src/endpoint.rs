use std::net::SocketAddr;

use quinn::Endpoint;

use crate::cert::{TransportIdentity, make_client_config, make_server_config};
use crate::config::{ReconnectPolicy, TransportEndpointConfig};
use crate::connection::{TransportConnection, accept_incoming, connect_with_retry};
use crate::error::TransportError;

pub struct TransportEndpoint {
    endpoint: Endpoint,
    config: TransportEndpointConfig,
    identity: TransportIdentity,
}

impl TransportEndpoint {
    pub fn bind(
        config: TransportEndpointConfig,
        identity: TransportIdentity,
    ) -> Result<Self, TransportError> {
        let server_config = make_server_config(&identity, &config)?;
        let endpoint = Endpoint::server(server_config, config.bind_addr)
            .map_err(|error| TransportError::Endpoint(error.to_string()))?;

        Ok(Self {
            endpoint,
            config,
            identity,
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, TransportError> {
        self.endpoint
            .local_addr()
            .map_err(|error| TransportError::Endpoint(error.to_string()))
    }

    pub fn identity(&self) -> &TransportIdentity {
        &self.identity
    }

    pub async fn accept(&self) -> Result<TransportConnection, TransportError> {
        let incoming = self
            .endpoint
            .accept()
            .await
            .ok_or(TransportError::ConnectionClosed)?;
        accept_incoming(incoming, self.config.max_frame_size).await
    }

    pub async fn connect(
        &self,
        remote_addr: SocketAddr,
        trusted_server_certificate: &[u8],
        policy: &ReconnectPolicy,
    ) -> Result<TransportConnection, TransportError> {
        let client_config = make_client_config(trusted_server_certificate, &self.config)?;
        let mut endpoint = self.endpoint.clone();
        endpoint.set_default_client_config(client_config);

        connect_with_retry(
            &endpoint,
            remote_addr,
            &self.config.server_name,
            policy,
            self.config.max_frame_size,
        )
        .await
    }
}
