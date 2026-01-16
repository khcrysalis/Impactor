use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use chrono::Utc;
use plume_core::{
    AnisetteConfiguration, CertificateIdentity, MobileProvision, developer::DeveloperSession,
};
use plume_store::{AccountStore, RefreshDevice};
use plume_utils::{Bundle, Device, Signer, SignerMode, SignerOptions};

use crate::defaults::get_data_path;

pub type ConnectedDevices = Arc<Mutex<HashMap<String, Device>>>;

pub struct RefreshDaemon {
    store_path: std::path::PathBuf,
    connected_devices: ConnectedDevices,
    check_interval: Duration,
}

impl RefreshDaemon {
    pub fn new() -> Self {
        Self {
            store_path: get_data_path().join("accounts.json"),
            connected_devices: Arc::new(Mutex::new(HashMap::new())),
            check_interval: Duration::from_secs(60 * 30), // Check every 30 minutes
        }
    }

    pub fn connected_devices(&self) -> ConnectedDevices {
        self.connected_devices.clone()
    }

    pub fn spawn(self) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            loop {
                if let Err(e) = rt.block_on(self.check_and_refresh()) {
                    eprintln!("Refresh daemon error: {}", e);
                }

                thread::sleep(self.check_interval);
            }
        })
    }

    async fn check_and_refresh(&self) -> Result<(), String> {
        let store = AccountStore::load(&Some(self.store_path.clone()))
            .await
            .map_err(|e| format!("Failed to load account store: {}", e))?;

        let now = Utc::now();

        for (udid, refresh_device) in store.refreshes() {
            for app in &refresh_device.apps {
                if app.scheduled_refresh <= now {
                    println!("App at {:?} needs refresh for device {}", app.path, udid);

                    let device = self.wait_for_device(udid).await?;

                    self.refresh_app(&store, refresh_device, app, &device)
                        .await?;
                }
            }
        }

        Ok(())
    }

    async fn wait_for_device(&self, udid: &str) -> Result<Device, String> {
        println!("Waiting for device {} to connect...", udid);

        if let Ok(devices) = self.connected_devices.lock() {
            if let Some(device) = devices.get(udid) {
                println!("Device {} is already connected", udid);
                return Ok(device.clone());
            }
        }

        let timeout = Duration::from_secs(60 * 60); // 1 hour timeout
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                return Err(format!("Timeout waiting for device {} to connect", udid));
            }

            if let Ok(devices) = self.connected_devices.lock() {
                if let Some(device) = devices.get(udid) {
                    println!("Device {} connected", udid);
                    return Ok(device.clone());
                }
            }

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    pub async fn refresh_app(
        &self,
        store: &AccountStore,
        refresh_device: &RefreshDevice,
        app: &plume_store::RefreshApp,
        device: &Device,
    ) -> Result<(), String> {
        println!("Starting refresh for app at {:?}", app.path);

        let account = store
            .get_account(&refresh_device.account)
            .ok_or_else(|| format!("Account {} not found", refresh_device.account))?;

        let session = DeveloperSession::new(
            account.adsid().clone(),
            account.xcode_gs_token().clone(),
            AnisetteConfiguration::default().set_configuration_path(get_data_path()),
        )
        .await
        .map_err(|e| format!("Failed to create session: {}", e))?;

        let teams_response = session
            .qh_list_teams()
            .await
            .map_err(|e| format!("Failed to list teams: {}", e))?;

        if teams_response.teams.is_empty() {
            return Err("No teams available for this account".to_string());
        }

        let team_id = if account.team_id().is_empty() {
            &teams_response.teams[0].team_id
        } else {
            account.team_id()
        };

        let identity_is_new = {
            let identity =
                CertificateIdentity::new_with_session(&session, get_data_path(), None, team_id)
                    .await
                    .map_err(|e| format!("Failed to create identity: {}", e))?;
            identity.new
        };

        let is_installed = if let Some(bundle_id) = app.bundle_id.as_deref() {
            device
                .is_app_installed(bundle_id)
                .await
                .map_err(|e| format!("Failed to check if app is installed: {}", e))?
        } else {
            false
        };

        let needs_reinstall = device.is_mac || identity_is_new || !is_installed;

        if needs_reinstall {
            println!("Resigning and reinstalling app...");
            self.resign_and_reinstall(app, device, &session, team_id)
                .await?;
        } else {
            println!("Certificate exists and app is installed, updating provisioning profiles...");
            self.update_provisioning_profiles(app, device, &session, team_id)
                .await?;
        }

        self.update_refresh_schedule(store, refresh_device, app)
            .await?;

        println!("Successfully refreshed app at {:?}", app.path);

        Ok(())
    }

    async fn resign_and_reinstall(
        &self,
        app: &plume_store::RefreshApp,
        device: &Device,
        session: &DeveloperSession,
        team_id: &str,
    ) -> Result<(), String> {
        let team_id_string = team_id.to_string();
        session
            .qh_ensure_device(&team_id_string, &device.name, &device.udid)
            .await
            .map_err(|e| format!("Failed to ensure device: {}", e))?;

        let bundle =
            Bundle::new(app.path.clone()).map_err(|e| format!("Failed to create bundle: {}", e))?;

        let options = SignerOptions {
            mode: SignerMode::Pem,
            ..Default::default()
        };

        let team_id_string = team_id.to_string();
        let signing_identity =
            CertificateIdentity::new_with_session(session, get_data_path(), None, &team_id_string)
                .await
                .map_err(|e| format!("Failed to create signing identity: {}", e))?;

        let mut signer = Signer::new(Some(signing_identity), options);

        signer
            .register_bundle(&bundle, session, &team_id.to_string(), true)
            .await
            .map_err(|e| format!("Failed to register bundle: {}", e))?;

        signer
            .sign_bundle(&bundle)
            .await
            .map_err(|e| format!("Failed to sign bundle: {}", e))?;

        if !device.is_mac {
            device
                .install_app(&app.path, |_| async {})
                .await
                .map_err(|e| format!("Failed to install app: {}", e))?;
        } else {
            plume_utils::install_app_mac(&app.path)
                .await
                .map_err(|e| format!("Failed to install app on Mac: {}", e))?;
        }

        Ok(())
    }

    async fn update_provisioning_profiles(
        &self,
        app: &plume_store::RefreshApp,
        device: &Device,
        session: &DeveloperSession,
        team_id: &str,
    ) -> Result<(), String> {
        let bundle =
            Bundle::new(app.path.clone()).map_err(|e| format!("Failed to create bundle: {}", e))?;

        let options = SignerOptions {
            mode: SignerMode::Pem,
            ..Default::default()
        };

        let mut signer = Signer::new(None, options);

        signer
            .register_bundle(&bundle, session, &team_id.to_string(), true)
            .await
            .map_err(|e| format!("Failed to register bundle: {}", e))?;

        for provision in &signer.provisioning_files {
            device
                .install_profile(provision)
                .await
                .map_err(|e| format!("Failed to install profile: {}", e))?;
        }

        Ok(())
    }

    async fn update_refresh_schedule(
        &self,
        store: &AccountStore,
        refresh_device: &RefreshDevice,
        app: &plume_store::RefreshApp,
    ) -> Result<(), String> {
        let embedded_prov_path = app.path.join("embedded.mobileprovision");
        if !embedded_prov_path.exists() {
            return Err("embedded.mobileprovision not found".to_string());
        }

        let provision = MobileProvision::load_with_path(&embedded_prov_path)
            .map_err(|e| format!("Failed to load mobile provision: {}", e))?;

        let expiration_date = provision.expiration_date().clone();
        let scheduled_refresh = expiration_date
            .to_xml_format()
            .parse::<chrono::DateTime<chrono::Utc>>()
            .unwrap_or_else(|_| Utc::now() + chrono::Duration::days(6));
        let scheduled_refresh = scheduled_refresh - chrono::Duration::days(1);

        let mut store = store.clone();
        let mut updated_device = refresh_device.clone();

        if let Some(existing_app) = updated_device.apps.iter_mut().find(|a| a.path == app.path) {
            existing_app.scheduled_refresh = scheduled_refresh;
        }

        store
            .add_or_update_refresh_device_sync(updated_device)
            .map_err(|e| format!("Failed to update refresh schedule: {}", e))?;

        println!("Next refresh scheduled for: {}", scheduled_refresh);

        Ok(())
    }
}

pub fn spawn_refresh_daemon() -> (thread::JoinHandle<()>, ConnectedDevices) {
    let daemon = RefreshDaemon::new();
    let devices = daemon.connected_devices();
    let handle = daemon.spawn();
    (handle, devices)
}
