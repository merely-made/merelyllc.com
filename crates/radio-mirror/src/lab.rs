//! Build-time screens for the community-radio page's message path lab.
//!
//! A node's face is the pair of radio-mirror documents a retinue-sim face track
//! carries for it. The lab shows the TRAFFIC page (Ruling 8), reached by
//! pressing the V4's one button through radio-mirror's `Mirror`, which runs the
//! firmware's own `Controller`; the browser reaches it the same way. This
//! module takes the documents as JSON text and knows nothing of the trace.

use radio_mirror::{Mirror, input, names};

use crate::{SURFACE, StaticScreen};

/// The firmware page the lab draws, in radio-mirror's screen names.
pub const LAB_PAGE: &str = "traffic";

/// The most presses the button sequence may take to reach [`LAB_PAGE`]: one
/// to wake a dark panel, then one per page.
const MAX_PRESSES: usize = 8;

/// A node's TRAFFIC page from its face-track documents, as a PNG at `path`
/// with radio-face's text projection. `host_json` is `None` before the node's
/// first entry, when its face is radio-face's default.
pub fn lab_screen(
    path: String,
    local_json: &str,
    host_json: Option<&str>,
) -> Result<StaticScreen, String> {
    let surface = names::surface(SURFACE).map_err(|error| error.to_string())?;
    let profile = names::input_profile("one-button").map_err(|error| error.to_string())?;
    let press = names::input_event("a-short").map_err(|error| error.to_string())?;
    let mut mirror = Mirror::new(surface, profile);
    mirror.set_local(input::local_from_json(local_json).map_err(|error| error.to_string())?);
    mirror.set_host(
        host_json
            .map(input::host_from_json)
            .transpose()
            .map_err(|error| error.to_string())?,
    );
    let mut presses = 0;
    while names::screen_name(mirror.screen()) != LAB_PAGE {
        if presses == MAX_PRESSES {
            return Err(format!(
                "the V4 button did not reach {LAB_PAGE} in {MAX_PRESSES} presses"
            ));
        }
        mirror.press(press);
        presses += 1;
    }
    mirror.render();
    let mut png = Vec::new();
    mirror
        .frame()
        .write_png(&mut png)
        .map_err(|error| format!("encode {path}: {error}"))?;
    Ok(StaticScreen {
        slug: LAB_PAGE,
        screen: LAB_PAGE.to_owned(),
        path,
        png,
        lines: mirror.text(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // One entry's documents from retinue-sim's cold face track at the pinned
    // revision: the county garage after the data frame arrives.
    const LOCAL: &str = r#"{"schema":"radio-mirror.local/v1","tx_frames":2,"rx_frames":8,"last_rx":{"frame_len":83,"rssi_dbm":0,"snr_tenths_db":0},"last_tx":{"sent":{"frame_len":118}}}"#;
    const HOST: &str = r#"{"schema":"radio-mirror.host/v1","valid_for_secs":15,"personality":"retinue","link_count":1,"admitted_links":1,"queue_depth":0,"event":{"source":"local","kind":"info","text":"link up"}}"#;

    #[test]
    fn a_face_track_entry_reaches_the_traffic_page() {
        let screen = lab_screen("lab.png".to_owned(), LOCAL, Some(HOST)).unwrap();
        assert_eq!(screen.screen, "traffic");
        assert!(screen.png.starts_with(b"\x89PNG\r\n\x1a\n"));
        let text = screen.lines.join("\n");
        assert!(text.contains("TRAFFIC"), "{text}");
        assert!(text.contains("link up"), "{text}");
    }

    #[test]
    fn a_default_face_reaches_the_traffic_page_too() {
        let screen = lab_screen(
            "lab.png".to_owned(),
            r#"{"schema":"radio-mirror.local/v1"}"#,
            None,
        )
        .unwrap();
        assert_eq!(screen.screen, "traffic");
    }

    #[test]
    fn lab_screens_are_deterministic() {
        assert_eq!(
            lab_screen("lab.png".to_owned(), LOCAL, Some(HOST)),
            lab_screen("lab.png".to_owned(), LOCAL, Some(HOST))
        );
    }
}
