use vstd::prelude::*;

verus! {

pub open spec fn flat_wave_slots(component_count: nat, wave_count: nat) -> nat {
    component_count + wave_count + 1
}

pub open spec fn schedule_work(
    component_count: nat,
    quotient_edges: nat,
    wave_count: nat,
) -> nat {
    6 * component_count + quotient_edges + 3 * wave_count + 1
}

pub open spec fn full_pipeline_work(
    vertices: nat,
    edges: nat,
    radix_work: nat,
    components: nat,
    quotient_edges: nat,
    waves: nat,
) -> nat {
    10 * vertices + 2 * edges + radix_work
        + 14 * components + 4 * quotient_edges + 3 * waves + 4
}

pub open spec fn component_in_wave(ranks: Seq<nat>, component: int, wave: nat) -> bool
    recommends 0 <= component < ranks.len(),
{
    ranks[component] == wave
}

proof fn every_component_has_one_rank_fiber(ranks: Seq<nat>, component: int, wave_count: nat)
    requires
        0 <= component < ranks.len(),
        ranks[component] < wave_count,
    ensures
        component_in_wave(ranks, component, ranks[component]),
        ranks[component] < wave_count,
{
}

proof fn distinct_rank_fibers_are_disjoint(
    ranks: Seq<nat>,
    component: int,
    left_wave: nat,
    right_wave: nat,
)
    requires
        0 <= component < ranks.len(),
        left_wave != right_wave,
    ensures
        !(component_in_wave(ranks, component, left_wave)
            && component_in_wave(ranks, component, right_wave)),
{
}

proof fn flat_storage_is_linear(component_count: nat, wave_count: nat)
    requires
        wave_count <= component_count,
    ensures
        flat_wave_slots(component_count, wave_count) <= 2 * component_count + 1,
{
}

proof fn schedule_charge_is_linear(
    component_count: nat,
    quotient_edges: nat,
    wave_count: nat,
)
    requires
        wave_count <= component_count,
    ensures
        schedule_work(component_count, quotient_edges, wave_count)
            <= 9 * component_count + quotient_edges + 1,
{
}

proof fn schedule_charge_fits_u64(
    component_count: nat,
    quotient_edges: nat,
    wave_count: nat,
)
    requires
        component_count <= 4_294_967_295,
        quotient_edges <= 4_294_967_295,
        wave_count <= component_count,
    ensures
        schedule_work(component_count, quotient_edges, wave_count)
            <= 18_446_744_073_709_551_615,
{
}

proof fn full_pipeline_uniform_bound(
    vertices: nat,
    edges: nat,
    radix_work: nat,
    components: nat,
    quotient_edges: nat,
    waves: nat,
)
    requires
        components <= vertices,
        quotient_edges <= edges,
        waves <= components,
        radix_work <= 14 * edges + 26_624,
    ensures
        full_pipeline_work(vertices, edges, radix_work, components, quotient_edges, waves)
            <= 27 * vertices + 20 * edges + 26_628,
{
}

}

fn main() {}
