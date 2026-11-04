use super::*;
use serde_json::Value;

#[test]
fn tech_article_does_not_emit_an_offer() {
    let data = PublicSiteData::load(env!("CARGO_MANIFEST_DIR")).expect("public site data");
    let device = data.devices.ordered()[0];
    let canonical = format!("https://merelyllc.com/devices/{}/", device.id);
    let encoded = device_json_ld(device, &canonical);
    let value: Value = serde_json::from_str(&encoded).expect("valid JSON-LD");
    assert_eq!(value["@context"], "https://schema.org");
    assert!(!encoded.contains("\"offers\""));
}
