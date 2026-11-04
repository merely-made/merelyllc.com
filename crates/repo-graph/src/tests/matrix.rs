use super::*;

#[test]
fn two_independent_readings_form_a_provenance_carrying_matrix_capture() {
    let artifact = matrix_projection(&sample_matrix_request()).expect("Matrix projection");
    let receipt = consume_matrix_projection(&artifact).expect("Matrix receipt");
    assert_eq!(artifact.rows.reading, "neighbors");
    assert_eq!(artifact.columns.reading, "graph");
    assert_eq!(receipt.cells, receipt.row_sources * receipt.column_sources);
    assert!(
        receipt.relation_cells >= 2,
        "cross-scope relations remain cells"
    );
    assert!(receipt.accessible_table);

    let mere_source = artifact
        .capture
        .scene
        .tables
        .sources
        .iter()
        .position(|source| {
            source
                .as_ref()
                .is_some_and(|source| source.adapter == PROJECTION_ADAPTER && source.id == "mere")
        })
        .expect("Mere source");
    let appearances = artifact
        .capture
        .scene
        .tables
        .items
        .iter()
        .flatten()
        .filter(|item| item.source.0 as usize == mere_source)
        .count();
    assert!(appearances >= 2, "one source backs both Matrix headings");

    assert!(
        artifact
            .accessible_html
            .contains("<table data-projection-family=\"matrix\"")
    );
    assert!(artifact.accessible_html.contains("scope=\"row\""));
    assert!(artifact.accessible_html.contains("scope=\"col\""));
}

fn scene_source_mapping(snapshot: &scenotime::SceneSnapshot) -> Vec<(InstanceId, SourceRef)> {
    snapshot
        .tables
        .items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let item = item.as_ref()?;
            if !item.visible {
                return None;
            }
            let source = snapshot
                .tables
                .sources
                .get(item.source.0 as usize)?
                .as_ref()?;
            Some((InstanceId(index as u32), source.clone()))
        })
        .collect()
}

#[derive(Clone)]
struct MatrixSnapshotEndpoint {
    snapshot: chirograph::ProjectionSnapshot,
    resources: BTreeMap<chirograph::ContentHash, Vec<u8>>,
}

fn matrix_presentation(
    artifact: &MatrixProjectionArtifact,
) -> (
    chirograph::PresentationManifest,
    BTreeMap<chirograph::ContentHash, Vec<u8>>,
) {
    let labels = artifact
        .rows
        .sources
        .iter()
        .chain(&artifact.columns.sources)
        .map(|source| source.name.clone())
        .chain(artifact.cells.iter().map(|cell| cell.description.clone()))
        .collect::<Vec<_>>();
    assert_eq!(
        labels.len(),
        scene_source_mapping(&artifact.capture.scene).len(),
        "Matrix construction and presentation keep the same row-column-cell order"
    );
    let mut presentation = chirograph::PresentationManifest::default();
    let mut resources = BTreeMap::new();
    for ((instance, _), label) in scene_source_mapping(&artifact.capture.scene)
        .into_iter()
        .zip(labels)
    {
        let glyph = chirograph::NativeGlyphV1 {
            label: label.clone(),
            icon: None,
            color: None,
        };
        let bytes = serde_json::to_vec(&glyph).expect("encode Matrix glyph");
        let resource = chirograph::ContentHash::of(&bytes);
        let key = chirograph::PresentationKey(format!("matrix:{}", instance.0));
        presentation.bindings.push(chirograph::PresentationBinding {
            instance,
            key: key.clone(),
        });
        presentation.offers.insert(
            key,
            vec![chirograph::PresentationOffer {
                codec: chirograph::PresentationCodec::NativeGlyphV1,
                resource,
                byte_size: bytes.len() as u64,
                requires: chirograph::PresentationCapability::NativeGlyph,
                semantics: chirograph::PresentationSemantics {
                    label,
                    role: chirograph::SemanticRole::Graphic,
                    bounds: chirograph::BoundsRelationship::FillFootprint,
                    actions: Vec::new(),
                },
            }],
        );
        resources.insert(resource, bytes);
    }
    assert_eq!(
        presentation.bindings.len(),
        artifact.capture.scene.active_item_count(),
        "every remote Matrix instance has presentation semantics"
    );
    (presentation, resources)
}

