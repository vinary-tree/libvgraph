use libvgraph::{BuildOptions, CsrGraph, DenseId, ReversePolicy};
use libvgraph_interop::{
    decode_snapshot, decode_verified_snapshot, digest_snapshot, encode_snapshot, SemanticProfileId,
    SnapshotLimits, DIGEST_CONTEXT, SNAPSHOT_HEADER_BYTES, SNAPSHOT_SCHEMA_ID, SNAPSHOT_VERSION,
};
use proptest::prelude::*;

fn profile(seed: u8) -> SemanticProfileId {
    SemanticProfileId::new([seed; 32])
}

fn dense_graph(vertices: u32, raw_edges: &[(u8, u8)]) -> CsrGraph<DenseId> {
    let edges = raw_edges.iter().filter_map(|&(source, target)| {
        (vertices != 0).then(|| {
            (
                DenseId::from_raw(u32::from(source) % vertices),
                DenseId::from_raw(u32::from(target) % vertices),
            )
        })
    });
    CsrGraph::from_dense_edges_with_options(
        vertices,
        edges,
        BuildOptions {
            reverse: ReversePolicy::Omit,
            ..BuildOptions::default()
        },
    )
    .expect("bounded generated edges must form a canonical dense graph")
}

fn raw_edges(graph: &CsrGraph<DenseId>) -> Vec<(u32, u32)> {
    graph
        .edges()
        .map(|(source, target)| (source.get(), target.get()))
        .collect()
}

proptest! {
    #[test]
    fn contract_round_trip(
        vertices in 0_u32..32,
        edges in prop::collection::vec((any::<u8>(), any::<u8>()), 0..256),
        profile_seed in any::<u8>(),
    ) {
        let graph = dense_graph(vertices, &edges);
        let semantic_profile = profile(profile_seed);
        let bytes = encode_snapshot(&graph, semantic_profile)
            .expect("a bounded canonical graph must encode");
        let decoded = decode_snapshot(
            &bytes,
            semantic_profile,
            SnapshotLimits::unbounded(),
        )
        .expect("an emitted snapshot must decode");
        prop_assert_eq!(decoded.vertex_count(), graph.vertex_count());
        prop_assert_eq!(raw_edges(&decoded), raw_edges(&graph));
        prop_assert!(!decoded.has_reverse());
    }

    #[test]
    fn contract_unique_encoding(
        vertices in 0_u32..32,
        mut edges in prop::collection::vec((any::<u8>(), any::<u8>()), 0..256),
        profile_seed in any::<u8>(),
    ) {
        let semantic_profile = profile(profile_seed);
        let first = dense_graph(vertices, &edges);
        let first_bytes = encode_snapshot(&first, semantic_profile)
            .expect("a bounded canonical graph must encode");
        let duplicated = edges.clone();
        edges.reverse();
        edges.extend(duplicated);
        let second = dense_graph(vertices, &edges);
        let second_bytes = encode_snapshot(&second, semantic_profile)
            .expect("a reordered duplicate enumeration must encode");
        prop_assert_eq!(first_bytes, second_bytes);
    }

    #[test]
    fn contract_insertion_order_and_duplicate_invariance(
        vertices in 0_u32..32,
        mut edges in prop::collection::vec((any::<u8>(), any::<u8>()), 0..256),
    ) {
        let original = dense_graph(vertices, &edges);
        let mut reordered = edges.clone();
        reordered.reverse();
        edges.append(&mut reordered);
        let repeated = dense_graph(vertices, &edges);
        prop_assert_eq!(raw_edges(&original), raw_edges(&repeated));
        prop_assert_eq!(
            encode_snapshot(&original, profile(1)),
            encode_snapshot(&repeated, profile(1)),
        );
    }

    #[test]
    fn contract_lawful_renaming_equivariance(
        vertices in 0_u32..32,
        edges in prop::collection::vec((any::<u8>(), any::<u8>()), 0..256),
    ) {
        let original = dense_graph(vertices, &edges);
        let renamed_edges: Vec<_> = original
            .edges()
            .map(|(source, target)| {
                let rename = |value: u32| vertices - 1 - value;
                (
                    DenseId::from_raw(rename(source.get())),
                    DenseId::from_raw(rename(target.get())),
                )
            })
            .collect();
        let renamed = CsrGraph::from_dense_edges_with_options(
            vertices,
            renamed_edges,
            BuildOptions {
                reverse: ReversePolicy::Omit,
                ..BuildOptions::default()
            },
        )
        .expect("a bijective dense renaming must remain in-domain");
        let bytes = encode_snapshot(&renamed, profile(2))
            .expect("a lawfully renamed graph must encode");
        let decoded = decode_snapshot(&bytes, profile(2), SnapshotLimits::unbounded())
            .expect("a lawfully renamed graph must decode");
        prop_assert_eq!(raw_edges(&decoded), raw_edges(&renamed));
    }

    #[test]
    fn contract_profile_separation(
        vertices in 0_u32..16,
        edges in prop::collection::vec((any::<u8>(), any::<u8>()), 0..64),
        first_seed in any::<u8>(),
        second_seed in any::<u8>(),
    ) {
        prop_assume!(first_seed != second_seed);
        let graph = dense_graph(vertices, &edges);
        let first_profile = profile(first_seed);
        let second_profile = profile(second_seed);
        let first = encode_snapshot(&graph, first_profile)
            .expect("the first profile must encode");
        let second = encode_snapshot(&graph, second_profile)
            .expect("the second profile must encode");
        prop_assert_ne!(&first, &second);
        prop_assert_ne!(
            digest_snapshot(&first, first_profile),
            digest_snapshot(&second, second_profile),
        );
        prop_assert!(decode_snapshot(
            &first,
            second_profile,
            SnapshotLimits::unbounded(),
        ).is_err());
    }

    #[test]
    fn contract_digest_domain_schema_and_payload_separation(
        vertices in 0_u32..16,
        edges in prop::collection::vec((any::<u8>(), any::<u8>()), 0..64),
    ) {
        let graph = dense_graph(vertices, &edges);
        let semantic_profile = profile(9);
        let bytes = encode_snapshot(&graph, semantic_profile)
            .expect("the generated graph must encode");
        let digest = digest_snapshot(&bytes, semantic_profile);
        let mut changed = bytes.clone();
        changed.push(0);
        prop_assert_ne!(digest, digest_snapshot(&changed, semantic_profile));
        prop_assert_eq!(SNAPSHOT_SCHEMA_ID.len(), 16);
        prop_assert_eq!(DIGEST_CONTEXT,
            "libvgraph-interop 2026-09-02 17:22:31 UTC canonical snapshot digest v1");
    }
}

