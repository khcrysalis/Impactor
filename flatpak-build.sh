#!/usr/bin/env bash
set -euo pipefail

tooling_dir="flatpak-builder-tools"
manifest="com.khcrysalis.PlumeImpactor.json"
generated_manifest="com.khcrysalis.PlumeImpactor.generated.json"

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

python3 - <<'PY'
import json

manifest_path = "com.khcrysalis.PlumeImpactor.json"
cargo_sources_path = "cargo-sources.json"
generated_path = "com.khcrysalis.PlumeImpactor.generated.json"

with open(manifest_path, "r", encoding="utf-8") as f:
    manifest = json.load(f)

with open(cargo_sources_path, "r", encoding="utf-8") as f:
    cargo_sources = json.load(f)

for module in manifest.get("modules", []):
    if module.get("name") == "plumeimpactor":
        original_sources = module.get("sources", [])
        filtered_sources = [
            src for src in original_sources
            if not (src.get("type") == "file" and src.get("path") == "cargo-sources.json")
        ]
        module["sources"] = cargo_sources + filtered_sources
        break
else:
    raise SystemExit("Could not find plumeimpactor module in manifest.")

with open(generated_path, "w", encoding="utf-8") as f:
    json.dump(manifest, f, indent=2)
    f.write("\n")
PY

flatpak-builder --force-clean --install --user build-dir "$generated_manifest"
