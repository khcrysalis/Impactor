use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{Error, auth::Account, developer::DeveloperSession};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AccountStore {
    selected_account: Option<String>,
    accounts: HashMap<String, GsaAccount>,
    path: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GsaAccount {
    email: String,
    first_name: String,
    adsid: String,
    xcode_gs_token: String,
    status: AccountStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum AccountStatus {
    Valid,
    NeedsReauth,
}

impl GsaAccount {
    pub fn new(
        email: String,
        first_name: String,
        adsid: String,
        xcode_gs_token: String,
        status: AccountStatus,
    ) -> Self {
        GsaAccount {
            email,
            first_name,
            adsid,
            xcode_gs_token,
            status,
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
    pub fn status(&self) -> &AccountStatus {
        &self.status
    }
}

impl AccountStore {
    pub async fn load(path: &Option<PathBuf>) -> Result<Self, Error> {
        if let Some(path) = path {
            let mut settings = if !path.exists() {
                Self::default()
            } else {
                let contents = tokio::fs::read_to_string(path).await?;
                serde_json::from_str(&contents)?
            };
            settings.path = Some(path.clone());
            Ok(settings)
        } else {
            Ok(Self::default())
        }
    }

    pub async fn save(&self) -> Result<(), Error> {
        if let Some(path) = &self.path {
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            tokio::fs::write(path, serde_json::to_string_pretty(self)?).await?;
        }
        Ok(())
    }

    pub fn save_sync(&self) -> Result<(), Error> {
        if let Some(path) = &self.path {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        }
        Ok(())
    }

    pub fn accounts(&self) -> &HashMap<String, GsaAccount> {
        &self.accounts
    }

    pub fn get_account(&self, email: &str) -> Option<&GsaAccount> {
        self.accounts.get(email)
    }

    pub async fn accounts_add(&mut self, account: GsaAccount) -> Result<(), Error> {
        let email = account.email.clone();
        self.accounts.insert(email.clone(), account);
        self.selected_account = Some(email);
        self.save().await
    }

    pub fn accounts_add_sync(&mut self, account: GsaAccount) -> Result<(), Error> {
        let email = account.email.clone();
        self.accounts.insert(email.clone(), account);
        self.selected_account = Some(email);
        self.save_sync()
    }

    pub async fn accounts_remove(&mut self, email: &str) -> Result<(), Error> {
        self.accounts.remove(email);
        if self.selected_account.as_ref() == Some(&email.to_string()) {
            self.selected_account = None;
        }
        self.save().await
    }

    pub fn accounts_remove_sync(&mut self, email: &str) -> Result<(), Error> {
        self.accounts.remove(email);
        if self.selected_account.as_ref() == Some(&email.to_string()) {
            self.selected_account = None;
        }
        self.save_sync()
    }

    pub async fn account_select(&mut self, email: &str) -> Result<(), Error> {
        if self.accounts.contains_key(email) {
            self.selected_account = Some(email.to_string());
            self.save().await
        } else {
            Err(Error::Parse) // we need better errors
        }
    }

    pub fn account_select_sync(&mut self, email: &str) -> Result<(), Error> {
        if self.accounts.contains_key(email) {
            self.selected_account = Some(email.to_string());
            self.save_sync()
        } else {
            Err(Error::Parse) // we need better errors
        }
    }

    pub fn selected_account(&self) -> Option<&GsaAccount> {
        if let Some(email) = &self.selected_account {
            self.accounts.get(email)
        } else {
            None
        }
    }

    pub async fn accounts_add_from_session(
        &mut self,
        email: String,
        account: Account,
    ) -> Result<(), Error> {
        let first_name = account.get_name().0;
        let s = DeveloperSession::using_account(account).await?;
        s.qh_list_teams().await?;
        let adsid = s.adsid().clone();
        let xcode_gs_token = s.xcode_gs_token().clone();

        let account = GsaAccount {
            email,
            first_name,
            adsid,
            xcode_gs_token,
            status: AccountStatus::Valid,
        };

        self.accounts_add(account).await?;

        Ok(())
    }
}

pub async fn account_from_session(email: String, account: Account) -> Result<GsaAccount, Error> {
    let first_name = account.get_name().0;
    let s = DeveloperSession::using_account(account).await?;
    s.qh_list_teams().await?;
    let adsid = s.adsid().clone();
    let xcode_gs_token = s.xcode_gs_token().clone();

    Ok(GsaAccount::new(
        email,
        first_name,
        adsid,
        xcode_gs_token,
        AccountStatus::Valid,
    ))
}
