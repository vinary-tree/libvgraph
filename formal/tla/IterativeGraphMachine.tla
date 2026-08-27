---- MODULE IterativeGraphMachine ----
EXTENDS Naturals, Sequences, FiniteSets, TLC

CONSTANTS Nodes, Successors(_)

ASSUME /\ IsFiniteSet(Nodes)
       /\ \A node \in Nodes : Successors(node) \subseteq Nodes

VARIABLES phase, discovered, finished, frames

variables == <<phase, discovered, finished, frames>>

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

Begin ==
    /\ phase = "Ready"
    /\ phase' = "Running"
    /\ UNCHANGED <<discovered, finished, frames>>

StartRoot ==
    /\ phase = "Running"
    /\ frames = <<>>
    /\ discovered # Nodes
    /\ \E root \in Nodes \ discovered :
        /\ discovered' = discovered \cup {root}
        /\ frames' = <<root>>
        /\ UNCHANGED <<phase, finished>>

ExpandFrame ==
    /\ phase = "Running"
    /\ frames # <<>>
    /\ \E successor \in Successors(Top(frames)) \ discovered :
        /\ discovered' = discovered \cup {successor}
        /\ frames' = Append(frames, successor)
        /\ UNCHANGED <<phase, finished>>

FinishFrame ==
    /\ phase = "Running"
    /\ frames # <<>>
    /\ Successors(Top(frames)) \subseteq discovered
    /\ finished' = finished \cup {Top(frames)}
    /\ frames' = SubSeq(frames, 1, Len(frames) - 1)
    /\ UNCHANGED <<phase, discovered>>

Complete ==
    /\ phase = "Running"
    /\ frames = <<>>
    /\ discovered = Nodes
    /\ phase' = "Done"
    /\ UNCHANGED <<discovered, finished, frames>>

Cancel ==
    /\ phase \in {"Ready", "Running"}
    /\ phase' = "Cancelled"
    /\ UNCHANGED <<discovered, finished, frames>>

Next == Begin \/ StartRoot \/ ExpandFrame \/ FinishFrame \/ Complete \/ Cancel
Spec == Init /\ [][Next]_variables

TypeInvariant ==
    /\ phase \in {"Ready", "Running", "Done", "Cancelled"}
    /\ discovered \subseteq Nodes
    /\ finished \subseteq discovered
    /\ frames \in Seq(Nodes)

FrameOwnership == SequenceSet(frames) \subseteq discovered
FrameUniqueness == UniqueSequence(frames)
ExplicitFrameBound == Len(frames) <= Cardinality(Nodes)
DoneIsComplete == phase = "Done" => finished = Nodes
CancelledIsNotDone == phase = "Cancelled" => phase # "Done"

MCNodes == 0..4
MCSuccessors(node) ==
    CASE node = 0 -> {1}
      [] node = 1 -> {0, 2}
      [] node = 2 -> {3}
      [] node = 3 -> {4}
      [] OTHER -> {}

====
