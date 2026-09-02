# Snapshot security and resource safety

Portable graph bytes are untrusted until admission completes. This document defines the threat
model, mandatory controls, and residual risks for `libvgraph-interop`.

## Threat model

An attacker may control every snapshot byte, declared count, version field, flag bit, semantic
profile, cache key, transport boundary, truncation point, and request timing. Inputs may be tiny,
deep, wide, duplicate-heavy before canonical construction, intentionally noncanonical, stale, or
selected to maximize work. Concurrent requests may race with cancellation.

The attacker cannot change the compiled schema identity or digest context without changing the
program. The digest is not an authenticity mechanism unless the expected value arrives through an
authenticated channel; an attacker who controls bytes and expected digest can replace both.

## Security properties

### Fail-closed admission

Exact publication is the conjunction of all independent checks:

```math
P = M \land S \land V \land F \land R \land L \land B
    \land C \land D \land \neg X.
```

The symbols mean magic `M`, schema `S`, version and flags `V` and
`F`, semantic profile `R`, exact length `L`, resource budget
`B`, canonical CSR `C`, expected digest `D`, and cancellation
`X`. Every symbol is defined before it is used in the formula, and every false conjunct
prevents publication.

TLC checks the complete finite interleaving model. Three causal mutants remove schema checking,
canonicality checking, and cancellation enforcement; each produces a concrete
`PublicationSound` counterexample.

### Allocation safety

The decoder reads the fixed header into scalars before allocating. It rejects counts above
`SnapshotLimits`, checked-length overflow, a declared payload mismatch, an actual-length
mismatch, and a host-address-space conversion failure. Only then may it allocate exactly
`V + 1 + E` 32-bit words.

Configured byte limits apply to the complete slice before digesting. This prevents an attacker
from forcing unbounded BLAKE3 work merely by supplying a syntactically plausible header.

### Work and stack bounds

For a structurally valid candidate, decoder validation performs no more than:

```math
W(V,E) = 8 + 2(V + 1) + 3E
```

logical operations in the formal cost model. Digest verification adds one linear streaming pass
over the exact bytes. Both are $`O(V + E)`$.

All cursors, offsets, targets, errors, and BLAKE3 state are heap or fixed local values. Graph depth
never adds a native call frame. The executable refinement exercises 100,000 vertices on a 64 KiB
thread stack through build, encode, decode, comparison, error unwinding, and destruction.

### Canonicality

The decoder rejects:

- a missing or nonzero origin offset;
- decreasing offsets;
- a terminal offset different from the edge count;
- any target outside the vertex domain;
- duplicate or decreasing targets within a row;
- truncation; and
- trailing bytes.

It never normalizes untrusted wire input. Normalization could allow multiple hostile byte strings
to acquire one trusted identity and conceal an upstream producer defect.

### Digest use

BLAKE3 derive-key mode provides application-specific domain separation
([formal specification](https://github.com/BLAKE3-team/BLAKE3-specs/blob/master/blake3.tex)).
The code must compare all 32 digest bytes without an early data-dependent exit when the comparison
protects a security boundary. Structural proofs establish distinct tagged preimages, not
mathematical collision impossibility.

For collision-intolerant evidence, retain and compare complete canonical bytes in addition to the
digest. For untrusted-network authenticity, protect the expected digest with an authenticated
signature, message authentication code, or authenticated transport. Those mechanisms are outside
this crate.

## Concurrency and cancellation

Requests do not share mutable decoder state. A caller may execute them concurrently without a
codec-global mutex. Cancellation is first-writer sticky at the coordinating layer: once observed,
the request cannot publish. A cancellation check belongs before expensive digesting, during
chunked long-running work when the API accepts a cancellation source, and immediately before
publication.

The initial synchronous codec does not spawn threads. This makes CPU, memory, and task budgets
caller-governed and avoids nested executor oversubscription. Parallel request scheduling can use
schedlib or another injected executor.

## Error information

Typed errors may report field, byte offset, expected value, observed value, and configured limit.
They must not include unrelated memory, secret profile construction inputs, authentication
credentials, or the complete hostile payload by default. Display text is diagnostic; callers
must match stable error variants rather than parse messages.

## Residual risks

- A valid maximum-size input can still consume its configured linear work and memory budget.
- Digest collision resistance depends on BLAKE3 rather than the structural proofs.
- A semantic profile is only as accurate as the domain adapter that constructs it.
- Dense identifier renaming changes bytes and digests unless the transported graph is identical.
- Exact v1.0 rejection of later schemas requires an explicit migration deployment before readers
  can consume new bytes.

These are explicit boundary conditions, not silent approximations.
