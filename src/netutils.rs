use std::path::Path;

use reqwest::{blocking::Client, header::AUTHORIZATION, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Meta {
    pub server_ip: String,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Serialize, Debug)]
pub struct LoginRequest {
    pub key: String,
}

#[derive(Deserialize, Debug)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
}

pub fn make_get_request<T>(
    url_path: &str,
    data_dir: &Path,
    query: Option<&[(&str, &str)]>,
) -> Result<T, Box<dyn std::error::Error>>
where
    T: serde::de::DeserializeOwned,
{
    let meta_path = data_dir.join("meta.json");
    let meta_json = std::fs::read_to_string(meta_path)?;
    let meta: Meta = serde_json::from_str(&meta_json)?;

    let client = Client::new();
    let response = client
        .get(format!("http://{}{}", meta.server_ip, url_path))
        .header(AUTHORIZATION, format!("Bearer {}", meta.access_token));

    let response = match query {
        Some(query) => response.query(query),
        None => response,
    };
    let response = response.send()?;

    match response.status() {
        StatusCode::OK => {
            let response = response.json::<T>()?;
            Ok(response)
        }
        StatusCode::UNAUTHORIZED => Err("Unauthorized".into()),
        StatusCode::NETWORK_AUTHENTICATION_REQUIRED => {
            let refresh_token = meta.refresh_token;
            let new_tokens = client
                .post(format!("http://{}/auth/refresh", meta.server_ip))
                .body(refresh_token)
                .send()?;

            let new_tokens = match new_tokens.status() {
                StatusCode::OK => new_tokens.json::<LoginResponse>()?,
                StatusCode::UNAUTHORIZED => {
                    return Err("Unauthorized".into());
                }
                StatusCode::NETWORK_AUTHENTICATION_REQUIRED => {
                    return Err("Login required".into());
                }
                _ => {
                    return Err("Unknown error".into());
                }
            };

            let meta = Meta {
                server_ip: meta.server_ip,
                access_token: new_tokens.access_token,
                refresh_token: new_tokens.refresh_token,
            };

            let meta_json = serde_json::to_string(&meta)?;
            let meta_path = data_dir.join("meta.json");
            std::fs::write(meta_path, meta_json)?;

            let response = client
                .get(format!("http://{}{}", meta.server_ip, url_path))
                .header(AUTHORIZATION, format!("Bearer {}", meta.access_token))
                .send()?
                .json::<T>()?;

            Ok(response)
        }
        _ => Err("Unknown error".into()),
    }
}
