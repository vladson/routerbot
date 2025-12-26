# routerbot

`routerbot` is a lightweight, self-hosted chat-bot for home automation control — focused on **local-first** operations and **safe-by-default** administration.

The initial target is a Telegram bot running on (or near) a home router / homelab node. The project is designed around a small **core + providers + frontends** architecture so it can grow beyond a single device or chat platform.

## Why

Home automation often becomes cloud-dependent, slow, and hard to debug. `routerbot` aims to make common "home ops" actions:

- fast (LAN / local APIs),
- scriptable (predictable commands),
- safe (explicit authorization rules),
- deployable on constrained devices (router/Entware, small Linux boxes, NAS).

## Scope (v1 focus)

We start with three capabilities:

1. **DLNA library refresh**
    - Trigger a DLNA rescan on the router (Keenetic via `ndmc` initially).

2. **Transmission downloads management**
    - List current downloads with status.
    - Add new download (magnet / URL).
    - Add a torrent by sending a `.torrent` file.
    - Remove a download (optionally purge local data).

3. **Router reboot**
    - Trigger a controlled reboot from the bot (provider-specific implementation).

> More devices (OpenWrt, MikroTik, generic Linux) and more frontends (Discord/CLI) are planned, but out of scope for the first release.

## Status

**Planning / scaffold stage.**  
This README describes the intended layout and milestones.

## Roadmap (milestones)

### Milestone 0 — Repository scaffold
- [ ] Cargo workspace structure (`core`, `providers/*`, `frontends/*`)
- [ ] Dual-license MIT OR Apache-2.0
- [ ] CI: `fmt`, `clippy`, `test`
- [ ] Example config + security notes

### Milestone 1 — Core API (provider traits + use-cases)
- [ ] Domain types: downloads, media library, router control
- [ ] Error model (typed errors)
- [ ] Auth policy (deny-by-default, allowlist by user/chat)
- [ ] Use-cases layer: list/add/remove/rescan/reboot (no Telegram types)

### Milestone 2 — Providers (initial set)
- [ ] `provider-transmission` (RPC client: list/add/remove/purge)
- [ ] `provider-keenetic` (DLNA rescan via `ndmc`)
- [ ] `provider-keenetic` (router reboot)

### Milestone 3 — Telegram frontend (v1 bot)
- [ ] `/whoami` (bootstrap identity)
- [ ] `/list` + inline buttons for delete/purge
- [ ] `/add <magnet|url>`
- [ ] `.torrent` document upload support
- [ ] `/rescan` (DLNA rescan)
- [ ] `/reboot` (with confirmation)
- [ ] Friendly error messages (no raw stack traces)

### Milestone 4 — Packaging & deployment
- [ ] Entware/OPKG-friendly install notes (Keenetic)
- [ ] Init script example for `/opt/etc/init.d`
- [ ] Minimal runtime footprint build profile (LTO, strip)

### Milestone 5 — Quality & hardening
- [ ] Integration tests (mock providers)
- [ ] Rate limiting / spam protection
- [ ] Safer destructive actions (two-step confirm / time-limited tokens)

### Later (not v1)
- [ ] Discord frontend
- [ ] CLI frontend (`routerbotctl`)
- [ ] OpenWrt provider
- [ ] Generic Linux provider (systemctl + local services)
- [ ] Plugin-like provider registry (if needed)

## Architecture

`routerbot` is split into two axes:

- **Frontends**: user interaction (Telegram now; Discord/CLI later)
- **Providers**: integrations with devices/services (Transmission, Keenetic/ndmc, etc.)

The `core` crate defines **traits** and **use-cases** so frontends and providers stay decoupled.

## Security model

`routerbot` is **deny-by-default**.

- If no allowlist is configured, all commands (except optional `/whoami`) are rejected.
- Access can be granted by:
    - Telegram `user_id` allowlist
    - Telegram `chat_id` allowlist (for a trusted private group)
- Destructive actions (purge/reboot) should require explicit confirmation.

See `docs/security.md` for the threat model and recommended setup.

## Telegram commands (planned)

- `/help` — help
- `/whoami` — print `user_id` and `chat_id` for configuration
- `/list` — list current downloads (with inline actions)
- `/add <magnet|url>` — add a download
- (send `.torrent` file) — add a torrent upload
- `/del <id>` — remove torrent (keep data)
- `/purge <id>` — remove torrent and delete local data
- `/rescan` — trigger DLNA rescan
- `/reboot` — reboot router (confirmation required)

## Configuration (planned)

Configuration will be loaded from a TOML file (path via `ROUTERBOT_CONFIG`) plus environment variables for secrets.

Example file: `configs/routerbot.example.toml`

Key points:
- Telegram token is read from `TELOXIDE_TOKEN` (environment)
- Allowlist must be explicitly set
- Providers are optional; you can enable only what you use

## Running (planned)

High-level development flow:

1. Create a Telegram bot with BotFather and export `TELOXIDE_TOKEN`.
2. Configure allowlist:
    - start the bot and run `/whoami`
    - copy your `user_id` and/or `chat_id` into config
3. Run `routerbot` in foreground with logs enabled.

Deployment on Keenetic/Entware will be documented in `docs/keenetic.md`.

## Non-goals (for v1)

- Cloud connectivity or cloud-only features
- Auto-discovering devices on the network without explicit config
- Executing arbitrary shell commands from chat (intentionally not supported)

## Contributing

Contributions are welcome once the scaffold lands.

Planned contribution workflow:
- small PRs (one feature / one fix)
- `cargo fmt`, `cargo clippy`, `cargo test` must be green
- security-related changes should include threat/abuse analysis

## License

Licensed under **MIT OR Apache-2.0**.

