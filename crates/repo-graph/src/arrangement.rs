//! The arrangement catalog and the graph work that feeds it.
//!
//! Before the scenograph absorption this came from `arrangements::registry` —
//! a catalog holding both built-in layouts and their metadata. Scenograph's
//! eleven families are enum variants rather than registry entries, so the
//! catalog that remains is this one: the arrangements *this site* offers, with
//! the names it shows for them.
//!
//! The producers below are the other half of that change. Radial rings and
//! stack layers are graph facts, and `sceno`'s contract is that a solver never
//! learns a source's native truth — so the walk happens here and what reaches
//! the score is one number per node.

use std::collections::{HashMap, HashSet, VecDeque};

/// One arrangement this site offers, and how it is described in the picker.
pub struct ArrangementSpec {
    pub id: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
}

/// Every arrangement the site knows about, offered or not.
///
/// The descriptions are the ones the registry carried, except where the site
/// already overrode them for a repository audience.
pub const CATALOG: &[ArrangementSpec] = &[
    ArrangementSpec {
        id: "graph_layout:radial",
        display_name: "Radial",
        description: "Neighborhood rings around the selected node.",
    },
    ArrangementSpec {
        id: "graph_layout:stack",
        display_name: "Stack",
        description: "Directed relations arranged into readable topology layers.",
    },
    ArrangementSpec {
        id: "graph_layout:grid",
        display_name: "Grid",
        description: "Row-major grid with configurable traversal.",
    },
    ArrangementSpec {
        id: "graph_layout:phyllotaxis",
        display_name: "Spiral",
        description: "Fibonacci-family spiral placement. Golden angle by default; configurable for other arm counts.",
    },
    ArrangementSpec {
        id: "graph_layout:timeline",
        display_name: "Timeline",
        description: "Repositories grouped by their last public push date.",
    },
    ArrangementSpec {
        id: "graph_layout:kanban",
        display_name: "Columns",
        description: "Repositories grouped by public project status.",
    },
    ArrangementSpec {
        id: "graph_layout:penrose",
        display_name: "Penrose",
        description: "Aperiodic tiling (P2 kite-dart or P3 rhombus) via Robinson subdivision.",
    },
    ArrangementSpec {
        id: "graph_layout:lsystem",
        display_name: "Fractal",
        description: "Turtle-walked Lindenmayer grammar (Hilbert, Koch, or Dragon).",
    },
    ArrangementSpec {
        id: "graph_layout:semantic_embedding",
        display_name: "Semantic Embedding",
        description: "Places nodes at host-precomputed 2D embeddings (UMAP / t-SNE / PCA supplied by the host's ML pipeline).",
    },
];

pub fn spec(id: &str) -> Option<&'static ArrangementSpec> {
    CATALOG.iter().find(|entry| entry.id == id)
}

/// Undirected adjacency over the given edges, restricted to known nodes.
fn adjacency<'a>(
    node_ids: &HashSet<&'a str>,
    edges: &'a [(String, String)],
) -> HashMap<&'a str, Vec<&'a str>> {
    let mut adjacency: HashMap<&str, Vec<&str>> =
        node_ids.iter().map(|id| (*id, Vec::new())).collect();
    for (source, target) in edges {
        let (source, target) = (source.as_str(), target.as_str());
        if !node_ids.contains(source) || !node_ids.contains(target) || source == target {
            continue;
        }
        adjacency.entry(source).or_default().push(target);
        adjacency.entry(target).or_default().push(source);
    }
    adjacency
}

/// Breadth-first ring index from `focus`; the focus is ring zero.
///
/// Nodes the walk never reaches are absent rather than given a sentinel, so the
/// caller can tell "outside the neighborhood" from "on a distant ring".
pub fn radial_rings(
    node_ids: &HashSet<&str>,
    edges: &[(String, String)],
    focus: &str,
) -> HashMap<String, u32> {
    let mut rings: HashMap<String, u32> = HashMap::new();
    if !node_ids.contains(focus) {
        return rings;
    }
    let adjacency = adjacency(node_ids, edges);
    rings.insert(focus.to_owned(), 0);
    let mut queue: VecDeque<&str> = VecDeque::from([focus]);
    while let Some(id) = queue.pop_front() {
        let next = rings[id] + 1;
        for neighbour in adjacency.get(id).into_iter().flatten() {
            if !rings.contains_key(*neighbour) {
                rings.insert((*neighbour).to_owned(), next);
                queue.push_back(neighbour);
            }
        }
    }
    rings
}

/// Undirected degree plus one, as the angular weight for a weighted ring.
///
/// The plus-one keeps a zero-degree node's slot the same width as an
/// undisclosed one, which is what the solver defaults to.
pub fn degree_weights(
    node_ids: &HashSet<&str>,
    edges: &[(String, String)],
) -> HashMap<String, f32> {
    let adjacency = adjacency(node_ids, edges);
    node_ids
        .iter()
        .map(|id| {
            let degree = adjacency.get(*id).map_or(0, |list| list.len());
            ((*id).to_owned(), (degree + 1) as f32)
        })
        .collect()
}

