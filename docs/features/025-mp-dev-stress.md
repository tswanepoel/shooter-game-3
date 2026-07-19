# Feature 025 - MP dev stress

Development builds stress the multiplayer path so ordering, tick load, and partial delivery show up early. Stress is opt-in or always-on in dev only; production keeps the normal tick and full delivery.

Depends on **022**–**024** (queues and join path exist).

## Server

- Dev server tick rate is **elevated** above the intended production rate (exact Hz is a dev cvar or constant).
- Tick still runs the same pipeline: read inputs → sim apply → broadcast snapshots.
- High rate is load on apply, encode, and client handle — not a second protocol.

## Client

- In dev, the inbound path **discards** a configurable fraction or pattern of successfully decoded S2C messages before apply (or drops by kind/tick).
- Discard sits on the **inbound queue** seam (022): after decode, before session/world apply.
- Outbound and solo paths stay unchanged by discard.
- Session key and seq still follow whatever snapshots **were** applied; stress exercises echo and authority under loss.

## Observation

- Devtools expose stress controls and simple counters (ticks received, discarded, inputs sent, echo rejects) enough to confirm the harness is alive.

## Acceptance criteria

- Dev server can run at an elevated fixed tick rate while joined clients still move under authority.
- Dev client can discard inbound net messages at the queue seam; remaining snapshots still drive local and remote present.
- Stress controls and counters are available from the dev console.
- Production / non-dev builds use the normal tick rate and deliver all decoded messages to apply.
