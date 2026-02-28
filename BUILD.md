# Building Cosmic Cliphoard

## Requirements

- Rust (Edition 2024)
- Cargo
- [Just](https://github.com/casey/just) command runner (optional, recommended for development)

## Build

Using `just`:

```sh
just build-release
```

Using `cargo`:

```sh
cargo build --release
```

For a debug build:

```sh
just build-debug
# or
cargo build
```

### Vendored build

To build with vendored dependencies (offline-capable):

```sh
just vendor
just build-vendored
# or
cargo vendor
cargo build --release --frozen
```

## Lint

```sh
just check
# or
cargo clippy --all-features -- -W clippy::pedantic
```

## Install

Installs to `/usr` by default.

Using `just` (customizable with `rootdir` and `prefix`):

```sh
just install
```

Using standard `install` (requires `sudo` for system paths):

```sh
# Example for installing to /usr
sudo install -Dm0755 target/release/cliphoard /usr/bin/cliphoard
sudo install -Dm0644 resources/com.github.al_ula.Cliphoard.desktop /usr/share/applications/com.github.al_ula.Cliphoard.desktop
sudo install -Dm0644 resources/com.github.al_ula.Cliphoard.Applet.desktop /usr/share/applications/com.github.al_ula.Cliphoard.Applet.desktop
sudo install -Dm0644 resources/com.github.al_ula.Cliphoard.metainfo.xml /usr/share/appdata/com.github.al_ula.Cliphoard.metainfo.xml
sudo install -Dm0644 resources/icons/hicolor/scalable/apps/com.github.al_ula.Cliphoard.svg /usr/share/icons/hicolor/scalable/apps/com.github.al_ula.Cliphoard.svg
sudo install -Dm0644 resources/systemd/cliphoard-daemon.service /usr/lib/systemd/user/cliphoard-daemon.service
```

This installs:

- Binary to `$prefix/bin/cliphoard`
- Desktop entries to `$prefix/share/applications/`
- AppStream metadata to `$prefix/share/appdata/`
- Icon to `$prefix/share/icons/hicolor/scalable/apps/`
- Systemd user service to `$prefix/lib/systemd/user/`

After installing, enable and start the daemon:

```sh
systemctl --user enable --now cliphoard-daemon.service
```

## Uninstall

```sh
just uninstall
systemctl --user disable --now cliphoard-daemon.service
```

## Release tarball

Build and package a release archive:

```sh
just release
```

Produces `cliphoard-<version>-<target>.tar.gz` in the working directory.

## Clean

```sh
just clean          # remove build artifacts
just clean-vendor   # remove vendored sources
just clean-dist     # both
```
