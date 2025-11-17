use sha1::{Sha1, Digest};
use hex;

use rsa::{
    RsaPrivateKey,
    RsaPublicKey,
    pkcs1::{DecodeRsaPublicKey, EncodeRsaPublicKey},
    pkcs8::{DecodePrivateKey, EncodePrivateKey},
};
use rand::rngs::OsRng;
use rcgen::{CertificateParams, Certificate, KeyPair, DnType, PKCS_RSA_SHA256};
use x509_certificate::X509Certificate;

use pem_rfc7468::{encode_string, LineEnding};

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::Error;
use crate::developer::{DeveloperSession};

#[derive(Debug, Clone)]
pub struct CertificateIdentity {
    pub certificate: Option<X509Certificate>,
    pub private_key: RsaPrivateKey,
    pub key_file: PathBuf,
    pub cert_file: PathBuf,
    pub machine_name: String,
}

impl CertificateIdentity {
    pub async fn new(
        configuration_path: &Path,
        dev_session: &DeveloperSession,
        apple_id: String,
        machine_name: String,
        team: &str,
    ) -> Result<Self, Error> {
        let mut hasher = Sha1::new();
        hasher.update(apple_id.as_bytes());
        let hash_string = hex::encode(hasher.finalize()).to_lowercase();

        let key_path = configuration_path.join("keys").join(hash_string);
        fs::create_dir_all(&key_path)?;

        let key_file = key_path.join("key.pem");
        let cert_file = key_path.join("cert.pem");

        // --- Load or generate key ---
        let private_key = if key_file.exists() {
            let pem = fs::read_to_string(&key_file)
                .map_err(|e| Error::Certificate(format!("Failed to read key: {e}")))?;
            RsaPrivateKey::from_pkcs8_pem(&pem)
                .map_err(|e| Error::Certificate(format!("Failed to parse private key: {e}")))?
        } else {
            let mut rng = OsRng;
            let key = RsaPrivateKey::new(&mut rng, 2048)
                .map_err(|e| Error::Certificate(format!("Failed to generate key: {e}")))?;

            let pem = key
                .to_pkcs8_pem(Default::default())
                .map_err(|e| Error::Certificate(format!("Failed to encode key: {e}")))?
                .to_string();

            fs::write(&key_file, pem)?;

            key
        };

        let mut ci = CertificateIdentity {
            certificate: None,
            private_key,
            key_file,
            cert_file,
            machine_name,
        };

        // --- Try to find existing certificate ---
        if let Ok(cert) = ci.find_matching_certificate(dev_session, team).await {
            let pem = encode_string(
                "CERTIFICATE",
                LineEnding::LF,
                cert.encode_der().map_err(|e| Error::Certificate(format!("{e}")))?.as_slice(),
            ).unwrap();

            fs::write(&ci.cert_file, pem)?;

            ci.certificate = Some(cert);
            return Ok(ci);
        }

        // --- Request new certificate ---
        ci.request_new_certificate(dev_session, team).await?;

        Ok(ci)
    }

    async fn find_matching_certificate(
        &self,
        dev_session: &DeveloperSession,
        team: &str,
    ) -> Result<X509Certificate, Error> {
        let certs = dev_session
            .qh_list_certs(team)
            .await?
            .certificates;
        // Our RSA public key (PKCS#1 DER)
        let our_pub_pkcs1_der = self.private_key
            .to_public_key()
            .to_pkcs1_der()
            .map_err(|e| Error::Certificate(format!("Failed to encode public key (pkcs1): {e}")))?
            .as_bytes()
            .to_vec();

        for cert_meta in certs.iter().filter(|c| c.machine_name == Some(self.machine_name.clone())) {
            if let Ok(cert) = X509Certificate::from_der(&cert_meta.cert_content) {
                // Extract BIT STRING containing PKCS#1 public key
                let bit_string = &cert.tbs_certificate().subject_public_key_info.subject_public_key;
                let raw = bit_string.octet_slice().unwrap_or_default();
                if raw.is_empty() {
                    continue;
                }
                // First byte is number of unused bits (should be 0 for public key bit strings)
                let unused_bits = raw[0];
                if unused_bits != 0 {
                    continue;
                }
                let pkcs1_bytes = &raw[1..];
                if let Ok(cert_pub) = RsaPublicKey::from_pkcs1_der(pkcs1_bytes) {
                    let cert_pub_pkcs1_der = cert_pub
                        .to_pkcs1_der()
                        .map_err(|e| Error::Certificate(format!("Failed to re-encode cert public key: {e}")))?
                        .as_bytes()
                        .to_vec();
                    if cert_pub_pkcs1_der == our_pub_pkcs1_der {
                        return Ok(cert);
                    }
                }
            }
        }

        Err(Error::Certificate("No matching certificate found".into()))
    }

