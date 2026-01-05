pub mod account;
pub mod app_groups;
pub mod app_ids;
pub mod certs;
pub mod devices;
pub mod profile;
pub mod teams;

use crate::developer::DeveloperSession;
use plist::Integer;
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QHResponseMeta {
    pub creation_timestamp: String,
    pub user_string: Option<String>,
    pub result_string: Option<String>,
    pub result_code: Integer,
    pub http_code: Option<Integer>,
    pub user_locale: String,
    pub protocol_version: String,
    pub request_id: Option<String>,
    pub result_url: Option<String>,
    pub response_id: String,
    pub page_number: Option<Integer>,
    pub page_size: Option<Integer>,
    pub total_records: Option<Integer>,
}

impl QHResponseMeta {
    pub fn to_error(self, url: String) -> crate::Error {
        let message = self
            .user_string
            .or(self.result_string)
            .unwrap_or_else(|| "Unknown API error".to_string());

        crate::Error::DeveloperApi {
            url,
            result_code: self.result_code.as_signed().unwrap_or(0),
            http_code: self.http_code.and_then(|c| c.as_signed().map(|v| v as u16)),
            message,
        }
    }
}
