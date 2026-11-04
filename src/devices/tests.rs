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
source_repository = "https://github.com/merely-made/murm"
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