impl graphshell_endpoint::ProjectionCatalog for MatrixSnapshotEndpoint {
    fn describe(&self) -> chirograph::EndpointDescriptor {
        chirograph::EndpointDescriptor {
            label: "Mer3ly Matrix receipt".into(),
            projections: vec![chirograph::ProjectionOffer {
                label: "Two-reading Matrix".into(),
                request: chirograph::ProjectionRequest {
                    version: chirograph::ProtocolVersion::V1,
                    session: self.snapshot.session.clone(),
                    score: Score::new(SceneArrangement::Grid(sceno::Grid::default())),
                },
            }],
        }
    }
}

impl graphshell_endpoint::ProjectionSource for MatrixSnapshotEndpoint {
    type Error = String;

    fn snapshot(
        &mut self,
        request: chirograph::ProjectionRequest,
    ) -> Result<chirograph::ProjectionSnapshot, Self::Error> {
        if request.session != self.snapshot.session {
            return Err("unknown Matrix projection".into());
        }
        Ok(self.snapshot.clone())
    }
}

impl graphshell_endpoint::PresentationSource for MatrixSnapshotEndpoint {
    type Error = String;

    fn resource(
        &mut self,
        request: chirograph::ResourceRequest,
    ) -> Result<chirograph::ResourceResponse, Self::Error> {
        if request.session != self.snapshot.session {
            return Err("unknown Matrix projection".into());
        }
        let bytes = self
            .resources
            .get(&request.resource)
            .cloned()
            .ok_or_else(|| "unknown Matrix presentation resource".to_owned())?;
        Ok(chirograph::ResourceResponse::new(request.session, bytes))
    }
}

impl graphshell_endpoint::IntentSink for MatrixSnapshotEndpoint {
    type Error = String;

    fn invoke(
        &mut self,
        _: chirograph::IntentInvocation,
    ) -> Result<chirograph::IntentResult, Self::Error> {
        Err("the Matrix receipt is read-only".into())
    }
}

impl graphshell_endpoint::ProjectionNoticeSource for MatrixSnapshotEndpoint {
    type Error = String;

    fn poll_notice(&mut self) -> Result<Option<chirograph::CarrierNotice>, Self::Error> {
        Ok(None)
    }
}

const FT7_NETWORK: notochord::NetworkId = notochord::NetworkId([3; 32]);

const FT7_ROOT_AUTHORITY: [u8; 32] = [7; 32];

const FT7_NOW_MS: u64 = 50;

fn ft7_owner() -> personae::InMemoryProvider {
    personae::InMemoryProvider::from_seed([1; 32])
}

fn ft7_viewer() -> personae::InMemoryProvider {
    personae::InMemoryProvider::from_seed([4; 32])
}

fn ft7_profile_ref() -> notochord::ProfileRef {
    notochord::ProfileRef {
        id: "mere.base".into(),
        revision: 1,
    }
}

fn ft7_grant(subject: [u8; 32]) -> insigne::delegation::SignedDelegationCertificate {
    use personae::IdentityProvider as _;
    use personae::delegation::Issue as _;

    let owner = ft7_owner();
    insigne::delegation::SignedDelegationCertificate::issue(
        &owner,
        insigne::delegation::DelegationCertificate::new(
            insigne::delegation::DelegationParent::Root(FT7_ROOT_AUTHORITY),
            owner.master_public_key().to_bytes(),
            subject,
            insigne::delegation::CapabilityScope {
                domain: graphshell_host::admission::GRAPHSHELL_DOMAIN.into(),
                resource: FT7_NETWORK.0.to_vec(),
                path_prefix: graphshell_host::admission::PROJECTION_SERVICE.into(),
                actions: [graphshell_host::admission::CONNECT_ACTION.to_owned()]
                    .into_iter()
                    .collect(),
            },
            5,
            10,
            Some(FT7_NOW_MS + 3_600_000),
            1,
            [1; 32],
        ),
    )
    .expect("issue Matrix viewer certificate")
}

