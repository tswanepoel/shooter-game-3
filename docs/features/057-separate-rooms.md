# Feature 057 - Separate rooms

Each distinct room code is its own **room**: own members, always-on free-for-all **match**, and scores. Members in different rooms never see or fight each other. Display names stay unique within a room only. Same empty **map** and join surface as **051**; `dev` is just one normal code.

Depends on **051**.

## Acceptance criteria

- A non-blank room code joins that room, creating it if needed; a blank code is rejected with a clear reason.
- Different codes: no shared roster, scores, or combat.
- Same code: shared roster, scores, and combat.
- Display-name clash only against current members of that room.
- When the last member leaves, the room is gone; rejoining that code starts fresh (new match, scores at zero).
- Failed join still shows the reason on the join surface.
