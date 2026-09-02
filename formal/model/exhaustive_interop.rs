use std::collections::HashMap;
use std::convert::TryInto;

const MAGIC: [u8; 8] = *b"LVGSNP\0\x01";
const SCHEMA_ID: [u8; 16] = *b"LVGI-CSR-FWD-V1!";
const SCHEMA_MAJOR: u16 = 1;
const SCHEMA_MINOR: u16 = 0;
const HEADER_BYTES: usize = 80;
const WORD_BYTES: usize = 4;
const DIGEST_CONTEXT: &str =
    "libvgraph-interop 2026-09-02 17:22:31 UTC canonical snapshot digest v1";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Snapshot {
    vertices: u32,
    offsets: Vec<u32>,
    targets: Vec<u32>,
    profile: [u8; 32],
}

impl Snapshot {
    fn edges(&self) -> Vec<(u32, u32)> {
        let mut edges = Vec::with_capacity(self.targets.len());
        for source in 0..self.vertices {
            let start = self.offsets[source as usize] as usize;
            let end = self.offsets[source as usize + 1] as usize;
            for &target in &self.targets[start..end] {
                edges.push((source, target));
            }
        }
        edges
    }
}

#[derive(Clone, Copy, Debug)]
struct Limits {
    max_vertices: u32,
    max_edges: u32,
    max_bytes: u64,
}

