# PlumeImpactor

[![GitHub Release](https://img.shields.io/github/v/release/khcrysalis/PlumeImpactor?include_prereleases)](https://github.com/khcrysalis/PlumeImpactor/releases)
[![GitHub License](https://img.shields.io/github/license/khcrysalis/PlumeImpactor?color=%23C96FAD)](https://github.com/khcrysalis/PlumeImpactor/blob/main/LICENSE)
[![Sponsor Me](https://img.shields.io/static/v1?label=Sponsor&message=%E2%9D%A4&logo=GitHub&color=%23fe8e86)](https://github.com/sponsors/khcrysalis)

Open-source, cross-platform, and feature rich iOS sideloading application. Supporting macOS, Linux[^1], and Windows[^2].

[^1]: On Linux, usbmuxd must be installed on your system. Don't worry though, it comes with most popular distributions by default already! However, due to some distributions [udev](https://man7.org/linux/man-pages/man7/udev.7.html) rules `usbmuxd` may stop running after no devices are connected causing Impactor to not detect the device after plugging it in. You can mitigate this by plugging your phone first then restarting the app.

[^2]: On Windows, [iTunes](https://support.apple.com/en-us/106372) must be downloaded so Impactor is able to use the drivers for interacting with Apple devices.

| ![Demo of app](demo.webp)   |
| :----------------------:    |
| Demo of sideloading a working [LiveContainer](https://github.com/LiveContainer/LiveContainer) build. |

### Features

- User friendly and clean UI.
- Supports Linux.
- Sign and sideload applications on iOS 9.0+ & Mac with your Apple ID.
- Simple customization options for the app.
- Generates P12 for SideStore/AltStore to use, similar to how Altserver works.
- Automatically populate pairing files for specific apps like SideStore, Antrag, and Protokolle.
- Almost *Proper* entitlement handling and can register app plugins.

## Download

Visit [releases](https://github.com/khcrysalis/PlumeImpactor/releases) and get the latest version for your computer.

## Structure

The project is seperated in multiple modules, all serve single or multiple uses depending on their importance.

| Module               | Description                                                                                                                   |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `apps/plumeimpactor` | GUI interface for the crates shown below, backend using wxWidgets (with a rust ffi wrapper, wxDragon).                        |
| `apps/plumesign`     | Simple CLI interface for signing, using `clap`.                                                                               |
| `apps/udid`          | libMobileGestalt.dylib wrapper for obtaining Macs udid.                                                                       |
| `crates/core`.       | Handles all api request used for communicating with Apple developer services, along with providing auth for Apple's grandslam |
| `crates/utils`       | Shared code between GUI and CLI, contains signing and modification logic, and helpers.                                        |

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

- [Xcode](https://developer.apple.com/xcode/) or [Command Line Tools](https://developer.apple.com/download/all/)

#### Windows Requirements

- Download and install [Visual Studio 2022 Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022) with:
- Windows 10/11 SDK

## Sponsors

| Thanks to all my [sponsors](https://github.com/sponsors/khcrysalis)!! |
|:-:|
| <img src="https://raw.githubusercontent.com/khcrysalis/github-sponsor-graph/main/graph.png"> |
| _**"samara is cute" - Vendicated**_ |

## Acknowledgements

- [SAMSAM](https://github.com/khcrysalis) – The maker.
- [SideStore](https://github.com/SideStore/apple-private-apis) – Grandslam auth & Omnisette.
- [Sideloader](https://github.com/Dadoum/Sideloader) – Apple Developer API references.
- [idevice](https://github.com/jkcoxson/idevice) – Used for communication with `installd`, specifically for sideloading the apps to your devices.

## License

Project is licensed under the MIT license. You can see the full details of the license [here](https://github.com/khcrysalis/PlumeImpactor/blob/main/LICENSE). Some components may be licensed under different licenses, see their respective directories for details.
