---- MODULE BorrowedCsrMachine ----
EXTENDS Integers, Naturals, Sequences, FiniteSets, TLC

CONSTANTS VertexCount, Offsets, Targets, Mutant

ASSUME /\ VertexCount \in Nat
       /\ Offsets \in Seq(Int)
       /\ Targets \in Seq(Int)
       /\ Mutant \in {
            "None",
            "Header",
            "Offset",
            "Target",
            "Order",
            "Duplicate",
            "EarlyPublish"
          }

Vertices == 0..(VertexCount - 1)
Positions == 0..(Len(Targets) - 1)

OffsetAt(vertex) ==
    IF vertex + 1 \in DOMAIN Offsets
    THEN Offsets[vertex + 1]
    ELSE -1

TargetAt(position) ==
    IF position + 1 \in DOMAIN Targets
    THEN Targets[position + 1]
    ELSE -1

HeaderCanonical ==
    /\ Len(Offsets) = VertexCount + 1
    /\ OffsetAt(0) = 0
    /\ OffsetAt(VertexCount) = Len(Targets)

RowCanonical(vertex) ==
    /\ OffsetAt(vertex) \in 0..Len(Targets)
    /\ OffsetAt(vertex + 1) \in 0..Len(Targets)
    /\ OffsetAt(vertex) <= OffsetAt(vertex + 1)

TargetRangeCanonical(position) ==
    /\ position \in Positions
    /\ TargetAt(position) \in Vertices

TargetOrderCanonical(position, rowStart) ==
    \/ position = rowStart
    \/ TargetAt(position - 1) < TargetAt(position)

RowsCanonical == \A vertex \in Vertices : RowCanonical(vertex)
TargetsCanonical == \A position \in Positions : TargetRangeCanonical(position)
OrderingCanonical ==
    \A vertex \in Vertices :
      \A position \in OffsetAt(vertex)..(OffsetAt(vertex + 1) - 1) :
        TargetOrderCanonical(position, OffsetAt(vertex))

CanonicalInput ==
    /\ HeaderCanonical
    /\ RowsCanonical
    /\ TargetsCanonical
    /\ OrderingCanonical

VARIABLES
    phase,
    vertex,
    cursor,
    rowStart,
    rowStop,
    checkedRows,
    rangeChecked,
    orderChecked,
    indexed,
    published,
    work

variables ==
    <<phase, vertex, cursor, rowStart, rowStop, checkedRows,
      rangeChecked, orderChecked, indexed, published, work>>

Init ==
    /\ phase = "Ready"
    /\ vertex = 0
    /\ cursor = 0
    /\ rowStart = 0
    /\ rowStop = 0
    /\ checkedRows = {}
    /\ rangeChecked = {}
    /\ orderChecked = {}
    /\ indexed = {}
    /\ published = FALSE
    /\ work = 0

CheckHeader ==
    /\ phase = "Ready"
    /\ work' = work + 1
    /\ IF Mutant = "EarlyPublish"
          THEN /\ phase' = "NeedRow"
               /\ published' = TRUE
               /\ UNCHANGED <<vertex, cursor, rowStart, rowStop, checkedRows,
                               rangeChecked, orderChecked, indexed>>
          ELSE IF HeaderCanonical \/ Mutant = "Header"
          THEN /\ phase' = "NeedRow"
               /\ UNCHANGED <<vertex, cursor, rowStart, rowStop, checkedRows,
                               rangeChecked, orderChecked, indexed, published>>
          ELSE /\ phase' = "Rejected"
               /\ UNCHANGED <<vertex, cursor, rowStart, rowStop, checkedRows,
                               rangeChecked, orderChecked, indexed, published>>

CheckRow ==
    /\ phase = "NeedRow"
    /\ vertex \in Vertices
    /\ work' = work + 1
    /\ IF RowCanonical(vertex) \/ Mutant = "Offset"
          THEN /\ phase' = "Scanning"
               /\ rowStart' = OffsetAt(vertex)
               /\ rowStop' = OffsetAt(vertex + 1)
               /\ cursor' = OffsetAt(vertex)
               /\ checkedRows' = checkedRows \cup {vertex}
               /\ UNCHANGED <<vertex, rangeChecked, orderChecked, indexed,
                               published>>
          ELSE /\ phase' = "Rejected"
               /\ UNCHANGED <<vertex, cursor, rowStart, rowStop, checkedRows,
                               rangeChecked, orderChecked, indexed, published>>

