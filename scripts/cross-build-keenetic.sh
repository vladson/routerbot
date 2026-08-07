#!/usr/bin/env bash
set -euo pipefail

readonly target="mipsel-unknown-linux-musl"
readonly package_arch="mipsel-3.4_kn"
readonly project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly version="$(awk -F '"' '/^version =/ { print $2; exit }' "${project_root}/Cargo.toml")"
readonly binary="${project_root}/target/${target}/release/routerbot"
readonly output_dir="${project_root}/dist"
readonly package="${output_dir}/routerbot_${version}_${package_arch}.ipk"

if ! command -v cross >/dev/null 2>&1; then
    echo "cross 0.2.5 is required; install it with: cargo install cross --version 0.2.5 --locked" >&2
    exit 1
fi

if ! command -v docker >/dev/null 2>&1 || ! docker info >/dev/null 2>&1; then
    echo "Docker must be installed and running." >&2
    exit 1
fi

export OPENSSL_STATIC=1
cross build \
    --locked \
    --release \
    --target "${target}" \
    --no-default-features \
    --features telegram,transmission,keenetic,json-state \
    --package routerbot

file "${binary}" | grep -Eq 'ELF 32-bit LSB.*MIPS.*statically linked'
cross run \
    --locked \
    --release \
    --target "${target}" \
    --no-default-features \
    --features telegram,transmission,keenetic,json-state \
    --package routerbot \
    -- \
    --version | grep -Fx "routerbot ${version}"
if cargo tree --locked --package routerbot --target "${target}" -i ring 2>/dev/null \
    | grep -q '^ring '; then
    echo "The Keenetic target dependency graph must not contain ring." >&2
    exit 1
fi

readonly stage="$(mktemp -d "${TMPDIR:-/tmp}/routerbot-ipk.XXXXXX")"
trap 'rm -rf "${stage}"' EXIT
mkdir -p \
    "${stage}/control" \
    "${stage}/data/opt/bin" \
    "${stage}/data/opt/etc/init.d" \
    "${stage}/data/opt/etc/routerbot" \
    "${output_dir}"

install -m 0755 "${binary}" "${stage}/data/opt/bin/routerbot"
install -m 0755 \
    "${project_root}/packaging/keenetic/S99routerbot" \
    "${stage}/data/opt/etc/init.d/S99routerbot"
install -m 0600 \
    "${project_root}/packaging/keenetic/config.yaml" \
    "${stage}/data/opt/etc/routerbot/config.yaml"
install -m 0600 \
    "${project_root}/packaging/keenetic/routerbot.env.example" \
    "${stage}/data/opt/etc/routerbot/routerbot.env.example"

sed \
    -e "s/@VERSION@/${version}/g" \
    -e "s/@ARCH@/${package_arch}/g" \
    "${project_root}/packaging/keenetic/control" >"${stage}/control/control"
cp "${project_root}/packaging/keenetic/conffiles" "${stage}/control/conffiles"
printf '2.0\n' >"${stage}/debian-binary"

export COPYFILE_DISABLE=1
tar -C "${stage}/control" -czf "${stage}/control.tar.gz" .
tar -C "${stage}/data" -czf "${stage}/data.tar.gz" .
(
    cd "${stage}"
    ar r "${package}" debian-binary control.tar.gz data.tar.gz
)

echo "Created ${package}"