#[test]
fn contract_schema_rejection() {
    let graph = dense_graph(2, &[(0, 1)]);
    let semantic_profile = profile(3);
    let mut bytes = encode_snapshot(&graph, semantic_profile).expect("the graph must encode");
    bytes[8] ^= 1;
    assert!(decode_snapshot(&bytes, semantic_profile, SnapshotLimits::unbounded(),).is_err());
}

#[test]
fn contract_version_rejection() {
    let graph = dense_graph(2, &[(0, 1)]);
    let semantic_profile = profile(4);
    let mut bytes = encode_snapshot(&graph, semantic_profile).expect("the graph must encode");
    bytes[24..26].copy_from_slice(&(SNAPSHOT_VERSION.0 + 1).to_le_bytes());
    assert!(decode_snapshot(&bytes, semantic_profile, SnapshotLimits::unbounded(),).is_err());
}

#[test]
fn contract_length_index_and_trailing_rejection() {
    let graph = dense_graph(2, &[(0, 1)]);
    let semantic_profile = profile(5);
    let bytes = encode_snapshot(&graph, semantic_profile).expect("the graph must encode");
    assert_eq!(SNAPSHOT_HEADER_BYTES, 80);
    for prefix_length in 0..bytes.len() {
        assert!(decode_snapshot(
            &bytes[..prefix_length],
            semantic_profile,
            SnapshotLimits::unbounded(),
        )
        .is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(decode_snapshot(&trailing, semantic_profile, SnapshotLimits::unbounded(),).is_err());
    let mut out_of_range = bytes;
    let target_start = SNAPSHOT_HEADER_BYTES + 3 * 4;
    out_of_range[target_start..target_start + 4].copy_from_slice(&2_u32.to_le_bytes());
    assert!(
        decode_snapshot(&out_of_range, semantic_profile, SnapshotLimits::unbounded(),).is_err()
    );
}

#[test]
fn contract_resource_limits_fail_before_publication() {
    let graph = dense_graph(2, &[(0, 1)]);
    let semantic_profile = profile(6);
    let bytes = encode_snapshot(&graph, semantic_profile).expect("the graph must encode");
    assert!(decode_snapshot(
        &bytes,
        semantic_profile,
        SnapshotLimits {
            max_vertices: 1,
            max_edges: 1,
            max_bytes: bytes.len() as u64,
        },
    )
    .is_err());
    assert!(decode_snapshot(
        &bytes,
        semantic_profile,
        SnapshotLimits {
            max_vertices: 2,
            max_edges: 0,
            max_bytes: bytes.len() as u64,
        },
    )
    .is_err());
    assert!(decode_snapshot(
        &bytes,
        semantic_profile,
        SnapshotLimits {
            max_vertices: 2,
            max_edges: 1,
            max_bytes: bytes.len() as u64 - 1,
        },
    )
    .is_err());
}

#[test]
fn contract_stale_digest_rejection() {
    let graph = dense_graph(2, &[(0, 1)]);
    let semantic_profile = profile(7);
    let bytes = encode_snapshot(&graph, semantic_profile).expect("the graph must encode");
    let digest = digest_snapshot(&bytes, semantic_profile);
    let mut changed = bytes.clone();
    changed[SNAPSHOT_HEADER_BYTES] ^= 1;
    assert!(decode_verified_snapshot(
        &changed,
        semantic_profile,
        digest,
        SnapshotLimits::unbounded(),
    )
    .is_err());
}

#[test]
fn contract_exact_cross_version_policy() {
    assert_eq!(SNAPSHOT_VERSION, (1, 0));
    let graph = dense_graph(0, &[]);
    let semantic_profile = profile(8);
    let bytes = encode_snapshot(&graph, semantic_profile).expect("the empty graph must encode");
    assert!(decode_snapshot(&bytes, semantic_profile, SnapshotLimits::unbounded(),).is_ok());
    for replacement in [(0_u16, 9_u16), (1, 1), (2, 0)] {
        let mut incompatible = bytes.clone();
        incompatible[24..26].copy_from_slice(&replacement.0.to_le_bytes());
        incompatible[26..28].copy_from_slice(&replacement.1.to_le_bytes());
        assert!(
            decode_snapshot(&incompatible, semantic_profile, SnapshotLimits::unbounded(),).is_err()
        );
    }
}

#[test]
fn contract_deep_codec_lifecycle_is_native_stack_independent() {
    const VERTICES: u32 = 100_000;
    std::thread::Builder::new()
        .name("interop-contract-small-stack".to_owned())
        .stack_size(64 * 1024)
        .spawn(|| {
            let edges: Vec<_> = (0..VERTICES - 1)
                .map(|source| (DenseId::from_raw(source), DenseId::from_raw(source + 1)))
                .collect();
            let graph = CsrGraph::from_dense_edges_with_options(
                VERTICES,
                edges,
                BuildOptions {
                    reverse: ReversePolicy::Omit,
                    ..BuildOptions::default()
                },
            )
            .expect("the deep chain must build");
            let semantic_profile = profile(10);
            let bytes =
                encode_snapshot(&graph, semantic_profile).expect("the deep graph must encode");
            let decoded = decode_snapshot(&bytes, semantic_profile, SnapshotLimits::unbounded())
                .expect("the deep graph must decode");
            assert_eq!(decoded.vertex_count(), VERTICES as usize);
        })
        .expect("the small-stack contract thread must start")
        .join()
        .expect("the small-stack contract thread must complete");
}

#[derive(Clone, Copy)]
struct ReleaseEvidence {
    protected_signer: bool,
    protected_head: bool,
    gates_passed: bool,
    portable_tool_closure: bool,
    draft_created: bool,
    assets_complete: bool,
    registry_matches: bool,
    publication_count: u8,
}

fn release_publishable(evidence: ReleaseEvidence) -> bool {
    evidence.protected_signer
        && evidence.protected_head
        && evidence.gates_passed
        && evidence.portable_tool_closure
        && evidence.draft_created
        && evidence.assets_complete
        && evidence.registry_matches
        && evidence.publication_count == 1
}

#[test]
fn contract_release_publication_is_fail_closed() {
    let complete = ReleaseEvidence {
        protected_signer: true,
        protected_head: true,
        gates_passed: true,
        portable_tool_closure: true,
        draft_created: true,
        assets_complete: true,
        registry_matches: true,
        publication_count: 1,
    };
    assert!(release_publishable(complete));

    let rejected = [
        ReleaseEvidence {
            protected_signer: false,
            ..complete
        },
        ReleaseEvidence {
            protected_head: false,
            ..complete
        },
        ReleaseEvidence {
            gates_passed: false,
            ..complete
        },
        ReleaseEvidence {
            portable_tool_closure: false,
            ..complete
        },
        ReleaseEvidence {
            draft_created: false,
            ..complete
        },
        ReleaseEvidence {
            assets_complete: false,
            ..complete
        },
        ReleaseEvidence {
            registry_matches: false,
            ..complete
        },
        ReleaseEvidence {
            publication_count: 0,
            ..complete
        },
        ReleaseEvidence {
            publication_count: 2,
            ..complete
        },
    ];
    assert!(rejected.into_iter().all(|case| !release_publishable(case)));
}

#[test]
fn contract_release_portable_tool_closure_is_explicit() {
    let publishable = ReleaseEvidence {
        protected_signer: true,
        protected_head: true,
        gates_passed: true,
        portable_tool_closure: true,
        draft_created: true,
        assets_complete: true,
        registry_matches: true,
        publication_count: 1,
    };
    assert!(release_publishable(publishable));
    assert!(!release_publishable(ReleaseEvidence {
        portable_tool_closure: false,
        ..publishable
    }));
}
