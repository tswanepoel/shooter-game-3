# Feature 064 - Room leader, FFA map, and match start

On join, the server elects the **room leader** for that **room**. The client learns whether it is **room leader** from server roster truth, not from join order or local inference; it does not know why or when the server elected. The **room leader** picks the **map** and starts the **free-for-all** **match**. Today both are implicit presentation actions with no picker UI: the room leader’s client auto-picks **map** `a` and starts the **match**. **Map** `a` is a first-class cooked map (not today’s implicit empty scene or debug grid): a very empty place with one shipment-container-sized landmark so load honours map id.

Depends on **051**, **057**, **room leader**, **match**, **map**, and **free-for-all** in [concepts](../concepts.md).

## Acceptance criteria

- Server elects **room leader**; election policy is server-owned (client does not infer it).
- Roster tells the local member whether they are **room leader**.
- **Room leader** client performs implicit map pick (`map a`) and **free-for-all** **match** start with no UI.
- A **match** may be unstarted until the **room leader** starts it.
- Non-leaders do not pick the **map** or start the **match**.
- Started **match** loads **map** `a` from the map catalog; scene content is not hard-wired to the legacy empty scene.
