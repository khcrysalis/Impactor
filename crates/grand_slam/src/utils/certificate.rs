use std::{fs, path::PathBuf, vec};

use apple_codesign::{cryptography::{InMemoryPrivateKey, PrivateKey}, SigningSettings};
// TODO: why do we have pem and pem_rfc7468 deps again?
use pem_rfc7468::{LineEnding, encode_string};
use rand::rngs::OsRng;
use rcgen::{DnType, KeyPair, PKCS_RSA_SHA256};
use rsa::{RsaPrivateKey, pkcs1::EncodeRsaPublicKey, pkcs8::{DecodePrivateKey, EncodePrivateKey}};
use x509_certificate::{CapturedX509Certificate, X509Certificate};

use crate::{Error, developer::{DeveloperSession, qh::certs::Cert}};

pub struct CertificateIdentity {
    pub cert: Option<CapturedX509Certificate>,
    pub key: Option<Box<dyn PrivateKey>>,
}

impl CertificateIdentity {
    // Use for cli context or if you actually store pems? why would you do that though
    pub async fn new_with_paths(paths: Option<Vec<PathBuf>>) -> Result<Self, Error> {
        let mut cert = Self { 
            cert: None,
            key: None
        };

        if let Some(paths) = paths {
            for path in &paths {
                let pem_data = fs::read(path)?;
                cert.resolve_certificate_from_contents(pem_data)?;
            }
        }

        Ok(cert)
    }

    pub async fn new_with_session(
        session: &DeveloperSession,
        config_path: PathBuf,
        machine_name: Option<String>,
        team_id: &String,
    ) -> Result<Self, Error> {
        let machine_name = machine_name.unwrap_or_else(|| "AltStore".to_string());

        let key_path = Self::key_dir(config_path, &team_id)?.join("key.pem");

        // To same some unnecessary requests, we're going to list our certificates first here
        // then pass them into the necessary functions that need it, if the functions absolutely
        // need to request certificates (after submitting a CSR, for example), they can do so
        let certs = session
            .qh_list_certs(&team_id)
            .await?
            .certificates;

        // Only the key will be written to disk, certificate can just be gotten via the request
        // request we've made, by trying to match our public key with the requests public key
        let key_pair: [Vec<u8>; 2] = if key_path.exists() {
            let key_string = fs::read_to_string(&key_path)?;
            let priv_key = RsaPrivateKey::from_pkcs8_pem(&key_string)?;

            if let Some(cert) = Self::find_certificate(certs.clone(), &priv_key, &machine_name).await? {
                let cert_pem = encode_string("CERTIFICATE", LineEnding::LF, cert.cert_content.as_ref()).unwrap();
                let key_pem = priv_key.to_pkcs8_pem(Default::default())?.to_string();

                [cert_pem.into_bytes(), key_pem.into_bytes()]
            } else {
                let (cert, priv_key) = Self::request_new_certificate(session, team_id, &machine_name, certs).await?;
                let cert_pem = encode_string("CERTIFICATE", LineEnding::LF, cert.cert_content.as_ref()).unwrap();
                let key_pem = priv_key.to_pkcs8_pem(Default::default())?.to_string();

                fs::write(&key_path, &key_pem)?;
                [cert_pem.into_bytes(), key_pem.into_bytes()]
            }
        } else {
            let (cert, priv_key) = Self::request_new_certificate(session, team_id, &machine_name, certs).await?;
            let cert_pem = encode_string("CERTIFICATE", LineEnding::LF, cert.cert_content.as_ref()).unwrap();
            let key_pem = priv_key.to_pkcs8_pem(Default::default())?.to_string();

            fs::write(&key_path, &key_pem)?;
            [cert_pem.into_bytes(), key_pem.into_bytes()]
        };

        let mut cert = Self { 
            cert: None, 
            key: None 
        };

        for pem in key_pair {
            cert.resolve_certificate_from_contents(pem)?;
        }

        Ok(cert)
    }

    // <config_path>/keys/<team_id>
    fn key_dir(path: PathBuf, team_id: &String) -> Result<PathBuf, Error> {
        let dir = path.join("keys").join(team_id);

        fs::create_dir_all(&dir)?;

        Ok(dir)
    }

