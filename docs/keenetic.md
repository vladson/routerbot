# Keenetic and Entware

The supported Keenetic target is little-endian MIPS32r2 with the Entware
`mipsel-3.4_kn` package architecture. Routerbot is cross-compiled as a static
`mipsel-unknown-linux-musl` executable, so it does not depend on the router's glibc
2.27 runtime. Telegram and Transmission HTTPS use vendored OpenSSL because Rustls'
default cryptography provider does not support MIPS.

## Build

Install Docker and Cross 0.2.5 on the build host:

```bash
cargo install cross --version 0.2.5 --locked
./scripts/cross-build-keenetic.sh
```

The script builds the default Keenetic runtime, verifies that the ELF is static
little-endian MIPS and that the dependency graph does not contain `ring`, then writes
`dist/routerbot_<version>_mipsel-3.4_kn.ipk`.

## Install

Copy the package to the router and install it without automatically starting the
service:

```sh
/opt/bin/opkg install ./routerbot_0.1.0_mipsel-3.4_kn.ipk
cp /opt/etc/routerbot/routerbot.env.example /opt/etc/routerbot/routerbot.env
chmod 600 /opt/etc/routerbot/routerbot.env
```

Edit `routerbot.env` with the Telegram token. Edit `config.yaml` with at least one
allowed private chat or user identifier; an empty allow-list intentionally rejects
every caller. Add both Transmission credential variables only when its RPC endpoint
requires HTTP authentication.

Validate and start the service:

```sh
/opt/bin/routerbot --check-config --config /opt/etc/routerbot/config.yaml
/opt/etc/init.d/S99routerbot start
/opt/etc/init.d/S99routerbot check
```

The init script uses Entware `rc.func` and supports `start`, `stop`, `restart`,
`check`, and `kill`. It runs as root because `/bin/ndmc` device operations may require
router privileges. The package marks `config.yaml` as a conffile, so `opkg` preserves
local edits during upgrades. The secret environment file is created by the operator
and is never owned or overwritten by the package.

## Device Commands

`/status` invokes `/bin/ndmc -c "show system"` and reports the numeric router uptime.
`/reboot` invokes `/bin/ndmc -c "system reboot"` only after the existing Telegram
confirmation flow succeeds. Commands are passed directly to `ndmc`; no shell or
chat-provided command text is executed.

The router clock and CA bundle must be valid for Telegram HTTPS. The package depends
on Entware `ca-bundle` and defaults `SSL_CERT_FILE` to
`/opt/etc/ssl/certs/ca-certificates.crt`; override it in `routerbot.env` if the local
bundle uses another path.

## Troubleshooting

Run Routerbot in the foreground with `RUST_LOG=debug` to diagnose startup failures:

```sh
RUST_LOG=debug /opt/bin/routerbot --config /opt/etc/routerbot/config.yaml
```

Check `ndmc` independently with:

```sh
/bin/ndmc -c 'show system'
```

Uninstalling Routerbot removes package-owned files. Back up `config.yaml` and remove
`routerbot.env` manually if its secrets are no longer needed.
