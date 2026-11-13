//! retinue-sim's generated route traces and face tracks, committed under
//! `content/retinue-traces/` with their provenance (Ruling 127).
//!
//! The traces are generated outside the site, at the pinned retinue revision
//! the provenance names, and stored byte for byte. The site links neither
//! `retinue` nor `retinue-sim`: it reads the files. The reader types below hold
//! only the fields the message path lab states, so the schema ids are checked
//! exactly and anything else in a v1 document is ignored rather than mirrored.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

/// Where the traces live in the source tree.
pub const TRACE_DIRECTORY: &str = "content/retinue-traces";
/// The provenance manifest, inside [`TRACE_DIRECTORY`].
pub const PROVENANCE_FILE: &str = "provenance.toml";
pub const PROVENANCE_SCHEMA: &str = "mer3ly.retinue-trace-provenance/v1";
pub const ROUTE_SCHEMA: &str = "retinue-sim.route-trace/v1";
pub const FACE_SCHEMA: &str = "retinue-sim.face-track/v1";
/// radio-mirror's document schemas, which each face-track entry carries.
pub const LOCAL_DOCUMENT_SCHEMA: &str = "radio-mirror.local/v1";
pub const HOST_DOCUMENT_SCHEMA: &str = "radio-mirror.host/v1";
/// Where the traces are published in the site artifact.
pub const ARTIFACT_DIRECTORY: &str = "retinue-traces";
/// The scenarios the lab reads, in the order it offers them.
pub const SCENARIO_IDS: [&str; 2] = ["cold", "warm"];

// This is the repository locator recorded when the immutable traces were
// generated. Public navigation uses Murm; the custody record keeps its bytes.
const RETINUE_REPOSITORY: &str = "https://github.com/merely-made/retinue";
const SIM_CRATE: &str = "retinue-sim";

const EMBEDDED_PROVENANCE: &str = include_str!("../content/retinue-traces/provenance.toml");
const EMBEDDED_FILES: [(&str, &[u8]); 4] = [
    (
        "cold.route-trace.json",
        include_bytes!("../content/retinue-traces/cold.route-trace.json"),
    ),
    (
        "cold.face-track.json",
        include_bytes!("../content/retinue-traces/cold.face-track.json"),
    ),
    (
        "warm.route-trace.json",
        include_bytes!("../content/retinue-traces/warm.route-trace.json"),
    ),
    (
        "warm.face-track.json",
        include_bytes!("../content/retinue-traces/warm.face-track.json"),
    ),
];

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub schema: String,
    pub repository: String,
    pub revision: String,
    #[serde(rename = "crate")]
    pub sim_crate: String,
    pub crate_license: String,
    pub trace: Vec<TraceProvenance>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceProvenance {
    pub id: String,
    pub route: String,
    pub route_schema: String,
    pub route_command: String,
    pub route_sha256: String,
    pub face: String,
    pub face_schema: String,
    pub face_command: String,
    pub face_sha256: String,
}

impl Provenance {
    pub fn short_revision(&self) -> &str {
        &self.revision[..7]
    }
}

/// A route trace, as far as the lab reads it.
#[derive(Clone, Debug, Deserialize)]
pub struct RouteTrace {
    pub schema: String,
    pub scenario: String,
    pub timing: TraceTiming,
    pub nodes: Vec<TraceNode>,
    pub edges: Vec<TraceEdge>,
    pub cuts: Vec<TraceCut>,
    pub sends: Vec<TraceSend>,
    pub events: Vec<TraceEvent>,
    pub messages: Vec<TraceMessage>,
}

