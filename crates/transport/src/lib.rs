#![forbid(unsafe_code)]

mod cert;
mod config;
mod connection;
mod endpoint;
mod error;
mod frame;

pub use cert::{TransportIdentity, make_client_config, make_server_config};
pub use config::{ReconnectPolicy, TransportEndpointConfig};
pub use connection::TransportConnection;
pub use endpoint::TransportEndpoint;
pub use error::TransportError;
pub use frame::{TransportFrame, read_frame, write_frame};

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
    use std::time::Duration;

    use crate::{
        ReconnectPolicy, TransportEndpoint, TransportEndpointConfig, TransportFrame,
        TransportIdentity,
    };

    #[tokio::test]
    #[ignore = "network-required: binds UDP sockets"]
    async fn quic_frame_round_trip_over_loopback() {
        let server_config =
            TransportEndpointConfig::recommended(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)));
        let server_identity = TransportIdentity::generate(server_config.server_name.clone())
            .expect("identity should generate");
        let server = TransportEndpoint::bind(server_config, server_identity.clone())
            .expect("server should bind");

        let client_config =
            TransportEndpointConfig::recommended(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)));
        let client_identity = TransportIdentity::generate(client_config.server_name.clone())
            .expect("identity should generate");
        let client =
            TransportEndpoint::bind(client_config, client_identity).expect("client should bind");

        let server_addr = server
            .local_addr()
            .expect("server addr should be available");
        let accept_task = tokio::spawn(async move {
            let connection = server.accept().await.expect("server should accept");
            connection
                .receive_frame()
                .await
                .expect("server should receive frame")
        });

        let connection = client
            .connect(
                server_addr,
                &server_identity.certificate_der,
                &ReconnectPolicy::lan_default(),
            )
            .await
            .expect("client should connect");
        connection
            .send_frame(&TransportFrame::payload(b"hello quic".to_vec()))
            .await
            .expect("client should send frame");

        let received = accept_task.await.expect("join should succeed");
        assert_eq!(received, TransportFrame::payload(b"hello quic".to_vec()));
    }

    #[tokio::test]
    #[ignore = "network-required: binds UDP sockets"]
    async fn connect_retries_until_server_appears() {
        let reserved = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).expect("port should reserve");
        let server_addr = reserved.local_addr().expect("reserved addr should exist");
        drop(reserved);

        let server_config = TransportEndpointConfig::recommended(server_addr);
        let server_identity = TransportIdentity::generate(server_config.server_name.clone())
            .expect("identity should generate");

        let client_config =
            TransportEndpointConfig::recommended(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)));
        let client_identity = TransportIdentity::generate(client_config.server_name.clone())
            .expect("identity should generate");
        let client =
            TransportEndpoint::bind(client_config, client_identity).expect("client should bind");

        let retry_policy =
            ReconnectPolicy::new(12, Duration::from_millis(50), Duration::from_millis(200));
        let client_task = {
            let trusted_cert = server_identity.certificate_der.clone();
            tokio::spawn(async move {
                client
                    .connect(server_addr, &trusted_cert, &retry_policy)
                    .await
            })
        };

        tokio::time::sleep(Duration::from_millis(180)).await;

        let server =
            TransportEndpoint::bind(server_config, server_identity).expect("server should bind");
        let accept_task = tokio::spawn(async move { server.accept().await });

        let connection = client_task
            .await
            .expect("client task should join")
            .expect("client should eventually connect");
        assert_eq!(connection.remote_address(), server_addr);

        let server_connection = accept_task
            .await
            .expect("accept task should join")
            .expect("server should accept eventual client");
        assert_eq!(server_connection.remote_address().ip(), Ipv4Addr::LOCALHOST);
    }
}
