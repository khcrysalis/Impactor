use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RefreshDevice {
    pub udid: String,          // Device UDID
    pub name: String,          // Device name
    pub account: String,       // Email
    pub apps: Vec<RefreshApp>, // Device apps to refresh
    pub is_mac: bool,          // m1 sideloading
}

// custom entitlements not supported
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RefreshApp {
    pub path: PathBuf,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub bundle_id: Option<String>,
    pub scheduled_refresh: DateTime<Utc>, // the scheduled refresh time will happen a day before expiration
}

// to support autorefreshing of apps we need to store a modified copy of the app first
// MISAGENT:
//   for this, we can just reregister the bundle and collect the provisioning profiles,
//
// MANUAL (CERTIFICATE REVOKED):
//   we have a modified copy of the app already, we can just resign and register the bundle and attempt to install it

// TODO: replace substrate with ellekit
// TODO: maybe some 26.0 macho fixes.
