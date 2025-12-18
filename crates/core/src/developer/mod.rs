pub mod qh;
pub mod v1;

use plist::{Dictionary, Value};
use uuid::Uuid;

use crate::Error;

use crate::SessionRequestTrait;
use crate::auth::{Account, account::request::RequestType};
use crate::developer::qh::ResponseMeta;

#[macro_export]
macro_rules! developer_endpoint {
    ($endpoint:expr) => {
        format!("https://developerservices2.apple.com/services{}", $endpoint)
    };
}

pub struct DeveloperSession {
    pub account: Account,
}

impl DeveloperSession {
    pub fn with(account: Account) -> Self {
        DeveloperSession {
            account
        }
    }
}

impl SessionRequestTrait for DeveloperSession {
    async fn qh_send_request(
        &self,
        url: &str,
        body: Option<Dictionary>,
    ) -> Result<Dictionary, Error> {
        let mut request = Dictionary::new();
        request.insert(
            "requestId".to_string(),
            Value::String(Uuid::new_v4().to_string().to_uppercase()),
        );
        if let Some(body) = body {
            for (key, value) in body {
                request.insert(key, value);
            }
        }

        let response = self.account.qh_send_request(url, Some(request)).await?;

        let response_data: ResponseMeta = plist::from_value(&Value::Dictionary(response.clone()))?;
        if response_data.result_code.as_signed().unwrap_or(0) != 0 {
            let msg = response_data.result_string.as_deref().unwrap_or("Unknown");
            let code = response_data.result_code.as_signed().unwrap_or(0);
            return Err(Error::DeveloperSession(code, msg.to_string()));
        }

        Ok(response)
    }
    
    async fn v1_send_request(&self, url: &str, body: Option<serde_json::Value>, request_type: Option<RequestType>) -> Result<serde_json::Value, Error> {
        let response = self.account.v1_send_request(url, body, request_type).await?;
        
        let response_data: serde_json::Value = serde_json::from_value(response.clone())?;
        if let Some(errors) = response_data.get("errors").and_then(|v| v.as_array()) {
            if let Some(error_obj) = errors.first() {
            let status = error_obj.get("status").and_then(|s| s.as_str()).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
            let detail = error_obj.get("detail").and_then(|d| d.as_str()).unwrap_or("Unknown error").to_string();
            return Err(Error::DeveloperSession(status, detail));
            }
        }
        
        Ok(response_data)
    }
}
