use super::*;

/// The repositories page's first view (Ruling 157): the v2 history byte for
/// byte, the page citing it and the sandbox at their content versions, only
/// the mount script loading up front, both openings, and the frozen scene,
/// matrix and history exactly as the shared readers render them.
pub(super) fn validate_repositories_first_view(
    artifact_root: &Path,
    source_root: &Path,
    authority: &Authority,
    metadata: &PublicMetadataCache,
    repositories: &str,
    errors: &mut Vec<String>,
) -> Option<FrozenFirstView> {
    let history = match repositories_page::site_history(source_root, authority, metadata) {
        Ok(history) => history,
        Err(error) => {
            errors.push(format!("repository history could not be derived: {error}"));
            return None;
        }
    };
    let published = read_text(artifact_root, HOST_HISTORY_FILE, errors);
    let frozen = match repositories_page::frozen_first_view(authority, metadata, &history) {
        Ok(frozen) => frozen,
        Err(error) => {
            errors.push(format!(
                "repository first view could not be derived: {error}"
            ));
            return None;
        }
    };
    if published != frozen.history_json {
        errors.push("repository viewer history differs from the public authority".to_owned());
    }
    let mount = format!(
        "<script type=\"module\" src=\"{}\"></script>",
        repositories_page::graph_sandbox_mount_href()
    );
    if !repositories.contains(&mount) {
        errors.push("repository page is missing its versioned sandbox mount script".to_owned());
    }
    if repositories.contains("<script type=\"module\" src=\"/graph-sandbox.js") {
        errors.push("repository page loads the graph sandbox before interaction".to_owned());
    }
    for (attribute, href) in [
        ("data-sandbox-runtime", crate::viewer::href("viewer.js")),
        (
            "data-sandbox-history-src",
            host_history_href(&frozen.history_json),
        ),
    ] {
        if !repositories.contains(&format!("{attribute}=\"{href}\"")) {
            errors.push(format!("repository page does not cite {href}"));
        }
    }
    let lead = format!(
        "<span data-sandbox-reading-lead=\"\" data-scripted-lead=\"{}\" data-failed-lead=\"{}\">{}</span>",
        escape_html_text(repository_frozen::SCRIPTED_LEAD),
        escape_html_text(repository_frozen::FAILED_LEAD),
        escape_html_text(repository_frozen::NO_SCRIPT_LEAD),
    );
    if !repositories.contains(&lead) {
        errors.push("repository page's frozen view does not carry its openings".to_owned());
    }
    if !repositories.contains(&frozen.scene_html()) {
        errors.push("repository page's frozen scene differs from the shared reader's".to_owned());
    }
    if !repositories.contains(&frozen.grid_html()) {
        errors.push("repository page's frozen matrix differs from the shared reader's".to_owned());
    }
    let revisions = attribute_values(repositories, "data-sandbox-history-revision");
    let expected = frozen
        .history
        .revisions
        .iter()
        .map(|revision| revision.sequence.to_string())
        .collect::<Vec<_>>();
    if revisions != expected {
        errors.push("repository page's history text differs from the v2 history".to_owned());
    }
    Some(frozen)
}

/// The projection proof's published capture, trace and shelfmark: each must
/// be byte for byte what the current authority derives, and together with
/// the published host dataset they must consume and join (Rulings 132-136).
pub(super) fn validate_projection_artifacts(
    artifact_root: &Path,
    authority: &Authority,
    metadata: &PublicMetadataCache,
    host_dataset: &str,
    errors: &mut Vec<String>,
) -> Option<ProjectionProof> {
    let expected = match ProjectionProof::derive(authority, metadata) {
        Ok(proof) => proof,
        Err(error) => {
            errors.push(format!("projection proof could not be derived: {error}"));
            return None;
        }
    };
    let mut published = Vec::new();
    for (name, bytes) in expected.files() {
        match fs::read(artifact_root.join(name)) {
            Ok(actual) if actual == bytes => published.push(actual),
            Ok(actual) => {
                errors.push(format!(
                    "projection artifact {name} differs from its re-derivation"
                ));
                published.push(actual);
            }
            Err(error) => {
                errors.push(format!(
                    "could not read projection artifact {name}: {error}"
                ));
                return None;
            }
        }
    }
    let [capture, trace, shelfmark] = <[Vec<u8>; 3]>::try_from(published).ok()?;
    match ProjectionProof::read(
        PortableProjection {
            capture,
            trace,
            shelfmark,
        },
        host_dataset,
    ) {
        Ok(proof) => Some(proof),
        Err(error) => {
            errors.push(format!("projection artifacts are invalid: {error}"));
            None
        }
    }
}

