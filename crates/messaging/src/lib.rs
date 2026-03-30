#![forbid(unsafe_code)]

mod engine;
mod envelope;
mod error;
mod group;
mod handshake;
mod session;

pub use engine::{DeliveredMessage, MessagingEngine, OutgoingMessage, ReceiveOutcome};
pub use envelope::MessageKind;
pub use error::MessagingError;
pub use group::{
    GroupDecryptedMessage, GroupEncryptedMessage, GroupEpochRotation, GroupMembership,
    GroupParticipant, GroupRotationReason, GroupSenderKeyDistribution, GroupSession,
};
pub use session::{
    RemoteSessionOffer, SecureSession, SessionInitiator, SessionResponder,
    transport_certificate_sha256,
};

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddr};

    use localmessenger_core::{Device, DeviceId, MemberId};
    use localmessenger_crypto::{IdentityKeyPair, LocalPrekeyStore, SessionRole};
    use localmessenger_transport::{
        ReconnectPolicy, TransportEndpoint, TransportEndpointConfig, TransportIdentity,
    };
    use rand_core::OsRng;

    use crate::{
        MessageKind, MessagingEngine, MessagingError, RemoteSessionOffer, SecureSession,
        SessionInitiator, SessionResponder, transport_certificate_sha256,
    };

    struct SessionPair {
        alice_device: Device,
        bob_device: Device,
        alice_session: SecureSession,
        bob_session: SecureSession,
        bob_transport_identity: TransportIdentity,
    }

    fn make_device(
        member_id: &str,
        device_id: &str,
        device_name: &str,
        identity: &IdentityKeyPair,
    ) -> Device {
        Device::from_identity_keypair(
            DeviceId::new(device_id).expect("device id should be valid"),
            MemberId::new(member_id).expect("member id should be valid"),
            device_name,
            identity,
        )
        .expect("device should be created")
    }

    async fn establish_session_pair() -> SessionPair {
        let server_config =
            TransportEndpointConfig::recommended(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)));
        let server_identity = TransportIdentity::generate(server_config.server_name.clone())
            .expect("server transport identity should generate");
        let server = TransportEndpoint::bind(server_config, server_identity.clone())
            .expect("server endpoint should bind");
        let server_addr = server.local_addr().expect("server address should exist");

        let client_config =
            TransportEndpointConfig::recommended(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)));
        let client_identity = TransportIdentity::generate(client_config.server_name.clone())
            .expect("client transport identity should generate");
        let client = TransportEndpoint::bind(client_config, client_identity)
            .expect("client endpoint should bind");

        let mut rng = OsRng;
        let alice_identity = IdentityKeyPair::generate(&mut rng);
        let bob_identity = IdentityKeyPair::generate(&mut rng);

        let alice_device = make_device("alice", "alice-phone", "Alice Phone", &alice_identity);
        let bob_device = make_device("bob", "bob-laptop", "Bob Laptop", &bob_identity);

        let bob_prekeys = LocalPrekeyStore::generate(&mut rng, &bob_identity, 41, 4, 1000);
        let mut responder = SessionResponder::new(
            bob_device.clone(),
            bob_identity,
            bob_prekeys,
            &server_identity.certificate_der,
        )
        .expect("responder context should build");
        let offer = responder
            .remote_session_offer()
            .expect("remote session offer should build");

        let accept_task = tokio::spawn(async move {
            let connection = server
                .accept()
                .await
                .expect("server should accept transport");
            responder
                .accept(connection)
                .await
                .expect("secure session should accept")
        });

        let connection = client
            .connect(
                server_addr,
                &server_identity.certificate_der,
                &ReconnectPolicy::lan_default(),
            )
            .await
            .expect("client should connect");
        let initiator = SessionInitiator::new(alice_device.clone(), alice_identity)
            .expect("initiator context should build");
        let alice_session = initiator
            .establish(connection, &offer, &server_identity.certificate_der)
            .await
            .expect("initiator should establish secure session");
        let bob_session = accept_task.await.expect("join should succeed");

        SessionPair {
            alice_device,
            bob_device,
            alice_session,
            bob_session,
            bob_transport_identity: server_identity,
        }
    }

    #[tokio::test]
    async fn secure_session_bootstrap_supports_encrypted_round_trip() {
        let SessionPair {
            alice_device,
            bob_device,
            mut alice_session,
            mut bob_session,
            bob_transport_identity,
        } = establish_session_pair().await;

        assert_eq!(alice_session.role(), SessionRole::Initiator);
        assert_eq!(
            alice_session.remote_device().device_id(),
            bob_device.device_id()
        );
        assert_eq!(
            alice_session.remote_transport_certificate_sha256(),
            Some(transport_certificate_sha256(
                &bob_transport_identity.certificate_der
            ))
        );

        alice_session
            .send_encrypted(b"hello-from-alice")
            .await
            .expect("client should encrypt message");
        let inbound = bob_session
            .receive_encrypted()
            .await
            .expect("server should decrypt message");
        bob_session
            .send_encrypted(b"reply-from-bob")
            .await
            .expect("server should encrypt reply");
        let reply = alice_session
            .receive_encrypted()
            .await
            .expect("client should decrypt reply");

        assert_eq!(bob_session.role(), SessionRole::Responder);
        assert_eq!(inbound, b"hello-from-alice");
        assert_eq!(reply, b"reply-from-bob");
        assert_eq!(alice_session.session_id(), bob_session.session_id());
        assert_eq!(
            bob_session.remote_device().device_id(),
            alice_device.device_id()
        );
        assert_eq!(bob_session.remote_transport_certificate_sha256(), None);
    }

    #[tokio::test]
    async fn initiator_rejects_transport_binding_mismatch_before_handshake() {
        let server_config =
            TransportEndpointConfig::recommended(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)));
        let server_identity = TransportIdentity::generate(server_config.server_name.clone())
            .expect("server transport identity should generate");
        let server = TransportEndpoint::bind(server_config, server_identity.clone())
            .expect("server endpoint should bind");
        let server_addr = server.local_addr().expect("server address should exist");

        let client_config =
            TransportEndpointConfig::recommended(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)));
        let client_identity = TransportIdentity::generate(client_config.server_name.clone())
            .expect("client transport identity should generate");
        let client = TransportEndpoint::bind(client_config, client_identity)
            .expect("client endpoint should bind");

        let wrong_transport_identity = TransportIdentity::generate("wrong.local")
            .expect("wrong transport identity should generate");

        let mut rng = OsRng;
        let alice_identity = IdentityKeyPair::generate(&mut rng);
        let bob_identity = IdentityKeyPair::generate(&mut rng);

        let alice_device = make_device("alice", "alice-main", "Alice Main", &alice_identity);
        let bob_device = make_device("bob", "bob-main", "Bob Main", &bob_identity);
        let bob_prekeys = LocalPrekeyStore::generate(&mut rng, &bob_identity, 51, 3, 2000);

        let responder = SessionResponder::new(
            bob_device.clone(),
            bob_identity,
            bob_prekeys,
            &server_identity.certificate_der,
        )
        .expect("responder context should build");
        let offer = RemoteSessionOffer::from_parts(
            bob_device,
            responder
                .remote_session_offer()
                .expect("offer should build")
                .prekey_bundle()
                .clone(),
            transport_certificate_sha256(&wrong_transport_identity.certificate_der),
        )
        .expect("mismatched offer should still be constructible");
        let accept_task = tokio::spawn(async move {
            let connection = server
                .accept()
                .await
                .expect("server should accept transport");
            connection.close("test complete");
        });

        let connection = client
            .connect(
                server_addr,
                &server_identity.certificate_der,
                &ReconnectPolicy::lan_default(),
            )
            .await
            .expect("client should connect");
        let initiator = SessionInitiator::new(alice_device, alice_identity)
            .expect("initiator context should build");

        let error = initiator
            .establish(connection, &offer, &server_identity.certificate_der)
            .await
            .err()
            .expect("mismatched binding must fail");
        assert!(matches!(error, MessagingError::TransportBindingMismatch));
        accept_task.await.expect("accept task should join");
    }

    #[tokio::test]
    async fn messaging_engine_orders_out_of_order_delivery_and_processes_acks() {
        let SessionPair {
            mut alice_session,
            mut bob_session,
            ..
        } = establish_session_pair().await;
        let mut alice_engine = MessagingEngine::from_session(&alice_session);
        let mut bob_engine = MessagingEngine::from_session(&bob_session);

        alice_engine
            .queue_message(
                "msg-1",
                "chat-main",
                MessageKind::Text,
                1_711_111_111_000,
                b"first".to_vec(),
            )
            .expect("first message should queue");
        alice_engine
            .queue_message(
                "msg-2",
                "chat-main",
                MessageKind::Text,
                1_711_111_112_000,
                b"second".to_vec(),
            )
            .expect("second message should queue");

        alice_engine
            .retry_message(&mut alice_session, "msg-2")
            .await
            .expect("second message should send");
        alice_engine
            .retry_message(&mut alice_session, "msg-1")
            .await
            .expect("first message should send");

        let first_receive = bob_engine
            .receive_next(&mut bob_session)
            .await
            .expect("out-of-order message should be processed");
        assert!(first_receive.is_idle());

        let second_receive = bob_engine
            .receive_next(&mut bob_session)
            .await
            .expect("in-order message should release buffered deliveries");
        assert_eq!(second_receive.delivered_messages().len(), 2);
        assert_eq!(second_receive.delivered_messages()[0].message_id(), "msg-1");
        assert_eq!(second_receive.delivered_messages()[1].message_id(), "msg-2");
        assert_eq!(second_receive.delivered_messages()[0].body(), b"first");
        assert_eq!(second_receive.delivered_messages()[1].body(), b"second");
        assert_eq!(bob_engine.next_expected_incoming_order(), 2);

        let ack_one = alice_engine
            .receive_next(&mut alice_session)
            .await
            .expect("first ack should arrive");
        let ack_two = alice_engine
            .receive_next(&mut alice_session)
            .await
            .expect("second ack should arrive");
        let mut acked = ack_one.acknowledged_message_ids().to_vec();
        acked.extend(ack_two.acknowledged_message_ids().iter().cloned());
        acked.sort();

        assert_eq!(acked, vec!["msg-1".to_string(), "msg-2".to_string()]);
        assert_eq!(alice_engine.pending_count(), 0);
        assert!(!alice_engine.is_pending("msg-1"));
        assert!(!alice_engine.is_pending("msg-2"));
    }

    #[tokio::test]
    async fn messaging_engine_retry_is_duplicate_safe() {
        let SessionPair {
            mut alice_session,
            mut bob_session,
            ..
        } = establish_session_pair().await;
        let mut alice_engine = MessagingEngine::from_session(&alice_session);
        let mut bob_engine = MessagingEngine::from_session(&bob_session);

        let first_attempt = alice_engine
            .send_message(
                &mut alice_session,
                "dup-1",
                "chat-main",
                MessageKind::Text,
                1_711_111_120_000,
                b"hello".to_vec(),
            )
            .await
            .expect("message should send");
        assert_eq!(first_attempt.attempt_count(), 1);

        let second_attempt = alice_engine
            .retry_message(&mut alice_session, "dup-1")
            .await
            .expect("message retry should send duplicate copy");
        assert_eq!(second_attempt.attempt_count(), 2);

        let first_receive = bob_engine
            .receive_next(&mut bob_session)
            .await
            .expect("first delivery should succeed");
        let second_receive = bob_engine
            .receive_next(&mut bob_session)
            .await
            .expect("duplicate delivery should still be processed");

        assert_eq!(first_receive.delivered_messages().len(), 1);
        assert_eq!(first_receive.delivered_messages()[0].message_id(), "dup-1");
        assert!(second_receive.delivered_messages().is_empty());

        let ack_one = alice_engine
            .receive_next(&mut alice_session)
            .await
            .expect("first ack should arrive");
        let ack_two = alice_engine
            .receive_next(&mut alice_session)
            .await
            .expect("duplicate ack should arrive");

        assert_eq!(ack_one.acknowledged_message_ids(), &["dup-1".to_string()]);
        assert!(ack_two.acknowledged_message_ids().is_empty());
        assert_eq!(alice_engine.pending_count(), 0);
    }

    #[test]
    fn messaging_engine_rejects_duplicate_outgoing_message_ids() {
        let mut rng = OsRng;
        let alice_identity = IdentityKeyPair::generate(&mut rng);
        let bob_identity = IdentityKeyPair::generate(&mut rng);
        let alice_device = make_device("alice", "alice-phone", "Alice Phone", &alice_identity);
        let bob_device = make_device("bob", "bob-phone", "Bob Phone", &bob_identity);
        let mut engine = MessagingEngine::new(alice_device, bob_device);

        engine
            .queue_message(
                "same-id",
                "chat-main",
                MessageKind::Text,
                1,
                b"one".to_vec(),
            )
            .expect("first message should queue");
        let error = engine
            .queue_message(
                "same-id",
                "chat-main",
                MessageKind::Text,
                2,
                b"two".to_vec(),
            )
            .expect_err("duplicate ids must be rejected");

        assert!(matches!(
            error,
            MessagingError::DuplicateOutgoingMessageId(message_id) if message_id == "same-id"
        ));
    }
}
