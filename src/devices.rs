use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::repositories::AuthorityError;

pub const DEVICE_CATALOG_PATH: &str = "content/devices.toml";
pub const DEVICE_CATALOG_SCHEMA: &str = "mer3ly.device-catalog/v2";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceCatalog {
    pub schema: String,
    #[serde(default)]
    pub device: Vec<DeviceRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceRecord {
    pub id: String,
    pub order: u32,
    pub name: String,
    pub role: String,
    pub summary: String,
    pub status: DeviceStatus,
    pub form: String,
    pub board: String,
    pub processor: String,
    pub radio: String,
    pub interaction: String,
    pub power: String,
    pub antenna: String,
    pub enclosure: String,
    pub recipe_state: String,
    pub source_repository: String,
    pub source_document: String,
    #[serde(default)]
    pub network_support: Vec<NetworkSupport>,
    #[serde(default)]
    pub flash_recipe: Vec<FlashRecipe>,
    pub authorization: AuthorizationRecord,
    pub sale: SaleRecord,
    #[serde(default)]
    pub evidence: Vec<DeviceEvidence>,
    #[serde(default)]
    pub open_requirement: Vec<OpenRequirement>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeviceStatus {
    Candidate,
    DevelopmentSpecimen,
    ProvenRecipe,
    AssembledPrototype,
    Sellable,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkSupport {
    pub name: String,
    pub state: NetworkSupportState,
    pub note: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NetworkSupportState {
    Demonstrated,
    UpstreamSupported,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FlashRecipe {
    pub package_id: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorizationRecord {
    pub state: AuthorizationState,
    pub device: String,
    pub antenna: String,
    pub operating_envelope: String,
    pub note: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthorizationState {
    Open,
    Reviewed,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SaleRecord {
    pub state: SaleState,
    pub note: String,
    #[serde(default)]
    pub purchase_url: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SaleState {
    NotOffered,
    Available,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceEvidence {
    pub label: String,
    pub state: EvidenceState,
    pub note: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceState {
    Proven,
    Partial,
    Open,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OpenRequirement {
    pub label: String,
    pub note: String,
}

impl DeviceCatalog {
    pub fn load(root: impl AsRef<Path>) -> Result<Self, AuthorityError> {
        let path = root.as_ref().join(DEVICE_CATALOG_PATH);
        let text = fs::read_to_string(&path).map_err(|error| {
            AuthorityError::from_message(format!("read {}: {error}", path.display()))
        })?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self, AuthorityError> {
        let catalog: Self = toml::from_str(text).map_err(|error| {
            AuthorityError::from_message(format!("parse {DEVICE_CATALOG_PATH}: {error}"))
        })?;
        catalog.validate().map_err(|errors| {
            AuthorityError::from_message(format!(
                "device catalog validation failed:\n{}",
                errors.join("\n")
            ))
        })?;
        Ok(catalog)
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.schema != DEVICE_CATALOG_SCHEMA {
            errors.push(format!(
                "device catalog schema must be {DEVICE_CATALOG_SCHEMA}, got {}",
                self.schema
            ));
        }
        if self.device.is_empty() {
            errors.push("device catalog has no records".to_owned());
        }

        let mut ids = BTreeSet::new();
        let mut orders = BTreeSet::new();
        for device in &self.device {
            check_slug("device id", &device.id, &mut errors);
            if !ids.insert(device.id.clone()) {
                errors.push(format!("duplicate device id {}", device.id));
            }
            if device.order == 0 {
                errors.push(format!("device {} order must be positive", device.id));
            }
            if !orders.insert(device.order) {
                errors.push(format!("duplicate device order {}", device.order));
            }

            for (field, value) in [
                ("name", device.name.as_str()),
                ("role", device.role.as_str()),
                ("summary", device.summary.as_str()),
                ("form", device.form.as_str()),
                ("board", device.board.as_str()),
                ("processor", device.processor.as_str()),
                ("radio", device.radio.as_str()),
                ("interaction", device.interaction.as_str()),
                ("power", device.power.as_str()),
                ("antenna", device.antenna.as_str()),
                ("enclosure", device.enclosure.as_str()),
                ("recipe_state", device.recipe_state.as_str()),
            ] {
                check_public_text(&format!("device {} {field}", device.id), value, &mut errors);
            }

            if device.source_repository != "https://github.com/merely-made/retinue" {
                errors.push(format!(
                    "device {} source_repository must be the public Retinue repository",
                    device.id
                ));
            }
            check_source_document(&device.id, &device.source_document, &mut errors);

            if device.network_support.is_empty() {
                errors.push(format!(
                    "device {} has no network support records",
                    device.id
                ));
            }
            let mut network_names = BTreeSet::new();
            for network in &device.network_support {
                check_public_text(
                    &format!("device {} network name", device.id),
                    &network.name,
                    &mut errors,
                );
                check_public_text(
                    &format!("device {} network {} note", device.id, network.name),
                    &network.note,
                    &mut errors,
                );
                if !network_names.insert(network.name.to_ascii_lowercase()) {
                    errors.push(format!(
                        "device {} repeats network support {}",
                        device.id, network.name
                    ));
                }
            }

            if device.flash_recipe.is_empty() {
                errors.push(format!("device {} has no flash recipes", device.id));
            }
            let mut flash_names = BTreeSet::new();
            let mut flash_packages = BTreeSet::new();
            for recipe in &device.flash_recipe {
                check_package_id(
                    &format!("device {} flash recipe package", device.id),
                    &recipe.package_id,
                    &mut errors,
                );
                check_public_text(
                    &format!("device {} flash recipe name", device.id),
                    &recipe.name,
                    &mut errors,
                );
                if !flash_names.insert(recipe.name.to_ascii_lowercase()) {
                    errors.push(format!(
                        "device {} repeats flash recipe {}",
                        device.id, recipe.name
                    ));
                }
                if !flash_packages.insert(recipe.package_id.to_ascii_lowercase()) {
                    errors.push(format!(
                        "device {} repeats flash recipe package {}",
                        device.id, recipe.package_id
                    ));
                }
            }

            for (field, value) in [
                ("authorization device", device.authorization.device.as_str()),
                (
                    "authorization antenna",
                    device.authorization.antenna.as_str(),
                ),
                (
                    "authorization operating_envelope",
                    device.authorization.operating_envelope.as_str(),
                ),
                ("authorization note", device.authorization.note.as_str()),
                ("sale note", device.sale.note.as_str()),
            ] {
                check_public_text(&format!("device {} {field}", device.id), value, &mut errors);
            }

            if device.evidence.is_empty() {
                errors.push(format!("device {} has no evidence records", device.id));
            }
            let mut evidence_labels = BTreeSet::new();
            for evidence in &device.evidence {
                check_public_text(
                    &format!("device {} evidence label", device.id),
                    &evidence.label,
                    &mut errors,
                );
                check_public_text(
                    &format!("device {} evidence {} note", device.id, evidence.label),
                    &evidence.note,
                    &mut errors,
                );
                if !evidence_labels.insert(evidence.label.to_ascii_lowercase()) {
                    errors.push(format!(
                        "device {} repeats evidence label {}",
                        device.id, evidence.label
                    ));
                }
            }

            let mut requirement_labels = BTreeSet::new();
            for requirement in &device.open_requirement {
                check_public_text(
                    &format!("device {} requirement label", device.id),
                    &requirement.label,
                    &mut errors,
                );
                check_public_text(
                    &format!(
                        "device {} requirement {} note",
                        device.id, requirement.label
                    ),
                    &requirement.note,
                    &mut errors,
                );
                if !requirement_labels.insert(requirement.label.to_ascii_lowercase()) {
                    errors.push(format!(
                        "device {} repeats requirement label {}",
                        device.id, requirement.label
                    ));
                }
            }

            match device.status {
                DeviceStatus::Sellable => {
                    if device.authorization.state != AuthorizationState::Reviewed {
                        errors.push(format!(
                            "sellable device {} requires reviewed authorization",
                            device.id
                        ));
                    }
                    if device.sale.state != SaleState::Available {
                        errors.push(format!(
                            "sellable device {} requires an available sale record",
                            device.id
                        ));
                    }
                    if device.sale.purchase_url.is_none() {
                        errors.push(format!(
                            "sellable device {} requires a purchase_url",
                            device.id
                        ));
                    }
                    if !device.open_requirement.is_empty() {
                        errors.push(format!(
                            "sellable device {} still has open requirements",
                            device.id
                        ));
                    }
                }
                _ => {
                    if device.sale.state != SaleState::NotOffered {
                        errors.push(format!(
                            "non-sellable device {} must be marked not-offered",
                            device.id
                        ));
                    }
                    if device.sale.purchase_url.is_some() {
                        errors.push(format!(
                            "non-sellable device {} cannot have a purchase_url",
                            device.id
                        ));
                    }
                    if device.open_requirement.is_empty() {
                        errors.push(format!(
                            "non-sellable device {} must disclose open requirements",
                            device.id
                        ));
                    }
                }
            }
            if let Some(url) = &device.sale.purchase_url {
                check_https_url(
                    &format!("device {} purchase_url", device.id),
                    url,
                    &mut errors,
                );
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn ordered(&self) -> Vec<&DeviceRecord> {
        let mut devices = self.device.iter().collect::<Vec<_>>();
        devices.sort_by_key(|device| device.order);
        devices
    }

    pub fn by_id(&self, id: &str) -> Option<&DeviceRecord> {
        self.device.iter().find(|device| device.id == id)
    }
}

impl DeviceStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::DevelopmentSpecimen => "development specimen",
            Self::ProvenRecipe => "proven recipe",
            Self::AssembledPrototype => "assembled prototype",
            Self::Sellable => "available to buy",
        }
    }
}

impl NetworkSupportState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Demonstrated => "demonstrated",
            Self::UpstreamSupported => "upstream-supported",
        }
    }
}

impl AuthorizationState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Open => "authorization record open",
            Self::Reviewed => "authorization reviewed",
        }
    }
}

impl SaleState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::NotOffered => "not offered for sale",
            Self::Available => "available to buy",
        }
    }
}

impl EvidenceState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Proven => "proven",
            Self::Partial => "partial",
            Self::Open => "open",
        }
    }
}

fn check_slug(label: &str, value: &str, errors: &mut Vec<String>) {
    if value.is_empty()
        || value.starts_with('-')
        || value.ends_with('-')
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        errors.push(format!(
            "{label} must be a lowercase ASCII slug, got {value:?}"
        ));
    }
}

