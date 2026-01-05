pub mod app_ids;
pub mod capabilities;

use serde::Deserialize;

use crate::developer::{DeveloperSession, RequestType};

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct V1ErrorResponse {
    pub errors: Vec<V1ErrorDetail>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct V1ErrorDetail {
    pub code: String,
    pub detail: Option<String>,
    pub id: String,
    pub result_code: i64,
    pub status: String,
    pub title: Option<String>,
}

impl V1ErrorDetail {
    pub fn to_error(&self, url: String) -> crate::Error {
        let message = self
            .detail
            .clone()
            .or(self.title.clone())
            .unwrap_or_else(|| "Unknown API error".to_string());

        crate::Error::DeveloperApi {
            url,
            result_code: self.result_code,
            http_code: self.status.parse().ok(),
            message,
        }
    }
}
