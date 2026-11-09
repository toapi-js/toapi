toapi_client::generate!("../users-server/openapi.json", derive(bloom_core::Identify));

thread_local! {
    pub static CLIENT: Api = Api::new("http://localhost:5010");
}
