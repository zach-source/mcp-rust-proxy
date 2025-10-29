//! TLS certificate generation and handling for HTTPS proxy
//!
//! This module implements certificate generation and caching for MITM proxy functionality.
//! It generates a root CA certificate on first run, then creates per-domain certificates
//! on-demand, signed by the CA and cached in memory.

use dashmap::DashMap;
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DistinguishedName, DnType, IsCa, KeyPair,
    KeyUsagePurpose,
};
use rustls::{pki_types::PrivateKeyDer, ServerConfig};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use time::{Duration, OffsetDateTime};

#[derive(Debug, Error)]
pub enum TlsError {
    #[error("Failed to generate certificate: {0}")]
    CertGeneration(String),

    #[error("Failed to load certificate: {0}")]
    CertLoad(String),

    #[error("Failed to save certificate: {0}")]
    CertSave(String),

    #[error("Invalid certificate: {0}")]
    InvalidCert(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Handles TLS certificate generation and caching for the Claude API proxy
pub struct TlsHandler {
    /// Root CA certificate for signing domain certificates
    ca_cert: Arc<Certificate>,

    /// Root CA private key for signing domain certificates
    ca_key: Arc<KeyPair>,

    /// Cache of generated ServerConfig by domain
    server_configs: Arc<DashMap<String, Arc<ServerConfig>>>,

    /// Path to CA certificate file
    ca_cert_path: PathBuf,

    /// Path to CA private key file
    ca_key_path: PathBuf,
}

impl TlsHandler {
    /// Create a new TLS handler, loading or creating the CA certificate
    pub fn new() -> Result<Self, TlsError> {
        let ca_dir = dirs::home_dir()
            .ok_or_else(|| TlsError::CertLoad("Could not find home directory".to_string()))?
            .join(".claude-proxy");

        let ca_cert_path = ca_dir.join("ca.crt");
        let ca_key_path = ca_dir.join("ca.key");

        // Ensure directory exists
        fs::create_dir_all(&ca_dir)?;

        let (ca_cert, ca_key) = Self::load_or_create_ca(&ca_cert_path, &ca_key_path)?;

        Ok(Self {
            ca_cert: Arc::new(ca_cert),
            ca_key: Arc::new(ca_key),
            server_configs: Arc::new(DashMap::new()),
            ca_cert_path,
            ca_key_path,
        })
    }

    /// Load existing CA certificate or generate a new one
    fn load_or_create_ca(
        cert_path: &PathBuf,
        key_path: &PathBuf,
    ) -> Result<(Certificate, KeyPair), TlsError> {
        if cert_path.exists() && key_path.exists() {
            tracing::info!("Loading existing CA certificate from {:?}", cert_path);
            Self::load_ca_from_disk(cert_path, key_path)
        } else {
            tracing::info!("Generating new CA certificate");
            let (ca_cert, ca_key) = Self::generate_root_ca()?;
            Self::save_ca_to_disk(&ca_cert, &ca_key, cert_path, key_path)?;
            Ok((ca_cert, ca_key))
        }
    }

    /// Generate a new root CA certificate
    fn generate_root_ca() -> Result<(Certificate, KeyPair), TlsError> {
        let mut params = CertificateParams::new(vec!["Claude Proxy CA".to_string()])
            .map_err(|e| TlsError::CertGeneration(e.to_string()))?;

        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "Claude Proxy Root CA");
        dn.push(DnType::OrganizationName, "Claude Code");
        params.distinguished_name = dn;

        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];

        // Set validity period to 1 year
        params.not_before = OffsetDateTime::now_utc() - Duration::days(1);
        params.not_after = OffsetDateTime::now_utc() + Duration::days(365);

        // Generate key pair
        let key_pair = KeyPair::generate().map_err(|e| TlsError::CertGeneration(e.to_string()))?;

        // Generate self-signed certificate
        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| TlsError::CertGeneration(e.to_string()))?;

