use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use iroh_quinn::{ClientConfig, Endpoint, EndpointConfig, ServerConfig};
use rcgen::CertifiedKey;

use crate::socket::VirtualSocket;

pub struct TestEndpoint {
    endpoint: Endpoint,
    cert: CertifiedKey,
}

impl Deref for TestEndpoint {
    type Target = Endpoint;

    fn deref(&self) -> &Self::Target {
        &self.endpoint
    }
}

impl DerefMut for TestEndpoint {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.endpoint
    }
}

impl TestEndpoint {
    pub fn server(socket: VirtualSocket) -> Self {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let key = rustls::pki_types::PrivateKeyDer::Pkcs8(cert.key_pair.serialize_der().into());

        let server_config =
            ServerConfig::with_single_cert(vec![cert.cert.der().clone()], key).unwrap();

        let endpoint = Endpoint::new_with_abstract_socket(
            EndpointConfig::default(),
            Some(server_config),
            Box::new(socket),
            Arc::new(iroh_quinn::TokioRuntime),
        )
        .unwrap();
        Self { endpoint, cert }
    }

    pub fn client(socket: VirtualSocket) -> Self {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();

        let endpoint = Endpoint::new_with_abstract_socket(
            EndpointConfig::default(),
            None,
            Box::new(socket),
            Arc::new(iroh_quinn::TokioRuntime),
        )
        .unwrap();
        Self { endpoint, cert }
    }

    pub fn make_client_for(&mut self, server: &TestEndpoint) {
        self.endpoint
            .set_default_client_config(server.client_config());
    }

    pub fn client_config(&self) -> ClientConfig {
        let mut roots = rustls::RootCertStore::empty();
        roots.add(self.cert.cert.der().clone()).unwrap();
        ClientConfig::with_root_certificates(Arc::new(roots)).unwrap()
    }
}