/// Simulated milliseconds.
#[derive(Clone, Debug, Deserialize)]
pub struct TraceTiming {
    pub announce_interval: u64,
    pub end: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TraceNode {
    pub name: String,
    pub destination: String,
    pub transit: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct TraceEdge {
    pub a: String,
    pub b: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TraceCut {
    pub at: u64,
    pub a: String,
    pub b: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TraceSend {
    pub at: u64,
    pub from: String,
    pub to: String,
    pub payload: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TraceEvent {
    Cut {
        t: u64,
        a: String,
        b: String,
    },
    Send {
        t: u64,
        message: u32,
        from: String,
        to: String,
        via: Option<String>,
        hops: Option<u8>,
    },
    SendRefused {
        t: u64,
        message: u32,
        from: String,
        to: String,
        reason: String,
    },
    Transmit {
        t: u64,
        frame: u32,
        node: String,
        origin: String,
        packet: TracePacket,
        heard_by: Vec<String>,
        blocked: Vec<String>,
    },
    Receive {
        t: u64,
        frame: u32,
        node: String,
    },
    LinkRequestExpired {
        t: u64,
        node: String,
        message: Option<u32>,
    },
    Delivered {
        t: u64,
        message: u32,
        node: String,
        path: Vec<String>,
    },
}

impl TraceEvent {
    pub fn t(&self) -> u64 {
        match self {
            Self::Cut { t, .. }
            | Self::Send { t, .. }
            | Self::SendRefused { t, .. }
            | Self::Transmit { t, .. }
            | Self::Receive { t, .. }
            | Self::LinkRequestExpired { t, .. }
            | Self::Delivered { t, .. } => *t,
        }
    }

    /// The node whose state the event carries, for the events a face track follows.
    pub fn state_node(&self) -> Option<&str> {
        match self {
            Self::Transmit { node, .. }
            | Self::Receive { node, .. }
            | Self::LinkRequestExpired { node, .. } => Some(node),
            _ => None,
        }
    }

    /// Every node name the event mentions.
    fn named_nodes(&self) -> Vec<&str> {
        let mut names = Vec::new();
        match self {
            Self::Cut { a, b, .. } => names.extend([a.as_str(), b.as_str()]),
            Self::Send { from, to, via, .. } => {
                names.extend([from.as_str(), to.as_str()]);
                names.extend(via.as_deref());
            }
            Self::SendRefused { from, to, .. } => names.extend([from.as_str(), to.as_str()]),
            Self::Transmit {
                node,
                packet,
                heard_by,
                blocked,
                ..
            } => {
                names.push(node.as_str());
                names.extend(packet.destination_node.as_deref());
                names.extend(packet.transport.as_deref());
                names.extend(heard_by.iter().map(String::as_str));
                names.extend(blocked.iter().map(String::as_str));
            }
            Self::Receive { node, .. } | Self::LinkRequestExpired { node, .. } => {
                names.push(node.as_str());
            }
            Self::Delivered { node, path, .. } => {
                names.push(node.as_str());
                names.extend(path.iter().map(String::as_str));
            }
        }
        names
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct TracePacket {
    pub packet_type: String,
    pub hops: u8,
    pub destination_node: Option<String>,
    pub transport: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TraceMessage {
    pub id: u32,
    pub from: String,
    pub to: String,
    pub sent_at: u64,
    pub delivered: Option<TraceDelivery>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TraceDelivery {
    pub t: u64,
    pub path: Vec<String>,
}

/// A face track. The two documents stay JSON values: they are handed to
/// radio-mirror unchanged.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaceTrack {
    pub schema: String,
    pub route_trace: String,
    pub trace_sha256: String,
    pub scenario: String,
    pub entries: Vec<FaceEntry>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaceEntry {
    pub event: usize,
    pub t: u64,
    pub node: String,
    pub local: serde_json::Value,
    pub host: serde_json::Value,
}

/// One scenario's pair of files, checked against each other and the provenance.
#[derive(Clone, Debug)]
pub struct TraceScenario {
    pub id: String,
    pub route_file: String,
    pub face_file: String,
    pub route_bytes: Vec<u8>,
    pub face_bytes: Vec<u8>,
    pub trace: RouteTrace,
    pub faces: FaceTrack,
}

impl TraceScenario {
    /// The SHA-256 of the route trace's canonical JSON, as its face track names it.
    pub fn trace_sha256(&self) -> &str {
        &self.faces.trace_sha256
    }

    /// `node`'s face after event `step`: its latest entry at or before it.
    pub fn face_at(&self, node: &str, step: usize) -> Option<&FaceEntry> {
        self.faces
            .entries
            .iter()
            .rev()
            .find(|entry| entry.node == node && entry.event <= step)
    }
}

#[derive(Clone, Debug)]
pub struct TraceSet {
    pub provenance: Provenance,
    pub scenarios: Vec<TraceScenario>,
}

impl TraceSet {
    /// The traces compiled into the site build.
    pub fn embedded() -> Result<Self, Vec<String>> {
        Self::parse(EMBEDDED_PROVENANCE, |name| {
            EMBEDDED_FILES
                .iter()
                .find(|(file, _)| *file == name)
                .map(|(_, bytes)| bytes.to_vec())
        })
    }

    /// The traces under `root`'s [`TRACE_DIRECTORY`]. The directory holds the
    /// provenance and the files it names, and nothing else.
    pub fn load(root: &Path) -> Result<Self, Vec<String>> {
        let directory = root.join(TRACE_DIRECTORY);
        let provenance = fs::read_to_string(directory.join(PROVENANCE_FILE)).map_err(|error| {
            vec![format!(
                "could not read {TRACE_DIRECTORY}/{PROVENANCE_FILE}: {error}"
            )]
        })?;
        let set = Self::parse(&provenance, |name| fs::read(directory.join(name)).ok())?;
        let mut expected = set.file_names();
        expected.insert(PROVENANCE_FILE.to_owned());
        let present = fs::read_dir(&directory)
            .map_err(|error| vec![format!("could not list {TRACE_DIRECTORY}: {error}")])?
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect::<BTreeSet<_>>();
        if present != expected {
            return Err(vec![format!(
                "{TRACE_DIRECTORY} holds files its provenance does not name: {:?}",
                present.difference(&expected).collect::<Vec<_>>()
            )]);
        }
        Ok(set)
    }

    /// Parses and checks a provenance manifest and the files it names.
    pub fn parse(
        provenance: &str,
        read: impl Fn(&str) -> Option<Vec<u8>>,
    ) -> Result<Self, Vec<String>> {
        let provenance: Provenance = toml::from_str(provenance)
            .map_err(|error| vec![format!("trace provenance is invalid: {error}")])?;
        let mut errors = Vec::new();
        check_provenance(&provenance, &mut errors);
        let mut scenarios = Vec::new();
        for entry in &provenance.trace {
            if let Some(scenario) = check_scenario(entry, &read, &mut errors) {
                scenarios.push(scenario);
            }
        }
        if errors.is_empty() {
            check_shared_topology(&scenarios, &mut errors);
        }
        if errors.is_empty() {
            Ok(Self {
                provenance,
                scenarios,
            })
        } else {
            Err(errors)
        }
    }

    pub fn scenario(&self, id: &str) -> Option<&TraceScenario> {
        self.scenarios.iter().find(|scenario| scenario.id == id)
    }

    /// The trace file names, as published under [`ARTIFACT_DIRECTORY`].
    pub fn file_names(&self) -> BTreeSet<String> {
        self.scenarios
            .iter()
            .flat_map(|scenario| [scenario.route_file.clone(), scenario.face_file.clone()])
            .collect()
    }

    /// Every published file: its artifact path and its bytes.
    pub fn artifact_files(&self) -> Vec<(String, &[u8])> {
        self.scenarios
            .iter()
            .flat_map(|scenario| {
                [
                    (&scenario.route_file, scenario.route_bytes.as_slice()),
                    (&scenario.face_file, scenario.face_bytes.as_slice()),
                ]
            })
            .map(|(name, bytes)| (format!("{ARTIFACT_DIRECTORY}/{name}"), bytes))
            .collect()
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    base16ct::lower::encode_string(&Sha256::digest(bytes))
}

mod validation;

use validation::{check_provenance, check_scenario, check_shared_topology};

pub use validation::same_edge;
