# Architecture

Routerbot follows a hexagonal architecture.

Chat input is normalized by a chat adapter, routed through the application layer,
checked by authorization, and executed through capability traits implemented by
concrete adapters.

The core domain must not depend on Telegram, Transmission, Kubernetes, Keenetic,
shell commands, or runtime-specific infrastructure.
