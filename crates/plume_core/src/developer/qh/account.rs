use plist::{Dictionary, Value};
use serde::Deserialize;

use crate::Error;

use super::{DeveloperSession, QHResponseMeta};
use crate::developer_endpoint;

impl DeveloperSession {
    pub async fn qh_get_account_info(
        &self,
        team_id: &String,
    ) -> Result<ViewDeveloperResponse, Error> {
        let endpoint = developer_endpoint!("/QH65B2/viewDeveloper.action");

        let mut body = Dictionary::new();
        body.insert("teamId".to_string(), Value::String(team_id.clone()));

        let response = self.qh_send_request(&endpoint, Some(body)).await?;
        let response_data: ViewDeveloperResponse = plist::from_value(&Value::Dictionary(response))?;

        Ok(response_data)
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ViewDeveloperResponse {
    // pub teams: Vec<Team>,
    pub developer: Developer,
    #[serde(flatten)]
    pub meta: QHResponseMeta,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]

pub struct Developer {
    // pub developer_id: String,
    // pub person_id: String,
    pub first_name: String,
    pub last_name: String,
    pub ds_first_name: String,
    pub ds_last_name: String,
    pub email: String,
    pub developer_status: String,
}
