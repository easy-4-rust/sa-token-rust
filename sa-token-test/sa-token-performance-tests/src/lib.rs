use std::collections::HashMap;

use sa_token_adapter::SaRequest;

#[derive(Clone)]
pub struct BenchRequest {
    pub token: String,
}

impl SaRequest for BenchRequest {
    fn get_header(&self, name: &str) -> Option<String> {
        name.eq_ignore_ascii_case("satoken")
            .then(|| self.token.clone())
    }

    fn get_cookie(&self, _: &str) -> Option<String> {
        None
    }

    fn get_param(&self, _: &str) -> Option<String> {
        None
    }

    fn get_headers(&self) -> HashMap<String, String> {
        HashMap::from([("satoken".to_string(), self.token.clone())])
    }

    fn get_path(&self) -> String {
        "/api/orders/42".to_string()
    }

    fn get_method(&self) -> String {
        "GET".to_string()
    }
}
