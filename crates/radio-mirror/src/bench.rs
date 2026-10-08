//! Build-time screens and status documents for the devices-page bench.
//!
//! Every screen is reached by pressing the V4's one fitted button through
//! radio-mirror's `Mirror`, which runs the firmware's own `Controller` and
//! renderer. A screen the controller does not reach in that sequence fails the
//! build rather than being drawn by name.

use radio_mirror::{Mirror, input, names};
use serde_json::{Value, json};

use crate::{HOST_FIXTURE, LOCAL_FIXTURE, RETINUE_REPOSITORY, RETINUE_REVISION, SURFACE};

/// The schema of the inline scenario document the bench reads.
pub const SCENARIOS_SCHEMA: &str = "mer3ly.radio-bench-scenarios/v1";

/// The fault the V4 firmware raises when its SX1262 does not initialize
/// (`firmware/heltec-v4-phy/src/main.rs` at the pinned revision).
const V4_INIT_FAULT_CODE: u8 = 1;
const V4_INIT_FAULT_MESSAGE: &str = "SX1262 INIT";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scenario {
    /// No host attached: the four local pages.
    Local,
    /// retinue's host fixture attached: all seven pages, and VERIFY.
    Host,
    /// The V4's radio-init fault, which preempts every page.
    Fault,
}

impl Scenario {
    pub const ALL: [Self; 3] = [Self::Local, Self::Host, Self::Fault];

    pub const fn id(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Host => "host",
            Self::Fault => "fault",
        }
    }
}

/// One scenario's documents, in radio-mirror's JSON schemas.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioDocuments {
    pub local: Value,
    pub host: Option<Value>,
}

fn fixture(text: &str) -> Value {
    serde_json::from_str(text).expect("pinned radio-mirror fixture is JSON")
}

pub fn scenario_documents(scenario: Scenario) -> ScenarioDocuments {
    let mut local = fixture(LOCAL_FIXTURE);
    match scenario {
        Scenario::Host => ScenarioDocuments {
            local,
            host: Some(fixture(HOST_FIXTURE)),
        },
        Scenario::Local => {
            local["host"] = json!("detached");
            ScenarioDocuments { local, host: None }
        }
        Scenario::Fault => {
            local["host"] = json!("detached");
            local["radio"] = json!("fault");
            local["fault"] = json!({
                "code": V4_INIT_FAULT_CODE,
                "message": V4_INIT_FAULT_MESSAGE,
            });
            ScenarioDocuments { local, host: None }
        }
    }
}