    async fn request_new_certificate(
        &mut self,
        dev_session: &DeveloperSession,
        team: &str,
    ) -> Result<(), Error> {
        // Convert RSA private key → PKCS8 DER → rcgen KeyPair
        let pkcs8 = self.private_key
            .to_pkcs8_der()
            .map_err(|e| Error::Certificate(format!("Failed to encode pkcs8: {e}")))?;

        let keypair = KeyPair::from_der(pkcs8.as_bytes())
            .map_err(|e| Error::Certificate(format!("Failed to load rcgen key: {e}")))?;

        // --- Build CSR ---
        let mut params = CertificateParams::new(vec![]);
        // Use an RSA signature algorithm to match the RSA key pair
        params.alg = &PKCS_RSA_SHA256;
        params.key_pair = Some(keypair);

        let dn = &mut params.distinguished_name;
        dn.push(DnType::CountryName, "US");
        dn.push(DnType::StateOrProvinceName, "STATE");
        dn.push(DnType::LocalityName, "LOCAL");
        dn.push(DnType::OrganizationName, "ORGNIZATION");
        dn.push(DnType::CommonName, "CN");

        let csr = Certificate::from_params(params)
            .map_err(|e| Error::Certificate(format!("Failed to build CSR params: {e}")))?;

        let csr_pem = csr.serialize_request_pem()
            .map_err(|e| Error::Certificate(format!("Failed to serialize CSR: {e}")))?;

        let cert_id = loop {
            match dev_session
                .qh_submit_cert_csr(
                    team,
                    csr_pem.clone(),
                    &self.machine_name.clone(),
                )
                .await
            {
                Ok(id) => break id,
                Err(e) => {
                    if let Error::DeveloperSession(code, _) = &e {
                        if *code == 7460 {
                            // TODO: this isn't really the ideal way to revoke certificiates
                            // trying to match the machine id with another existing id
                            // because if they dont exist and still taking up slots, it would
                            // most likely error with too many certificates
                            let certs = dev_session
                                .qh_list_certs(team)
                                .await?
                                .certificates;

                            if let Some(target) = certs.iter().find(|c| c.machine_name.as_deref() == Some(self.machine_name.as_ref())) {
                                let cid = target.serial_number.clone();
                                dev_session
                                    .qh_revoke_cert(team, &cid)
                                    .await
                                    .map_err(|err| {
                                        Error::Certificate(format!(
                                            "Failed to revoke certificate {cid}: {err:?}"
                                        ))
                                    })?;
                                // Retry submit after revocation
                                continue;
                            } else {
                                return Err(Error::Certificate(
                                    "Too many certificates".into(),
                                ));
                            }
                        }
                    }
                    return Err(Error::Certificate(format!("Submit CSR failed: {:?}", e)));
                }
            }
        };

        // --- Fetch new certificate from Apple ---
        let certs = dev_session
            .qh_list_certs(team)
            .await?.certificates;

        let found = certs
            .iter()
            .find(|c| c.certificate_id == cert_id.cert_request.certificate_id)
            .ok_or_else(|| Error::Certificate("Certificate not found after submission".into()))?;

        let parsed = X509Certificate::from_der(&found.cert_content)
            .map_err(|e| Error::Certificate(format!("Failed to parse DER: {e}")))?;

        // Save PEM
        let pem = encode_string(
            "CERTIFICATE",
            LineEnding::LF,
            found.cert_content.as_ref(),
        ).unwrap();

        fs::write(&self.cert_file, pem)?;

        self.certificate = Some(parsed);

        Ok(())
    }

    pub fn get_certificate_file_path(&self) -> &Path {
        &self.cert_file
    }

    pub fn get_private_key_file_path(&self) -> &Path {
        &self.key_file
    }

    pub fn get_serial_number(&self) -> Result<String, Error> {
        let cert = self.certificate.as_ref()
            .ok_or_else(|| Error::Certificate("No certificate loaded".into()))?;

        let serial = cert
            .tbs_certificate()
            .serial_number
            .clone()
            .into_bytes();

        Ok(hex::encode(serial).trim_start_matches('0').to_string())
    }
}