fn check_package_id(label: &str, value: &str, errors: &mut Vec<String>) {
    if value.is_empty()
        || value.starts_with('.')
        || value.ends_with('.')
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'.' || byte == b'-'
        })
    {
        errors.push(format!(
            "{label} must be a lowercase package id, got {value:?}"
        ));
    }
}

fn check_public_text(label: &str, value: &str, errors: &mut Vec<String>) {
    if value.trim().is_empty() {
        errors.push(format!("{label} must not be empty"));
    }
    let normalized = value.to_ascii_lowercase().replace('/', "\\");
    for marker in [
        "c:\\users\\",
        "\\\\?\\",
        "localhost",
        "127.0.0.1",
        "192.168.",
        "10.0.",
        "com6",
        "com10",
    ] {
        if normalized.contains(marker) {
            errors.push(format!("{label} contains private marker {marker:?}"));
        }
    }
}

fn check_source_document(device_id: &str, value: &str, errors: &mut Vec<String>) {
    check_public_text(
        &format!("device {device_id} source_document"),
        value,
        errors,
    );
    if value.contains('\\')
        || value.starts_with('/')
        || value.split('/').any(|segment| segment == "..")
        || !(value.starts_with("design_docs/") || value.starts_with("firmware/"))
    {
        errors.push(format!(
            "device {device_id} source_document must be a public relative Retinue evidence path"
        ));
    }
}

