%undefine _debugsource_packages

Name:           cliphoard
Version:        %{cargo_version}
Release:        1%{?dist}
Summary:        Clipboard manager for COSMIC desktop

License:        MPL-2.0
URL:            https://github.com/al-ula/cosmic-cliphoard
Source0:        %{url}/archive/v%{version}/cosmic-cliphoard-%{version}.tar.gz

BuildRequires:  cargo >= 1.85
BuildRequires:  rust >= 1.85
BuildRequires:  gcc
BuildRequires:  git
BuildRequires:  pkgconfig(xkbcommon)
BuildRequires:  pkgconfig(wayland-client)
BuildRequires:  pkgconfig(x11)
BuildRequires:  pkgconfig(dbus-1)
BuildRequires:  pkgconfig(fontconfig)
BuildRequires:  pkgconfig(freetype2)
BuildRequires:  vulkan-loader-devel
BuildRequires:  mesa-libGL-devel
BuildRequires:  systemd-rpm-macros
BuildRequires:  desktop-file-utils
BuildRequires:  libappstream-glib

Requires:       hicolor-icon-theme

%description
A clipboard manager built for the COSMIC desktop environment.
Features a panel applet, system tray icon, and background daemon
with D-Bus integration.

%prep
# Copy source tree from checkout into rpmbuild's per-package build directory
find %{_sourcedir} -mindepth 1 -maxdepth 1 ! -name target ! -name '*.spec' ! -name '*.rpm' ! -name '*.tar.gz' -exec cp -a {} . \;

%build
cargo build --release --locked --target-dir target

%install
install -Dm0755 target/release/cliphoard %{buildroot}%{_bindir}/cliphoard
install -Dm0644 resources/com.github.al_ula.Cliphoard.desktop %{buildroot}%{_datadir}/applications/com.github.al_ula.Cliphoard.desktop
install -Dm0644 resources/com.github.al_ula.Cliphoard.Applet.desktop %{buildroot}%{_datadir}/applications/com.github.al_ula.Cliphoard.Applet.desktop
install -Dm0644 resources/com.github.al_ula.Cliphoard.metainfo.xml %{buildroot}%{_datadir}/appdata/com.github.al_ula.Cliphoard.metainfo.xml
install -Dm0644 resources/icons/hicolor/scalable/apps/com.github.al_ula.Cliphoard.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/com.github.al_ula.Cliphoard.svg
install -Dm0644 resources/systemd/cliphoard-daemon.service %{buildroot}%{_prefix}/lib/systemd/user/cliphoard-daemon.service

%check
desktop-file-validate %{buildroot}%{_datadir}/applications/*.desktop
appstream-util validate-relax --nonet %{buildroot}%{_datadir}/appdata/*.metainfo.xml

%post
%systemd_user_post cliphoard-daemon.service

%preun
%systemd_user_preun cliphoard-daemon.service

%postun
%systemd_user_postun_with_restart cliphoard-daemon.service

%files
%license LICENSE
%doc BUILD.md
%{_bindir}/cliphoard
%{_datadir}/applications/com.github.al_ula.Cliphoard.desktop
%{_datadir}/applications/com.github.al_ula.Cliphoard.Applet.desktop
%{_datadir}/appdata/com.github.al_ula.Cliphoard.metainfo.xml
%{_datadir}/icons/hicolor/scalable/apps/com.github.al_ula.Cliphoard.svg
%{_prefix}/lib/systemd/user/cliphoard-daemon.service

%changelog
* Fri Feb 27 2026 Isa Al-Ula <isaalula@proton.me> - 0.1.0-1
- Initial package
