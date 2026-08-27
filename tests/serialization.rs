#![cfg(feature = "serde")]

use core::fmt::Debug;

use libvgraph::{BuildOptions, CsrGraph, ReversePolicy};
use serde_json::{json, Value};

fn must<T, E: Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("serialization fixture failed: {error:?}"),
    }
}

#[test]
fn versioned_csr_round_trip_preserves_both_reverse_policies() {
    let graph = must(CsrGraph::from_edges(
        [30, 10, 20],
        [(10, 20), (20, 20), (30, 10)],
    ));
    let encoded = must(serde_json::to_vec(&graph));
    let decoded: CsrGraph<u32> = must(serde_json::from_slice(&encoded));
    assert_eq!(decoded, graph);
    assert_eq!(decoded.validate(), Ok(()));

    let forward_only = must(CsrGraph::from_edges_with_options(
        0..4,
        [(0, 1), (1, 2), (2, 3)],
        BuildOptions {
            reverse: ReversePolicy::Omit,
            ..BuildOptions::default()
        },
    ));
    let encoded = must(serde_json::to_vec(&forward_only));
    let decoded: CsrGraph<u32> = must(serde_json::from_slice(&encoded));
    assert_eq!(decoded, forward_only);
    assert!(!decoded.has_reverse());
}

#[test]
fn deserialization_rejects_unknown_versions_and_every_unsafe_shape() {
    let valid = json!({
        "format_version": 1,
        "nodes": [0, 1],
        "forward_offsets": [0, 1, 1],
        "forward_targets": [1],
        "reverse_offsets": [0, 0, 1],
        "reverse_targets": [0]
    });
    let decoded: CsrGraph<u32> = must(serde_json::from_value(valid.clone()));
    assert_eq!(decoded.validate(), Ok(()));

    let mut malformed = Vec::<Value>::new();
    let mut unknown_version = valid.clone();
    unknown_version["format_version"] = json!(2);
    malformed.push(unknown_version);

    let mut missing_reverse_half = valid.clone();
    missing_reverse_half["reverse_targets"] = Value::Null;
    malformed.push(missing_reverse_half);

    let mut wrong_offset_length = valid.clone();
    wrong_offset_length["forward_offsets"] = json!([0, 1]);
    malformed.push(wrong_offset_length);

    let mut nonzero_origin = valid.clone();
    nonzero_origin["forward_offsets"] = json!([1, 1, 1]);
    malformed.push(nonzero_origin);

    let mut decreasing_offsets = valid.clone();
    decreasing_offsets["forward_offsets"] = json!([0, 1, 0]);
    malformed.push(decreasing_offsets);

    let mut wrong_terminal = valid.clone();
    wrong_terminal["forward_offsets"] = json!([0, 0, 0]);
    malformed.push(wrong_terminal);

    let mut out_of_range_target = valid.clone();
    out_of_range_target["forward_targets"] = json!([2]);
    malformed.push(out_of_range_target);

    let mut duplicate_target = valid.clone();
    duplicate_target["forward_offsets"] = json!([0, 2, 2]);
    duplicate_target["forward_targets"] = json!([1, 1]);
    duplicate_target["reverse_offsets"] = json!([0, 0, 2]);
    duplicate_target["reverse_targets"] = json!([0, 0]);
    malformed.push(duplicate_target);

    let mut reverse_mismatch = valid;
    reverse_mismatch["reverse_offsets"] = json!([0, 1, 1]);
    reverse_mismatch["reverse_targets"] = json!([1]);
    malformed.push(reverse_mismatch);

    for value in malformed {
        assert!(serde_json::from_value::<CsrGraph<u32>>(value).is_err());
    }
}
