# Routerbot vendor note

This directory contains `transmission-rpc` 0.5.0 from crates.io.

Routerbot changes only the crate's Reqwest feature wiring: the original Rustls
dependency is now an explicit default feature, and a `native-tls` alternative is
available for targets that `ring` does not support. Source APIs and behavior are
otherwise unchanged.

Upstream: <https://github.com/j0rsa/transmission-rpc>
License: MIT
