//! The site's build crate for retinue's `radio-mirror`.
//!
//! - On wasm32 it only links `radio-mirror`, whose own `wasm-bindgen` exports
//!   (`RadioMirror`, `render_screen`, `screen_text`) become the devices page's
//!   runtime. Nothing here reimplements a page, a button or a screen.
//! - Natively it renders the no-script screens and the bench's status
//!   documents at site build time, from the same pinned revision.
//!
//! The status documents are retinue's receipt fixtures at that revision,
//! copied byte for byte into `fixtures/` (checked by `tests/pin.rs`).

#![forbid(unsafe_code)]

#[cfg(target_arch = "wasm32")]
use radio_mirror as _;

#[cfg(not(target_arch = "wasm32"))]
mod bench;
#[cfg(not(target_arch = "wasm32"))]
pub use bench::{
    SCENARIOS_SCHEMA, Scenario, ScenarioDocuments, StaticScreen, scenario_documents,
    scenarios_json, static_screens,
};

/// Where radio-mirror and radio-face come from.
pub const RETINUE_REPOSITORY: &str = "https://github.com/merely-made/retinue";
/// The pinned retinue revision; `Cargo.toml` names the same one.
pub const RETINUE_REVISION: &str = "6aa78fc0d0ddd30e94db59470f30f47e669321dc";
/// The Heltec V4's panel, in radio-mirror's surface names.
pub const SURFACE: &str = "oled-128x64";
/// retinue's `crates/radio-mirror/fixtures/receipts-local.json` at [`RETINUE_REVISION`].
pub const LOCAL_FIXTURE: &str = include_str!("../fixtures/receipts-local.json");
/// retinue's `crates/radio-mirror/fixtures/receipts-host.json` at [`RETINUE_REVISION`].
pub const HOST_FIXTURE: &str = include_str!("../fixtures/receipts-host.json");

/// The revision as the page states it.
pub fn short_revision() -> &'static str {
    &RETINUE_REVISION[..7]
}
