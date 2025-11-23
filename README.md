# PlumeImpactor

[![GitHub Release](https://img.shields.io/github/v/release/khcrysalis/PlumeImpactor?include_prereleases)](https://github.com/khcrysalis/PlumeImpactor/releases)
[![GitHub License](https://img.shields.io/github/license/khcrysalis/PlumeImpactor?color=%23C96FAD)](https://github.com/khcrysalis/PlumeImpactor/blob/main/LICENSE)
[![Sponsor Me](https://img.shields.io/static/v1?label=Sponsor&message=%E2%9D%A4&logo=GitHub&color=%23fe8e86)](https://github.com/sponsors/khcrysalis)

WIP open-source, cross-platform, and feature rich iOS/tvOS sideloading application. Supporting macOS, Linux, and Windows.

### Features
- User friendly and clean UI.
- Sign and sideload applications with your Apple ID.
- Simple customization options for the app.
- Generates P12 for SideStore/AltStore to use, similar to how Altserver works.
- Automatically populate pairing files for specific apps like SideStore, Antrag, and Protokolle.
- *Proper* entitlement handling and can register app plugins.

## Download

##### ETA SON

## Structure

The project is seperated in multiple modules, all serve single or multiple uses depending on their importance.

| Module               | Description                                                                                                                   |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `apps/plumeimpactor` | GUI interface for the crates shown below, backend using wxWidgets (with a rust ffi wrapper, wxDragon)                         |
| `apps/plumesign`     | Simple CLI interface for signing, using `clap`.                                                                       |
| `crates/grand_slam`  | Handles all api request used for communicating with Apple developer services, along with providing auth for Apple's grandslam |
| `crates/utils`       | Shared code between GUI and CLI, contains signing and modification logic, and helpers.  |

## Building

Building is going to be a bit convoluted for each platform, each having their own unique specifications, but the best reference for building should be looking at how [GitHub actions](./.github/workflows/build.yml) does it.


You need:
- [Rust](https://rustup.rs/)
- [CMake](https://cmake.org/download/) (and a c++ compiler)

```sh
# Applies our patches in ./patches 
cargo install patch-crate
cargo patch-crate --force && cargo fetch --locked

# Building / testing
cargo run --bin plumeimpactor
```

Extra requirements are shown below for building if you don't have these already, and trust me, it is convoluted.

#### Linux Requirements

```sh
# Ubuntu/Debian
sudo apt-get install libclang-dev pkg-config libgtk-3-dev libpng-dev libjpeg-dev libgl1-mesa-dev libglu1-mesa-dev libxkbcommon-dev libexpat1-dev libtiff-dev

# Fedora/RHEL
sudo dnf install clang-devel pkg-config gtk3-devel libpng-devel libjpeg-devel mesa-libGL-devel mesa-libGLU-devel libxkbcommon-devel expat-devel libtiff-devel
```

#### macOS Requirements

```sh
# Xcode Command Line Tools
xcode-select --install
```

#### Windows Requirements

Download and install [Visual Studio 2022 Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022) with:
- Desktop development with C++ workload
- Windows 10/11 SDK

## Acknowledgements

- [SAMSAM](https://github.com/khcrysalis) – The maker.
- [SideStore](https://github.com/SideStore/apple-private-apis) – Grandslam auth & Omnisette.
- [Sideloader](https://github.com/Dadoum/Sideloader) – Apple Developer API references.
- [idevice](https://github.com/jkcoxson/idevice) – Used for communication with `installd`, specifically for sideloading the apps to your devices.

## License

Project is licensed under the MIT license. You can see the full details of the license [here](https://github.com/khcrysalis/PlumeImpactor/blob/main/LICENSE).
- Some components may be licensed under different licenses, see their respective directories for details.
