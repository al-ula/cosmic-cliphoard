# Cliphoard

A clipboard manager built for the [COSMIC](https://system76.com/cosmic) desktop environment.

## Features

- **Clipboard history** — automatically records copied text and images
- **Panel applet** — integrates directly into the COSMIC panel
- **System tray icon** — accessible from any system tray
- **Background daemon** — monitors the clipboard via Wayland and D-Bus
- **Search & filter** — quickly find past clipboard entries
- **Pin entries** — keep important items from being evicted
- **CLI interface** — list, paste, delete, pin/unpin, and clear history from the terminal

## Usage

Running `cliphoard` with no arguments opens the clipboard overlay. Subcommands:

```
cliphoard                  # open the clipboard overlay
cliphoard toggle           # toggle the overlay
cliphoard settings         # open the settings overlay
cliphoard applet           # run as a COSMIC panel applet
cliphoard tray             # run the system tray icon
cliphoard daemon           # run the clipboard daemon
cliphoard list [-n LIMIT] [-q QUERY]
cliphoard paste <ID>
cliphoard delete <ID>
cliphoard pin <ID>
cliphoard unpin <ID>
cliphoard clear            # clear all history
cliphoard decode [-m MIME] [-j]
cliphoard generate-service [-p PATH]
```

### Running the daemon

Enable the systemd user service to start the daemon automatically:

```sh
systemctl --user enable --now cliphoard-daemon.service
```

Or generate and install a service file manually:

```sh
cliphoard generate-service
systemctl --user daemon-reload
systemctl --user enable --now cliphoard-daemon.service
```

## Building

See [BUILD.md](BUILD.md) for build instructions, including how to build RPM and DEB packages locally.

### Quick start

```sh
# requires Rust 1.85+ and just
just build-release
just install
```

## License

[MPL-2.0](LICENSE)
