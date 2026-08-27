---- MODULE IterativeGraphMachine ----
EXTENDS Naturals, Sequences, FiniteSets, TLC

CONSTANTS Nodes, Successors(_)

ASSUME /\ IsFiniteSet(Nodes)
       /\ \A node \in Nodes : Successors(node) \subseteq Nodes

VARIABLES phase, discovered, finished, frames, inspected, work

variables == <<phase, discovered, finished, frames, inspected, work>>

Edges == {edge \in Nodes \X Nodes : edge[2] \in Successors(edge[1])}

SequenceSet(sequence) == {sequence[index] : index \in 1..Len(sequence)}

UniqueSequence(sequence) ==
    \A left, right \in 1..Len(sequence) :
        sequence[left] = sequence[right] => left = right

Top(sequence) == sequence[Len(sequence)]

Init ==
    /\ phase = "Ready"
    /\ discovered = {}
    /\ finished = {}
    /\ frames = <<>>
    /\ inspected = {}
    /\ work = 0

Begin ==
    /\ phase = "Ready"
    /\ phase' = "Running"
    /\ UNCHANGED <<discovered, finished, frames, inspected, work>>

StartRoot ==
    /\ phase = "Running"
    /\ frames = <<>>
    /\ discovered # Nodes
    /\ \E root \in Nodes \ discovered :
        /\ discovered' = discovered \cup {root}
        /\ frames' = <<root>>
        /\ work' = work + 1
        /\ UNCHANGED <<phase, finished, inspected>>

InspectFrame ==
    /\ phase = "Running"
    /\ frames # <<>>
    /\ \E successor \in Successors(Top(frames)) :
        LET edge == <<Top(frames), successor>> IN
        /\ edge \notin inspected
        /\ inspected' = inspected \cup {edge}
        /\ IF successor \notin discovered
              THEN /\ discovered' = discovered \cup {successor}
                   /\ frames' = Append(frames, successor)
                   /\ work' = work + 2
              ELSE /\ UNCHANGED <<discovered, frames>>
                   /\ work' = work + 1
        /\ UNCHANGED <<phase, finished>>

FinishFrame ==
    /\ phase = "Running"
    /\ frames # <<>>
    /\ {<<Top(frames), successor>> :
          successor \in Successors(Top(frames))} \subseteq inspected
    /\ finished' = finished \cup {Top(frames)}
    /\ frames' = SubSeq(frames, 1, Len(frames) - 1)
    /\ work' = work + 1
    /\ UNCHANGED <<phase, discovered, inspected>>

Complete ==
    /\ phase = "Running"
    /\ frames = <<>>
    /\ discovered = Nodes
    /\ phase' = "Done"
    /\ UNCHANGED <<discovered, finished, frames, inspected, work>>

Cancel ==
    /\ phase \in {"Ready", "Running"}
    /\ phase' = "Cancelled"
    /\ UNCHANGED <<discovered, finished, frames, inspected, work>>

Next == Begin \/ StartRoot \/ InspectFrame \/ FinishFrame \/ Complete \/ Cancel
Spec == Init /\ [][Next]_variables

TypeInvariant ==
    /\ phase \in {"Ready", "Running", "Done", "Cancelled"}
    /\ discovered \subseteq Nodes
    /\ finished \subseteq discovered
    /\ frames \in Seq(Nodes)
    /\ inspected \subseteq Edges
    /\ work \in Nat

FrameOwnership == SequenceSet(frames) \subseteq discovered
FrameUniqueness == UniqueSequence(frames)
ExplicitFrameBound == Len(frames) <= Cardinality(Nodes)
FinishedFrameDisjointness == SequenceSet(frames) \cap finished = {}
InspectedSourceOwnership ==
    \A edge \in inspected : edge[1] \in discovered
WorkAccounting ==
    work = Cardinality(discovered) + Cardinality(inspected) + Cardinality(finished)
LinearWorkBound == work <= 2 * Cardinality(Nodes) + Cardinality(Edges)
DoneIsComplete == phase = "Done" => finished = Nodes
DoneScannedEveryEdge == phase = "Done" => inspected = Edges
DoneWorkIsExact ==
    phase = "Done" => work = 2 * Cardinality(Nodes) + Cardinality(Edges)
CancelledIsNotDone == phase = "Cancelled" => phase # "Done"

MCNodes == 0..4
MCSuccessors(node) ==
    CASE node = 0 -> {1}
      [] node = 1 -> {0, 2}
      [] node = 2 -> {3}
      [] node = 3 -> {4}
      [] OTHER -> {}

====