CheckTarget ==
    /\ phase = "Scanning"
    /\ cursor < rowStop
    /\ work' = work + 1
    /\ LET rangeOk == TargetRangeCanonical(cursor)
           orderOk == TargetOrderCanonical(cursor, rowStart)
           accepted ==
             /\ rangeOk \/ Mutant = "Target"
             /\ orderOk \/ Mutant \in {"Order", "Duplicate"}
       IN
       /\ IF accepted
             THEN /\ phase' = "TargetChecked"
                  /\ rangeChecked' =
                       IF rangeOk THEN rangeChecked \cup {cursor}
                       ELSE rangeChecked
                  /\ orderChecked' =
                       IF orderOk THEN orderChecked \cup {cursor}
                       ELSE orderChecked
                  /\ UNCHANGED <<vertex, cursor, rowStart, rowStop, checkedRows,
                                  indexed, published>>
             ELSE /\ phase' = "Rejected"
                  /\ UNCHANGED <<vertex, cursor, rowStart, rowStop, checkedRows,
                                  rangeChecked, orderChecked, indexed, published>>

IndexTarget ==
    /\ phase = "TargetChecked"
    /\ phase' = "Scanning"
    /\ indexed' = indexed \cup {cursor}
    /\ cursor' = cursor + 1
    /\ UNCHANGED <<vertex, rowStart, rowStop, checkedRows, rangeChecked,
                    orderChecked, published, work>>

FinishRow ==
    /\ phase = "Scanning"
    /\ cursor >= rowStop
    /\ phase' = "NeedRow"
    /\ vertex' = vertex + 1
    /\ UNCHANGED <<cursor, rowStart, rowStop, checkedRows, rangeChecked,
                    orderChecked, indexed, published, work>>

Publish ==
    /\ phase = "NeedRow"
    /\ vertex = VertexCount
    /\ phase' = "Done"
    /\ published' = TRUE
    /\ UNCHANGED <<vertex, cursor, rowStart, rowStop, checkedRows, rangeChecked,
                    orderChecked, indexed, work>>

Cancel ==
    /\ phase \in {"Ready", "NeedRow", "Scanning", "TargetChecked"}
    /\ phase' = "Cancelled"
    /\ published' = FALSE
    /\ UNCHANGED <<vertex, cursor, rowStart, rowStop, checkedRows, rangeChecked,
                    orderChecked, indexed, work>>

Next ==
    CheckHeader \/ CheckRow \/ CheckTarget \/ IndexTarget \/ FinishRow \/
    Publish \/ Cancel

Spec == Init /\ [][Next]_variables

TypeInvariant ==
    /\ phase \in {
         "Ready", "NeedRow", "Scanning", "TargetChecked", "Done", "Rejected",
         "Cancelled"
       }
    /\ vertex \in Nat
    /\ cursor \in Int
    /\ rowStart \in Int
    /\ rowStop \in Int
    /\ checkedRows \subseteq Vertices
    /\ rangeChecked \subseteq Positions
    /\ orderChecked \subseteq Positions
    /\ indexed \subseteq Positions
    /\ published \in BOOLEAN
    /\ work \in Nat

CheckedBeforeIndexed ==
    indexed \subseteq (rangeChecked \cap orderChecked)

CheckedRowBeforeIndexed ==
    \A position \in indexed :
      \E source \in checkedRows :
        position \in OffsetAt(source)..(OffsetAt(source + 1) - 1)

NoPartialPublication ==
    published =>
      /\ phase = "Done"
      /\ checkedRows = Vertices
      /\ indexed = Positions

PublishedInputIsCanonical == published => CanonicalInput
RejectedPublishesNothing == phase = "Rejected" => ~published
CancelledPublishesNothing == phase = "Cancelled" => ~published
ValidationWorkLinear == work <= 1 + VertexCount + Len(Targets)
DoneWorkExact == phase = "Done" => work = 1 + VertexCount + Len(Targets)

MCVertexCount == 4
MCOffsets == <<0, 2, 3, 4, 5>>
MCTargets == <<1, 3, 2, 0, 3>>
MCNoMutant == "None"

HeaderMutantVertexCount == 2
HeaderMutantOffsets == <<0, 0, 0>>
HeaderMutantTargets == <<1>>
HeaderMutantName == "Header"

OffsetMutantVertexCount == 3
OffsetMutantOffsets == <<0, 1, 0, 1>>
OffsetMutantTargets == <<0>>
OffsetMutantName == "Offset"

TargetMutantVertexCount == 1
TargetMutantOffsets == <<0, 1>>
TargetMutantTargets == <<1>>
TargetMutantName == "Target"

OrderMutantVertexCount == 2
OrderMutantOffsets == <<0, 2, 2>>
OrderMutantTargets == <<1, 0>>
OrderMutantName == "Order"

DuplicateMutantVertexCount == 1
DuplicateMutantOffsets == <<0, 2>>
DuplicateMutantTargets == <<0, 0>>
DuplicateMutantName == "Duplicate"

PublicationMutantVertexCount == 2
PublicationMutantOffsets == <<0, 1, 1>>
PublicationMutantTargets == <<1>>
PublicationMutantName == "EarlyPublish"

====
