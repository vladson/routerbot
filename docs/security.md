# Security

Routerbot is deny-by-default.

Access must be granted explicitly through configured Telegram chat IDs or user IDs.
Dangerous actions, such as device reboot or workload restart, must require
confirmation and should have cooldowns.

Routerbot must not expose arbitrary shell execution through chat input.
