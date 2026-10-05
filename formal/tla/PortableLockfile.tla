---- MODULE PortableLockfile ----
EXTENDS FiniteSets

CONSTANT Lockfiles
ASSUME /\ Lockfiles # {} /\ IsFiniteSet(Lockfiles)

VARIABLES ambient, certified, published
vars == <<ambient, certified, published>>

\* An ambient Cargo patch may be present before verification begins.
Init ==
  /\ ambient \in SUBSET Lockfiles
  /\ certified = FALSE
  /\ published = FALSE

\* Any change invalidates an earlier clean-source certificate.
IntroducePatch(file) ==
  /\ ~published
  /\ file \in Lockfiles \ ambient
  /\ ambient' = ambient \cup {file}
  /\ certified' = FALSE
  /\ UNCHANGED published

RemovePatch(file) ==
  /\ ~published
  /\ file \in ambient
  /\ ambient' = ambient \ {file}
  /\ certified' = FALSE
  /\ UNCHANGED published

Certify ==
  /\ ~published
  /\ ambient = {}
  /\ certified' = TRUE
  /\ UNCHANGED <<ambient, published>>

Publish ==
  /\ ~published
  /\ certified
  /\ ambient = {}
  /\ published' = TRUE
  /\ UNCHANGED <<ambient, certified>>

Idle == published /\ UNCHANGED vars

Next ==
  \/ \E file \in Lockfiles : IntroducePatch(file)
  \/ \E file \in Lockfiles : RemovePatch(file)
  \/ Certify
  \/ Publish
  \/ Idle

Spec == Init /\ [][Next]_vars

TypeOK ==
  /\ ambient \in SUBSET Lockfiles
  /\ certified \in BOOLEAN
  /\ published \in BOOLEAN

CertifiedImpliesClean == certified => ambient = {}
PublishedImpliesClean == published => ambient = {}
PublishedImpliesCertified == published => certified

====
