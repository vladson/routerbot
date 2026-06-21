#!/usr/bin/env bash
set -euo pipefail

target="${ROUTERBOT_KEENETIC_TARGET:?set ROUTERBOT_KEENETIC_TARGET to the Rust target triple}"

cargo build \
  --release \
  --target "${target}" \
  --no-default-features \
  --features telegram,transmission,keenetic,kubernetes,dlna-k8s,json-state \
  -p routerbot
