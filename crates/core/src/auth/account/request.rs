use plist::Dictionary;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;

use crate::Error;

use crate::{SessionRequestTrait, auth::Account};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestType {
    Get,
    Post,
    Patch,
}

impl SessionRequestTrait for Account {
    async fn qh_send_request(
        &self,
        url: &str,
        body: Option<Dictionary>,
    ) -> Result<Dictionary, Error> {
        let spd = self.spd.as_ref().unwrap();
        let app_token = self.get_app_token("com.apple.gs.xcode.auth").await?;
        let valid_anisette = self.get_anisette().await;

        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("text/x-xml-plist"));
        headers.insert("Accept", HeaderValue::from_static("text/x-xml-plist"));
        headers.insert("Accept-Language", HeaderValue::from_static("en-us"));
        headers.insert("User-Agent", HeaderValue::from_static("Xcode"));
        headers.insert(
            "X-Apple-I-Identity-Id",
            HeaderValue::from_str(spd.get("adsid").unwrap().as_string().unwrap()).unwrap(),
        );
        headers.insert(
            "X-Apple-GS-Token",
            HeaderValue::from_str(&app_token.auth_token).unwrap(),
        );

        for (k, v) in valid_anisette.generate_headers(false, true, true) {
            headers.insert(
                HeaderName::from_bytes(k.as_bytes()).unwrap(),
                HeaderValue::from_str(&v).unwrap(),
            );
        }

        if let Ok(locale) = valid_anisette.get_header("x-apple-locale") {
            headers.insert("X-Apple-Locale", HeaderValue::from_str(&locale).unwrap());
        }

        log::debug!("Sending QH request to {}", url);

        let response = if let Some(body) = body {
            let mut buf = Vec::new();
            plist::to_writer_xml(&mut buf, &body)?;
            self.client
                .post(url)
                .headers(headers)
                .body(buf)
                .send()
                .await?
        } else {
            self.client.get(url).headers(headers).send().await?
        };

        let response = response.text().await?;
        let response_data: Dictionary = plist::from_bytes(response.as_bytes())?;

        log::debug!("Received QH response bytes from request: {:?}", response.as_bytes().len());

        Ok(response_data)
    }
    
    async fn v1_send_request(
        &self,
        url: &str,
        body: Option<Value>,
        request_type: Option<RequestType>,
    ) -> Result<Value, Error> {
        let spd = self.spd.as_ref().unwrap();
        let app_token = self.get_app_token("com.apple.gs.xcode.auth").await?;
        let valid_anisette = self.get_anisette().await;

        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/vnd.api+json"));
        headers.insert("Accept", HeaderValue::from_static("application/json, text/plain, */*"));
        headers.insert("Accept-Language", HeaderValue::from_static("en-us"));
        headers.insert("User-Agent", HeaderValue::from_static("Xcode"));
        headers.insert("X-Requested-With", HeaderValue::from_static("XMLHttpRequest"));
        
        headers.insert(
            "X-Apple-I-Identity-Id",
            HeaderValue::from_str(spd.get("adsid").unwrap().as_string().unwrap()).unwrap(),
        );
        headers.insert(
            "X-Apple-GS-Token",
            HeaderValue::from_str(&app_token.auth_token).unwrap()
        );

        for (k, v) in valid_anisette.generate_headers(false, true, true) {
            headers.insert(
                HeaderName::from_bytes(k.as_bytes()).unwrap(),
                HeaderValue::from_str(&v).unwrap()
            );
        }

        if let Ok(locale) = valid_anisette.get_header("x-apple-locale") {
            headers.insert("X-Apple-Locale", HeaderValue::from_str(&locale).unwrap());
        }
        
        if let Some(RequestType::Get) = request_type {
            headers.insert(
                "X-HTTP-Method-Override",
                HeaderValue::from_static("GET"),
            );
        }

        log::debug!("Sending V1 request to {}", url);

        let response = match (request_type, body) {
            (Some(RequestType::Post), Some(body)) => {
                self.client
                    .post(url)
                    .headers(headers)
                    .json(&body)
                    .send()
                    .await?
            }
            (Some(RequestType::Patch), Some(body)) => {
                self.client
                    .patch(url)
                    .headers(headers)
                    .json(&body)
                    .send()
                    .await?
            }
            (_, Some(body)) => {
                self.client
                    .post(url)
                    .headers(headers)
                    .json(&body)
                    .send()
                    .await?
            }
            _ => {
                self.client.get(url).headers(headers).send().await?
            }
        };

        let response = response.text().await?;
        let response_data: Value = serde_json::from_str(&response)?;
        
        log::debug!("Received V1 response bytes from request: {:?}", response.as_bytes().len());

        Ok(response_data)
    }
}
