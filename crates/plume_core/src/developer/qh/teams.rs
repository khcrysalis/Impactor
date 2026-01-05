use plist::{Date, Integer, Value};
use serde::Deserialize;

use crate::Error;

use super::{DeveloperSession, QHResponseMeta};
use crate::developer_endpoint;

impl DeveloperSession {
    pub async fn qh_list_teams(&self) -> Result<TeamsResponse, Error> {
        let endpoint = developer_endpoint!("/QH65B2/listTeams.action");

        let response = self.qh_send_request(&endpoint, None).await?;
        let response_data: TeamsResponse = plist::from_value(&Value::Dictionary(response))?;

        Ok(response_data)
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TeamsResponse {
    pub teams: Vec<Team>,
    #[serde(flatten)]
    pub meta: QHResponseMeta,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    pub status: String,
    pub name: String,
    pub team_id: String,
    #[serde(rename = "type")]
    pub _type: String,
    team_agent: Option<TeamMember>,
    memberships: Vec<Membership>,
    current_team_member: TeamMember,
    date_created: Option<Date>,
    xcode_free_only: bool,
    team_provisioning_settings: TeamProvisionSettings,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct Membership {
    membership_id: String,
    membership_product_id: String,
    status: String,
    in_ios_reset_window: Option<bool>,
    in_renewal_window: bool,
    date_start: Option<Date>,
    platform: String,
    delete_devices_on_expiry: bool,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct TeamMember {
    team_member_id: String,
    person_id: Integer,
    first_name: String,
    last_name: String,
    email: String,
    developer_status: Option<String>,
    // privileges: ...
    roles: Option<Vec<String>>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct TeamProvisionSettings {
    can_developer_role_register_devices: bool,
    can_developer_role_add_app_ids: bool,
    can_developer_role_update_app_ids: bool,
}
