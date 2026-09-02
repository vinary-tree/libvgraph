---- MODULE WitnessMachine ----
EXTENDS Naturals, Sequences, FiniteSets, TLC

CONSTANTS Nodes, EdgeIds, Source(_), Target(_), Root, Goal, Budgets

ASSUME /\ IsFiniteSet(Nodes)
       /\ IsFiniteSet(EdgeIds)
       /\ Root \in Nodes
       /\ Goal \in Nodes
       /\ Budgets \subseteq Nat
       /\ Budgets # {}
       /\ \A edge \in EdgeIds :
            /\ Source(edge) \in Nodes
            /\ Target(edge) \in Nodes

VARIABLES phase, queue, head, discovered, scanned, parent,
          current, witness, work, budget

variables ==
  <<phase, queue, head, discovered, scanned, parent,
    current, witness, work, budget>>

TerminalPhases ==
  {"Exact", "Unreachable", "Invalid", "LimitExceeded", "Cancelled"}

ActivePhases == {"Searching", "Reconstructing"}

SetOfSequence(sequence) ==
  {sequence[index] : index \in 1..Len(sequence)}

UniqueSequence(sequence) ==
  \A left, right \in 1..Len(sequence) :
    sequence[left] = sequence[right] => left = right

Outgoing(node) == {edge \in EdgeIds : Source(edge) = node}

Minimum(edges) ==
  CHOOSE edge \in edges :
    \A alternative \in edges : edge <= alternative

Only(edges) == CHOOSE edge \in edges : TRUE

ParentEdges == UNION {parent[node] : node \in Nodes}

RECURSIVE Replay(_, _)
Replay(vertex, path) ==
  IF path = <<>>
    THEN vertex
    ELSE LET edge == Head(path) IN
      IF /\ edge \in EdgeIds
         /\ Source(edge) = vertex
        THEN Replay(Target(edge), Tail(path))
        ELSE "InvalidReplay"

Init ==
  /\ phase = "Ready"
  /\ queue = <<>>
  /\ head = 1
  /\ discovered = {}
  /\ scanned = {}
  /\ parent = [node \in Nodes |-> {}]
  /\ current = Root
  /\ witness = <<>>
  /\ work = 0
  /\ budget = 0

Begin ==
  /\ phase = "Ready"
  /\ \E selected_budget \in Budgets :
      /\ phase' = "Searching"
      /\ queue' = <<Root>>
      /\ head' = 1
      /\ discovered' = {Root}
      /\ budget' = selected_budget
      /\ UNCHANGED <<scanned, parent, current, witness, work>>

BeginReconstruction ==
  /\ phase = "Searching"
  /\ Goal \in discovered
  /\ phase' = "Reconstructing"
  /\ current' = Goal
  /\ witness' = <<>>
  /\ UNCHANGED
       <<queue, head, discovered, scanned, parent, work, budget>>

InspectEdge ==
  /\ phase = "Searching"
  /\ Goal \notin discovered
  /\ head <= Len(queue)
  /\ work < budget
  /\ LET node == queue[head]
         remaining == Outgoing(node) \ scanned
     IN
       /\ remaining # {}
       /\ LET edge == Minimum(remaining)
              successor == Target(edge)
          IN
            /\ scanned' = scanned \cup {edge}
            /\ work' = work + 1
            /\ IF successor \in discovered
                 THEN /\ UNCHANGED <<queue, discovered, parent>>
                 ELSE /\ discovered' = discovered \cup {successor}
                      /\ queue' = Append(queue, successor)
                      /\ parent' =
                           [parent EXCEPT ![successor] = {edge}]
            /\ UNCHANGED
                 <<phase, head, current, witness, budget>>

FinishRow ==
  /\ phase = "Searching"
  /\ Goal \notin discovered
  /\ head <= Len(queue)
  /\ work < budget
  /\ Outgoing(queue[head]) \subseteq scanned
  /\ head' = head + 1
  /\ work' = work + 1
  /\ UNCHANGED
       <<phase, queue, discovered, scanned, parent,
         current, witness, budget>>

ReportUnreachable ==
  /\ phase = "Searching"
  /\ Goal \notin discovered
  /\ head > Len(queue)
  /\ phase' = "Unreachable"
  /\ UNCHANGED
       <<queue, head, discovered, scanned, parent,
         current, witness, work, budget>>

