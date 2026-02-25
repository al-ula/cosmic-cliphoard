# cosmic-cliphoard Progress

**Complete. 26 tests passing. 4 workspace crates.**

## Crates

| Crate | Modules | Purpose |
|-------|---------|---------|
| `cliphoard-schema` | mime, entry, history, codec, dbus | Shared types, D-Bus proxy, codecs |
| `cliphoard-decode` | detect, decode | MIME detection, content decoding |
| `cliphoard-daemon` | watcher, storage, service | D-Bus daemon, wl-paste watcher, persistence |
| `cliphoard` | app, applet, tray, cli, commands | Unified binary (overlay, applet, tray, CLI) |

## CLI Commands

```
cliphoard              # Layer-shell overlay (default)
cliphoard daemon       # D-Bus daemon
cliphoard applet       # COSMIC panel applet
cliphoard tray         # System tray
cliphoard decode       # Stdin decoder
cliphoard list         # List entries
cliphoard paste <id>   # Paste by ID
cliphoard delete <id>  # Delete by ID
cliphoard pin <id>     # Pin entry
cliphoard unpin <id>   # Unpin entry
cliphoard clear        # Clear history
```

## Key Decisions

- **IPC**: D-Bus (zbus 5) with oxicode binary encoding
- **Storage**: JSON at `$XDG_DATA_HOME/cliphoard/history.json`
- **Tray**: ksni (StatusNotifierItem protocol)
- **MIME**: Magic-byte detection, no external deps

## Future Work

- Image thumbnails
- D-Bus signal updates (no polling)
- Configurable limits
- Keyboard shortcuts
