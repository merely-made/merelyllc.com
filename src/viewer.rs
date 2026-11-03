//! Immutable browser-qualified Graphshell viewer; the site owns data and mounting.
use sha2::{Digest, Sha256};
pub const WASM_SHA256: &str = "cbe148bce26990274974a23bedcc6b32b80611deaa3b4766a2aac6e0568c255e";
pub const FILES: &[(&str, &[u8])] = &[
    (
        "viewer.js",
        include_bytes!("../assets/graphshell/viewer.js"),
    ),
    ("mount.js", include_bytes!("../assets/graphshell/mount.js")),
    (
        "styles.css",
        include_bytes!("../assets/graphshell/styles.css"),
    ),
    (
        "pkg/graphshell_web.js",
        include_bytes!("../assets/graphshell/pkg/graphshell_web.js"),
    ),
    (
        "pkg/graphshell_web_bg.wasm",
        include_bytes!("../assets/graphshell/pkg/graphshell_web_bg.wasm"),
    ),
    (
        "provenance.json",
        include_bytes!("../assets/graphshell/provenance.json"),
    ),
];
pub fn prefix() -> &'static str {
    static PREFIX: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    PREFIX.get_or_init(|| {
        let mut digest = Sha256::new();
        for (name, bytes) in FILES {
            digest.update(name.as_bytes());
            digest.update(bytes);
        }
        format!("graphshell/{}", &format!("{:x}", digest.finalize())[..12])
    })
}
pub fn href(file: &str) -> String {
    format!("/{}/{file}", prefix())
}
