# Architecture

Routerbot follows a hexagonal architecture.

Chat input is normalized by a chat adapter, routed through the application layer,
checked by authorization, and executed through capability traits implemented by
concrete adapters.

The core domain must not depend on Telegram, Transmission, Kubernetes, Keenetic,
shell commands, or runtime-specific infrastructure.

## Command Model

Chat adapters normalize user input into `BotCommand` values. A `BotCommand`
represents intent only; it does not contain Telegram update data, external client
types, or adapter-specific request structures.

The application layer uses each command's required permission and action risk to
perform authorization and confirmation before routing the command to a capability.
Capabilities are implemented by adapters outside the core crate.
