---- MODULE HermeticCargoMachine ----
EXTENDS Naturals, TLC

CONSTANT Mutant

ASSUME Mutant \in {"None", "AmbientCwd", "DeveloperCargoHome", "CopiedConfig",
                   "RelativeManifest", "ExternalStorage", "NetworkEnabled",
                   "MaskedStatus", "IgnoreLockMutation"}

Phases == {"Prepare", "Execute", "Validate", "Accepted", "Rejected"}

VARIABLES phase,
          lockMutationAttempted,
          cwdOutsideHomeAncestry,
          isolatedCargoHome,
          ambientConfigExcluded,
          absoluteManifest,
          repositoryBackedStorage,
          offlineResolution,
          exitStatusPreserved,
          lockfileUnchanged,
          evidenceAccepted

vars == <<phase, lockMutationAttempted, cwdOutsideHomeAncestry,
          isolatedCargoHome, ambientConfigExcluded, absoluteManifest,
          repositoryBackedStorage, offlineResolution, exitStatusPreserved,
          lockfileUnchanged, evidenceAccepted>>

Init ==
  /\ phase = "Prepare"
  /\ lockMutationAttempted \in BOOLEAN
  /\ cwdOutsideHomeAncestry = FALSE
  /\ isolatedCargoHome = FALSE
  /\ ambientConfigExcluded = FALSE
  /\ absoluteManifest = FALSE
  /\ repositoryBackedStorage = FALSE
  /\ offlineResolution = FALSE
  /\ exitStatusPreserved = FALSE
  /\ lockfileUnchanged = FALSE
  /\ evidenceAccepted = FALSE

Prepare ==
  /\ phase = "Prepare"
  /\ phase' = "Execute"
  /\ cwdOutsideHomeAncestry' = (Mutant # "AmbientCwd")
  /\ isolatedCargoHome' = (Mutant # "DeveloperCargoHome")
  /\ ambientConfigExcluded' = (Mutant # "CopiedConfig")
  /\ absoluteManifest' = (Mutant # "RelativeManifest")
  /\ repositoryBackedStorage' = (Mutant # "ExternalStorage")
  /\ offlineResolution' = (Mutant # "NetworkEnabled")
  /\ UNCHANGED <<lockMutationAttempted, exitStatusPreserved,
                  lockfileUnchanged, evidenceAccepted>>

Execute ==
  /\ phase = "Execute"
  /\ phase' = "Validate"
  /\ exitStatusPreserved' = (Mutant # "MaskedStatus")
  /\ lockfileUnchanged' = ~lockMutationAttempted
  /\ UNCHANGED <<lockMutationAttempted, cwdOutsideHomeAncestry,
                  isolatedCargoHome, ambientConfigExcluded, absoluteManifest,
                  repositoryBackedStorage, offlineResolution, evidenceAccepted>>

AdmissionSatisfied ==
  /\ (cwdOutsideHomeAncestry \/ Mutant = "AmbientCwd")
  /\ (isolatedCargoHome \/ Mutant = "DeveloperCargoHome")
  /\ (ambientConfigExcluded \/ Mutant = "CopiedConfig")
  /\ (absoluteManifest \/ Mutant = "RelativeManifest")
  /\ (repositoryBackedStorage \/ Mutant = "ExternalStorage")
  /\ (offlineResolution \/ Mutant = "NetworkEnabled")
  /\ (exitStatusPreserved \/ Mutant = "MaskedStatus")
  /\ (lockfileUnchanged \/ Mutant = "IgnoreLockMutation")

Validate ==
  /\ phase = "Validate"
  /\ phase' = IF AdmissionSatisfied THEN "Accepted" ELSE "Rejected"
  /\ evidenceAccepted' = AdmissionSatisfied
  /\ UNCHANGED <<lockMutationAttempted, cwdOutsideHomeAncestry,
                  isolatedCargoHome, ambientConfigExcluded, absoluteManifest,
                  repositoryBackedStorage, offlineResolution,
                  exitStatusPreserved, lockfileUnchanged>>

Idle ==
  /\ phase \in {"Accepted", "Rejected"}
  /\ UNCHANGED vars

Next == Prepare \/ Execute \/ Validate \/ Idle

Spec == Init /\ [][Next]_vars

TypeOK ==
  /\ phase \in Phases
  /\ lockMutationAttempted \in BOOLEAN
  /\ cwdOutsideHomeAncestry \in BOOLEAN
  /\ isolatedCargoHome \in BOOLEAN
  /\ ambientConfigExcluded \in BOOLEAN
  /\ absoluteManifest \in BOOLEAN
  /\ repositoryBackedStorage \in BOOLEAN
  /\ offlineResolution \in BOOLEAN
  /\ exitStatusPreserved \in BOOLEAN
  /\ lockfileUnchanged \in BOOLEAN
  /\ evidenceAccepted \in BOOLEAN

AcceptedRunIsHermetic ==
  evidenceAccepted =>
    /\ cwdOutsideHomeAncestry
    /\ isolatedCargoHome
    /\ ambientConfigExcluded
    /\ absoluteManifest
    /\ offlineResolution

AcceptedRunUsesRepositoryStorage ==
  evidenceAccepted => repositoryBackedStorage

AcceptedRunPreservesStatus ==
  evidenceAccepted => exitStatusPreserved

AcceptedEvidencePreservesLockfile ==
  evidenceAccepted => lockfileUnchanged

RejectedRunPublishesNoEvidence ==
  phase = "Rejected" => ~evidenceAccepted

====
