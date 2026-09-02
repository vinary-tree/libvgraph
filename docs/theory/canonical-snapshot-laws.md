# Canonical snapshot laws

The snapshot boundary uses categorical language only where it yields executable laws. Public APIs
retain graph and codec terminology; no runtime `Category`, `Monad`, or
`Fibration` trait is introduced.

## Objects and morphisms

An **object** in this local model is a validated canonical dense graph paired with a semantic
profile identity. A **morphism** is a total structure-preserving transformation with an executable
law. The relevant transformations are:

- encoding from a canonical graph/profile pair to versioned bytes;
- successful decoding from admitted bytes back to a canonical graph;
- a bijective dense-vertex renaming applied to every edge; and
- explicit migration between two recognized wire schemas.

This follows the ordinary category-theoretic discipline that morphisms compose and preserve
specified structure, without claiming that every Rust function is a morphism. For background on
categories, functors, and natural transformations, see Mac Lane,
*Categories for the Working Mathematician*
([Springer book record](https://doi.org/10.1007/978-1-4612-9839-7)).

## Partial isomorphism

Let $`C_p`$ be the set of canonical dense graphs under semantic profile $`p`$, and
let $`B_{1,p}`$ be the set of byte strings admitted by the exact version 1.0 decoder under
that profile. Encoding $`E_p`$ and decoding $`D_p`$ satisfy:

```math
D_p \circ E_p = \mathrm{id}_{C_p}.
```

Canonical uniqueness strengthens this to the admitted image:

```math
E_p(G_1) = E_p(G_2) \Longrightarrow G_1 = G_2.
```

The decoder is partial over arbitrary bytes because malformed or incompatible input has no graph
image. In Rust this is represented by a typed `Result`, not by inventing a bottom graph
or normalizing hostile bytes.

## Indexed families

Semantic profiles partition otherwise identical graph bytes into an indexed family:

```math
\{C_p\}_{p \in P}.
```

For a fixed profile $`p`$, the admissible byte set $`B_{1,p}`$ is a fiber of the
profile projection. This is a useful fiber interpretation: decoding with the wrong expected
profile rejects rather than crossing fibers.

The campaign does **not** call this projection a fibration. A categorical fibration would require
a defined cartesian lifting operation and laws for reindexing graph meaning along profile
morphisms. No such operation is needed or claimed. If a domain adapter later supplies lawful
profile migration, that adapter can formalize the additional structure without changing the
neutral codec.

## Renaming equivariance

Let `r` be a bijection on the finite dense vertex domain. The renamed graph
`r_*G` maps every edge `(u,v)` to `(r(u),r(v))` and then restores
canonical row order. The codec law is:

```math
D_p(E_p(r_*G)) = r_*G.
```

This is equivariance: the transformation is preserved through the round trip. It is not byte
invariance:

```math
E_p(r_*G) = E_p(G)
```

is generally false. Requiring it would turn the codec into a graph-isomorphism canonical-labeling
algorithm, changing both semantics and complexity.

## Enumeration quotient

Raw edge lists are quotiented by permutation and duplication during canonical
`libvgraph` construction. If `q` maps a raw enumeration to its canonical CSR,
then:

```math
q(L) = q(\mathrm{permute}(L))
      = q(L \mathbin{+\!\!+} L).
```

Encoding factors through this quotient, which explains why raw insertion order and duplicates do
not change bytes. The codec does not perform the quotient itself; it requires a validated
canonical core object and revalidates at the trust boundary.

## Composition

The useful composition is:

```text
raw edges → canonical graph → snapshot bytes → admitted graph → graph analysis
```

Each arrow has a witnessable contract, and the composite preserves the canonical graph. Explicit
schema migration composes similarly:

```text
v1 bytes → validated graph → v2 bytes
```

Unknown bytes never compose directly into a later encoder.

## Monoids and monads

Byte concatenation is associative with an empty byte string, so bytes form a monoid. That monoid
does not preserve snapshot validity: concatenating two valid snapshots produces trailing bytes and
must be rejected. Therefore no snapshot-composition API is derived from byte concatenation.

Disjoint graph union can form a monoidal structure only after specifying identifier shifts,
profile compatibility, edge behavior, and canonical ordering. It is outside this codec contract.

Rust `Result` supports monadic-style error composition, and implementations may use
`?` internally. This does not justify a public monad abstraction. Typed errors and
fail-closed sequencing express the required semantics more directly and with no dynamic
indirection.

## Digest morphism boundary

The digest function maps a domain-tagged preimage to 32 bytes. Formal models prove separation at
the preimage constructor:

```math
(d,s,p,n,b) = (d',s',p',n',b')
\Longrightarrow
d=d' \land s=s' \land p=p' \land n=n' \land b=b'.
```

They do not prove the finite BLAKE3 output injective. A digest is an efficient content address and
stale-data guard, while equality of complete canonical bytes remains the exact witness when a
collision cannot be tolerated.
