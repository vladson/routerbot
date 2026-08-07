# Security

Routerbot is deny-by-default.

Access must be granted explicitly through configured Telegram chat IDs or user IDs.
An update is allowed when either its chat ID or sender user ID is configured. An
allowed chat therefore grants access to every user in that chat. Empty allow lists
deny all access.

Dangerous actions require confirmation. Device reboot and removal of downloaded
local data use single-use Telegram inline confirmations that are bound to the
requesting chat and user and expire after two minutes. Confirmation state is kept
in memory and is discarded when Routerbot restarts. Cooldowns may be added in a
later stage.

Routerbot must not expose arbitrary shell execution through chat input.
