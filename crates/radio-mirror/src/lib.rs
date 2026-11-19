//! The site's build crate for retinue's `radio-mirror`.
//!
//! - On wasm32 it only links `radio-mirror`, whose own `wasm-bindgen` exports
//!   (`RadioMirror`, `render_screen`, `screen_text`) become the devices page's
//!   runtime. Nothing here reimplements a page, a button or a screen.
//! - Natively it renders the no-script screens and the bench's status
//!   documents at site build time, from the same Murm source, and the
//!   message path lab's no-script screen from a node's face-track documents.
//!
//! The status documents are retinue's receipt fixtures, copied byte for byte
//! into `fixtures/`; `tests/pin.rs` checks them against the resolved source.

#![forbid(unsafe_code)]

#[cfg(target_arch = "wasm32")]
use radio_mirror as _;

#[cfg(not(target_arch = "wasm32"))]
mod bench;
#[cfg(not(target_arch = "wasm32"))]
mod lab;
#[cfg(not(target_arch = "wasm32"))]
pub use bench::{
    SCENARIOS_SCHEMA, Scenario, ScenarioDocuments, StaticScreen, scenario_documents,
    scenarios_json, static_screens,
};
#[cfg(not(target_arch = "wasm32"))]
pub use lab::{LAB_PAGE, lab_screen};

/// Where radio-mirror and radio-face come from.
pub const RETINUE_REPOSITORY: &str = "https://github.com/merely-made/murm";
/// The Murm branch the build follows; `Cargo.toml` names the same one. No
/// lock is tracked (engine split D139), so the page states the branch and a
/// commit's resolved note holds the exact revision a build took.
pub const RETINUE_BRANCH: &str = "main";
/// The Heltec V4's panel, in radio-mirror's surface names.
pub const SURFACE: &str = "oled-128x64";
/// Murm's `crates/radio-mirror/fixtures/receipts-local.json` on [`RETINUE_BRANCH`].
pub const LOCAL_FIXTURE: &str = include_str!("../fixtures/receipts-local.json");
/// Murm's `crates/radio-mirror/fixtures/receipts-host.json` on [`RETINUE_BRANCH`].
pub const HOST_FIXTURE: &str = include_str!("../fixtures/receipts-host.json");
