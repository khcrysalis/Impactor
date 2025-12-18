use std::path::PathBuf;
use std::collections::HashMap;

use omnisette::AnisetteConfiguration;
use serde::{Deserialize, Serialize};

use crate::{Error, auth::{Account, anisette_data::AnisetteData}, plist_get_string};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AccountStore {
    selected_account: Option<String>,
    accounts: HashMap<String, GsaAccount>,
    path: Option<PathBuf>,
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

            let json = serde_json::to_string_pretty(self)?;
            let tmp = path.with_extension("tmp");

            tokio::fs::write(&tmp, json).await?;
            tokio::fs::rename(tmp, path).await?;
        }
        
        Ok(())
    }

    pub fn accounts(&self) -> &HashMap<String, GsaAccount> {
        &self.accounts
    }

    // Synchronous methods for use in non-async contexts
    pub fn accounts_add_sync(&mut self, account: GsaAccount) -> Result<(), Error> {
        let email = account.email.clone();
        self.accounts.insert(email.clone(), account);
        self.selected_account = Some(email);
        self.save_sync()
    }

    pub fn accounts_remove_sync(&mut self, email: &str) -> Result<(), Error> {
        self.accounts.remove(email);
        if self.selected_account.as_ref() == Some(&email.to_string()) {
            self.selected_account = None;
        }
        self.save_sync()
    }

    pub fn account_select_sync(&mut self, email: &str) -> Result<(), Error> {
        if self.accounts.contains_key(email) {
            self.selected_account = Some(email.to_string());
            self.save_sync()
        } else {
            Err(Error::Parse)
        }
    }

    fn save_sync(&self) -> Result<(), Error> {
        if let Some(path) = &self.path {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let json = serde_json::to_string_pretty(self)?;
            std::fs::write(path, json)?;
        }
        Ok(())
    }

    // Async methods
    pub async fn accounts_add(&mut self, account: GsaAccount) -> Result<(), Error>{
        let email = account.email.clone();
        self.accounts.insert(email.clone(), account);
        self.selected_account = Some(email);
        self.save().await
    }

    pub async fn accounts_remove(&mut self, email: &str) -> Result<(), Error> {
        self.accounts.remove(email);
        if self.selected_account.as_ref() == Some(&email.to_string()) {
            self.selected_account = None;
        }
        self.save().await
    }

    pub async fn account_select(&mut self, email: &str) -> Result<(), Error> {
        if self.accounts.contains_key(email) {
            self.selected_account = Some(email.to_string());
            self.save().await
        } else {
            Err(Error::Parse)
        }
    }

    pub fn selected_account(&self) -> Option<&GsaAccount> {
        if let Some(email) = &self.selected_account {
            self.accounts.get(email)
        } else {
            None
        }
    }
    
    pub fn selected_account_mut(&mut self) -> Option<&mut GsaAccount> {
        if let Some(email) = &self.selected_account {
            self.accounts.get_mut(email)
        } else {
            None
        }
    }
    
    pub async fn update_account_status(&mut self, email: &str, status: AccountStatus) -> Result<(), Error> {
        if let Some(account) = self.accounts.get_mut(email) {
            account.status = status;
            self.save().await
        } else {
            Err(Error::Parse)
        }
    }
    
    pub fn get_account(&self, email: &str) -> Option<&GsaAccount> {
        self.accounts.get(email)
    }
    
    pub fn get_account_mut(&mut self, email: &str) -> Option<&mut GsaAccount> {
        self.accounts.get_mut(email)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GsaAccount {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub anisette_provider: GsaAnisetteProvider,
    pub adsid: String,
    pub gs_idms_token: String,
    pub session_key: Vec<u8>,
    pub c: Vec<u8>,
    pub status: AccountStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum AccountStatus {
    #[serde(rename = "valid")]
    Valid,
    #[serde(rename = "invalid")]
    Invalid,
    #[serde(rename = "needs_reauth")]
    NeedsReauth,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GsaAnisetteProvider {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "remote")]
    Remote,
}

impl Account {
    /// Convert Account to storable GsaAccount format
    pub fn to_gsa_account(&self, anisette_provider: GsaAnisetteProvider) -> Result<GsaAccount, Error> {
        let spd = self.spd.as_ref().ok_or(Error::Parse)?;
        
        let adsid = spd.get("adsid")
            .and_then(|v| v.as_string())
            .ok_or(Error::Parse)?
            .to_string();
        
        let gs_idms_token = spd.get("GsIdmsToken")
            .and_then(|v| v.as_string())
            .ok_or(Error::Parse)?
            .to_string();
        
        let session_key = spd.get("sk")
            .and_then(|v| v.as_data())
            .ok_or(Error::Parse)?
            .to_vec();
        
        let c = spd.get("c")
            .and_then(|v| v.as_data())
            .ok_or(Error::Parse)?
            .to_vec();
        
        let first_name = plist_get_string!(spd, "fn");
        let last_name = plist_get_string!(spd, "ln");
        
        let email = spd.get("appleId")
            .and_then(|v| v.as_string())
            .or_else(|| {
                spd.get("delegates")
                    .and_then(|v| v.as_dictionary())
                    .and_then(|delegates| {
                        delegates.get("com.apple.gs")
                            .and_then(|v| v.as_dictionary())
                            .and_then(|gs| gs.get("email").and_then(|v| v.as_string()))
                    })
            })
            .or_else(|| {
                spd.get("accountInfo")
                    .and_then(|v| v.as_dictionary())
                    .and_then(|info| info.get("appleId").and_then(|v| v.as_string()))
            })
            .ok_or_else(|| {
                log::error!("Failed to extract email from SPD. Available keys: {:?}", spd.keys().collect::<Vec<_>>());
                Error::Parse
            })?
            .to_string();
        
        Ok(GsaAccount {
            email,
            first_name,
            last_name,
            anisette_provider,
            adsid,
            gs_idms_token,
            session_key,
            c,
            status: AccountStatus::Valid,
        })
    }
    
    /// Restore Account from stored GsaAccount
    pub async fn from_gsa_account(gsa_account: &GsaAccount, config: AnisetteConfiguration) -> Result<Self, Error> {
        let anisette = AnisetteData::new(config).await?;
        let mut account = Self::new_with_anisette(anisette)?;
        
        let mut spd = plist::Dictionary::new();
        spd.insert("adsid".to_string(), plist::Value::String(gsa_account.adsid.clone()));
        spd.insert("GsIdmsToken".to_string(), plist::Value::String(gsa_account.gs_idms_token.clone()));
        spd.insert("sk".to_string(), plist::Value::Data(gsa_account.session_key.clone()));
        spd.insert("c".to_string(), plist::Value::Data(gsa_account.c.clone()));
        spd.insert("fn".to_string(), plist::Value::String(gsa_account.first_name.clone()));
        spd.insert("ln".to_string(), plist::Value::String(gsa_account.last_name.clone()));
        spd.insert("appleId".to_string(), plist::Value::String(gsa_account.email.clone()));
        
        let mut com_apple_gs = plist::Dictionary::new();
        com_apple_gs.insert("email".to_string(), plist::Value::String(gsa_account.email.clone()));
        
        let mut delegates = plist::Dictionary::new();
        delegates.insert("com.apple.gs".to_string(), plist::Value::Dictionary(com_apple_gs));
        spd.insert("delegates".to_string(), plist::Value::Dictionary(delegates));
        
        account.spd = Some(spd);
        Ok(account)
    }
    
    pub async fn validate_and_get_info(&self, team_id: &str) -> Result<(String, String), Error> {
        use crate::developer::DeveloperSession;
        
        let dev_session = DeveloperSession::with(self.clone());
        let account_info = dev_session.qh_get_account_info(team_id).await?;
        
        Ok((account_info.developer.first_name, account_info.developer.last_name))
    }
}
