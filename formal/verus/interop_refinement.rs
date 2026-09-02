use vstd::prelude::*;

verus! {

pub open spec fn wire_bytes(vertices: nat, edges: nat) -> nat {
    80 + 4 * (vertices + 1 + edges)
}

pub open spec fn decoder_heap_words(vertices: nat, edges: nat) -> nat {
    vertices + 1 + edges
}

pub open spec fn decoder_work_bound(vertices: nat, edges: nat) -> nat {
    8 + 2 * (vertices + 1) + 3 * edges
}

pub open spec fn admitted(
    vertices: nat,
    edges: nat,
    bytes: nat,
    maximum_vertices: nat,
    maximum_edges: nat,
    maximum_bytes: nat,
) -> bool {
    vertices <= maximum_vertices
        && edges <= maximum_edges
        && bytes == wire_bytes(vertices, edges)
        && bytes <= maximum_bytes
}

proof fn maximum_v1_wire_length_fits_u64(vertices: nat, edges: nat)
    requires
        vertices <= 4_294_967_295,
        edges <= 4_294_967_295,
    ensures
        wire_bytes(vertices, edges) <= 18_446_744_073_709_551_615,
{
}

proof fn maximum_v1_heap_words_fit_u64(vertices: nat, edges: nat)
    requires
        vertices <= 4_294_967_295,
        edges <= 4_294_967_295,
    ensures
        decoder_heap_words(vertices, edges) <= 18_446_744_073_709_551_615,
{
}

proof fn admission_is_fail_closed(
    vertices: nat,
    edges: nat,
    bytes: nat,
    maximum_vertices: nat,
    maximum_edges: nat,
    maximum_bytes: nat,
)
    requires
        admitted(
            vertices,
            edges,
            bytes,
            maximum_vertices,
            maximum_edges,
            maximum_bytes,
        ),
    ensures
        vertices <= maximum_vertices,
        edges <= maximum_edges,
        bytes <= maximum_bytes,
        bytes == wire_bytes(vertices, edges),
{
}

proof fn decoder_cursor_step_is_bounded(cursor: nat, length: nat)
    requires
        cursor < length,
    ensures
        cursor + 1 <= length,
        length - (cursor + 1) < length - cursor,
{
}

proof fn validation_work_is_linear(vertices: nat, edges: nat)
    ensures
        decoder_work_bound(vertices, edges)
            == 10 + 2 * vertices + 3 * edges,
{
}

proof fn fixed_header_prevents_empty_encoding(vertices: nat, edges: nat)
    ensures
        wire_bytes(vertices, edges) >= 84,
{
}

}

fn main() {}
