# Building Cosmic Cliphoard

## Requirements

- Rust (Edition 2024)
- Cargo
- [Just](https://github.com/casey/just) command runner

## Build

```sh
just build-release
```

For a debug build:

```sh
just build-debug
```

### Vendored build

To build with vendored dependencies (offline-capable):

```sh
just vendor
just build-vendored
```

## Lint

```sh
just check
```

## Install

Installs to `/usr` by default. Set `rootdir` and `prefix` to customize:

```sh
just install
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
