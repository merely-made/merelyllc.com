use super::*;

pub fn validate_public_artifact(
    artifact_root: &Path,
    source_root: &Path,
    authority: &Authority,
    metadata: &PublicMetadataCache,
    showcases: &ShowcaseManifest,
    devices: &DeviceCatalog,
    metadata_path: &Path,
) -> Result<ArtifactReceipt, Vec<String>> {
    let mut errors = Vec::new();
    let mut files = Vec::new();
    if let Err(error) = collect_files(artifact_root, artifact_root, &mut files) {
        return Err(vec![format!("could not read public artifact: {error}")]);
    }
    files.sort();

    let actual_paths = files
        .iter()
        .map(|path| artifact_relative_path(artifact_root, path))
        .collect::<BTreeSet<_>>();
    let mut expected_paths = BASE_FILES
        .iter()
        .map(|path| (*path).to_owned())
        .collect::<BTreeSet<_>>();
    expected_paths.extend(
        crate::viewer::FILES
            .iter()
            .map(|(name, _)| format!("{}/{name}", crate::viewer::prefix())),
    );
    for repository in authority
        .repositories
        .repository
        .iter()
        .filter(|repository| repository.public)
    {
        expected_paths.insert(format!("projects/{}/index.html", repository.id));
    }
    for device in devices.ordered() {
        expected_paths.insert(format!("devices/{}/index.html", device.id));
    }
    let radio_screens = match mer3ly_radio_mirror::static_screens() {
        Ok(screens) => screens,
        Err(error) => {
            errors.push(format!(
                "radio-mirror could not render the V4 screens: {error}"
            ));
            Vec::new()
        }
    };
    for screen in &radio_screens {
        expected_paths.insert(screen.path.clone());
    }
    // The lab's traces are checked from the source tree, against their
    // provenance, then compared byte for byte with the published copies.
    let traces = match TraceSet::load(source_root) {
        Ok(traces) => Some(traces),
        Err(trace_errors) => {
            errors.extend(
                trace_errors
                    .into_iter()
                    .map(|error| format!("retinue traces: {error}")),
            );
            None
        }
    };
    let lab_screen = traces.as_ref().and_then(|traces| {
        message_path::static_screen(traces)
            .map_err(|error| errors.push(format!("message path screen: {error}")))
            .ok()
    });
    if let Some(traces) = &traces {
        for (path, _) in traces.artifact_files() {
            expected_paths.insert(path);
        }
    }
    if let Some(screen) = &lab_screen {
        expected_paths.insert(screen.path.clone());
    }
    for showcase in &showcases.showcase {
        expected_paths.insert(showcase.image.clone());
        for extra in &showcase.images {
            expected_paths.insert(extra.image.clone());
        }
    }
    if actual_paths != expected_paths {
        errors.push("public artifact file set differs from the approved shape".to_owned());
    }

    let allowed_github_slugs = authority
        .repositories
        .repository
        .iter()
        .filter(|repository| repository.public)
        .map(|repository| repository.github_slug.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();

    let mut file_receipts = Vec::new();
    let mut total_bytes = 0_u64;
    for path in &files {
        let relative = artifact_relative_path(artifact_root, path);
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                errors.push(format!(
                    "could not read public artifact file {relative}: {error}"
                ));
                continue;
            }
        };
        total_bytes = total_bytes.saturating_add(bytes.len() as u64);
        if is_scannable(&relative) {
            scan_public_bytes(&relative, &bytes, &allowed_github_slugs, &mut errors);
        }
        file_receipts.push(ArtifactFileReceipt {
            path: relative,
            bytes: bytes.len() as u64,
            sha256: sha256(&bytes),
        });
    }

    validate_static_assets(artifact_root, source_root, &mut errors);

    if let Some(traces) = &traces {
        for name in traces.file_names() {
            validate_copied_asset(
                artifact_root,
                source_root,
                &format!("{}/{name}", crate::retinue_traces::ARTIFACT_DIRECTORY),
                &format!("{TRACE_DIRECTORY}/{name}"),
                &format!("retinue trace {name}"),
                &mut errors,
            );
        }
    }
    for screen in radio_screens.iter().chain(&lab_screen) {
        match fs::read(artifact_root.join(&screen.path)) {
            Ok(bytes) if bytes == screen.png => {}
            Ok(_) => errors.push(format!(
                "radio-mirror screen {} differs from its pinned rendering",
                screen.path
            )),
            Err(_) => errors.push(format!("radio-mirror screen {} is missing", screen.path)),
        }
    }

    let home = read_text(artifact_root, "index.html", &mut errors);
    let radio = read_text(artifact_root, "radio.html", &mut errors);
    let repositories = read_text(artifact_root, "repos/index.html", &mut errors);
    let host_dataset = read_text(artifact_root, HOST_DATASET_FILE, &mut errors);
    if let Err(error) = validate_repository_host_dataset(&host_dataset, authority, metadata) {
        errors.push(error);
    }
    // The roadmap is rebuilt from the same registry and fetched results; the
    // published bytes must match exactly.
    let roadmap = read_text(artifact_root, "roadmap/index.html", &mut errors);
    let roadmap_dataset = read_text(artifact_root, ROADMAP_DATASET_FILE, &mut errors);
    match crate::pages::roadmap::page(source_root) {
        Ok(expected) => {
            if roadmap != expected.html {
                errors.push("roadmap page differs from the registry it is built from".to_owned());
            }
            if roadmap_dataset != expected.dataset_json {
                errors
                    .push("roadmap dataset differs from the registry it is built from".to_owned());
            }
        }
        Err(error) => errors.push(format!("roadmap could not be rebuilt: {error}")),
    }
    validate_fixed_metadata(
        &roadmap,
        "https://merelyllc.com/roadmap/",
        DEFAULT_SOCIAL_IMAGE_URL,
        "image/jpeg",
        DEFAULT_SOCIAL_IMAGE_ALT,
        &mut errors,
    );
    let device_index = read_text(artifact_root, "devices/index.html", &mut errors);
    let robots = read_text(artifact_root, "robots.txt", &mut errors);
    let sitemap = read_text(artifact_root, "sitemap.xml", &mut errors);
    if !home.starts_with("<!doctype html>") || !radio.starts_with("<!doctype html>") {
        errors.push("home or community-radio output is not a complete HTML document".to_owned());
    }
    validate_fixed_metadata(
        &home,
        "https://merelyllc.com/",
        DEFAULT_SOCIAL_IMAGE_URL,
        "image/jpeg",
        DEFAULT_SOCIAL_IMAGE_ALT,
        &mut errors,
    );
    validate_fixed_metadata(
        &radio,
        "https://merelyllc.com/radio.html",
        DEFAULT_SOCIAL_IMAGE_URL,
        "image/jpeg",
        DEFAULT_SOCIAL_IMAGE_ALT,
        &mut errors,
    );
    if !radio.contains(&format!(
        "<script type=\"module\" src=\"{}\"></script>",
        radio_page::message_path_lab_href()
    )) {
        errors.push(
            "community-radio output is missing its versioned message path lab module".to_owned(),
        );
    }
    if !radio.contains("data-message-path-lab") {
        errors.push("community-radio output is missing its message path lab landmark".to_owned());
    }
    if let (Some(traces), Some(screen)) = (&traces, &lab_screen) {
        validate_message_path(&radio, traces, screen, &mut errors);
    }
    validate_fixed_metadata(
        &repositories,
        "https://merelyllc.com/repos/",
        DEFAULT_SOCIAL_IMAGE_URL,
        "image/jpeg",
        DEFAULT_SOCIAL_IMAGE_ALT,
        &mut errors,
    );
    validate_fixed_metadata(
        &device_index,
        "https://merelyllc.com/devices/",
        DEFAULT_SOCIAL_IMAGE_URL,
        "image/jpeg",
        DEFAULT_SOCIAL_IMAGE_ALT,
        &mut errors,
    );
    if !device_index.contains("href=\"/devices.css?v=") {
        errors.push("device index is missing its content-addressed stylesheet".to_owned());
    }
    if !home.contains("href=\"/devices/\"") {
        errors.push("home page is missing the hardware catalog link".to_owned());
    }
    if robots != ROBOTS_TXT {
        errors.push("robots policy differs from the approved public policy".to_owned());
    }
    let expected_sitemap_urls = canonical_urls_from_authority_and_devices(authority, devices);
    let sitemap_urls = validate_sitemap(&sitemap, &expected_sitemap_urls, &mut errors);

    let repository_ids = attribute_values(&repositories, "data-repository-id");
    let relation_ids = attribute_values(&repositories, "data-relation-id");
    validate_static_authority(&repository_ids, &relation_ids, authority, &mut errors);
    let repository_count = repository_ids.len();
    let relation_text_projections = relation_ids.len();
    let frozen = validate_repositories_first_view(
        artifact_root,
        source_root,
        authority,
        metadata,
        &repositories,
        &mut errors,
    );
    if !repositories.contains("data-graph-sandbox") || !repositories.contains("data-viewer-style") {
        errors.push(
            "repository page is missing the Graphshell viewer landmark or asset reference"
                .to_owned(),
        );
    }

    let mut project_ids = Vec::new();
    let mut project_relation_ids = Vec::new();
    let mut project_social_previews = 0;
    let mut project_structured_records = 0;
    let projection = validate_projection_artifacts(
        artifact_root,
        authority,
        metadata,
        &host_dataset,
        &mut errors,
    );
    for repository in authority
        .repositories
        .repository
        .iter()
        .filter(|repository| repository.public)
    {
        let relative = format!("projects/{}/index.html", repository.id);
        let project = read_text(artifact_root, &relative, &mut errors);
        if !project.starts_with("<!doctype html>") {
            errors.push(format!(
                "project profile {} is not a complete HTML document",
                repository.id
            ));
        }
        project_ids.extend(attribute_values(&project, "data-project-id"));
        project_relation_ids.extend(attribute_values(&project, "data-relation-id"));
        let canonical = format!("https://merelyllc.com/projects/{}/", repository.id);
        let showcase = showcases.for_repository(&repository.id);
        let social_image = showcase.map_or(DEFAULT_SOCIAL_IMAGE_URL.to_owned(), |showcase| {
            format!("https://merelyllc.com/{}", showcase.image)
        });
        let social_type = if showcase.is_some() {
            "image/png"
        } else {
            "image/jpeg"
        };
        let social_alt =
            showcase.map_or(DEFAULT_SOCIAL_IMAGE_ALT, |showcase| showcase.alt.as_str());
        let expected_social = ExpectedSocialMetadata {
            canonical: &canonical,
            image_url: &social_image,
            image_type: social_type,
            image_alt: social_alt,
        };
        let (social_valid, structured_valid) = validate_project_metadata(
            &project,
            repository,
            metadata,
            &expected_social,
            &mut errors,
        );
        if social_valid {
            project_social_previews += 1;
        }
        if structured_valid {
            project_structured_records += 1;
        }
        let profile_href = format!("/projects/{}/", repository.id);
        if !home.contains(&profile_href) && showcases.for_repository(&repository.id).is_some() {
            errors.push(format!(
                "home page is missing showcased project profile {}",
                repository.id
            ));
        }
        if !repositories.contains(&format!("data-project-href=\"{profile_href}\"")) {
            errors.push(format!(
                "repository page is missing project profile link {}",
                repository.id
            ));
        }
        if let Some(showcase) = showcases.for_repository(&repository.id) {
            let mut missing = !project.contains(&format!("src=\"/{}\"", showcase.image))
                || !project.contains(&showcase.source_url)
                || !project.contains(&showcase.alt);
            for extra in &showcase.images {
                missing = missing
                    || !project.contains(&format!("src=\"/{}\"", extra.image))
                    || !project.contains(&extra.source_url)
                    || !project.contains(&extra.alt);
            }
            if missing {
                errors.push(format!(
                    "project profile {} is missing approved showcase evidence",
                    repository.id
                ));
            }
        }
        if repository.id == "mere" {
            if !project.contains("data-projection-proof")
                || !project.contains("<script type=\"module\" src=\"/projection-proof.js?v=")
            {
                errors.push(
                    "Mere project profile is missing its portable projection proof".to_owned(),
                );
            }
            if let Some(proof) = &projection {
                validate_projection_page(&project, proof, &mut errors);
            }
        } else if project.contains("data-projection-proof")
            || project.contains("/projection-proof.js?v=")
        {
            errors.push(format!(
                "project profile {} unexpectedly includes the Mere projection proof",
                repository.id
            ));
        }
    }
    validate_project_authority(&project_ids, &project_relation_ids, authority, &mut errors);

    let mut device_ids = Vec::new();
    let mut device_structured_records = 0;
    let mut sellable_devices = 0;
    for device in devices.ordered() {
        let relative = format!("devices/{}/index.html", device.id);
        let document = read_text(artifact_root, &relative, &mut errors);
        if !document.starts_with("<!doctype html>") {
            errors.push(format!(
                "device profile {} is not a complete HTML document",
                device.id
            ));
        }
        if !document.contains("href=\"/devices.css?v=") {
            errors.push(format!(
                "device profile {} is missing its content-addressed stylesheet",
                device.id
            ));
        }
        if device.id == "v4-desktop-radio" {
            validate_radio_bench(&document, &radio_screens, &mut errors);
        }
        if device.id != "v4-desktop-radio" && document.contains("data-radio-simulator") {
            errors.push(format!(
                "device profile {} unexpectedly includes the V4 radio simulator",
                device.id
            ));
        }
        let ids = attribute_values(&document, "data-device-id");
        if ids != [device.id.clone()] {
            errors.push(format!(
                "device profile {} does not project its exact authority id",
                device.id
            ));
        }
        device_ids.extend(ids);
        let canonical = format!("https://merelyllc.com/devices/{}/", device.id);
        let expected_social = ExpectedSocialMetadata {
            canonical: &canonical,
            image_url: DEFAULT_SOCIAL_IMAGE_URL,
            image_type: "image/jpeg",
            image_alt: DEFAULT_SOCIAL_IMAGE_ALT,
        };
        if validate_device_metadata(&document, device, &expected_social, &mut errors) {
            device_structured_records += 1;
        }
        if !device_index.contains(&format!("data-device-id=\"{}\"", device.id)) {
            errors.push(format!(
                "device index is missing catalog record {}",
                device.id
            ));
        }
        let source_url = format!(
            "{}/blob/main/{}",
            device.source_repository, device.source_document
        );
        if !document.contains(&source_url) {
            errors.push(format!(
                "device profile {} is missing its public evidence link",
                device.id
            ));
        }
        match (&device.status, &device.sale.purchase_url) {
            (DeviceStatus::Sellable, Some(url)) => {
                sellable_devices += 1;
                if !document.contains(&format!("href=\"{url}\""))
                    || !document.contains("class=\"button button-primary purchase-link\"")
                {
                    errors.push(format!(
                        "sellable device profile {} is missing its final purchase link",
                        device.id
                    ));
                }
            }
            _ => {
                if !document.contains("data-purchase-status=\"unavailable\"")
                    || document.contains("class=\"button button-primary purchase-link\"")
                {
                    errors.push(format!(
                        "non-sellable device profile {} has an invalid purchase control",
                        device.id
                    ));
                }
            }
        }
    }
    let expected_device_ids = devices
        .ordered()
        .into_iter()
        .map(|device| device.id.clone())
        .collect::<Vec<_>>();
    if device_ids != expected_device_ids {
        errors.push("device profile ids differ from catalog authority".to_owned());
    }

    for showcase in &showcases.showcase {
        let images = std::iter::once(showcase.image.as_str())
            .chain(showcase.images.iter().map(|extra| extra.image.as_str()));
        for image in images {
            let artifact_image = artifact_root.join(image);
            let source_image = source_root.join("assets").join(image);
            match (fs::read(&artifact_image), fs::read(&source_image)) {
                (Ok(artifact_bytes), Ok(source_bytes)) if artifact_bytes == source_bytes => {}
                (Ok(_), Ok(_)) => errors.push(format!(
                    "showcase {} artifact image differs from its approved source",
                    showcase.repository
                )),
                _ => errors.push(format!(
                    "showcase {} artifact or approved source image is missing",
                    showcase.repository
                )),
            }
        }
    }
    let timestamp = format!(
        "{} {} UTC",
        &metadata.generated_at_utc[..10],
        &metadata.generated_at_utc[11..16]
    );
    let refresh_statement = format!("Refreshed {timestamp}.");
    if !repositories.contains(&refresh_statement) {
        errors.push("repository page does not display the validated metadata timestamp".to_owned());
    }

    let graph = parse_graph_payload(&repositories, &mut errors);
    let graph_nodes = graph.as_ref().map_or(0, |payload| payload.nodes.len());
    let graph_edges = graph.as_ref().map_or(0, |payload| payload.edges.len());
    if let Some(payload) = &graph {
        validate_graph_payload(payload, authority, &mut errors);
    }

    let wasm_path = artifact_root.join("mer3ly_repo_graph_bg.wasm");
    match fs::read(&wasm_path) {
        Ok(bytes) if bytes.starts_with(b"\0asm") => {}
        Ok(_) => errors.push("repository graph Wasm has an invalid magic header".to_owned()),
        Err(error) => errors.push(format!("could not read repository graph Wasm: {error}")),
    }
    match fs::read(artifact_root.join("radio_mirror_bg.wasm")) {
        Ok(bytes) if bytes.starts_with(b"\0asm") => {}
        Ok(_) => errors.push("radio-mirror Wasm has an invalid magic header".to_owned()),
        Err(error) => errors.push(format!("could not read radio-mirror Wasm: {error}")),
    }

    let metadata_bytes = match fs::read(metadata_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            errors.push(format!("could not hash public metadata input: {error}"));
            Vec::new()
        }
    };

    if errors.is_empty() {
        Ok(ArtifactReceipt {
            schema: RECEIPT_SCHEMA,
            source_sha: env::var("GITHUB_SHA").unwrap_or_else(|_| "local".to_owned()),
            files: file_receipts,
            total_bytes,
            repositories: repository_count,
            relation_text_projections,
            graph_nodes,
            graph_edges,
            project_profiles: project_ids.len(),
            project_relation_projections: project_relation_ids.len(),
            showcase_images: showcases.showcase.len(),
            sitemap_urls,
            project_social_previews,
            project_structured_records,
            projection_score_items: projection
                .as_ref()
                .map_or(0, |proof| proof.reading.receipt.score_items),
            projection_final_revision: projection
                .as_ref()
                .map_or(0, |proof| proof.reading.receipt.final_revision),
            projection_active_relations: projection
                .as_ref()
                .map_or(0, |proof| proof.reading.receipt.active_relations),
            projection_trace_steps: projection
                .as_ref()
                .map_or(0, |proof| proof.reading.receipt.trace_steps),
            projection_capture_address: projection.as_ref().map_or_else(String::new, |proof| {
                proof.reading.receipt.capture_address.clone()
            }),
            repository_history_revisions: frozen
                .as_ref()
                .map_or(0, |frozen| frozen.history.revisions.len()),
            repository_frozen_matrix_cells: frozen
                .as_ref()
                .map_or(0, |frozen| frozen.matrix.cells.len()),
            device_profiles: device_ids.len(),
            device_structured_records,
            radio_mirror_screens: radio_screens.len(),
            message_path_traces: traces.as_ref().map_or(0, |traces| traces.scenarios.len()),
            message_path_events: traces.as_ref().map_or(0, |traces| {
                traces
                    .scenarios
                    .iter()
                    .map(|scenario| scenario.trace.events.len())
                    .sum()
            }),
            message_path_ledger_rows: traces.as_ref().map_or(0, |traces| {
                traces
                    .scenarios
                    .iter()
                    .map(|scenario| message_path::milestones(scenario).len())
                    .sum()
            }),
            sellable_devices,
            metadata_generated_at_utc: metadata.generated_at_utc.clone(),
            metadata_sha256: sha256(&metadata_bytes),
        })
    } else {
        Err(errors)
    }
}
