---- MODULE InteropCodecMachine ----
EXTENDS Naturals, FiniteSets, TLC

CONSTANTS Requests, InputKinds, MaxWork, MaxHeap, Mutant

ASSUME /\ Requests # {}
       /\ InputKinds =
            {"Valid", "BadMagic", "BadSchema", "BadVersion", "BadProfile",
             "BadLength", "OverBudget", "BadIndex", "NonCanonical",
             "Trailing", "BadDigest"}
       /\ MaxWork = 5
       /\ MaxHeap = 2
       /\ Mutant \in {"None", "SkipSchema", "SkipCanonical", "IgnoreCancellation"}

Phases == {"Reading", "Admitted", "Published", "Rejected", "Released"}
Results == {"None", "Exact", "Rejected"}

VARIABLES phase, stage, inputKind, cancelled, work, heap, result

vars == <<phase, stage, inputKind, cancelled, work, heap, result>>

FailureAt(kind, position) ==
  CASE position = 0 -> kind \in {"BadMagic", "BadSchema"}
    [] position = 1 -> kind \in {"BadVersion", "BadProfile"}
    [] position = 2 -> kind \in {"BadLength", "OverBudget"}
    [] position = 3 -> kind \in {"BadIndex", "NonCanonical"}
    [] position = 4 -> kind \in {"Trailing", "BadDigest"}
    [] OTHER -> FALSE

Bypassed(kind, position) ==
  \/ /\ Mutant = "SkipSchema"
     /\ kind = "BadSchema"
     /\ position = 0
  \/ /\ Mutant = "SkipCanonical"
     /\ kind = "NonCanonical"
     /\ position = 3

Init ==
  /\ inputKind \in [Requests -> InputKinds]
  /\ phase = [request \in Requests |-> "Reading"]
  /\ stage = [request \in Requests |-> 0]
  /\ cancelled = [request \in Requests |-> FALSE]
  /\ work = [request \in Requests |-> 0]
  /\ heap = [request \in Requests |-> 0]
  /\ result = [request \in Requests |-> "None"]

Read(request) ==
  /\ phase[request] = "Reading"
  /\ stage[request] < 5
  /\ LET rejected ==
           FailureAt(inputKind[request], stage[request]) /\
           ~Bypassed(inputKind[request], stage[request])
         nextStage == stage[request] + 1
     IN
       /\ stage' = [stage EXCEPT ![request] = nextStage]
       /\ work' = [work EXCEPT ![request] = @ + 1]
       /\ heap' =
            [heap EXCEPT ![request] =
              IF stage[request] = 2 /\ ~rejected THEN MaxHeap ELSE @]
       /\ phase' =
            [phase EXCEPT ![request] =
              IF rejected
              THEN "Rejected"
              ELSE IF nextStage = 5 THEN "Admitted" ELSE "Reading"]
       /\ result' =
            [result EXCEPT ![request] =
              IF rejected THEN "Rejected" ELSE @]
  /\ UNCHANGED <<inputKind, cancelled>>

Cancel(request) ==
  /\ phase[request] \in {"Reading", "Admitted"}
  /\ cancelled' = [cancelled EXCEPT ![request] = TRUE]
  /\ IF Mutant = "IgnoreCancellation"
     THEN /\ UNCHANGED <<phase, result>>
     ELSE /\ phase' = [phase EXCEPT ![request] = "Rejected"]
          /\ result' = [result EXCEPT ![request] = "Rejected"]
  /\ UNCHANGED <<stage, inputKind, work, heap>>

Publish(request) ==
  /\ phase[request] = "Admitted"
  /\ IF Mutant = "IgnoreCancellation" THEN TRUE ELSE ~cancelled[request]
  /\ phase' = [phase EXCEPT ![request] = "Published"]
  /\ result' = [result EXCEPT ![request] = "Exact"]
  /\ UNCHANGED <<stage, inputKind, cancelled, work, heap>>

Release(request) ==
  /\ phase[request] \in {"Published", "Rejected"}
  /\ phase' = [phase EXCEPT ![request] = "Released"]
  /\ UNCHANGED <<stage, inputKind, cancelled, work, heap, result>>

Idle ==
  /\ \A request \in Requests: phase[request] = "Released"
  /\ UNCHANGED vars

Next ==
  \/ \E request \in Requests:
       Read(request) \/ Cancel(request) \/ Publish(request) \/ Release(request)
  \/ Idle

Spec == Init /\ [][Next]_vars

TypeOK ==
  /\ phase \in [Requests -> Phases]
  /\ stage \in [Requests -> 0..5]
  /\ inputKind \in [Requests -> InputKinds]
  /\ cancelled \in [Requests -> BOOLEAN]
  /\ work \in [Requests -> 0..MaxWork]
  /\ heap \in [Requests -> 0..MaxHeap]
  /\ result \in [Requests -> Results]

CursorBound ==
  \A request \in Requests:
    /\ stage[request] <= 5
    /\ work[request] = stage[request]

ResourceBound ==
  \A request \in Requests:
    /\ work[request] <= MaxWork
    /\ heap[request] <= MaxHeap

AllocationAfterLengthAdmission ==
  \A request \in Requests:
    stage[request] <= 2 => heap[request] = 0

RejectedNeverPublishes ==
  \A request \in Requests:
    result[request] = "Rejected" => result[request] # "Exact"

ExactHasCompleteTape ==
  \A request \in Requests:
    result[request] = "Exact" =>
      /\ stage[request] = 5
      /\ work[request] = MaxWork
      /\ phase[request] \in {"Published", "Released"}

AdmissionSound ==
  \A request \in Requests:
    phase[request] = "Admitted" =>
      /\ inputKind[request] = "Valid"
      /\ ~cancelled[request]

PublicationSound ==
  \A request \in Requests:
    result[request] = "Exact" =>
      /\ inputKind[request] = "Valid"
      /\ ~cancelled[request]

CancellationSticky ==
  \A request \in Requests:
    cancelled[request] => phase[request] # "Reading"

NativeControlDepthBound ==
  \A request \in Requests: 1 = 1

====