impl Limits {
    const UNBOUNDED_MODEL: Self = Self {
        max_vertices: u32::MAX,
        max_edges: u32::MAX,
        max_bytes: u64::MAX,
    };
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum DecodeError {
    HeaderTruncated,
    Magic,
    Schema,
    Version { major: u16, minor: u16 },
    Flags(u32),
    Profile,
    VertexLimit,
    EdgeLimit,
    LengthOverflow,
    ByteLimit,
    PayloadLength,
    Truncated,
    Trailing,
    OffsetOrigin,
    OffsetOrder,
    OffsetTerminal,
    TargetOutOfRange,
    AdjacencyOrder,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DecodeReport {
    snapshot: Snapshot,
    work: u64,
    heap_words: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DigestInvocation {
    context: &'static str,
    material: Vec<u8>,
}

fn canonical_snapshot(
    vertices: u32,
    raw_edges: &[(u32, u32)],
    profile: [u8; 32],
) -> Snapshot {
    let mut edges = raw_edges.to_vec();
    assert!(
        edges
            .iter()
            .all(|&(source, target)| source < vertices && target < vertices),
        "the independent model requires in-domain edge endpoints"
    );
    edges.sort_unstable();
    edges.dedup();

    let mut offsets = vec![0_u32; vertices as usize + 1];
    let mut targets = Vec::with_capacity(edges.len());
    for &(source, target) in &edges {
        offsets[source as usize + 1] += 1;
        targets.push(target);
    }
    for index in 1..offsets.len() {
        offsets[index] += offsets[index - 1];
    }

    Snapshot {
        vertices,
        offsets,
        targets,
        profile,
    }
}

fn expected_payload_bytes(vertices: u32, edges: u32) -> Result<u64, DecodeError> {
    u64::from(vertices)
        .checked_add(1)
        .and_then(|offsets| offsets.checked_add(u64::from(edges)))
        .and_then(|words| words.checked_mul(WORD_BYTES as u64))
        .ok_or(DecodeError::LengthOverflow)
}

fn expected_wire_bytes(vertices: u32, edges: u32) -> Result<u64, DecodeError> {
    expected_payload_bytes(vertices, edges)?
        .checked_add(HEADER_BYTES as u64)
        .ok_or(DecodeError::LengthOverflow)
}

fn encode(snapshot: &Snapshot) -> Vec<u8> {
    validate_snapshot(snapshot).expect("a model snapshot must remain canonical");
    let edge_count =
        u32::try_from(snapshot.targets.len()).expect("the model edge count must fit u32");
    let payload_bytes = expected_payload_bytes(snapshot.vertices, edge_count)
        .expect("the u32 graph domain must have a representable payload");
    let wire_bytes = expected_wire_bytes(snapshot.vertices, edge_count)
        .and_then(|length| usize::try_from(length).map_err(|_| DecodeError::LengthOverflow))
        .expect("the model process must address its generated snapshot");

    let mut bytes = Vec::with_capacity(wire_bytes);
    bytes.extend_from_slice(&MAGIC);
    bytes.extend_from_slice(&SCHEMA_ID);
    bytes.extend_from_slice(&SCHEMA_MAJOR.to_le_bytes());
    bytes.extend_from_slice(&SCHEMA_MINOR.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&snapshot.profile);
    bytes.extend_from_slice(&snapshot.vertices.to_le_bytes());
    bytes.extend_from_slice(&edge_count.to_le_bytes());
    bytes.extend_from_slice(&payload_bytes.to_le_bytes());
    for &offset in &snapshot.offsets {
        bytes.extend_from_slice(&offset.to_le_bytes());
    }
    for &target in &snapshot.targets {
        bytes.extend_from_slice(&target.to_le_bytes());
    }
    assert_eq!(bytes.len(), wire_bytes);
    bytes
}

fn read_u16(bytes: &[u8], start: usize) -> u16 {
    u16::from_le_bytes(
        bytes[start..start + 2]
            .try_into()
            .expect("a header field must have two bytes"),
    )
}

fn read_u32(bytes: &[u8], start: usize) -> u32 {
    u32::from_le_bytes(
        bytes[start..start + 4]
            .try_into()
            .expect("a validated field must have four bytes"),
    )
}

fn read_u64(bytes: &[u8], start: usize) -> u64 {
    u64::from_le_bytes(
        bytes[start..start + 8]
            .try_into()
            .expect("a header field must have eight bytes"),
    )
}

fn decode(
    bytes: &[u8],
    expected_profile: [u8; 32],
    limits: Limits,
) -> Result<DecodeReport, DecodeError> {
    if bytes.len() < HEADER_BYTES {
        return Err(DecodeError::HeaderTruncated);
    }
    if bytes[..8] != MAGIC {
        return Err(DecodeError::Magic);
    }
    if bytes[8..24] != SCHEMA_ID {
        return Err(DecodeError::Schema);
    }
    let major = read_u16(bytes, 24);
    let minor = read_u16(bytes, 26);
    if (major, minor) != (SCHEMA_MAJOR, SCHEMA_MINOR) {
        return Err(DecodeError::Version { major, minor });
    }
    let flags = read_u32(bytes, 28);
    if flags != 0 {
        return Err(DecodeError::Flags(flags));
    }
    if bytes[32..64] != expected_profile {
        return Err(DecodeError::Profile);
    }

    let vertices = read_u32(bytes, 64);
    let edges = read_u32(bytes, 68);
    let declared_payload = read_u64(bytes, 72);
    if vertices > limits.max_vertices {
        return Err(DecodeError::VertexLimit);
    }
    if edges > limits.max_edges {
        return Err(DecodeError::EdgeLimit);
    }
    let exact_payload = expected_payload_bytes(vertices, edges)?;
    if declared_payload != exact_payload {
        return Err(DecodeError::PayloadLength);
    }
    let exact_wire = expected_wire_bytes(vertices, edges)?;
    if exact_wire > limits.max_bytes {
        return Err(DecodeError::ByteLimit);
    }
    let actual_wire = u64::try_from(bytes.len()).map_err(|_| DecodeError::LengthOverflow)?;
    if actual_wire < exact_wire {
        return Err(DecodeError::Truncated);
    }
    if actual_wire > exact_wire {
        return Err(DecodeError::Trailing);
    }

    let offset_count = usize::try_from(u64::from(vertices) + 1)
        .map_err(|_| DecodeError::LengthOverflow)?;
    let target_count = usize::try_from(edges).map_err(|_| DecodeError::LengthOverflow)?;
    let heap_words = u64::from(vertices) + 1 + u64::from(edges);
    let mut offsets = Vec::with_capacity(offset_count);
    let mut targets = Vec::with_capacity(target_count);
    let mut cursor = HEADER_BYTES;
    let mut work = 8_u64;

    for _ in 0..offset_count {
        offsets.push(read_u32(bytes, cursor));
        cursor += WORD_BYTES;
        work += 1;
    }
    for _ in 0..target_count {
        targets.push(read_u32(bytes, cursor));
        cursor += WORD_BYTES;
        work += 1;
    }
    assert_eq!(cursor, bytes.len());

    let snapshot = Snapshot {
        vertices,
        offsets,
        targets,
        profile: expected_profile,
    };
    work += validate_snapshot_with_work(&snapshot)?;
    let bound = 8 + 2 * (u64::from(vertices) + 1) + 3 * u64::from(edges);
    assert!(
        work <= bound,
        "decoder work {work} must remain below the proven {bound} bound"
    );
    Ok(DecodeReport {
        snapshot,
        work,
        heap_words,
    })
}

fn validate_snapshot(snapshot: &Snapshot) -> Result<(), DecodeError> {
    validate_snapshot_with_work(snapshot).map(|_| ())
}

fn validate_snapshot_with_work(snapshot: &Snapshot) -> Result<u64, DecodeError> {
    let expected_offsets = snapshot.vertices as usize + 1;
    if snapshot.offsets.len() != expected_offsets {
        return Err(DecodeError::PayloadLength);
    }
    let mut work = 0_u64;
    for &offset in &snapshot.offsets {
        work += 1;
        if offset as usize > snapshot.targets.len() {
            return Err(DecodeError::OffsetTerminal);
        }
    }
    if snapshot.offsets.first() != Some(&0) {
        return Err(DecodeError::OffsetOrigin);
    }
    if snapshot.offsets.windows(2).any(|pair| pair[0] > pair[1]) {
        return Err(DecodeError::OffsetOrder);
    }
    if snapshot.offsets.last().copied() != Some(snapshot.targets.len() as u32) {
        return Err(DecodeError::OffsetTerminal);
    }
    for &target in &snapshot.targets {
        work += 1;
        if target >= snapshot.vertices {
            return Err(DecodeError::TargetOutOfRange);
        }
    }
    for source in 0..snapshot.vertices as usize {
        let start = snapshot.offsets[source] as usize;
        let end = snapshot.offsets[source + 1] as usize;
        for pair in snapshot.targets[start..end].windows(2) {
            work += 1;
            if pair[0] >= pair[1] {
                return Err(DecodeError::AdjacencyOrder);
            }
        }
    }
    Ok(work)
}

fn digest_invocation(
    schema: [u8; 16],
    profile: [u8; 32],
    snapshot_bytes: &[u8],
) -> DigestInvocation {
    let mut material = Vec::with_capacity(16 + 32 + 8 + snapshot_bytes.len());
    material.extend_from_slice(&schema);
    material.extend_from_slice(&profile);
    material.extend_from_slice(
        &u64::try_from(snapshot_bytes.len())
            .expect("the model snapshot length must fit u64")
            .to_le_bytes(),
    );
    material.extend_from_slice(snapshot_bytes);
    DigestInvocation {
        context: DIGEST_CONTEXT,
        material,
    }
}

fn all_edges(vertices: u32, mask: u64) -> Vec<(u32, u32)> {
    let mut edges = Vec::with_capacity(vertices as usize * vertices as usize);
    let mut bit = 0_u32;
    for source in 0..vertices {
        for target in 0..vertices {
            if mask & (1_u64 << bit) != 0 {
                edges.push((source, target));
            }
            bit += 1;
        }
    }
    edges
}

fn next_permutation(values: &mut [u32]) -> bool {
    let Some(pivot) = (0..values.len().saturating_sub(1))
        .rev()
        .find(|&index| values[index] < values[index + 1])
    else {
        return false;
    };
    let successor = (pivot + 1..values.len())
        .rev()
        .find(|&index| values[pivot] < values[index])
        .expect("a permutation pivot must have a greater suffix member");
    values.swap(pivot, successor);
    values[pivot + 1..].reverse();
    true
}

fn permutations(vertices: u32) -> Vec<Vec<u32>> {
    let mut permutation: Vec<u32> = (0..vertices).collect();
    let mut all = vec![permutation.clone()];
    while next_permutation(&mut permutation) {
        all.push(permutation.clone());
    }
    all
}

fn renamed_edges(snapshot: &Snapshot, permutation: &[u32]) -> Vec<(u32, u32)> {
    let mut edges: Vec<_> = snapshot
        .edges()
        .into_iter()
        .map(|(source, target)| {
            (
                permutation[source as usize],
                permutation[target as usize],
            )
        })
        .collect();
    edges.sort_unstable();
    edges
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        encoded.push(char::from(DIGITS[(byte >> 4) as usize]));
        encoded.push(char::from(DIGITS[(byte & 0x0f) as usize]));
    }
    encoded
}

fn assert_golden_vectors() {
    let empty = canonical_snapshot(0, &[], [0; 32]);
    let empty_hex = hex(&encode(&empty));
    assert_eq!(
        empty_hex,
        "4c5647534e5000014c5647492d4353522d4657442d563121010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000040000000000000000000000"
    );

    let mut ascending_profile = [0_u8; 32];
    for (index, byte) in ascending_profile.iter_mut().enumerate() {
        *byte = u8::try_from(index).expect("the golden profile index must fit u8");
    }
    let one_edge = canonical_snapshot(2, &[(0, 1)], ascending_profile);
    let one_edge_hex = hex(&encode(&one_edge));
    assert_eq!(
        one_edge_hex,
        "4c5647534e5000014c5647492d4353522d4657442d5631210100000000000000000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0200000001000000100000000000000000000000010000000100000001000000"
    );
}

fn assert_malformed_rejections() {
    let profile = [7_u8; 32];
    let base = canonical_snapshot(2, &[(0, 1)], profile);
    let bytes = encode(&base);

    for prefix_length in 0..bytes.len() {
        let result = decode(
            &bytes[..prefix_length],
            profile,
            Limits::UNBOUNDED_MODEL,
        );
        assert!(result.is_err(), "every strict prefix must be rejected");
    }

    let mut mutated = bytes.clone();
    mutated[0] ^= 1;
    assert_eq!(
        decode(&mutated, profile, Limits::UNBOUNDED_MODEL),
        Err(DecodeError::Magic)
    );

    let mut mutated = bytes.clone();
    mutated[8] ^= 1;
    assert_eq!(
        decode(&mutated, profile, Limits::UNBOUNDED_MODEL),
        Err(DecodeError::Schema)
    );

    let mut mutated = bytes.clone();
    mutated[24..26].copy_from_slice(&2_u16.to_le_bytes());
    assert_eq!(
        decode(&mutated, profile, Limits::UNBOUNDED_MODEL),
        Err(DecodeError::Version { major: 2, minor: 0 })
    );

    let mut mutated = bytes.clone();
    mutated[26..28].copy_from_slice(&1_u16.to_le_bytes());
    assert_eq!(
        decode(&mutated, profile, Limits::UNBOUNDED_MODEL),
        Err(DecodeError::Version { major: 1, minor: 1 })
    );

    let mut mutated = bytes.clone();
    mutated[28..32].copy_from_slice(&1_u32.to_le_bytes());
    assert_eq!(
        decode(&mutated, profile, Limits::UNBOUNDED_MODEL),
        Err(DecodeError::Flags(1))
    );

    assert_eq!(
        decode(&bytes, [8; 32], Limits::UNBOUNDED_MODEL),
        Err(DecodeError::Profile)
    );

    let mut mutated = bytes.clone();
    mutated[72..80].copy_from_slice(&0_u64.to_le_bytes());
    assert_eq!(
        decode(&mutated, profile, Limits::UNBOUNDED_MODEL),
        Err(DecodeError::PayloadLength)
    );

    let mut trailing = bytes.clone();
    trailing.push(0);
    assert_eq!(
        decode(&trailing, profile, Limits::UNBOUNDED_MODEL),
        Err(DecodeError::Trailing)
    );

    assert_eq!(
        decode(
            &bytes,
            profile,
            Limits {
                max_vertices: 1,
                ..Limits::UNBOUNDED_MODEL
            }
        ),
        Err(DecodeError::VertexLimit)
    );
    assert_eq!(
        decode(
            &bytes,
            profile,
            Limits {
                max_edges: 0,
                ..Limits::UNBOUNDED_MODEL
            }
        ),
        Err(DecodeError::EdgeLimit)
    );
    assert_eq!(
        decode(
            &bytes,
            profile,
            Limits {
                max_bytes: bytes.len() as u64 - 1,
                ..Limits::UNBOUNDED_MODEL
            }
        ),
        Err(DecodeError::ByteLimit)
    );

    let mut mutated = bytes.clone();
    mutated[HEADER_BYTES..HEADER_BYTES + 4].copy_from_slice(&1_u32.to_le_bytes());
    assert_eq!(
        decode(&mutated, profile, Limits::UNBOUNDED_MODEL),
        Err(DecodeError::OffsetOrigin)
    );

    let mut mutated = bytes.clone();
    mutated[HEADER_BYTES + 8..HEADER_BYTES + 12]
        .copy_from_slice(&0_u32.to_le_bytes());
    assert_eq!(
        decode(&mutated, profile, Limits::UNBOUNDED_MODEL),
        Err(DecodeError::OffsetOrder)
    );

    let mut mutated = bytes.clone();
    mutated[HEADER_BYTES + 8..HEADER_BYTES + 12]
        .copy_from_slice(&2_u32.to_le_bytes());
    assert_eq!(
        decode(&mutated, profile, Limits::UNBOUNDED_MODEL),
        Err(DecodeError::OffsetTerminal)
    );

    let mut mutated = bytes.clone();
    mutated[HEADER_BYTES + 12..HEADER_BYTES + 16]
        .copy_from_slice(&2_u32.to_le_bytes());
    assert_eq!(
        decode(&mutated, profile, Limits::UNBOUNDED_MODEL),
        Err(DecodeError::TargetOutOfRange)
    );

    let ordered = canonical_snapshot(2, &[(0, 0), (0, 1)], profile);
    let mut mutated = encode(&ordered);
    let target_start = HEADER_BYTES + (ordered.offsets.len() * WORD_BYTES);
    mutated[target_start + 4..target_start + 8].copy_from_slice(&0_u32.to_le_bytes());
    assert_eq!(
        decode(&mutated, profile, Limits::UNBOUNDED_MODEL),
        Err(DecodeError::AdjacencyOrder)
    );
}

fn assert_digest_domains() {
    let profile = [3_u8; 32];
    let snapshot = canonical_snapshot(2, &[(0, 1)], profile);
    let bytes = encode(&snapshot);
    let base = digest_invocation(SCHEMA_ID, profile, &bytes);

    let mut other_schema = SCHEMA_ID;
    other_schema[0] ^= 1;
    assert_ne!(base, digest_invocation(other_schema, profile, &bytes));

    let mut other_profile = profile;
    other_profile[31] ^= 1;
    assert_ne!(base, digest_invocation(SCHEMA_ID, other_profile, &bytes));

    let mut other_payload = bytes;
    other_payload[HEADER_BYTES] ^= 1;
    assert_ne!(
        base,
        digest_invocation(SCHEMA_ID, profile, &other_payload)
    );

    let mut other_context = base.clone();
    other_context.context =
        "libvgraph-interop 2026-09-02 17:22:31 UTC different digest purpose";
    assert_ne!(base, other_context);
}

fn assert_deep_small_stack() {
    const VERTICES: u32 = 100_000;
    std::thread::Builder::new()
        .name("interop-model-small-stack".to_owned())
        .stack_size(64 * 1024)
        .spawn(|| {
            let mut edges = Vec::with_capacity(VERTICES as usize - 1);
            for source in 0..VERTICES - 1 {
                edges.push((source, source + 1));
            }
            let profile = [11_u8; 32];
            let snapshot = canonical_snapshot(VERTICES, &edges, profile);
            let bytes = encode(&snapshot);
            let report = decode(&bytes, profile, Limits::UNBOUNDED_MODEL)
                .expect("a deep canonical snapshot must decode");
            assert_eq!(report.snapshot, snapshot);
            assert_eq!(
                report.heap_words,
                u64::from(VERTICES) + u64::try_from(edges.len()).expect("edge count fits") + 1
            );
            assert!(
                report.work
                    <= 8 + 2 * (u64::from(VERTICES) + 1)
                        + 3 * u64::try_from(edges.len()).expect("edge count fits")
            );
        })
        .expect("the small-stack model thread must start")
        .join()
        .expect("the small-stack model thread must complete");
}

fn main() {
    assert_golden_vectors();
    assert_malformed_rejections();
    assert_digest_domains();

    let profiles = [[0_u8; 32], [1_u8; 32], [0xa5_u8; 32]];
    let mut unique_encodings: HashMap<Vec<u8>, Snapshot> = HashMap::new();
    let mut graph_count = 0_u64;
    let mut encoding_count = 0_u64;
    let mut renaming_count = 0_u64;
    let mut truncation_count = 0_u64;

    for vertices in 0..=3_u32 {
        let possible_edges = vertices * vertices;
        for mask in 0..(1_u64 << possible_edges) {
            let raw_edges = all_edges(vertices, mask);
            graph_count += 1;
            for profile in profiles {
                let snapshot = canonical_snapshot(vertices, &raw_edges, profile);
                let bytes = encode(&snapshot);
                let report = decode(&bytes, profile, Limits::UNBOUNDED_MODEL)
                    .expect("canonical exhaustive snapshot must decode");
                assert_eq!(report.snapshot, snapshot);
                assert_eq!(
                    report.heap_words,
                    u64::from(vertices) + 1 + snapshot.targets.len() as u64
                );

                if let Some(previous) = unique_encodings.insert(bytes.clone(), snapshot.clone()) {
                    assert_eq!(
                        previous, snapshot,
                        "one canonical byte string cannot denote different snapshots"
                    );
                }
                encoding_count += 1;

                let mut reordered = raw_edges.clone();
                reordered.reverse();
                reordered.extend(raw_edges.iter().copied());
                if !reordered.is_empty() {
                    reordered.rotate_left(1);
                }
                let reordered_snapshot = canonical_snapshot(vertices, &reordered, profile);
                assert_eq!(snapshot, reordered_snapshot);
                assert_eq!(bytes, encode(&reordered_snapshot));

                for permutation in permutations(vertices) {
                    let renamed = canonical_snapshot(
                        vertices,
                        &renamed_edges(&snapshot, &permutation),
                        profile,
                    );
                    let renamed_bytes = encode(&renamed);
                    let renamed_report =
                        decode(&renamed_bytes, profile, Limits::UNBOUNDED_MODEL)
                            .expect("a lawfully renamed snapshot must decode");
                    assert_eq!(renamed_report.snapshot.edges(), renamed_edges(&snapshot, &permutation));
                    renaming_count += 1;
                }

                for prefix_length in 0..bytes.len() {
                    assert!(
                        decode(
                            &bytes[..prefix_length],
                            profile,
                            Limits::UNBOUNDED_MODEL
                        )
                        .is_err()
                    );
                    truncation_count += 1;
                }
            }
        }
    }

    assert_eq!(graph_count, 531);
    assert_eq!(encoding_count, 1_593);
    assert_eq!(renaming_count, 9_321);
    assert_eq!(unique_encodings.len() as u64, encoding_count);
    assert_deep_small_stack();

    println!(
        "verified {graph_count} directed graphs, {encoding_count} profile-bound encodings, \
         {renaming_count} lawful renamings, {truncation_count} strict-prefix rejections, \
         exact-version compatibility, domain-separated digest invocations, and a \
         100000-vertex 64-KiB-stack lifecycle"
    );
}