    // applecodesign-rs needs our contents as strings to sign
    fn resolve_certificate_from_contents(&mut self, contents: Vec<u8>) -> Result<(), Error> {
         for pem in pem::parse_many(contents).map_err(Error::Pem)? {
            match pem.tag() {
                "CERTIFICATE" => {
                    println!("CERTIFICATE loaded!"); // TODO: REMOVE SOME DEBUG STATEMENTS IF THIS WORKS WONDERFULY
                    self.cert = Some(CapturedX509Certificate::from_der(pem.contents())?);
                }
                "PRIVATE KEY" => {
                    println!("PRIVATE KEY loaded!"); // TODO: REMOVE SOME DEBUG STATEMENTS IF THIS WORKS WONDERFULY
                    self.key = Some(Box::new(InMemoryPrivateKey::from_pkcs8_der(pem.contents())?));
                }
                "RSA PRIVATE KEY" => {
                    println!("RSA PRIVATE KEY loaded!"); // TODO: REMOVE SOME DEBUG STATEMENTS IF THIS WORKS WONDERFULY
                    self.key = Some(Box::new(InMemoryPrivateKey::from_pkcs1_der(pem.contents())?));
                }
                tag => println!("(unhandled PEM tag {}; ignoring)", tag),
            }
        }

        Ok(())
    }

    pub fn load_into_signing_settings<'settings, 'slf: 'settings>(
        &'slf self,
        settings: &'settings mut SigningSettings<'slf>,
    ) -> Result<(), Error> {
        let signing_cert = self.cert.clone().ok_or(Error::CertificatePemMissing)?;
        let signing_key = self.key.as_ref().ok_or(Error::CertificatePemMissing)?;

        settings.set_signing_key(signing_key.as_key_info_signer(), signing_cert);
        settings.chain_apple_certificates();

        Ok(())
    }
}

impl CertificateIdentity {
    async fn find_certificate(
        certs: Vec<Cert>,
        priv_key: &RsaPrivateKey,
        machine_name: &str,
    ) -> Result<Option<Cert>, Error> {
        let pub_key_der_obj = priv_key
            .to_public_key()
            .to_pkcs1_der()?
            .as_bytes()
            .to_vec();

        for cert in certs {
            if cert.machine_name.as_deref() == Some(machine_name) {
                let parsed_cert = X509Certificate::from_der(&cert.cert_content)?;
                if pub_key_der_obj == parsed_cert.public_key_data().as_ref() {
                    return Ok(Some(cert));
                }
            }
        }

        Ok(None)
    }

    async fn request_new_certificate(
        session: &DeveloperSession,
        team_id: &String,
        machine_name: &String,
        certs: Vec<Cert>,
    ) -> Result<(Cert, RsaPrivateKey), Error> {
        let priv_key = RsaPrivateKey::new(&mut OsRng, 2048)?;
        let priv_key_der = priv_key.to_pkcs8_der()?;
        let priv_key_pair = KeyPair::from_der(priv_key_der.as_bytes())?;

        let mut params = rcgen::CertificateParams::new(vec![]);
        params.alg = &PKCS_RSA_SHA256;
        params.key_pair = Some(priv_key_pair);

        let dn = &mut params.distinguished_name;
        dn.push(DnType::CountryName, "US");
        dn.push(DnType::StateOrProvinceName, "STATE");
        dn.push(DnType::LocalityName, "LOCAL");
        dn.push(DnType::OrganizationName, "ORGNIZATION");
        dn.push(DnType::CommonName, "CN");

        let cert_csr = rcgen::Certificate::from_params(params)?
            .serialize_request_pem()?;

        let cert_serial_numbers = certs
            .iter()
            .map(|c| c.serial_number.clone())
            .collect::<Vec<_>>();

        // When we submit a CSR theres a high chance of it failing, at least
        // on free developer accounts, we put it in a loop so whenever it does
        // fail, we also look through all of our existing certificates through
        // the api until we have a success on a single revokage, then we can
        // successfully submit our csr, but if we just cannot at all, return 
        // an error
        let cert_id = loop {
            match session
                .qh_submit_cert_csr(
                    &team_id,
                    cert_csr.clone(),
                    machine_name,
                ).await {
                    Ok(id) => break id,
                    Err(e) => {
                        // 7460 is for too many certificates (I think)
                        if matches!(&e, Error::DeveloperSession(code, _) if *code == 7460) {
                            // Try to revoke certificates from the candidate list
                            let mut revoked_any = false;
                            for cid in &cert_serial_numbers {
                                if session
                                    .qh_revoke_cert(&team_id, cid)
                                    .await
                                    .is_ok()
                                {
                                    revoked_any = true;
                                }
                            }
                            
                            if revoked_any {
                                continue;
                            } else {
                                return Err(Error::Certificate(
                                    "Too many certificates and failed to revoke any".into(),
                                ));
                            }
                        }
                        
                        return Err(e)
                    }
                }
        }.cert_request;

        // We request again, and hope this has our new certificate 
        // ready.... if not then woops... thats too bad isnt it
        let certs = session
            .qh_list_certs(&team_id)
            .await?
            .certificates
            .into_iter()
            .find(|c| c.certificate_id == cert_id.certificate_id);

        Ok((certs.ok_or(Error::CertificatePemMissing)?, priv_key))
    }
}