fn ft7_policy() -> notochord::LocalNetworkPolicy {
    use personae::IdentityProvider as _;

    graphshell_host::carrier::projection_policy(
        FT7_NETWORK,
        vec![notochord::TrustedRoot {
            authority: FT7_ROOT_AUTHORITY,
            issuer: ft7_owner().master_public_key().to_bytes(),
        }],
        vec![ft7_profile_ref()],
        None,
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn one_matrix_mapping_survives_local_remote_and_frozen_realizations() {
    use chirograph::Carrier;
    use graphshell_client::{RetainedEndpointSession, frozen::FrozenScene};
    use graphshell_host::admission::open_session;
    use graphshell_host::carrier::accept_projection_session;
    use graphshell_host::lifecycle::SessionAuthority;
    use graphshell_host::network_carrier::{
        CarrierRuntime, NetworkCarrier, dial_projection_session, projection_binding,
    };
    use graphshell_host::session_notices::serve_admitted_session_notifying;
    use notochord::{RevocationLedger, TrafficClass};
    use personae::IdentityProvider as _;
    use std::sync::RwLock;
    use std::time::Duration;
    use transport::{PeerID, memory::MemoryTransport};

    let artifact = matrix_projection(&sample_matrix_request()).expect("Matrix projection");
    let source_mapping = scene_source_mapping(&artifact.capture.scene);
    assert!(
        source_mapping
            .iter()
            .enumerate()
            .any(|(left, (_, source))| source_mapping[left + 1..]
                .iter()
                .any(|(_, candidate)| candidate == source)),
        "the receipt keeps a genuinely repeated source instance"
    );

    let (presentation, resources) = matrix_presentation(&artifact);
    let snapshot = chirograph::ProjectionSnapshot {
        version: chirograph::ProtocolVersion::V1,
        session: chirograph::ProjectionSession("mer3ly.matrix.ft7".into()),
        scene: artifact.capture.scene.clone(),
        presentation,
        cache_policy: chirograph::CachePolicy::default(),
    };
    let local_carrier = graphshell_endpoint::local::LocalCarrier::new(
        MatrixSnapshotEndpoint {
            snapshot: snapshot.clone(),
            resources: resources.clone(),
        },
        |_: &mut MatrixSnapshotEndpoint, _| Err("the frozen receipt does not resume".into()),
    );
    let mut local_viewer = RetainedEndpointSession::over(
        Box::new(local_carrier),
        chirograph::CapabilityProfile::default(),
    )
    .expect("local Graphshell host discovers the Matrix endpoint");
    let local_session = local_viewer.mount(0).expect("local host mounts Matrix");
    assert_eq!(
        local_viewer
            .resolve_all(&local_session)
            .expect("local host realizes every Matrix item")
            .len(),
        source_mapping.len()
    );
    let local_mapping = scene_source_mapping(
        &local_viewer
            .client()
            .mounted(&local_session)
            .expect("local host owns the disclosed snapshot")
            .scene,
    );
    assert_eq!(local_mapping, source_mapping);
    // The in-process carrier has no session plane; dropping the handle is
    // its complete lifecycle rather than pretending it can answer Close.
    drop(local_viewer);

    let viewer_identity = ft7_viewer();
    let subject = viewer_identity.master_public_key().to_bytes();
    let client_peer = PeerID::from_bytes(&subject).expect("Matrix viewer peer");
    let server_peer =
        PeerID::from_bytes(&ft7_owner().master_public_key().to_bytes()).expect("Matrix owner peer");
    let (server_transport, client_transport) = MemoryTransport::pair(server_peer, client_peer);

    let serving = tokio::spawn(async move {
        let mut admitted = accept_projection_session(
            &server_transport,
            &ft7_policy(),
            &RevocationLedger::default(),
            FT7_NOW_MS,
            0,
        )
        .await
        .expect("Matrix accept path")
        .expect("the Matrix viewer is admitted");
        let authority = SessionAuthority::retain_admitted(&admitted);
        let mut endpoint = MatrixSnapshotEndpoint {
            snapshot,
            resources,
        };
        let mut resume = |_: &mut MatrixSnapshotEndpoint, _: chirograph::ResumeRequest| {
            Err("the fixed Matrix receipt does not resume".to_owned())
        };
        serve_admitted_session_notifying(
            &mut admitted,
            &authority,
            &RwLock::new(RevocationLedger::default()),
            &mut endpoint,
            &mut resume,
            || FT7_NOW_MS,
            Duration::from_millis(10),
        )
        .await
        .expect("serve admitted Matrix session")
    });

    let handle = tokio::runtime::Handle::current();
    let accessible_html = artifact.accessible_html.clone();
    tokio::task::spawn_blocking(move || {
        let binding = projection_binding(client_peer);
        let hello = open_session(
            &viewer_identity,
            FT7_NETWORK,
            ft7_profile_ref(),
            TrafficClass::Interactive,
            [5; 32],
            &binding,
            vec![ft7_grant(subject)],
        )
        .expect("issue Matrix session hello");
        let stream = handle
            .block_on(dial_projection_session(
                &client_transport,
                server_peer,
                &hello,
                &ft7_policy().limits,
            ))
            .expect("dial Matrix projection service")
            .expect("the owner admits the Matrix viewer");
        let mut carrier =
            NetworkCarrier::over(stream, CarrierRuntime::borrowed(handle.clone()));
        let opened = carrier
            .request(chirograph::CarrierRequestBody::Open(Box::new(
                chirograph::SessionOpen {
                    version: chirograph::ProtocolVersion::V1,
                    capabilities: chirograph::CapabilityProfile::default(),
                },
            )))
            .expect("viewer opens the admitted Graphshell session");
        match opened {
            chirograph::CarrierResponseBody::Opened(opened) => {
                assert_eq!(opened.status, chirograph::SessionStatus::Live);
                assert_eq!(opened.descriptor.projections.len(), 1);
                assert_eq!(opened.descriptor.projections[0].label, "Two-reading Matrix");
            }
            other => panic!("expected an opened Matrix session, got {other:?}"),
        }

        let mut viewer = RetainedEndpointSession::over(
            Box::new(carrier),
            chirograph::CapabilityProfile::default(),
        )
        .expect("source-free viewer discovers the Matrix endpoint");
        let session = viewer.mount(0).expect("remote viewer mounts Matrix");
        let resolved = viewer
            .resolve_all(&session)
            .expect("remote viewer realizes every Matrix item");
        assert_eq!(
            resolved
                .iter()
                .map(|(instance, _)| *instance)
                .collect::<Vec<_>>(),
            local_mapping
                .iter()
                .map(|(instance, _)| *instance)
                .collect::<Vec<_>>(),
            "the remote presentation covers the same instance table"
        );

        {
            let mounted = viewer
                .client()
                .mounted(&session)
                .expect("viewer owns the disclosed snapshot");
            let remote_mapping = scene_source_mapping(&mounted.scene);
            assert_eq!(
                remote_mapping, local_mapping,
                "the admitted Graphshell session preserves every Matrix binding"
            );

            let names = resolved
                .iter()
                .map(|(instance, presentation)| {
                    (*instance, presentation.semantics.label.clone())
                })
                .collect::<HashMap<_, _>>();
            let frozen =
                FrozenScene::freeze_snapshot(&mounted.scene, "Two-reading Matrix", &names);
            let frozen_mapping = frozen
                .instances
                .iter()
                .map(|instance| (instance.instance, instance.source.clone()))
                .collect::<Vec<_>>();
            assert_eq!(
                frozen_mapping, local_mapping,
                "the frozen semantic document preserves the remote mapping"
            );
            let frozen_html = frozen.to_html("mer3ly-matrix");
            for (instance, source) in &local_mapping {
                let carried = format!(
                    "data-projection-instance=\"{}\" data-source-adapter=\"{}\" data-source-id=\"{}\"",
                    instance.0, source.adapter, source.id
                );
                assert!(
                    frozen_html.contains(&carried),
                    "frozen navigation lost {carried}"
                );
                assert!(
                    accessible_html.contains(&carried),
                    "the Matrix table lost {carried}"
                );
            }
            assert!(frozen_html.contains("<table class=\"frozen-alternate\""));
        }

        viewer.close().expect("viewer closes the Graphshell session");
    })
    .await
    .expect("Matrix viewer thread");

    let summary = serving.await.expect("Matrix server task");
    assert!(
        summary.answered > 3,
        "the admitted session served the Matrix"
    );
}
