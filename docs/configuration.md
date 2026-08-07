# Configuration

Routerbot configuration describes adapter instances and capability bindings.

Secrets must be read from environment variables or protected files. Sensitive values
must not be logged or rendered in debug output.

The current configuration boundary validates the top-level deployment shape:

- `bot.kind` must be `telegram`.
- `bot.mode` must be `long_polling`.
- `bot.token_env` must name an environment variable and must not contain a token value.
- `auth.allow_chats` contains signed Telegram chat identifiers and
  `auth.allow_users` contains unsigned Telegram user identifiers. A caller is
  allowed when either identifier matches; negative user identifiers are invalid.
- `adapters` contains typed, named adapter instances. The Keenetic runtime supports
  `transmission.rpc` and `keenetic.local`. The latter accepts an absolute `ndmc_path`
  and defaults to `/bin/ndmc`.
- `capabilities.downloads` must reference a `transmission.rpc` adapter.
- `capabilities.device` must reference a `keenetic.local` adapter.
- `capabilities.media_index` must reference a `keenetic.dlna` adapter.
- Environment-variable references are validated at parse time and resolved separately
  at runtime. Resolved values use redacted debug output.

The Keenetic example connects to Transmission over its local RPC endpoint and binds
device control to the router-local Keenetic CLI. Adapter implementations determine
the exact runtime operations; configuration never accepts arbitrary chat-provided
commands.

The initial Transmission adapter targets the legacy protocol used by Transmission
3.00 / RPC 16 on Keenetic. It supports magnet and torrent URL additions, download
status listing, stopping torrents, and removing torrents by their stable hash.
Removal explicitly distinguishes preserving local data from deleting it; data
deletion is a dangerous action that requires confirmation at the chat boundary.
HTTP Basic credentials are installed as a sensitive HTTP header and are never stored
in the underlying RPC client's loggable authentication type. JSON-RPC 2.0 support for
Transmission 4.1 and later is a future compatibility stage.

Example configuration files live under `examples/`.

The executable accepts `--config PATH`; otherwise it reads `ROUTERBOT_CONFIG` and
then falls back to `/opt/etc/routerbot/config.yaml`. Use `--check-config` to load the
file, resolve referenced secrets, and construct enabled adapters without contacting
Telegram. Transmission `username_env` and `password_env` must either both be set or
both be omitted.

## Telegram Commands

The Telegram adapter supports:

```text
/start
/help
/status
/downloads
/add <magnet-or-http(s)-url>
/stop <download-id>
/remove <download-id> [--delete-data]
/rescan
/reboot
```

`/reboot` and `/remove <download-id> --delete-data` require confirmation through
inline buttons. The initial adapter uses explicit download identifiers; interactive
download selection and torrent file uploads are not yet supported.
