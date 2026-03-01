
name := 'cliphoard'
appid := 'com.github.al_ula.Cliphoard'

rootdir := ''
prefix := '/usr'
cargo-target-dir := env('CARGO_TARGET_DIR', 'target')

appdata := appid + '.metainfo.xml'
desktop := appid + '.desktop'
applet-desktop := appid + '.Applet.desktop'
icon-svg := appid + '.svg'

base-dir := absolute_path(clean(rootdir / prefix))
appdata-dst := base-dir / 'share' / 'appdata' / appdata
bin-dst := base-dir / 'bin' / name
desktop-dst := base-dir / 'share' / 'applications' / desktop
applet-desktop-dst := base-dir / 'share' / 'applications' / applet-desktop
icons-dst := base-dir / 'share' / 'icons' / 'hicolor'
icon-svg-dst := icons-dst / 'scalable' / 'apps'
systemd-user-dst := base-dir / 'lib' / 'systemd' / 'user'

default: build-release

clean:
    cargo clean

build-debug *args:
    cargo build {{args}}

build-release *args: (build-debug '--release' args)

check *args:
    cargo clippy --all-features {{args}} -- -W clippy::pedantic

check-json: (check '--message-format=json')

run *args:
    env RUST_BACKTRACE=full cargo run --release {{args}}

install:
    install -Dm0755 {{ cargo-target-dir / 'release' / name }} {{bin-dst}}
    install -Dm0644 {{ 'resources' / desktop }} {{desktop-dst}}
    install -Dm0644 {{ 'resources' / applet-desktop }} {{applet-desktop-dst}}
    install -Dm0644 {{ 'resources' / appdata }} {{appdata-dst}}
    install -Dm0644 {{ 'resources' / 'icons' / 'hicolor' / 'scalable' / 'apps' / icon-svg }} {{icon-svg-dst / icon-svg}}
    install -Dm0644 {{ 'resources' / 'systemd' / 'cliphoard-daemon.service' }} {{systemd-user-dst / 'cliphoard-daemon.service'}}
    install -Dm0644 {{ 'resources' / 'systemd' / 'cliphoard-tray.service' }} {{systemd-user-dst / 'cliphoard-tray.service'}}

uninstall:
    rm {{bin-dst}} {{desktop-dst}} {{applet-desktop-dst}} {{icon-svg-dst / icon-svg}} {{systemd-user-dst / 'cliphoard-daemon.service'}} {{systemd-user-dst / 'cliphoard-tray.service'}}

sync-version:
    set -euo pipefail
    version=$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
    date=$(date +%Y-%m-%d)
    metainfo="resources/{{ appdata }}"
    sed -i "s/<release version=\"[^\"]*\" date=\"[^\"]*\"/<release version=\"$version\" date=\"$date\"/" "$metainfo"
    echo "Synced metainfo to version $version ($date)"

tag version:
    find -type f -name Cargo.toml -exec sed -i '0,/^version/s/^version.*/version = "{{version}}"/' '{}' \; -exec git add '{}' \;
    just sync-version
    cargo check
    cargo clean
    git add Cargo.lock resources/{{ appdata }}
    git commit -m 'release: {{version}}'
    git commit --amend
    git tag -a {{version}} -m ''

release: build-release
    #!/bin/bash
    set -euo pipefail
    version=$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
    target=$(rustc -vV | sed -n 's|host: ||p')
    release_dir="dist/cliphoard-${version}-${target}"
    archive="cliphoard-${version}-${target}.tar.gz"
    rm -rf "$release_dir"
    mkdir -p "$release_dir/bin"
    mkdir -p "$release_dir/share/applications"
    mkdir -p "$release_dir/share/appdata"
    mkdir -p "$release_dir/share/icons/hicolor/scalable/apps"
    cp "{{ cargo-target-dir }}/release/{{ name }}" "$release_dir/bin/"
    cp "resources/{{ desktop }}" "$release_dir/share/applications/"
    cp "resources/{{ applet-desktop }}" "$release_dir/share/applications/"
    cp "resources/{{ appdata }}" "$release_dir/share/appdata/"
    cp "resources/icons/hicolor/scalable/apps/{{ icon-svg }}" "$release_dir/share/icons/hicolor/scalable/apps/"
    tar czf "$archive" -C dist "cliphoard-${version}-${target}"
    rm -rf "$release_dir"
    echo "Created $archive"