/// The Mere profile cites each artifact at its content version, and its
/// no-script reading states the trace it was built from.
pub(super) fn validate_projection_page(
    project: &str,
    proof: &ProjectionProof,
    errors: &mut Vec<String>,
) {
    for (attribute, (name, bytes)) in ["data-capture-src", "data-trace-src", "data-shelfmark-src"]
        .into_iter()
        .zip(proof.files())
    {
        let href = ProjectionProof::href(name, bytes);
        if !project.contains(&format!("{attribute}=\"{href}\"")) {
            errors.push(format!(
                "Mere project profile does not cite projection artifact {href}"
            ));
        }
    }
    if !project.contains(&format!("data-dataset-src=\"/{HOST_DATASET_FILE}\"")) {
        errors.push("Mere project profile does not cite the host dataset".to_owned());
    }
    // Ruling 140: the proof's replay comes from the graph runtime, under the
    // repositories page's own version query so both pages share one copy.
    let (runtime, wasm) = crate::pages::repositories::graph_runtime_hrefs();
    for (attribute, href) in [("data-graph-runtime", runtime), ("data-graph-wasm", wasm)] {
        if !project.contains(&format!("{attribute}=\"{href}\"")) {
            errors.push(format!(
                "Mere project profile does not cite the graph runtime at {href}"
            ));
        }
    }
    for (attribute, value) in [
        (
            "data-capture-address",
            &proof.reading.receipt.capture_address,
        ),
        ("data-scene-epoch", &proof.reading.epoch),
    ] {
        if !project.contains(&format!("{attribute}=\"{value}\"")) {
            errors.push(format!(
                "Mere project profile's {attribute} differs from the capture"
            ));
        }
    }
    if project.contains("mere-projection-artifact") || project.contains("projection-scene") {
        errors
            .push("Mere project profile still carries the retired projection artifact".to_owned());
    }
    // Ruling 140: the reading opens one way without a script and another while
    // replay waits for a first interaction. Both travel in the markup.
    let lead = format!(
        "<span data-projection-reading-lead=\"\" data-scripted-lead=\"{}\">{}</span>",
        escape_html_text(crate::projection_proof::SCRIPTED_READING_LEAD),
        escape_html_text(crate::projection_proof::NO_SCRIPT_READING_LEAD),
    );
    if !project.contains(&lead) {
        errors
            .push("Mere project profile's reading does not carry both of its openings".to_owned());
    }
    let steps = attribute_values(project, "data-projection-reading-step");
    let expected = (1..=proof.reading.steps.len())
        .map(|step| step.to_string())
        .collect::<Vec<_>>();
    if steps != expected {
        errors
            .push("Mere project profile's static trace reading differs from the trace".to_owned());
    }
    // S5: a folding step lists the frozen reader's fold group, its heading
    // (with the host's label) and every member it hides.
    for (_, folds) in &proof.folds {
        for fold in folds {
            let heading = format!("<summary>{}</summary>", escape_html_text(&fold.heading()));
            let listed = fold.members.iter().all(|member| {
                project.contains(&format!(
                    "data-projection-instance=\"{}\">{}</li>",
                    member.instance.0,
                    escape_html_text(&member.name)
                ))
            });
            if !project.contains(&heading) || !listed {
                errors.push(
                    "Mere project profile's reading does not list its fold as the frozen group"
                        .to_owned(),
                );
            }
        }
    }
    if !project.contains(&escape_html_text(&proof.summary())) {
        errors
            .push("Mere project profile's projection summary differs from the capture".to_owned());
    }
}

pub(super) fn parse_graph_payload(
    document: &str,
    errors: &mut Vec<String>,
) -> Option<GraphPayload> {
    let marker = "<script id=\"repository-graph-data\" type=\"application/json\">";
    let Some(start) = document.find(marker).map(|offset| offset + marker.len()) else {
        errors.push("repository page is missing graph authority data".to_owned());
        return None;
    };
    let Some(end) = document[start..]
        .find("</script>")
        .map(|offset| start + offset)
    else {
        errors.push("repository graph authority data is not terminated".to_owned());
        return None;
    };
    match serde_json::from_str(&document[start..end]) {
        Ok(payload) => Some(payload),
        Err(error) => {
            errors.push(format!(
                "repository graph authority data is invalid: {error}"
            ));
            None
        }
    }
}

