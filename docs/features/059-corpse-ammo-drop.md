# Feature 059 - Corpse and ammo drop

On [death](../concepts.md#death), the dead [figure](../concepts.md#figure) becomes a [corpse](../concepts.md#corpse): the same Kenney body that was the remote (or self), after the `die` clip, held on its back. Separately, death may pin an invisible [ammo drop](../concepts.md#ammo-drop) at that same world address. **Corpse ≠ loot.** They are two things that happen to share a place.

Walk-over takes the drop when the living player has room in [reserve ammo](../concepts.md#reserve-ammo). The client **claims**; the server **elects** one winner and **grants** them reserve.

Depends on **043** (death / `die` hold), **053** (death → bench; corpse stays in the world), **057** (room-scoped server), **058** (magazine, reserve, reload).

## Corpse

| Fact | Draft |
|------|--------|
| What | The character kit body already in present, after **043** `die`, holding the last frame on its back |
| Where | Pose at death; does not walk |
| Lifetime | Ends after a time (draft: long enough to loot; table in code) |
| Bench | Local player still returns to the **053** loadout bench; the corpse remains present for others (and for walk-over) until it ends |

## Ammo drop

| Fact | Draft |
|------|--------|
| What | Invisible loot: an ammo kind and a round count |
| Where | Pinned to the corpse’s world position (same address; not the corpse itself) |
| When | Spawned when death is accepted, if the dead life still held rounds to dump |
| Payload | One kind — the active blaster’s ammo at death. Count = remaining magazine of that blaster + remaining reserve of that kind. Both are cleared from the dead life into the drop. Zero rounds → no drop |
| Lifetime | Ends when granted, when its rounds are gone, when the corpse ends, or when its own timer ends — whichever first |

## Walk-over

A living player overlapping the drop’s take radius (draft metres in code), with reserve room for at least one round of that kind, may take.

- Overlap is enough.
- Client sends a **loot claim** for that drop (drop id + room membership).
- Claim only when reserve has room for that kind.

How much one take moves: as many rounds as fit in reserve capacity for that kind, up to the drop’s remaining count. Partial takes leave the rest on the drop until empty or lifetime end.

## Claim, elect, grant

Same honesty pattern as fire hits, but the **server** picks the winner for the shared pile.

| Word | Who | Meaning |
|------|-----|--------|
| **Loot claim** | Client → server | “I overlapped this drop and have reserve room.” |
| **Elect** | Server | Among claims for that drop in the room, choose one winner (draft: first valid claim). |
| **Loot grant** | Server → clients | Winner receives those rounds into **reserve** of that ammo kind; the drop loses them (and ends if empty). |

Only the elected player receives the grant. Stale claims (drop already gone, claimant dead or not in room, reserve full) are ignored.

Solo / offline: the local sim may grant on overlap without a wire round-trip; counts still match **058**.

## Wire (draft)

- On accepted death the room broadcasts `CorpseSpawn`; corpse ends with `CorpseEnd` on the room timer.
- Victim sends `AmmoDump { ammo, rounds, position }` once; when rounds are above zero the room mints a drop and broadcasts `AmmoDropSpawn`.
- `LootClaim { drop_id, position, room }` from living members; server elects using claim pose and free reserve room; `LootGrant` + optional `AmmoDropEnd`.

Protocol bump as usual for this alpha.

## Acceptance criteria

- Dead body is the existing kit after `die`, held on its back; that is the corpse.
- Ammo drop is invisible loot pinned at the corpse address.
- Death with leftover active-kind rounds spawns one drop; magazine and that kind’s reserve on the dead life are emptied into it.
- Living walk-over with reserve room sends a loot claim; server elects one winner and loot-grants reserve; drop shrinks or ends.
- Take requires reserve room for that kind.
- Corpse and drop end on their timers; grant can end the drop earlier.
- Dev HUD may reflect granted reserve.
