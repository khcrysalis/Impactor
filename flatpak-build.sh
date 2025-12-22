# sourcegen task
#
# needs:
# python3 python3-devel poetry git
(
	set -o pipefail
	export TOOLS=/tmp/flatpak-builder-tools

	if [ ! -d "$TOOLS" ]; then
		git clone https://github.com/flatpak/flatpak-builder-tools "$TOOLS"
		pushd "$TOOLS"
		git reset --hard f03a673abe6ce189cea1c2857e2b44af2dd79d1f --quiet
		popd
	fi

	poetry install --directory="$TOOLS/cargo"
	poetry run --directory="$TOOLS/cargo" \
		"$TOOLS/cargo/flatpak-cargo-generator.py" \
		"$(realpath ./Cargo.lock)" \
		-o "$(realpath ./cargo-sources.json)"
)

# compile task
# 
# needs:
# flatpak flatpak-builder
# 
# flatpaks needed:
# * org.freedesktop.Platform
# * org.freedesktop.Sdk
# * org.freedesktop.Sdk.Extension.rust-stable
# * org.freedesktop.Sdk.Extension.llvm21
(
	flatpak-builder --user --force-clean build-dir com.khcrysalis.PlumeImpactor.json
)