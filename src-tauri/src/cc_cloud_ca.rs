//! Custom root certificate for the Car-Controlling cloud proxy.
//!
//! The proxy uses a private mini-CA (no public domain), so its TLS
//! certificate is not trusted by the system store. We embed the CA's public
//! certificate and hand it to reqwest **only** for requests to the proxy host
//! — every other request keeps the normal webpki trust roots.

use once_cell::sync::Lazy;
use reqwest::Certificate;

/// Public CA certificate (safe to ship — no private key).
const CC_CA_PEM: &str = include_str!("../resources/cc-ca.pem");

/// The proxy host that the embedded CA is valid for.
const CC_CLOUD_HOST: &str = "82.25.97.160";

static CC_CA: Lazy<Option<Certificate>> =
    Lazy::new(|| match Certificate::from_pem(CC_CA_PEM.as_bytes()) {
        Ok(cert) => Some(cert),
        Err(e) => {
            log::error!("Failed to parse embedded CC cloud CA: {}", e);
            None
        }
    });

/// Returns the embedded CA certificate when `url` targets the proxy host,
/// otherwise `None` (standard trust roots apply).
pub fn certificate_for(url: &str) -> Option<Certificate> {
    if url.contains(CC_CLOUD_HOST) {
        CC_CA.clone()
    } else {
        None
    }
}