ReconstructEdge ==
  /\ phase = "Reconstructing"
  /\ current # Root
  /\ work < budget
  /\ Cardinality(parent[current]) = 1
  /\ LET edge == Only(parent[current]) IN
       /\ current' = Source(edge)
       /\ witness' = <<edge>> \o witness
       /\ work' = work + 1
       /\ UNCHANGED
            <<phase, queue, head, discovered, scanned, parent, budget>>

ReportExact ==
  /\ phase = "Reconstructing"
  /\ current = Root
  /\ Replay(Root, witness) = Goal
  /\ phase' = "Exact"
  /\ UNCHANGED
       <<queue, head, discovered, scanned, parent,
         current, witness, work, budget>>

ReportInvalid ==
  /\ phase = "Reconstructing"
  /\ current # Root
  /\ Cardinality(parent[current]) # 1
  /\ phase' = "Invalid"
  /\ UNCHANGED
       <<queue, head, discovered, scanned, parent,
         current, witness, work, budget>>

ExhaustBudget ==
  /\ phase \in ActivePhases
  /\ work = budget
  /\ phase' = "LimitExceeded"
  /\ UNCHANGED
       <<queue, head, discovered, scanned, parent,
         current, witness, work, budget>>

Cancel ==
  /\ phase \in {"Ready", "Searching", "Reconstructing"}
  /\ phase' = "Cancelled"
  /\ UNCHANGED
       <<queue, head, discovered, scanned, parent,
         current, witness, work, budget>>

Next ==
  \/ Begin
  \/ BeginReconstruction
  \/ InspectEdge
  \/ FinishRow
  \/ ReportUnreachable
  \/ ReconstructEdge
  \/ ReportExact
  \/ ReportInvalid
  \/ ExhaustBudget
  \/ Cancel

Spec == Init /\ [][Next]_variables

TypeInvariant ==
  /\ phase \in {"Ready", "Searching", "Reconstructing"}
                  \cup TerminalPhases
  /\ queue \in Seq(Nodes)
  /\ head \in Nat
  /\ discovered \subseteq Nodes
  /\ scanned \subseteq EdgeIds
  /\ parent \in [Nodes -> SUBSET EdgeIds]
  /\ current \in Nodes
  /\ witness \in Seq(EdgeIds)
  /\ work \in Nat
  /\ budget \in Nat

QueueUniqueness == UniqueSequence(queue)

QueueDiscoveryAgreement == SetOfSequence(queue) = discovered

QueueBound == Len(queue) <= Cardinality(Nodes)

HeadBound == head <= Len(queue) + 1

RootOwnership ==
  phase \in {"Searching", "Reconstructing", "Exact",
             "Unreachable", "Invalid", "LimitExceeded"} =>
    Root \in discovered

ScannedSourceOwnership ==
  \A edge \in scanned : Source(edge) \in discovered

ParentUniqueness ==
  \A node \in Nodes : Cardinality(parent[node]) <= 1

ParentValidity ==
  \A node \in Nodes :
    \A edge \in parent[node] :
      /\ edge \in scanned
      /\ Target(edge) = node
      /\ node \in discovered
      /\ node # Root
      /\ Source(edge) \in discovered

ReconstructedSuffixValid ==
  phase \in {"Reconstructing", "Exact"} =>
    Replay(current, witness) = Goal

ExactWitnessValid ==
  phase = "Exact" => Replay(Root, witness) = Goal

NoFalseUnreachable ==
  phase = "Unreachable" =>
    /\ Goal \notin discovered
    /\ head > Len(queue)

LimitIsNotExact == phase = "LimitExceeded" => phase # "Exact"

CancellationIsNotExact == phase = "Cancelled" => phase # "Exact"

WorkWithinBudget ==
  phase \in ActivePhases => work <= budget

LinearWorkBound ==
  work <= Cardinality(EdgeIds) + 2 * Cardinality(Nodes)

LinearHeapBound ==
  Len(queue) + Cardinality(scanned) +
    Cardinality(ParentEdges) + Len(witness)
    <= 3 * Cardinality(Nodes) + Cardinality(EdgeIds)

MCNodes == 0..5
MCEdgeIds == 0..5
MCSource(edge) ==
  CASE edge = 0 -> 0
    [] edge = 1 -> 0
    [] edge = 2 -> 1
    [] edge = 3 -> 2
    [] edge = 4 -> 3
    [] OTHER -> 1
MCTarget(edge) ==
  CASE edge = 0 -> 1
    [] edge = 1 -> 2
    [] edge = 2 -> 3
    [] edge = 3 -> 3
    [] edge = 4 -> 4
    [] OTHER -> 0
MCBudgets == {3, 20}

====
