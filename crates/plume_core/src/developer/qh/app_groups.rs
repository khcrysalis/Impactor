use plist::{Dictionary, Value};
use serde::Deserialize;

use crate::Error;

use super::{DeveloperSession, QHResponseMeta};
use crate::developer::strip_invalid_chars;
use crate::developer_endpoint;

impl DeveloperSession {
    pub async fn qh_list_app_groups(&self, team_id: &String) -> Result<AppGroupsResponse, Error> {
        let endpoint = developer_endpoint!("/QH65B2/ios/listApplicationGroups.action");

        let mut body = Dictionary::new();
        body.insert("teamId".to_string(), Value::String(team_id.clone()));

        let response = self.qh_send_request(&endpoint, Some(body)).await?;
        let response_data: AppGroupsResponse = plist::from_value(&Value::Dictionary(response))?;

        Ok(response_data)
    }

    pub async fn qh_add_app_group(
        &self,
        team_id: &String,
        name: &String,
        identifier: &String,
    ) -> Result<AppGroupResponse, Error> {
        let endpoint = developer_endpoint!("/QH65B2/ios/addApplicationGroup.action");

        let mut body = Dictionary::new();
        body.insert("teamId".to_string(), Value::String(team_id.clone()));
        body.insert("name".to_string(), Value::String(strip_invalid_chars(name)));
        body.insert("identifier".to_string(), Value::String(identifier.clone()));

        let response = self.qh_send_request(&endpoint, Some(body)).await?;
        let response_data: AppGroupResponse = plist::from_value(&Value::Dictionary(response))?;

        Ok(response_data)
    }

    pub async fn qh_get_app_group(
        &self,
        team_id: &String,
        app_group_identifier: &String,
    ) -> Result<Option<ApplicationGroup>, Error> {
        let response_data = self.qh_list_app_groups(team_id).await?;

        let app_group = response_data
            .application_group_list
            .into_iter()
            .find(|group| group.identifier == *app_group_identifier);

        Ok(app_group)
    }

    pub async fn qh_ensure_app_group(
        &self,
        team_id: &String,
        name: &String,
        identifier: &String,
    ) -> Result<ApplicationGroup, Error> {
        if let Some(app_group) = self.qh_get_app_group(team_id, identifier).await? {
            Ok(app_group)
        } else {
            let response = self.qh_add_app_group(team_id, name, identifier).await?;
            Ok(response.application_group)
        }
    }

    pub async fn qh_assign_app_group(
        &self,
        team_id: &String,
        app_id_id: &String,
        app_group_ids: &Vec<String>,
    ) -> Result<QHResponseMeta, Error> {
        let endpoint = developer_endpoint!("/QH65B2/ios/assignApplicationGroupToAppId.action");

        let mut body = Dictionary::new();
        body.insert("teamId".to_string(), Value::String(team_id.clone()));
        body.insert("appIdId".to_string(), Value::String(app_id_id.clone()));
        body.insert(
            "applicationGroups".to_string(),
            Value::Array(
                app_group_ids
                    .iter()
                    .map(|s| Value::String(s.clone()))
                    .collect(),
            ),
        );

        let response = self.qh_send_request(&endpoint, Some(body)).await?;
        let response_data: QHResponseMeta = plist::from_value(&Value::Dictionary(response))?;

        Ok(response_data)
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AppGroupsResponse {
    pub application_group_list: Vec<ApplicationGroup>,
    #[serde(flatten)]
    pub meta: QHResponseMeta,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AppGroupResponse {
    pub application_group: ApplicationGroup,
    #[serde(flatten)]
    pub meta: QHResponseMeta,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationGroup {
    pub application_group: String, // this is the actual identifier
    pub name: String,
    pub status: String,
    prefix: String,
    pub identifier: String, // this is the group.identifier
}
