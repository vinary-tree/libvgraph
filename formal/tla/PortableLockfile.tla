---- MODULE PortableLockfile ----
EXTENDS FiniteSets

CONSTANTS Lockfiles, RequireEnumeration, RequireReadable
ASSUME /\ Lockfiles # {} /\ IsFiniteSet(Lockfiles)
       /\ RequireEnumeration \in BOOLEAN /\ RequireReadable \in BOOLEAN

VARIABLES ambient, unreadable, enumerated, enumeration, certified, published
vars == <<ambient, unreadable, enumerated, enumeration, certified, published>>

\* Lockfiles is the actual tracked set. A successful Git enumeration returns all of it;
\* a failed enumeration may already have emitted any subset, including a nonempty one.
Init ==
  /\ ambient \in SUBSET Lockfiles
  /\ unreadable \in SUBSET Lockfiles
  /\ enumerated = {}
  /\ enumeration = "NotRun"
  /\ certified = FALSE
  /\ published = FALSE

EnumerateSuccess ==
  /\ ~published
  /\ enumeration' = "Success"
  /\ enumerated' = Lockfiles
  /\ certified' = FALSE
  /\ UNCHANGED <<ambient, unreadable, published>>

EnumerateFailure(subset) ==
  /\ ~published
  /\ subset \in SUBSET Lockfiles
  /\ enumeration' = "Failure"
  /\ enumerated' = subset
  /\ certified' = FALSE
  /\ UNCHANGED <<ambient, unreadable, published>>

IntroducePatch(file) ==
  /\ ~published
  /\ file \in Lockfiles \ ambient
  /\ ambient' = ambient \cup {file}
  /\ certified' = FALSE
  /\ UNCHANGED <<unreadable, enumerated, enumeration, published>>

RemovePatch(file) ==
  /\ ~published
  /\ file \in ambient
  /\ ambient' = ambient \ {file}
  /\ certified' = FALSE
  /\ UNCHANGED <<unreadable, enumerated, enumeration, published>>

IntroduceReadError(file) ==
  /\ ~published
  /\ file \in Lockfiles \ unreadable
  /\ unreadable' = unreadable \cup {file}
  /\ certified' = FALSE
  /\ UNCHANGED <<ambient, enumerated, enumeration, published>>

ClearReadError(file) ==
  /\ ~published
  /\ file \in unreadable
  /\ unreadable' = unreadable \ {file}
  /\ certified' = FALSE
  /\ UNCHANGED <<ambient, enumerated, enumeration, published>>

\* The two Boolean constants exist only for causal mutant configurations.
\* The production configuration requires both checks before certification.
Certify ==
  /\ ~published
  /\ enumeration # "NotRun"
  /\ enumerated # {}
  /\ ambient \cap enumerated = {}
  /\ (RequireEnumeration => enumeration = "Success")
  /\ (RequireReadable => unreadable \cap enumerated = {})
  /\ certified' = TRUE
  /\ UNCHANGED <<ambient, unreadable, enumerated, enumeration, published>>

Publish ==
  /\ ~published
  /\ certified
  /\ published' = TRUE
  /\ UNCHANGED <<ambient, unreadable, enumerated, enumeration, certified>>

Idle == published /\ UNCHANGED vars

Next ==
  \/ EnumerateSuccess
  \/ \E subset \in SUBSET Lockfiles : EnumerateFailure(subset)
  \/ \E file \in Lockfiles : IntroducePatch(file)
  \/ \E file \in Lockfiles : RemovePatch(file)
  \/ \E file \in Lockfiles : IntroduceReadError(file)
  \/ \E file \in Lockfiles : ClearReadError(file)
  \/ Certify
  \/ Publish
  \/ Idle

Spec == Init /\ [][Next]_vars

TypeOK ==
  /\ ambient \in SUBSET Lockfiles
  /\ unreadable \in SUBSET Lockfiles
  /\ enumerated \in SUBSET Lockfiles
  /\ enumeration \in {"NotRun", "Success", "Failure"}
  /\ certified \in BOOLEAN
  /\ published \in BOOLEAN

CertifiedHasCompleteEnumeration ==
  certified => enumeration = "Success" /\ enumerated = Lockfiles
CertifiedHasReadableLockfiles == certified => unreadable = {}
CertifiedHasNoAmbientPatch == certified => ambient = {}
PublishedOnlyAfterCertificate == published => certified
PublishedHasPortableSource ==
  published => enumeration = "Success" /\ enumerated = Lockfiles
               /\ unreadable = {} /\ ambient = {}

====
