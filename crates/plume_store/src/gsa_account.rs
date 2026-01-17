use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GsaAccount {
    email: String,
    first_name: String,
    adsid: String,
    xcode_gs_token: String,
    #[serde(default)]
    team_id: String,
}

impl GsaAccount {
    pub fn new(
        email: String,
        first_name: String,
        adsid: String,
        xcode_gs_token: String,
        team_id: String,
    ) -> Self {
        GsaAccount {
            email,
            first_name,
            adsid,
            xcode_gs_token,
            team_id,
        }
    }
    pub fn email(&self) -> &String {
        &self.email
    }
    pub fn first_name(&self) -> &String {
        &self.first_name
    }
    pub fn adsid(&self) -> &String {
        &self.adsid
    }
    pub fn xcode_gs_token(&self) -> &String {
        &self.xcode_gs_token
    }
    pub fn team_id(&self) -> &String {
        &self.team_id
    }
    pub fn set_team_id(&mut self, team_id: String) {
        self.team_id = team_id;
    }
}

pub async fn account_from_session(
    email: String,
    account: plume_core::auth::Account,
) -> Result<GsaAccount, plume_core::Error> {
    let first_name = account.get_name().0;
    let s = plume_core::developer::DeveloperSession::using_account(account).await?;
    let teams_response = s.qh_list_teams().await?;
    let adsid = s.adsid().clone();
    let xcode_gs_token = s.xcode_gs_token().clone();

    let team_id = if teams_response.teams.is_empty() {
        "".to_string()
    } else {
        teams_response.teams[0].team_id.clone()
    };

    Ok(GsaAccount::new(
        email,
        first_name,
        adsid,
        xcode_gs_token,
        team_id,
    ))
}