pub(super) fn inline_json<'a>(document: &'a str, id: &str) -> Option<&'a str> {
    let marker = format!("<script id=\"{id}\" type=\"application/json\">");
    let start = document.find(&marker)? + marker.len();
    let end = document[start..].find("</script>")? + start;
    Some(&document[start..end])
}

pub(super) fn attribute_values(document: &str, attribute: &str) -> Vec<String> {
    let pattern = format!(r#"{attribute}="([^"]+)""#);
    Regex::new(&pattern)
        .expect("valid generated attribute regex")
        .captures_iter(document)
        .filter_map(|captures| captures.get(1).map(|value| value.as_str().to_owned()))
        .collect()
}

pub(super) fn validate_static_authority(
    repository_ids: &[String],
    relation_ids: &[String],
    authority: &Authority,
    errors: &mut Vec<String>,
) {
    let expected_repositories = authority
        .repositories
        .repository
        .iter()
        .filter(|repository| repository.public)
        .map(|repository| repository.id.as_str())
        .collect::<BTreeSet<_>>();
    let actual_repositories = repository_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if repository_ids.len() != expected_repositories.len()
        || actual_repositories != expected_repositories
    {
        errors.push("public artifact repository ids differ from authority".to_owned());
    }

    let expected_relations = authority
        .relations
        .relation
        .iter()
        .map(|relation| (relation.id.as_str(), 2_usize))
        .collect::<BTreeMap<_, _>>();
    let mut actual_relations = BTreeMap::new();
    for relation_id in relation_ids {
        *actual_relations.entry(relation_id.as_str()).or_insert(0) += 1;
    }
    if actual_relations != expected_relations {
        errors.push("public artifact relation text ids differ from authority".to_owned());
    }
}

pub(super) fn validate_graph_payload(
    payload: &GraphPayload,
    authority: &Authority,
    errors: &mut Vec<String>,
) {
    if payload.schema != GRAPH_SCHEMA {
        errors.push("repository graph authority schema is not approved".to_owned());
    }
    let expected_nodes = authority
        .repositories
        .repository
        .iter()
        .filter(|repository| repository.public)
        .map(|repository| repository.id.as_str())
        .collect::<BTreeSet<_>>();
    let actual_nodes = payload
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<BTreeSet<_>>();
    if actual_nodes != expected_nodes {
        errors.push("repository graph node ids differ from authority".to_owned());
    }

    let expected_edges = authority
        .relations
        .relation
        .iter()
        .map(|edge| edge.id.as_str())
        .collect::<BTreeSet<_>>();
    let actual_edges = payload
        .edges
        .iter()
        .map(|edge| edge.id.as_str())
        .collect::<BTreeSet<_>>();
    if actual_edges != expected_edges {
        errors.push("repository graph edge ids differ from authority".to_owned());
    }
    if payload.edges.iter().any(|edge| {
        !actual_nodes.contains(edge.source.as_str()) || !actual_nodes.contains(edge.target.as_str())
    }) {
        errors.push("repository graph contains an edge with an unknown endpoint".to_owned());
    }
}

pub(super) fn validate_project_authority(
    project_ids: &[String],
    relation_ids: &[String],
    authority: &Authority,
    errors: &mut Vec<String>,
) {
    let expected_projects = authority
        .repositories
        .repository
        .iter()
        .filter(|repository| repository.public)
        .map(|repository| (repository.id.as_str(), 1_usize))
        .collect::<BTreeMap<_, _>>();
    let mut actual_projects = BTreeMap::new();
    for project_id in project_ids {
        *actual_projects.entry(project_id.as_str()).or_insert(0) += 1;
    }
    if actual_projects != expected_projects {
        errors.push("project profile ids differ from repository authority".to_owned());
    }

    let expected_relations = authority
        .relations
        .relation
        .iter()
        .map(|relation| (relation.id.as_str(), 2_usize))
        .collect::<BTreeMap<_, _>>();
    let mut actual_relations = BTreeMap::new();
    for relation_id in relation_ids {
        *actual_relations.entry(relation_id.as_str()).or_insert(0) += 1;
    }
    if actual_relations != expected_relations {
        errors.push("project profile relation ids differ from relation authority".to_owned());
    }
}
