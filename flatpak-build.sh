#!/usr/bin/env bash
set -euo pipefail

tooling_dir="flatpak-builder-tools"
manifest="dev.khcrysalis.PlumeImpactor.json"

if ! command -v poetry >/dev/null 2>&1; then
  echo "poetry is required to generate cargo-sources.json (install it first)." >&2
  exit 1
fi

if [ ! -d "$tooling_dir" ]; then
  git clone https://github.com/flatpak/flatpak-builder-tools.git "$tooling_dir"
fi

pushd "$tooling_dir/cargo" >/dev/null
poetry install
poetry run python flatpak-cargo-generator.py ../../Cargo.lock -o ../../cargo-sources.json
popd >/dev/null

flatpak-builder --force-clean --install --user build-dir "$manifest"
flatpak build-bundle ~/.local/share/flatpak/repo PlumeImpactor.flatpak dev.khcrysalis.PlumeImpactor