/// Topological layer per node: the longest path from any root, by Kahn's
/// algorithm over the directed edges.
///
/// Nodes in a cycle have no topological layer at all. They go to one shared
/// overflow layer past the deepest real one rather than being dropped or given
/// an arbitrary break — a repository in a dependency cycle is still a
/// repository the reader wants to see.
pub fn stack_layers(
    node_ids: &HashSet<&str>,
    edges: &[(String, String)],
) -> HashMap<String, i64> {
    let mut outgoing: HashMap<&str, Vec<&str>> =
        node_ids.iter().map(|id| (*id, Vec::new())).collect();
    let mut indegree: HashMap<&str, usize> = node_ids.iter().map(|id| (*id, 0usize)).collect();

    for (source, target) in edges {
        let (source, target) = (source.as_str(), target.as_str());
        if !node_ids.contains(source) || !node_ids.contains(target) {
            continue;
        }
        outgoing.entry(source).or_default().push(target);
        *indegree.entry(target).or_default() += 1;
    }
    for targets in outgoing.values_mut() {
        targets.sort_unstable();
        targets.dedup();
    }
    // Recount after dedup: a repeated edge is one dependency, not two, and a
    // stale indegree would strand its target in the cycle overflow.
    for degree in indegree.values_mut() {
        *degree = 0;
    }
    for targets in outgoing.values() {
        for target in targets {
            *indegree.entry(target).or_default() += 1;
        }
    }

    let mut ready: Vec<&str> = indegree
        .iter()
        .filter_map(|(id, degree)| (*degree == 0).then_some(*id))
        .collect();
    ready.sort_unstable();

    let mut queue: VecDeque<&str> = VecDeque::from(ready);
    let mut depth: HashMap<&str, i64> = node_ids.iter().map(|id| (*id, 0i64)).collect();
    let mut settled: HashSet<&str> = HashSet::with_capacity(node_ids.len());

    while let Some(id) = queue.pop_front() {
        settled.insert(id);
        let current = depth[id];
        let mut newly_ready = Vec::new();
        for target in outgoing.get(id).into_iter().flatten() {
            depth
                .entry(*target)
                .and_modify(|value| *value = (*value).max(current + 1));
            if let Some(degree) = indegree.get_mut(*target) {
                *degree = degree.saturating_sub(1);
                if *degree == 0 {
                    newly_ready.push(*target);
                }
            }
        }
        newly_ready.sort_unstable();
        queue.extend(newly_ready);
    }

    let overflow = depth.values().copied().max().unwrap_or(0) + 1;
    node_ids
        .iter()
        .map(|id| {
            let layer = if settled.contains(*id) {
                depth[*id]
            } else {
                overflow
            };
            ((*id).to_owned(), layer)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids<'a>(list: &'a [&'a str]) -> HashSet<&'a str> {
        list.iter().copied().collect()
    }

    fn edges(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
            .collect()
    }

    #[test]
    fn every_offered_arrangement_is_in_the_catalog() {
        for id in crate::ARRANGEMENT_ORDER {
            assert!(spec(id).is_some(), "{id} has no catalog entry");
        }
    }

    #[test]
    fn rings_count_hops_from_the_focus() {
        let nodes = ids(&["a", "b", "c", "d"]);
        let rings = radial_rings(&nodes, &edges(&[("a", "b"), ("b", "c")]), "a");
        assert_eq!(rings["a"], 0);
        assert_eq!(rings["b"], 1);
        assert_eq!(rings["c"], 2);
        assert!(!rings.contains_key("d"), "an unreachable node has no ring");
    }

    #[test]
    fn layers_follow_the_longest_path_not_the_shortest() {
        // a -> b -> c and a -> c: c must sit past b, or the edge a -> c would
        // draw backwards through the stack.
        let nodes = ids(&["a", "b", "c"]);
        let layers = stack_layers(&nodes, &edges(&[("a", "b"), ("b", "c"), ("a", "c")]));
        assert_eq!(layers["a"], 0);
        assert_eq!(layers["b"], 1);
        assert_eq!(layers["c"], 2);
    }

    #[test]
    fn a_cycle_lands_in_one_overflow_layer_rather_than_vanishing() {
        let nodes = ids(&["a", "b", "c"]);
        let layers = stack_layers(&nodes, &edges(&[("a", "b"), ("b", "c"), ("c", "b")]));
        assert_eq!(layers["a"], 0);
        assert_eq!(
            layers["b"], layers["c"],
            "both cyclic nodes share the overflow layer"
        );
        assert!(layers["b"] > layers["a"]);
    }

    #[test]
    fn a_repeated_edge_is_one_dependency() {
        // Counting it twice would leave the target's indegree above zero
        // forever, stranding it in the cycle overflow.
        let nodes = ids(&["a", "b"]);
        let layers = stack_layers(&nodes, &edges(&[("a", "b"), ("a", "b")]));
        assert_eq!(layers["a"], 0);
        assert_eq!(layers["b"], 1, "b is a real layer, not the overflow");
    }

    #[test]
    fn degree_weight_gives_an_isolated_node_the_default_slot() {
        let nodes = ids(&["a", "b", "lonely"]);
        let weights = degree_weights(&nodes, &edges(&[("a", "b")]));
        assert_eq!(weights["lonely"], 1.0);
        assert_eq!(weights["a"], 2.0);
    }
}
