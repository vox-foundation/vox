//! TOFU certificate pinning for Remote Axis. The server's leaf certificate is pinned by its
//! SHA3-256 fingerprint; the CA chain and hostname are deliberately not consulted (LAN servers use
//! self-signed certificates), so the pin is the only trust anchor.

use std::sync::{Arc, Mutex};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PinOutcome {
    Match,
    FirstUse { fingerprint: String },
    Changed { pinned: String, fingerprint: String },
}

#[derive(Debug)]
pub struct PinVerifier {
    expected: Option<String>,
    seen: Mutex<Option<String>>,
    provider: Arc<CryptoProvider>,
}

pub fn cert_fingerprint(der: &[u8]) -> String {
    vox_crypto::hex_encode(&vox_crypto::compliance_hash(der))
}

pub fn pin_outcome(expected: Option<&str>, seen: &str) -> PinOutcome {
    match expected {
        Some(pin) if pin.eq_ignore_ascii_case(seen) => PinOutcome::Match,
        Some(pin) => PinOutcome::Changed {
            pinned: pin.to_ascii_lowercase(),
            fingerprint: seen.to_string(),
        },
        None => PinOutcome::FirstUse {
            fingerprint: seen.to_string(),
        },
    }
}

impl PinVerifier {
    pub fn new(expected: Option<String>) -> Self {
        Self {
            expected,
            seen: Mutex::new(None),
            provider: Arc::new(rustls::crypto::ring::default_provider()),
        }
    }

    pub fn seen(&self) -> Option<String> {
        self.seen.lock().expect("pin verifier mutex").clone()
    }

    pub fn outcome(&self) -> Option<PinOutcome> {
        self.seen()
            .map(|fp| pin_outcome(self.expected.as_deref(), &fp))
    }
}

impl ServerCertVerifier for PinVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let fingerprint = cert_fingerprint(end_entity.as_ref());
        *self.seen.lock().expect("pin verifier mutex") = Some(fingerprint.clone());
        match pin_outcome(self.expected.as_deref(), &fingerprint) {
            PinOutcome::Match => Ok(ServerCertVerified::assertion()),
            _ => Err(rustls::Error::General(
                "axis remote: server certificate is not pinned".into(),
            )),
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

pub fn pinned_client_config(
    verifier: Arc<PinVerifier>,
) -> Result<rustls::ClientConfig, rustls::Error> {
    let provider = verifier.provider.clone();
    Ok(rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()?
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf() -> CertificateDer<'static> {
        CertificateDer::from(vec![0x30, 0x03, 0x02, 0x01, 0x01])
    }

    fn verify(v: &PinVerifier) -> Result<ServerCertVerified, rustls::Error> {
        let name = ServerName::try_from("vox.lan").expect("server name");
        v.verify_server_cert(&leaf(), &[], &name, &[], UnixTime::now())
    }

    #[test]
    fn fingerprint_is_lowercase_hex_sha3_256() {
        let fp = cert_fingerprint(b"abc");
        assert_eq!(fp.len(), 64);
        assert!(
            fp.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
        assert_eq!(
            fp,
            vox_crypto::hex_encode(&vox_crypto::compliance_hash(b"abc"))
        );
        assert_ne!(fp, cert_fingerprint(b"abd"));
    }

    #[test]
    fn pin_outcome_cases() {
        assert_eq!(pin_outcome(Some("ABCD"), "abcd"), PinOutcome::Match);
        assert_eq!(
            pin_outcome(None, "abcd"),
            PinOutcome::FirstUse {
                fingerprint: "abcd".into()
            }
        );
        assert_eq!(
            pin_outcome(Some("1111"), "abcd"),
            PinOutcome::Changed {
                pinned: "1111".into(),
                fingerprint: "abcd".into()
            }
        );
    }

    #[test]
    fn verifier_rejects_first_use_and_records_the_fingerprint() {
        let v = PinVerifier::new(None);
        assert!(verify(&v).is_err());
        let fp = cert_fingerprint(leaf().as_ref());
        assert_eq!(v.seen(), Some(fp.clone()));
        assert_eq!(v.outcome(), Some(PinOutcome::FirstUse { fingerprint: fp }));
    }

    #[test]
    fn verifier_accepts_only_the_pinned_certificate() {
        let fp = cert_fingerprint(leaf().as_ref());
        assert!(verify(&PinVerifier::new(Some(fp))).is_ok());
        let changed = PinVerifier::new(Some("00".repeat(32)));
        assert!(verify(&changed).is_err());
        assert!(matches!(
            changed.outcome(),
            Some(PinOutcome::Changed { .. })
        ));
    }

    #[test]
    fn pinned_client_config_builds_on_the_ring_provider() {
        let cfg = pinned_client_config(Arc::new(PinVerifier::new(None))).expect("config");
        assert!(cfg.alpn_protocols.is_empty());
    }
}
