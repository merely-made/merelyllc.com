//! The favicon is the site's derived face, not a drawing.
//!
//! `assets/favicon.svg` is pictograph's face for the site's canonical address,
//! written by emblem's SVG output in the site's cream, inset on the site's red
//! plate. It uses the detailed level-of-detail arm: for this address the coarse
//! arm is a plain square ring, while the detailed one reads at tab size. This test regenerates it
//! and fails when the committed file differs; run it with
//! `FAVICON_WRITE=1` to rewrite the file after a deliberate change.

use emblem::{Host, Palette, Rgba, SvgOptions, to_svg};

/// The site's canonical address, which the face is keyed on.
const ADDRESS: &str = "https://merelyllc.com/";
/// Above pictograph's level-of-detail threshold: the detailed arm.
const HEIGHT: f32 = 64.0;
/// The face's colour, in every palette entry a face can name.
const CREAM: Rgba = Rgba::new(0xf0, 0xeb, 0xdd, 0xff);
/// The face's ViewBox, which emblem writes.
const FACE_BOX: &str = r#"viewBox="-32 -32 64 64""#;
/// A wider ViewBox, so the face sits inset on the plate.
const ICON_BOX: &str = r#"viewBox="-38 -38 76 76""#;
/// The plate under the face, filling the wider ViewBox.
const PLATE: &str = r##"<rect x="-38" y="-38" width="76" height="76" rx="14" fill="#8b1511"/>"##;

fn favicon() -> String {
    let face = pictograph::derive(ADDRESS.as_bytes()).expect("face");
    let palette = Palette::new([CREAM; 64]).expect("palette");
    let host = Host {
        features: 0,
        height: HEIGHT,
    };
    let svg = to_svg(&face, &palette, host, &SvgOptions::default()).expect("svg");
    let (head, body) = svg.split_once('\n').expect("an svg element line");
    assert!(head.contains(FACE_BOX), "{head}");
    let head = head.replace(FACE_BOX, ICON_BOX);
    format!("{head}\n{PLATE}\n{body}")
}

#[test]
fn the_favicon_is_the_derived_face() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/favicon.svg");
    let want = favicon();
    if std::env::var_os("FAVICON_WRITE").is_some() {
        std::fs::write(path, &want).expect("write favicon");
    }
    let got = std::fs::read_to_string(path).expect("read favicon");
    assert_eq!(got, want, "regenerate with FAVICON_WRITE=1");
}