/// The inline document the bench's script reads: every scenario's status
/// documents, and where they came from.
pub fn scenarios_json() -> String {
    let scenarios = Scenario::ALL
        .iter()
        .map(|scenario| {
            let documents = scenario_documents(*scenario);
            (
                scenario.id().to_owned(),
                json!({ "local": documents.local, "host": documents.host }),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    serde_json::to_string(&json!({
        "schema": SCENARIOS_SCHEMA,
        "source": {
            "repository": RETINUE_REPOSITORY,
            "revision": RETINUE_REVISION,
            "crate": "radio-mirror",
            "fixtures": [
                "crates/radio-mirror/fixtures/receipts-local.json",
                "crates/radio-mirror/fixtures/receipts-host.json"
            ]
        },
        "surface": SURFACE,
        "scenarios": scenarios,
    }))
    .expect("scenario documents serialize")
}

/// One screen rendered for readers without script.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticScreen {
    /// Short name used in the file name, e.g. `status` or `menu`.
    pub slug: &'static str,
    /// radio-mirror's screen name, e.g. `menu:brightness:0`.
    pub screen: String,
    /// Path in the published artifact.
    pub path: String,
    pub png: Vec<u8>,
    /// radio-face's text projection: what the pixels say, one line per row.
    pub lines: Vec<String>,
}

fn mirror_for(scenario: Scenario) -> Result<Mirror, String> {
    let documents = scenario_documents(scenario);
    let surface = names::surface(SURFACE).map_err(|error| error.to_string())?;
    let profile = names::input_profile("one-button").map_err(|error| error.to_string())?;
    let mut mirror = Mirror::new(surface, profile);
    mirror.set_local(
        input::local_from_json(&documents.local.to_string()).map_err(|error| error.to_string())?,
    );
    let host = documents
        .host
        .map(|host| input::host_from_json(&host.to_string()))
        .transpose()
        .map_err(|error| error.to_string())?;
    mirror.set_host(host);
    Ok(mirror)
}

fn press(mirror: &mut Mirror, event: &str) -> Result<(), String> {
    let event = names::input_event(event).map_err(|error| error.to_string())?;
    mirror.press(event);
    Ok(())
}

fn capture(
    mirror: &mut Mirror,
    slug: &'static str,
    expected: &str,
    screens: &mut Vec<StaticScreen>,
) -> Result<(), String> {
    let screen = names::screen_name(mirror.screen());
    if screen != expected {
        return Err(format!(
            "the V4 button sequence reached {screen:?}, expected {expected:?}"
        ));
    }
    mirror.render();
    let mut png = Vec::new();
    mirror
        .frame()
        .write_png(&mut png)
        .map_err(|error| format!("encode {slug}: {error}"))?;
    screens.push(StaticScreen {
        slug,
        screen,
        path: format!("radio-mirror/{SURFACE}-{slug}.png"),
        png,
        lines: mirror.text(),
    });
    Ok(())
}

/// STATUS through PEERS, the menu, VERIFY and display-off with the host
/// attached, then the radio-init fault, each reached with the V4's one button.
pub fn static_screens() -> Result<Vec<StaticScreen>, String> {
    let mut screens = Vec::new();
    let mut mirror = mirror_for(Scenario::Host)?;
    for (index, page) in [
        "status", "power", "radio", "traffic", "identity", "links", "peers",
    ]
    .into_iter()
    .enumerate()
    {
        if index > 0 {
            press(&mut mirror, "a-short")?;
        }
        capture(&mut mirror, page, page, &mut screens)?;
    }
    // Tap wraps to STATUS; hold opens the menu at its first item.
    press(&mut mirror, "a-short")?;
    press(&mut mirror, "a-long")?;
    capture(&mut mirror, "menu", "menu:brightness:0", &mut screens)?;
    // Tap twice to VERIFY, hold to select it.
    press(&mut mirror, "a-short")?;
    press(&mut mirror, "a-short")?;
    press(&mut mirror, "a-long")?;
    capture(&mut mirror, "verify", "verify", &mut screens)?;
    // Any press leaves VERIFY; then the menu again, three taps to DISPLAY OFF.
    press(&mut mirror, "a-short")?;
    press(&mut mirror, "a-long")?;
    for _ in 0..3 {
        press(&mut mirror, "a-short")?;
    }
    press(&mut mirror, "a-long")?;
    capture(&mut mirror, "display-off", "display-off", &mut screens)?;

    let mut faulted = mirror_for(Scenario::Fault)?;
    capture(&mut faulted, "fault", "fault", &mut screens)?;
    Ok(screens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_scenario_is_accepted_by_radio_mirror() {
        for scenario in Scenario::ALL {
            mirror_for(scenario).unwrap_or_else(|error| panic!("{scenario:?}: {error}"));
        }
    }

    #[test]
    fn the_v4_button_reaches_every_static_screen() {
        let screens = static_screens().expect("controller reaches every screen");
        let names = screens
            .iter()
            .map(|screen| screen.screen.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                "status",
                "power",
                "radio",
                "traffic",
                "identity",
                "links",
                "peers",
                "menu:brightness:0",
                "verify",
                "display-off",
                "fault",
            ]
        );
        for screen in &screens {
            assert!(
                screen.png.starts_with(b"\x89PNG\r\n\x1a\n"),
                "{}",
                screen.slug
            );
            assert!(!screen.lines.is_empty(), "{} has no text", screen.slug);
        }
        let mut pngs = screens.iter().map(|screen| &screen.png).collect::<Vec<_>>();
        pngs.sort();
        pngs.dedup();
        assert_eq!(pngs.len(), screens.len(), "two screens render alike");
    }

    #[test]
    fn static_screens_are_deterministic() {
        assert_eq!(static_screens(), static_screens());
        assert_eq!(scenarios_json(), scenarios_json());
    }

    #[test]
    fn the_fault_is_the_v4_firmwares_init_fault() {
        let screens = static_screens().unwrap();
        let fault = screens
            .iter()
            .find(|screen| screen.slug == "fault")
            .unwrap();
        assert!(
            fault.lines.iter().any(|line| line.contains("SX1262 INIT")),
            "{:?}",
            fault.lines
        );
    }
}
