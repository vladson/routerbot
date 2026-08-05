# Configuration

Routerbot configuration describes adapter instances and capability bindings.

Secrets must be read from environment variables or protected files. Sensitive values
must not be logged or rendered in debug output.

The current configuration boundary validates the top-level deployment shape:

- `bot.kind` must be `telegram`.
- `bot.mode` must be `long_polling`.
- `bot.token_env` must name an environment variable and must not contain a token value.
- `auth.allow_chats` and `auth.allow_users` are allow lists of Telegram identifiers.
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

Example configuration files live under `examples/`.