        Ok((cert, key_pair))
    }

    /// Generate a domain-specific certificate signed by the CA
    fn generate_domain_cert(
        domain: &str,
        ca_cert: &Certificate,
        ca_key: &KeyPair,
    ) -> Result<(Certificate, KeyPair), TlsError> {
        let mut params = CertificateParams::new(vec![domain.to_string()])
            .map_err(|e| TlsError::CertGeneration(e.to_string()))?;

        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, domain);
        params.distinguished_name = dn;

        // Set validity to 90 days
        params.not_before = OffsetDateTime::now_utc() - Duration::days(1);
        params.not_after = OffsetDateTime::now_utc() + Duration::days(90);

        // Add subject alternative name
        params.subject_alt_names = vec![rcgen::SanType::DnsName(
            domain
                .to_string()
                .try_into()
                .map_err(|e| TlsError::CertGeneration(format!("Invalid domain name: {:?}", e)))?,
        )];

        // Generate key pair for domain certificate
        let key_pair = KeyPair::generate().map_err(|e| TlsError::CertGeneration(e.to_string()))?;

        // Sign domain certificate with CA
        // In rcgen 0.13, signed_by takes: subject_public_key, issuer_cert, issuer_key
        let cert = params
            .signed_by(&key_pair, ca_cert, ca_key)
            .map_err(|e| TlsError::CertGeneration(e.to_string()))?;

        Ok((cert, key_pair))
    }

    /// Load CA certificate from disk
    fn load_ca_from_disk(
        cert_path: &PathBuf,
        key_path: &PathBuf,
    ) -> Result<(Certificate, KeyPair), TlsError> {
        let _cert_pem = fs::read_to_string(cert_path)?;
        let key_pem = fs::read_to_string(key_path)?;

        // Load the key pair first
        let key_pair =
            KeyPair::from_pem(&key_pem).map_err(|e| TlsError::CertLoad(e.to_string()))?;

        // For loading a CA certificate, we need to reconstruct it from params
        // rcgen 0.13 doesn't provide a direct Certificate::from_pem method
        // So we regenerate the certificate with the same parameters using the loaded key

        // Recreate the CA certificate params (must match what we generated)
        let mut params = CertificateParams::new(vec!["Claude Proxy CA".to_string()])
            .map_err(|e| TlsError::CertLoad(e.to_string()))?;

        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "Claude Proxy Root CA");
        dn.push(DnType::OrganizationName, "Claude Code");
        params.distinguished_name = dn;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];

        // Self-sign with the loaded key to recreate certificate
        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| TlsError::CertLoad(e.to_string()))?;

        Ok((cert, key_pair))
    }

    /// Save CA certificate to disk
    fn save_ca_to_disk(
        ca: &Certificate,
        ca_key: &KeyPair,
        cert_path: &PathBuf,
        key_path: &PathBuf,
    ) -> Result<(), TlsError> {
        let cert_pem = ca.pem();
        let key_pem = ca_key.serialize_pem();

        fs::write(cert_path, cert_pem)?;
        fs::write(key_path, key_pem)?;

        tracing::info!(
            ca_cert_path = ?cert_path,
            ca_key_path = ?key_path,
            "Saved CA certificate to disk"
        );

        Ok(())
    }

    /// Get or generate a ServerConfig for the specified domain
    pub fn get_server_config(&self, domain: &str) -> Result<Arc<ServerConfig>, TlsError> {
        // Check cache first
        if let Some(config) = self.server_configs.get(domain) {
            tracing::debug!(domain = domain, "Using cached ServerConfig");
            return Ok(config.clone());
        }

        // Generate domain certificate
        tracing::debug!(domain = domain, "Generating new domain certificate");
        let (domain_cert, domain_key) =
            Self::generate_domain_cert(domain, &self.ca_cert, &self.ca_key)?;

        // Create certificate chain with domain cert and CA cert
        let signed_cert_pem = domain_cert
            .pem()
            .lines()
            .chain(self.ca_cert.pem().lines())
            .collect::<Vec<_>>()
            .join("\n");

        // Convert to rustls types
        let cert_chain = rustls_pemfile::certs(&mut signed_cert_pem.as_bytes())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| TlsError::InvalidCert(e.to_string()))?;

        let private_key = PrivateKeyDer::try_from(domain_key.serialize_der())
            .map_err(|e| TlsError::InvalidCert(format!("Invalid private key: {:?}", e)))?;

        // Build ServerConfig
        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key)
            .map_err(|e| TlsError::InvalidCert(e.to_string()))?;

        let config = Arc::new(config);

        // Cache it
        self.server_configs
            .insert(domain.to_string(), config.clone());

        tracing::info!(
            domain = domain,
            "Generated and cached ServerConfig for domain"
        );

        Ok(config)
    }

    /// Get a ClientConfig for making outbound HTTPS connections
    pub fn get_client_config() -> Arc<rustls::ClientConfig> {
        Arc::new(
            rustls::ClientConfig::builder()
                .with_root_certificates(rustls::RootCertStore::empty())
                .with_no_client_auth(),
        )
    }

    /// Get the CA certificate PEM for user installation
    pub fn get_ca_cert_pem(&self) -> String {
        self.ca_cert.pem()
    }

    /// Get the path to the CA certificate file
    pub fn ca_cert_path(&self) -> &PathBuf {
        &self.ca_cert_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_root_ca() {
        let (ca, key) = TlsHandler::generate_root_ca().expect("Failed to generate CA");

        let pem = ca.pem();
        assert!(pem.contains("BEGIN CERTIFICATE"));
        assert!(pem.contains("END CERTIFICATE"));

        let key_pem = key.serialize_pem();
        assert!(key_pem.contains("BEGIN PRIVATE KEY"));
    }

    #[test]
    fn test_generate_domain_cert() {
        let (ca, ca_key) = TlsHandler::generate_root_ca().expect("Failed to generate CA");
        let (domain_cert, _domain_key) =
            TlsHandler::generate_domain_cert("api.anthropic.com", &ca, &ca_key)
                .expect("Failed to generate domain cert");

        let pem = domain_cert.pem();
        assert!(pem.contains("BEGIN CERTIFICATE"));
    }

    #[test]
    fn test_load_or_create_ca() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let cert_path = temp_dir.path().join("ca.crt");
        let key_path = temp_dir.path().join("ca.key");

        // First call should create new CA
        let (ca1, key1) =
            TlsHandler::load_or_create_ca(&cert_path, &key_path).expect("Failed to create CA");
        assert!(cert_path.exists());
        assert!(key_path.exists());

        // Second call should load existing CA
        let (ca2, key2) =
            TlsHandler::load_or_create_ca(&cert_path, &key_path).expect("Failed to load CA");

        // Verify both have same key material
        assert_eq!(key1.serialize_pem(), key2.serialize_pem());
        // Note: Certs will be regenerated, but should be functionally equivalent
        assert!(ca1.pem().contains("BEGIN CERTIFICATE"));
        assert!(ca2.pem().contains("BEGIN CERTIFICATE"));
    }

    #[test]
    fn test_cert_caching() {
        let handler = TlsHandler::new().expect("Failed to create TlsHandler");

        // First call generates and caches
        let config1 = handler
            .get_server_config("api.anthropic.com")
            .expect("Failed to get config");

        // Second call should return cached version
        let config2 = handler
            .get_server_config("api.anthropic.com")
            .expect("Failed to get config");

        // Should be the same Arc
        assert!(Arc::ptr_eq(&config1, &config2));

        // Different domain should generate new config
        let config3 = handler
            .get_server_config("claude.ai")
            .expect("Failed to get config");

        assert!(!Arc::ptr_eq(&config1, &config3));
    }
}
