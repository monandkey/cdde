use std::env;

pub fn get_base_url() -> String {
    env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:3000/api/v1".to_string())
}

pub fn get_client() -> reqwest::Client {
    reqwest::Client::new()
}
