//! The retained public Retinue package index and its exact provenance.
//!
//! Mer3ly does not invent installer readiness in device prose. It renders the
//! package state, instructions, recovery path, and physical-receipt hosts from
//! the published Retinue artifact retained under `content/`.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::devices::DeviceCatalog;
use crate::repositories::AuthorityError;

pub const PACKAGE_INDEX_PATH: &str = "content/retinue-package-index.toml";
pub const PACKAGE_INDEX_SOURCE_PATH: &str = "content/retinue-package-index-source.toml";
pub const PACKAGE_INDEX_SCHEMA: &str = "retinue.package-index/v1";
pub const PACKAGE_INDEX_SOURCE_SCHEMA: &str = "mer3ly.firmware-index-source/v1";
pub const RETINUE_PACKAGE_INDEX_URL: &str = "https://github.com/merely-made/retinue/blob/bd71ee1886a4dea397854aa3084341b44c76b55f/firmware/packages/index.toml";

#[derive(Clone, Debug)]
pub struct FirmwareCatalog {
    index: PackageIndex,
    source: PackageIndexSource,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageIndex {
    schema: String,
    publisher: String,
    version: String,
    #[serde(default)]
    packages: Vec<FirmwarePackage>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FirmwarePackage {
    pub package_id: String,
    manifest: String,
    pub firmware_publisher: String,
    pub state: FirmwareRecipeState,
    pub instructions_url: String,
    pub recovery_url: String,
    pub installer_receipts: Vec<String>,
    pub recovery_receipts: Vec<String>,
    #[serde(default)]
    pub receipt_hosts: Vec<String>,
    #[serde(default)]
    purchase_url: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum FirmwareRecipeState {
    Partial,
    ProvenRecipe,
    Sellable,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageIndexSource {
    schema: String,
    source_url: String,
    source_sha256: String,
}

impl FirmwareCatalog {
    pub fn load(root: impl AsRef<Path>) -> Result<Self, AuthorityError> {
        let root = root.as_ref();
        let index_path = root.join(PACKAGE_INDEX_PATH);
        let index_bytes = fs::read(&index_path).map_err(|error| {
            AuthorityError::from_message(format!("read {}: {error}", index_path.display()))
        })?;
        let source_path = root.join(PACKAGE_INDEX_SOURCE_PATH);
        let source_text = fs::read_to_string(&source_path).map_err(|error| {
            AuthorityError::from_message(format!("read {}: {error}", source_path.display()))
        })?;
        let source: PackageIndexSource = toml::from_str(&source_text).map_err(|error| {
            AuthorityError::from_message(format!("parse {}: {error}", source_path.display()))
        })?;
        let index: PackageIndex = toml::from_slice(&index_bytes).map_err(|error| {
            AuthorityError::from_message(format!("parse {}: {error}", index_path.display()))
        })?;
        let catalog = Self { index, source };
        catalog.validate(&index_bytes)?;
        Ok(catalog)
    }

    pub fn package(&self, package_id: &str) -> Option<&FirmwarePackage> {
        self.index
            .packages
            .iter()
            .find(|package| package.package_id == package_id)
    }

    pub fn source_url(&self) -> &str {
        &self.source.source_url
    }

    pub fn validate_device_recipes(&self, devices: &DeviceCatalog) -> Result<(), AuthorityError> {
        for device in &devices.device {
            for recipe in &device.flash_recipe {
                if self.package(&recipe.package_id).is_none() {
                    return Err(AuthorityError::from_message(format!(
                        "device {} names unknown Retinue package {:?}",
                        device.id, recipe.package_id
                    )));
                }
            }
        }
        Ok(())
    }

    fn validate(&self, index_bytes: &[u8]) -> Result<(), AuthorityError> {
        if self.source.schema != PACKAGE_INDEX_SOURCE_SCHEMA {
            return Err(AuthorityError::from_message(format!(
                "firmware index source schema must be {PACKAGE_INDEX_SOURCE_SCHEMA}, got {}",
                self.source.schema
            )));
        }
        if self.source.source_url != RETINUE_PACKAGE_INDEX_URL {
            return Err(AuthorityError::from_message(format!(
                "firmware index source_url must be {RETINUE_PACKAGE_INDEX_URL}"
            )));
        }
        let actual_hash = format!("{:x}", Sha256::digest(index_bytes));
        if self.source.source_sha256 != actual_hash {
            return Err(AuthorityError::from_message(format!(
                "firmware index digest mismatch: source records {}, retained artifact is {}",
                self.source.source_sha256, actual_hash
            )));
        }
        if self.index.schema != PACKAGE_INDEX_SCHEMA {
            return Err(AuthorityError::from_message(format!(
                "firmware index schema must be {PACKAGE_INDEX_SCHEMA}, got {}",
                self.index.schema
            )));
        }
        for (name, value) in [
            ("publisher", self.index.publisher.as_str()),
            ("version", self.index.version.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(AuthorityError::from_message(format!(
                    "firmware index {name} must not be empty"
                )));
            }
        }
        if self.index.packages.is_empty() {
            return Err(AuthorityError::from_message(
                "firmware index must list at least one package",
            ));
        }

        let mut package_ids = BTreeSet::new();
        for package in &self.index.packages {
            if !package_ids.insert(package.package_id.as_str()) {
                return Err(AuthorityError::from_message(format!(
                    "firmware index repeats package {:?}",
                    package.package_id
                )));
            }
            validate_package(package)?;
        }
        Ok(())
    }
}

impl FirmwareRecipeState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Partial => "partial recipe",
            Self::ProvenRecipe => "proven recipe",
            Self::Sellable => "sellable firmware recipe",
        }
    }
}

fn validate_package(package: &FirmwarePackage) -> Result<(), AuthorityError> {
    for (name, value) in [
        ("package_id", package.package_id.as_str()),
        ("manifest", package.manifest.as_str()),
        ("firmware_publisher", package.firmware_publisher.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(AuthorityError::from_message(format!(
                "firmware package {name} must not be empty"
            )));
        }
    }
    validate_https_url("instructions_url", &package.instructions_url)?;
    validate_https_url("recovery_url", &package.recovery_url)?;
    match package.state {
        FirmwareRecipeState::Partial => {
            if !package.installer_receipts.is_empty()
                || !package.recovery_receipts.is_empty()
                || !package.receipt_hosts.is_empty()
                || package.purchase_url.is_some()
            {
                return Err(AuthorityError::from_message(format!(
                    "partial firmware package {:?} cannot claim receipt or purchase evidence",
                    package.package_id
                )));
            }
        }
        FirmwareRecipeState::ProvenRecipe => {
            validate_receipt_evidence(package)?;
            if package.purchase_url.is_some() {
                return Err(AuthorityError::from_message(format!(
                    "proven firmware package {:?} cannot have a purchase URL",
                    package.package_id
                )));
            }
        }
        FirmwareRecipeState::Sellable => {
            validate_receipt_evidence(package)?;
            let purchase_url = package.purchase_url.as_deref().ok_or_else(|| {
                AuthorityError::from_message(format!(
                    "sellable firmware package {:?} needs a purchase URL",
                    package.package_id
                ))
            })?;
            validate_https_url("purchase_url", purchase_url)?;
        }
    }
    Ok(())
}

fn validate_receipt_evidence(package: &FirmwarePackage) -> Result<(), AuthorityError> {
    if package.installer_receipts.is_empty()
        || package.recovery_receipts.is_empty()
        || package.receipt_hosts.is_empty()
    {
        return Err(AuthorityError::from_message(format!(
            "firmware package {:?} needs installer, recovery, and host receipt evidence",
            package.package_id
        )));
    }
    for receipt in package
        .installer_receipts
        .iter()
        .chain(package.recovery_receipts.iter())
    {
        validate_https_url("receipt", receipt)?;
    }
    let mut hosts = BTreeSet::new();
    for host in &package.receipt_hosts {
        if ![
            "windows-x86_64",
            "macos-x86_64",
            "macos-aarch64",
            "linux-x86_64",
            "linux-aarch64",
        ]
        .contains(&host.as_str())
        {
            return Err(AuthorityError::from_message(format!(
                "firmware package {:?} has unsupported receipt host {:?}",
                package.package_id, host
            )));
        }
        if !hosts.insert(host.as_str()) {
            return Err(AuthorityError::from_message(format!(
                "firmware package {:?} repeats receipt host {:?}",
                package.package_id, host
            )));
        }
    }
    Ok(())
}

fn validate_https_url(name: &str, value: &str) -> Result<(), AuthorityError> {
    if !value.starts_with("https://") || value.len() == "https://".len() || value.contains('@') {
        return Err(AuthorityError::from_message(format!(
            "firmware {name} must be a credential-free HTTPS URL"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_index_has_exact_provenance_and_expected_states() {
        let catalog = FirmwareCatalog::load(env!("CARGO_MANIFEST_DIR")).expect("firmware catalog");
        assert_eq!(
            catalog.package("retinue.heltec-v4").unwrap().state,
            FirmwareRecipeState::ProvenRecipe
        );
        assert_eq!(
            catalog.package("retinue.t114").unwrap().state,
            FirmwareRecipeState::ProvenRecipe
        );
        assert_eq!(
            catalog
                .package("meshtastic.heltec-mesh-node-t114")
                .unwrap()
                .state,
            FirmwareRecipeState::Partial
        );
    }

    #[test]
    fn tampered_retained_bytes_are_refused_before_site_generation() {
        let catalog = FirmwareCatalog::load(env!("CARGO_MANIFEST_DIR")).expect("firmware catalog");
        let error = catalog
            .validate(b"changed package-index bytes")
            .expect_err("changed bytes must not reach page rendering");

        assert!(error.to_string().contains("firmware index digest mismatch"));
    }
}
