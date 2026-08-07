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
- `adapters` contains typed, named adapter instances. The Keenetic milestone supports
  `transmission.rpc` and `keenetic.dlna`.
- `capabilities.downloads` must reference a `transmission.rpc` adapter.
- `capabilities.media_index` must reference a `keenetic.dlna` adapter.
- Environment-variable references are validated at parse time and resolved separately
  at runtime. Resolved values use redacted debug output.

The Keenetic example connects to Transmission over its local RPC endpoint and binds
media indexing to the router-local DLNA service. Adapter implementations determine
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
