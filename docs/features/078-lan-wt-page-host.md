# Feature 078 - LAN WebTransport without PUBLIC_HOST

Dev clients dial the **page hostname** on the game-server port. The host auto-covers local LAN IPs on the self-signed cert. No `GAME_SERVER_PUBLIC_HOST` for the usual LAN case.

Depends on **034** (WebTransport join / `wt-identity`).

## Behaviour

- `debug/wt-identity.json` carries `port` + `hash_sha256` only (no advertised URL).
- Client builds `https://{location.hostname}:{port}/` and pins the hash.
- Server cert SANs: `localhost`, `127.0.0.1`, `::1`, plus the primary outbound LAN IPv4 (UDP connect trick). Optional `GAME_SERVER_PUBLIC_HOST` (comma-separated) appends more SANs.
- Bind stays `0.0.0.0:4433` by default.

## Acceptance

- `cargo run -p game-server` then `npm run dev:lan` → remote PC on `https://<lan-ip>:3000/` can `mp join` without setting env vars.
- Localhost join still works.
- Missing identity file still fails join with a clear fetch error.
