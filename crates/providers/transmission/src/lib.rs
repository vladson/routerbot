use transmission_rpc::TransClient;

pub struct TransmissionProvider {
    client: TransClient,
}

impl TransmissionProvider {
    pub fn new(url: &str) -> Self {
        Self { client: TransClient::new(url) }
    }

    pub fn client(&self) -> &TransClient {
        &self.client
    }
}