fn check_https_url(label: &str, value: &str, errors: &mut Vec<String>) {
    if !value.starts_with("https://") || value.contains('@') {
        errors.push(format!("{label} must be a credential-free HTTPS URL"));
    }
    check_public_text(label, value, errors);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest() -> String {
        r#"
schema = "mer3ly.device-catalog/v2"

[[device]]
id = "bench-radio"
order = 1
name = "Bench radio"
role = "A radio for bench work"
summary = "A checked development specimen."
status = "development-specimen"
form = "desktop"
board = "Example board"
processor = "Example processor"
radio = "Example radio"
interaction = "One button"
power = "USB bench power"
antenna = "Open"
enclosure = "Open"
recipe_state = "Incomplete"
source_repository = "https://github.com/merely-made/retinue"
source_document = "firmware/example/README.md"

[device.authorization]
state = "open"
device = "Grant review open"
antenna = "Antenna review open"
operating_envelope = "Operating envelope review open"
note = "Authorization must be reviewed before sale."

[device.sale]
state = "not-offered"
note = "Development specimen only."

[[device.network_support]]
name = "Reticulum"
state = "demonstrated"
note = "Bench proof only."

[[device.flash_recipe]]
package_id = "retinue.test"
name = "Retinue"

[[device.evidence]]
label = "Radio carriage"
state = "partial"
note = "One hop only."

[[device.open_requirement]]
label = "Enclosure"
note = "Needs a checked print."
"#
        .to_owned()
    }

    #[test]
    fn valid_manifest_parses() {
        let catalog = DeviceCatalog::parse(&valid_manifest()).expect("valid device catalog");
        assert_eq!(catalog.device.len(), 1);
    }

    #[test]
    fn development_specimen_rejects_purchase_link() {
        let text = valid_manifest().replace(
            "state = \"not-offered\"\nnote = \"Development specimen only.\"",
            "state = \"not-offered\"\nnote = \"Development specimen only.\"\npurchase_url = \"https://example.com/buy\"",
        );
        let error = DeviceCatalog::parse(&text).expect_err("purchase link must fail");
        assert!(error.to_string().contains("cannot have a purchase_url"));
    }

    #[test]
    fn sellable_record_requires_purchase_link() {
        let text = valid_manifest()
            .replace("status = \"development-specimen\"", "status = \"sellable\"")
            .replace("state = \"open\"", "state = \"reviewed\"")
            .replace("state = \"not-offered\"", "state = \"available\"")
            .replace(
                "[[device.open_requirement]]\nlabel = \"Enclosure\"\nnote = \"Needs a checked print.\"",
                "",
            );
        let error = DeviceCatalog::parse(&text).expect_err("missing purchase link must fail");
        assert!(error.to_string().contains("requires a purchase_url"));
    }

    #[test]
    fn local_receipt_path_is_rejected() {
        let text =
            valid_manifest().replace("firmware/example/README.md", "C:/Users/person/private.txt");
        let error = DeviceCatalog::parse(&text).expect_err("local path must fail");
        assert!(error.to_string().contains("private marker"));
    }
}
