---- MODULE ReleaseMachine ----
EXTENDS Naturals, TLC

CONSTANT Mutant

ASSUME Mutant \in {"None", "CandidatePolicy", "SkipProtectedHead", "SkipGates",
                   "PublishEarly", "SkipEvidence", "SkipRegistryChecksum", "Republish"}

Phases == {"Verify", "CandidateVerified", "GatesPassed", "Draft", "AssetsAttached",
           "RegistryVerified", "Published", "Rejected"}
RegistryStates == {"Absent", "Matching", "Mismatched"}
Hashes == {"None", "CandidateArtifact", "OtherArtifact"}

VARIABLES phase,
          protectedSignerAccepts,
          candidateSignerAccepts,
          candidateAtProtectedHead,
          gatesSucceed,
          registryState,
          gatesRecorded,
          draftCreated,
          packageHash,
          assetHash,
          evidenceComplete,
          registryHash,
          published,
          publicationCount

vars == <<phase, protectedSignerAccepts, candidateSignerAccepts,
          candidateAtProtectedHead, gatesSucceed, registryState, gatesRecorded,
          draftCreated, packageHash, assetHash, evidenceComplete, registryHash,
          published, publicationCount>>

Inputs == <<protectedSignerAccepts, candidateSignerAccepts,
            candidateAtProtectedHead, gatesSucceed, registryState>>

Init ==
  /\ phase = "Verify"
  /\ protectedSignerAccepts \in BOOLEAN
  /\ candidateSignerAccepts \in BOOLEAN
  /\ candidateAtProtectedHead \in BOOLEAN
  /\ gatesSucceed \in BOOLEAN
  /\ registryState \in RegistryStates
  /\ gatesRecorded = FALSE
  /\ draftCreated = FALSE
  /\ packageHash = "None"
  /\ assetHash = "None"
  /\ evidenceComplete = FALSE
  /\ registryHash = "None"
  /\ published = FALSE
  /\ publicationCount = 0

VerifyCandidate ==
  /\ phase = "Verify"
  /\ LET signerAccepted ==
            IF Mutant = "CandidatePolicy"
            THEN candidateSignerAccepts
            ELSE protectedSignerAccepts
         headAccepted ==
            candidateAtProtectedHead \/ Mutant = "SkipProtectedHead"
     IN phase' =
          IF signerAccepted /\ headAccepted
          THEN "CandidateVerified"
          ELSE "Rejected"
  /\ UNCHANGED <<Inputs, gatesRecorded, draftCreated, packageHash, assetHash,
                  evidenceComplete, registryHash, published, publicationCount>>

RunGates ==
  /\ phase = "CandidateVerified"
  /\ phase' =
       IF gatesSucceed \/ Mutant = "SkipGates"
       THEN "GatesPassed"
       ELSE "Rejected"
  /\ gatesRecorded' = gatesSucceed
  /\ UNCHANGED <<Inputs, draftCreated, packageHash, assetHash, evidenceComplete,
                  registryHash, published, publicationCount>>

CreateDraft ==
  /\ phase = "GatesPassed"
  /\ phase' = "Draft"
  /\ draftCreated' = TRUE
  /\ UNCHANGED <<Inputs, gatesRecorded, packageHash, assetHash, evidenceComplete,
                  registryHash, published, publicationCount>>

AttachAssets ==
  /\ phase = "Draft"
  /\ phase' = "AssetsAttached"
  /\ packageHash' = "CandidateArtifact"
  /\ assetHash' = "CandidateArtifact"
  /\ evidenceComplete' = (Mutant # "SkipEvidence")
  /\ UNCHANGED <<Inputs, gatesRecorded, draftCreated, registryHash, published,
                  publicationCount>>

VerifyRegistry ==
  /\ phase = "AssetsAttached"
  /\ registryHash' =
       IF registryState = "Mismatched"
       THEN "OtherArtifact"
       ELSE "CandidateArtifact"
  /\ phase' =
       IF registryState = "Mismatched" /\ Mutant # "SkipRegistryChecksum"
       THEN "Rejected"
       ELSE "RegistryVerified"
  /\ UNCHANGED <<Inputs, gatesRecorded, draftCreated, packageHash, assetHash,
                  evidenceComplete, published, publicationCount>>

Publish ==
  /\ phase = "RegistryVerified"
  /\ phase' = "Published"
  /\ published' = TRUE
  /\ publicationCount' = publicationCount + 1
  /\ UNCHANGED <<Inputs, gatesRecorded, draftCreated, packageHash, assetHash,
                  evidenceComplete, registryHash>>

PublishEarly ==
  /\ Mutant = "PublishEarly"
  /\ phase = "GatesPassed"
  /\ phase' = "Published"
  /\ published' = TRUE
  /\ publicationCount' = publicationCount + 1
  /\ UNCHANGED <<Inputs, gatesRecorded, draftCreated, packageHash, assetHash,
                  evidenceComplete, registryHash>>

Republish ==
  /\ Mutant = "Republish"
  /\ phase = "Published"
  /\ publicationCount' = publicationCount + 1
  /\ UNCHANGED <<phase, Inputs, gatesRecorded, draftCreated, packageHash,
                  assetHash, evidenceComplete, registryHash, published>>

Idle ==
  /\ phase \in {"Published", "Rejected"}
  /\ UNCHANGED vars

Next ==
  \/ VerifyCandidate
  \/ RunGates
  \/ CreateDraft
  \/ AttachAssets
  \/ VerifyRegistry
  \/ Publish
  \/ PublishEarly
  \/ Republish
  \/ Idle

Spec == Init /\ [][Next]_vars

TypeOK ==
  /\ phase \in Phases
  /\ protectedSignerAccepts \in BOOLEAN
  /\ candidateSignerAccepts \in BOOLEAN
  /\ candidateAtProtectedHead \in BOOLEAN
  /\ gatesSucceed \in BOOLEAN
  /\ registryState \in RegistryStates
  /\ gatesRecorded \in BOOLEAN
  /\ draftCreated \in BOOLEAN
  /\ packageHash \in Hashes
  /\ assetHash \in Hashes
  /\ evidenceComplete \in BOOLEAN
  /\ registryHash \in Hashes
  /\ published \in BOOLEAN
  /\ publicationCount \in 0..2

PublishedUsesProtectedTrust ==
  published => protectedSignerAccepts

PublishedUsesProtectedHead ==
  published => candidateAtProtectedHead

PublishedHasPassedGates ==
  published => gatesSucceed /\ gatesRecorded

PublishedFromDraft ==
  published => draftCreated

PublishedHasCompleteAssets ==
  published =>
    /\ packageHash = "CandidateArtifact"
    /\ assetHash = packageHash
    /\ evidenceComplete

PublishedRegistryMatches ==
  published => registryHash = packageHash

AtMostOnePublication ==
  publicationCount <= 1

RejectedNeverPublishes ==
  phase = "Rejected" => ~published

====
