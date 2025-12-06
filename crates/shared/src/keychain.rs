use keyring::{Entry, Error};

const KEYRING_SERVICE: &str = env!("CARGO_PKG_NAME");
const KEYRING_EMAIL: &str = "Apple ID Email";
const KEYRING_PASS: &str = "Apple ID Password";

pub struct AccountCredentials;

impl AccountCredentials {
    pub fn set_credentials(&self, email: String, password: String) -> Result<(), Error> {
        let entry_email = Entry::new(KEYRING_SERVICE, KEYRING_EMAIL)?;
        let entry_pass  = Entry::new(KEYRING_SERVICE, KEYRING_PASS)?;
        entry_email.set_password(&email)?;
        entry_pass.set_password(&password)?;
        Ok(())
    }

    pub fn get_email(&self) -> Result<String, Error> {
        let entry = Entry::new(KEYRING_SERVICE, KEYRING_EMAIL)?;
        entry.get_password()
    }

    pub fn get_password(&self) -> Result<String, Error> {
        let entry = Entry::new(KEYRING_SERVICE, KEYRING_PASS)?;
        entry.get_password()
    }

    pub fn delete_password(&self) -> Result<(), Error> {
        let entry_email = Entry::new(KEYRING_SERVICE, KEYRING_EMAIL)?;
        let entry_pass = Entry::new(KEYRING_SERVICE, KEYRING_PASS)?;

        entry_email.delete_credential()?;
        entry_pass.delete_credential()?;

        Ok(())
    }
}
