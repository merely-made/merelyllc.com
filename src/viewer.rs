//! Immutable browser-qualified Graphshell viewer; the site owns data and mounting.
use sha2::{Digest, Sha256};
/// The qualified application binaries: the only files admitted past the public-bytes scan's parser-constant rules.
pub const QUALIFIED_WASM: &[(&str, &str)] = &[
    (
        "pkg/webgl2/graphshell_web_bg.wasm",
        "61daf58315098d57e26b2e6555f7a3b4b813c6e42b79ab254a052a05951ed4cc",
    ),
    (
        "pkg/physics/graphshell_physics_bg.wasm",
        "eae38540ca1a55f7989c3bf968e06210dccc9ff0a6e174e87f7de9528afe6f3b",
    ),
];
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
        "pkg/webgl2/graphshell_web.js",
        include_bytes!("../assets/graphshell/pkg/webgl2/graphshell_web.js"),
    ),
    (
        "pkg/webgl2/graphshell_web_bg.wasm",
        include_bytes!("../assets/graphshell/pkg/webgl2/graphshell_web_bg.wasm"),
    ),
    (
        "pkg/physics/graphshell_physics.js",
        include_bytes!("../assets/graphshell/pkg/physics/graphshell_physics.js"),
    ),
    (
        "pkg/physics/graphshell_physics_bg.wasm",
        include_bytes!("../assets/graphshell/pkg/physics/graphshell_physics_bg.wasm"),
    ),
    (
        "pkg/physics/worker.js",
        include_bytes!("../assets/graphshell/pkg/physics/worker.js"),
    ),
    (
        "pkg/webgl2/snippets/genet-presenter-efb9887771c95a54/inline0.js",
        include_bytes!(
            "../assets/graphshell/pkg/webgl2/snippets/genet-presenter-efb9887771c95a54/inline0.js"
        ),
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
        format!(
            "graphshell/{}",
            &base16ct::lower::encode_string(&digest.finalize())[..12]
        )
    })
}
pub fn href(file: &str) -> String {
    format!("/{}/{file}", prefix())
}
