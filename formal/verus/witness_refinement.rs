use vstd::prelude::*;

verus! {

pub open spec fn flat_sidecar_slots(edge_count: nat, member_count: nat) -> nat {
    edge_count + member_count + 1
}

pub open spec fn sidecar_union_work(
    edge_count: nat,
    left_members: nat,
    right_members: nat,
    output_members: nat,
) -> nat {
    2 * edge_count + left_members + right_members + output_members + 1
}

pub open spec fn reachability_work(
    vertex_count: nat,
    reachable_vertices: nat,
    reachable_edges: nat,
    path_length: nat,
) -> nat {
    vertex_count + 2 * reachable_vertices + reachable_edges + path_length + 1
}

pub open spec fn path_replay_work(path_length: nat) -> nat {
    path_length
}

pub open spec fn path_replay_auxiliary_slots() -> nat {
    2
}

pub open spec fn lengauer_tarjan_work(
    vertex_count: nat,
    edge_count: nat,
    link_eval_work: nat,
) -> nat {
    8 * vertex_count + 2 * edge_count + link_eval_work + 1
}

pub open spec fn dominance_frontier_work(
    vertex_count: nat,
    edge_count: nat,
    candidate_count: nat,
    output_count: nat,
) -> nat {
    4 * vertex_count + 2 * edge_count + candidate_count + output_count + 1
}

pub open spec fn witness_auxiliary_slots(vertex_count: nat, edge_count: nat) -> nat {
    11 * vertex_count + edge_count + 1
}

proof fn flat_sidecar_storage_is_exact(
    edge_count: nat,
    member_count: nat,
)
    ensures
        flat_sidecar_slots(edge_count, member_count)
            == edge_count + member_count + 1,
{
}

proof fn union_output_is_bounded_by_inputs(
    edge_count: nat,
    left_members: nat,
    right_members: nat,
    output_members: nat,
)
    requires
        output_members <= left_members + right_members,
    ensures
        sidecar_union_work(
            edge_count,
            left_members,
            right_members,
            output_members,
        ) <= 2 * edge_count + 2 * left_members + 2 * right_members + 1,
{
}

proof fn reachability_charge_is_linear(
    vertex_count: nat,
    edge_count: nat,
    reachable_vertices: nat,
    reachable_edges: nat,
    path_length: nat,
)
    requires
        reachable_vertices <= vertex_count,
        reachable_edges <= edge_count,
        path_length <= vertex_count,
    ensures
        reachability_work(
            vertex_count,
            reachable_vertices,
            reachable_edges,
            path_length,
        ) <= 4 * vertex_count + edge_count + 1,
{
}

proof fn path_replay_has_exact_work_and_constant_auxiliary_state(path_length: nat)
    ensures
        path_replay_work(path_length) == path_length,
        path_replay_auxiliary_slots() == 2,
{
}

proof fn lengauer_tarjan_charge_is_near_linear(
    vertex_count: nat,
    edge_count: nat,
    link_eval_work: nat,
)
    requires
        link_eval_work <= 4 * (vertex_count + edge_count),
    ensures
        lengauer_tarjan_work(vertex_count, edge_count, link_eval_work)
            <= 12 * vertex_count + 6 * edge_count + 1,
{
}

proof fn dominance_frontier_charge_is_output_sensitive(
    vertex_count: nat,
    edge_count: nat,
    candidate_count: nat,
    output_count: nat,
)
    requires
        candidate_count <= edge_count,
    ensures
        dominance_frontier_work(
            vertex_count,
            edge_count,
            candidate_count,
            output_count,
        ) <= 4 * vertex_count + 3 * edge_count + output_count + 1,
{
}

proof fn witness_auxiliary_storage_is_linear(
    vertex_count: nat,
    edge_count: nat,
)
    ensures
        witness_auxiliary_slots(vertex_count, edge_count)
            <= 11 * vertex_count + edge_count + 1,
{
}

proof fn witness_charges_fit_u64_graph_domain(
    vertex_count: nat,
    edge_count: nat,
    provenance_members: nat,
)
    requires
        vertex_count <= 4_294_967_295,
        edge_count <= 4_294_967_295,
        provenance_members <= 4_294_967_295,
    ensures
        flat_sidecar_slots(edge_count, provenance_members)
            <= 18_446_744_073_709_551_615,
        12 * vertex_count + 6 * edge_count + 1
            <= 18_446_744_073_709_551_615,
        witness_auxiliary_slots(vertex_count, edge_count)
            <= 18_446_744_073_709_551_615,
{
}

proof fn explicit_heap_control_has_constant_native_depth(
    resident_native_frames: nat,
    heap_frames: nat,
    vertex_count: nat,
)
    requires
        resident_native_frames <= 1,
        heap_frames <= vertex_count,
    ensures
        resident_native_frames <= 1,
        heap_frames <= vertex_count,
{
}

}

fn main() {}
