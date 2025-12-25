ID := dev.khcrysalis.PlumeImpactor
ifeq ($(OS),Windows_NT)
OS := windows
# TODO: i don't know how to get this on windows
ARCH ?= x86_64
else
ARCH = $(shell uname -m)
ifeq ($(shell uname -s),Linux)
OS := linux
endif
ifeq ($(shell uname -s),Darwin)
OS := darwin
endif
endif
PROFILE ?= debug
PREFIX ?= /usr/local
SUFFIX ?= $(OS)-$(PROFILE)-$(ARCH)

APPIMAGE ?= 0
APPIMAGE_APPDIR ?= /tmp/AppDir

FLATPAK ?= 0
FLATPAK_BUILDER_TOOLS ?= /tmp/flatpak-builder-tools/
FLATPAK_BUILDER_TOOLS_COMMIT ?= 3fc0620788a1dda1a3a539b8f972edadce8260ab
FLATPAK_BUILDER_DIR ?= ./.flatpak-out/
FLATPAK_BUILDER_MANIFEST ?= $(ID).json
FLATPAK_BUNDLE_REPO ?= ~/.local/share/flatpak/repo
FLATPAK_BUNDLE_FILENAME ?= Impactor-$(SUFFIX).flatpak
FLATPAK_BUNDLE_NAME ?= $(ID)

clean:
	@rm -rf ./dist
	@rm -rf ./build
	@rm -rf ./.flatpak-builder
	@rm -rf $(FLATPAK_BUILDER_DIR)
# TODO: fix wxwidgets from not cross-compiling
# TODO: fix embedding manifest in windows build
macos:
	@make clean
# 	@cargo build --workspace --$(PROFILE) --target x86_64-apple-darwin
	@cargo build --workspace --$(PROFILE) --target aarch64-apple-darwin

	@mkdir -p ./dist/macos

# 	@lipo -create -output ./dist/macos/plumeimpactor ./target/x86_64-apple-darwin/$(PROFILE)/plumeimpactor ./target/aarch64-apple-darwin/$(PROFILE)/plumeimpactor
# 	@lipo -create -output ./dist/macos/plumeimpactor ./target/aarch64-apple-darwin/$(PROFILE)/plumesign ./target/aarch64-apple-darwin/$(PROFILE)/plumesign
	@strip ./target/aarch64-apple-darwin/$(PROFILE)/plumeimpactor
	@strip ./target/aarch64-apple-darwin/$(PROFILE)/plumesign
	@cp ./target/aarch64-apple-darwin/$(PROFILE)/plumeimpactor ./dist/macos/plumeimpactor
	@cp ./target/aarch64-apple-darwin/$(PROFILE)/plumesign ./dist/plumesign-macos-universal

	@cp -R package/macos/Impactor.app ./dist/macos/Impactor.app
	@mkdir -p ./dist/macos/Impactor.app/Contents/MacOS
	@VERSION=$$(awk '/\[workspace.package\]/,/^$$/' Cargo.toml | sed -nE 's/version *= *"([^"]*)".*/\1/p'); \
		/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $$VERSION" ./dist/macos/Impactor.app/Contents/Info.plist; \
		/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $$VERSION" ./dist/macos/Impactor.app/Contents/Info.plist
	@mv ./dist/macos/plumeimpactor ./dist/macos/Impactor.app/Contents/MacOS/plumeimpactor
	@codesign --sign - --timestamp --options runtime ./dist/macos/Impactor.app
	@ditto -c -k --sequesterRsrc --keepParent ./dist/macos/Impactor.app ./dist/Impactor-macos-universal.zip
	@create-dmg --volname "Impactor" \
		--background "package/macos/background.png" \
		--window-pos 200 120 \
		--window-size 510 350 \
		--icon-size 100 \
		--icon Impactor.app 160 155 \
		--app-drop-link 360 155 \
		./dist/Impactor-macos-universal.dmg ./dist/macos/

linux:
ifeq ($(FLATPAK),1)
ifeq ($(wildcard $(FLATPAK_BUILDER_TOOLS)),)
	@git clone https://github.com/flatpak/flatpak-builder-tools.git "$(FLATPAK_BUILDER_TOOLS)"
	@cd $(FLATPAK_BUILDER_TOOLS); \
		git checkout $(FLATPAK_BUILDER_TOOLS_COMMIT)
