# Witness Resource and Security Contract

## Threat model

An untrusted caller can supply a graph, flat sidecar, path witness, root, target, selection policy,
or resource budget. The caller may attempt out-of-range access, arithmetic overflow, duplicate
amplification, excessive memory use, input-depth stack overflow, nondeterministic evidence, or
promotion of an incomplete result to exact.

Provenance keys are opaque. Their presence does not establish authenticity, freshness, authority,
or source-domain correctness.

## Validate before traversal

Before reading a sidecar member by edge index, validation checks:

- the offset count is exactly $`|E|+1`$;
- the first offset is zero;
- offsets are monotone;
- the terminal offset equals the member count;
- every fiber is strictly sorted and duplicate-free; and
- all integer conversions and allocation sizes fit the configured representation.

Before replaying a path edge, validation checks edge-index range and current-row ownership. The
target is reported exact only after the final current vertex equals the expected target.

## Resource limits

Every input-dependent allocation is derived with checked arithmetic. Limits cover:

- graph vertices and canonical edges;
- provenance members before and after union;
- BFS queue and parent-edge entries;
- witness length;
- reachable predecessor entries;
- link-eval work;
- dominance-frontier output entries; and
- total logical work.

Exhaustion returns `LimitExceeded`. Cancellation returns `Cancelled`. Neither result carries an
exactness bit or an exact witness.

## Stack safety

The following operations must be iterative:

- graph search;
- path reconstruction and replay;
- SCC witness-fiber construction;
- depth-first numbering for dominators;
- link-eval path compression;
- dominator-tree postorder;
- frontier propagation; and
- destruction of returned flat buffers.

The 20,000-vertex 256 KiB-stack model is a release gate. Recursive convenience methods are not an
acceptable substitute for explicit heap control, even if average graphs are shallow.

## Determinism and evidence integrity

Canonical edge order, sorted sidecar fibers, deterministic two-cursor union, and ordered flat
materialization prevent hash-map seeds, worker completion order, or allocation addresses from
changing exact output.

One representative or one shortest path is selected only under an explicit, transported strict
total order. Otherwise the API returns the complete witness fiber or an arbitrary-valid result
without an equivariance claim. This prevents a local CSR index from masquerading as a
representation-independent identity.

## Confidentiality and serialization

The core neither serializes nor hashes provenance. A consumer must not log opaque keys merely
because they appear in a structural witness. Versioned encoding, redaction policy, stable digests,
schema identity, and trust metadata belong to `libvgraph-interop` or a domain adapter.

## Failure atomicity

Validation, budget admission, and cancellation checks occur before publishing an exact result.
Temporary queues, parent maps, ancestor forests, sidecar members, and frontier candidates remain
private until completion. A failed operation drops its workspace and returns a typed non-exact
outcome; it does not expose a partially canonical result as reusable evidence.
