use std::io::Write;

use clap::{Args, Subcommand};
use anyhow::{Ok, Result};
use dialoguer::Select;

use plume_core::{AnisetteConfiguration, auth::Account, developer::DeveloperSession};
use plume_shared::{AccountCredentials, get_data_path};

#[derive(Debug, Args)]
#[command(arg_required_else_help = true)]
pub struct AccountArgs {
    #[command(subcommand)]
    pub command: AccountCommands,
}

#[derive(Debug, Subcommand)]
#[command(arg_required_else_help = true)]
pub enum AccountCommands {
    /// Login to Apple Developer account
    Login(LoginArgs),
    /// Logout from Apple Developer account
    Logout,
    /// List certificates for a team
    Certificates(CertificatesArgs),
    /// List devices registered to the account
    Devices(DevicesArgs),
    /// Register a new device
    RegisterDevice(RegisterDeviceArgs),
}

#[derive(Debug, Args)]
#[command(arg_required_else_help = true)]
pub struct LoginArgs {
    /// Apple ID email
    #[arg(short = 'u', long = "username", value_name = "EMAIL")]
    pub username: Option<String>,
    /// Password (will prompt if not provided)
    #[arg(short = 'p', long = "password", value_name = "PASSWORD")]
    pub password: Option<String>,
}

#[derive(Debug, Args)]
pub struct CertificatesArgs {
    /// Team ID to list certificates for
    #[arg(short = 't', long = "team", value_name = "TEAM_ID")]
    pub team_id: Option<String>,
    /// Filter by certificate type (development, distribution)
    #[arg(long = "type", value_name = "TYPE")]
    pub cert_type: Option<String>,
}

#[derive(Debug, Args)]
pub struct DevicesArgs {
    /// Team ID to list devices for
    #[arg(short = 't', long = "team", value_name = "TEAM_ID")]
    pub team_id: Option<String>,
    /// Filter by device platform (ios, tvos, watchos)
    #[arg(long = "platform", value_name = "PLATFORM")]
    pub platform: Option<String>,
}

#[derive(Debug, Args)]
pub struct RegisterDeviceArgs {
    /// Team ID to list devices for
    #[arg(short = 't', long = "team", value_name = "TEAM_ID")]
    pub team_id: Option<String>,
    /// Device UDID
    #[arg(short = 'u', long = "udid", value_name = "UDID", required = true)]
    pub udid: String,
    /// Device name
    #[arg(short = 'n', long = "name", value_name = "NAME", required = true)]
    pub name: String,
}

pub async fn execute(args: AccountArgs) -> Result<()> {
    match args.command {
        AccountCommands::Login(login_args) => login(login_args).await,
        AccountCommands::Logout => logout().await,
        AccountCommands::Certificates(cert_args) => certificates(cert_args).await,
        AccountCommands::Devices(device_args) => devices(device_args).await,
        AccountCommands::RegisterDevice(register_args) => register_device(register_args).await,
    }
}

pub async fn get_authenticated_account() -> Result<DeveloperSession> {
    let credentials = AccountCredentials;
    
    let email = credentials.get_email()
        .map_err(|_| anyhow::anyhow!("No stored credentials found. Please login first using 'plumesign account login'"))?;
    
    let password = credentials.get_password()
        .map_err(|_| anyhow::anyhow!("No stored credentials found. Please login first using 'plumesign account login'"))?;
    
    let tfa_closure = || -> std::result::Result<String, String> {
        println!("Enter 2FA code: ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).map_err(|e| e.to_string())?;
        std::result::Result::Ok(input.trim().to_string())
    };

    let anisette_config = AnisetteConfiguration::default()
        .set_configuration_path(get_data_path());
    
    let login_closure = || -> std::result::Result<(String, String), String> {
        std::result::Result::Ok((email.clone(), password.clone()))
    };

    log::info!("Authenticating with Apple...");
    let account = Account::login(login_closure, tfa_closure, anisette_config).await
        .map_err(|e| anyhow::anyhow!("Authentication failed: {}. Please login again.", e))?;
    
    Ok(DeveloperSession::with(account))
}

async fn login(args: LoginArgs) -> Result<()> {
    let tfa_closure = || -> std::result::Result<String, String> {
        log::info!("Enter 2FA code: ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).map_err(|e| e.to_string())?;
        std::result::Result::Ok(input.trim().to_string())
    };

    let anisette_config = AnisetteConfiguration::default()
        .set_configuration_path(get_data_path());
    
    let username = if let Some(user) = args.username {
        user
    } else {
        log::info!("Enter Apple ID email: ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        input.trim().to_string()
    };
    
    let password = if let Some(pass) = args.password {
        pass
    } else {
        print!("Enter password: ");
        std::io::stdout().flush()?;
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        input.trim().to_string()
    };
        
    let login_closure = || -> std::result::Result<(String, String), String> {
        std::result::Result::Ok((username.clone(), password.clone()))
    };

    println!("Logging in...");
    Account::login(login_closure, tfa_closure, anisette_config).await?;

    AccountCredentials.set_credentials(
        username,
        password,
    )?;

    log::info!("Successfully logged in and credentials saved to keychain.");

    Ok(())
}

async fn logout() -> Result<()> {
    AccountCredentials.delete_password()?;
    
    log::info!("Successfully logged out and removed credentials from keychain.");
    
    Ok(())
}

async fn certificates(args: CertificatesArgs) -> Result<()> {
    let session = get_authenticated_account().await?;

    let team_id = if args.team_id.is_none() {
        teams(&session).await?
    } else {
        args.team_id.unwrap()
    };

    let p = session.qh_list_certs(&team_id)
        .await?
        .certificates;
    
    log::info!("{:#?}", p);
    
    Ok(())
}

async fn devices(args: DevicesArgs) -> Result<()> {
    let session = get_authenticated_account().await?;

    let team_id = if args.team_id.is_none() {
        teams(&session).await?
    } else {
        args.team_id.unwrap()
    };

    let p = session.qh_list_devices(&team_id)
        .await?
        .devices;

    log::info!("{:#?}", p);

    Ok(())
}

async fn register_device(args: RegisterDeviceArgs) -> Result<()> {
    let session = get_authenticated_account().await?;

    let team_id = if args.team_id.is_none() {
        teams(&session).await?
    } else {
        args.team_id.unwrap()
    };

    let p = session.qh_add_device(&team_id, &args.name, &args.udid)
        .await?
        .device;

    log::info!("{:#?}", p);

    Ok(())
}

pub async fn teams(session: &DeveloperSession) -> Result<String> {
    let teams = session.qh_list_teams().await?.teams;

    if teams.len() == 1 {
        return Ok(teams[0].team_id.clone());
    }

    let team_names: Vec<String> = teams.iter()
        .map(|t| format!("{} ({})", t.name, t.team_id))
        .collect();
    
    let selection = Select::new()
        .items(&team_names)
        .default(0)
        .interact()?;

    Ok(teams[selection].team_id.clone())
}