endif
	@poetry --project "$(FLATPAK_BUILDER_TOOLS)/cargo" install
	@poetry --project "$(FLATPAK_BUILDER_TOOLS)/cargo" run \
		python "$(FLATPAK_BUILDER_TOOLS)/cargo/flatpak-cargo-generator.py" Cargo.lock -o package/linux/cargo-sources.json
	@flatpak run --command=flatpak-builder org.flatpak.Builder \
		--ccache --force-clean --user --install $(FLATPAK_BUILDER_DIR) "package/linux/$(FLATPAK_BUILDER_MANIFEST)"
	@mkdir -p dist
	@cd package/linux; \
		flatpak build-bundle $(FLATPAK_BUNDLE_REPO) $(FLATPAK_BUNDLE_FILENAME) $(FLATPAK_BUNDLE_NAME)
	@cp package/linux/$(FLATPAK_BUNDLE_FILENAME) ./dist/$(FLATPAK_BUNDLE_FILENAME)
	@rm package/linux/$(FLATPAK_BUNDLE_FILENAME)
endif
	@cargo build --bins --workspace --$(PROFILE)
	@mkdir -p dist
	@cp target/$(PROFILE)/plumeimpactor ./dist/Impactor-$(SUFFIX)
	@cp target/$(PROFILE)/plumesign ./dist/plumesign-$(SUFFIX)
	@strip dist/Impactor-$(SUFFIX)
ifeq ($(APPIMAGE),1)
	@wget https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-$(ARCH).AppImage -O /tmp/linuxdeploy.appimage
	@chmod +x /tmp/linuxdeploy.appimage
	@make install PREFIX=$(APPIMAGE_APPDIR)/usr
	@NO_STRIP=true \
		/tmp/linuxdeploy.appimage --appimage-extract-and-run \
			--appdir $(APPIMAGE_APPDIR) \
			--executable target/$(PROFILE)/plumeimpactor \
			--desktop-file package/linux/$(ID).desktop \
			--output appimage
	@rm /tmp/linuxdeploy.appimage
	@mv Plume_Impactor-$(ARCH).AppImage dist/Impactor-$(SUFFIX).appimage
	@rm -rf $(APPIMAGE_APPDIR)
endif

install:
ifeq ($(OS),linux)
ifneq ($(PREFIX),$(APPIMAGE_APPDIR)/usr)
	@install -Dm755 target/$(PROFILE)/plumesign $(PREFIX)/bin/plumesign
	@install -Dm755 target/$(PROFILE)/plumeimpactor $(PREFIX)/bin/plumeimpactor
endif
	@install -Dm644 package/linux/$(ID).desktop $(PREFIX)/share/applications/$(ID).desktop
	@install -Dm644 package/linux/$(ID).metainfo.xml $(PREFIX)/share/metainfo/$(ID).metainfo.xml
	@install -Dm644 package/linux/icons/hicolor/16x16/apps/$(ID).png $(PREFIX)/share/icons/hicolor/16x16/apps/$(ID).png
	@install -Dm644 package/linux/icons/hicolor/32x32/apps/$(ID).png $(PREFIX)/share/icons/hicolor/32x32/apps/$(ID).png
	@install -Dm644 package/linux/icons/hicolor/48x48/apps/$(ID).png $(PREFIX)/share/icons/hicolor/48x48/apps/$(ID).png
	@install -Dm644 package/linux/icons/hicolor/64x64/apps/$(ID).png $(PREFIX)/share/icons/hicolor/64x64/apps/$(ID).png
	@install -Dm644 package/linux/icons/hicolor/128x128/apps/$(ID).png $(PREFIX)/share/icons/hicolor/128x128/apps/$(ID).png
	@install -Dm644 package/linux/icons/hicolor/256x256/apps/$(ID).png $(PREFIX)/share/icons/hicolor/256x256/apps/$(ID).png
endif
ifeq ($(OS),darwin)
	@cp -r ./dist/macos/Impactor.app $(PREFIX)/Impactor.app
endif

windows:
	@cargo build --bins --workspace --$(PROFILE)
	@mkdir -p dist
	@cp ./target/$(PROFILE)/plumeimpactor.exe ./dist/Impactor-$(SUFFIX).exe
	@cp ./target/$(PROFILE)/plumesign.exe ./dist/plumesign-$(SUFFIX).exe
