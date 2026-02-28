%undefine _debugsource_packages

Name:           cliphoard
Version:        0.1.0
Release:        1%{?dist}
Summary:        Clipboard manager for COSMIC desktop

License:        MPL-2.0
URL:            https://github.com/al-ula/cosmic-cliphoard
Source0:        %{url}/archive/v%{version}/cosmic-cliphoard-%{version}.tar.gz

BuildRequires:  cargo >= 1.85
BuildRequires:  rust >= 1.85
BuildRequires:  just
BuildRequires:  gcc
BuildRequires:  git
BuildRequires:  pkgconfig(xkbcommon)
BuildRequires:  pkgconfig(wayland-client)
BuildRequires:  pkgconfig(dbus-1)
BuildRequires:  desktop-file-utils
BuildRequires:  libappstream-glib

Requires:       wl-clipboard

%description
A clipboard manager built for the COSMIC desktop environment.
Features a panel applet, system tray icon, and background daemon
with D-Bus integration.

%prep
%autosetup -n cosmic-cliphoard-%{version}

%build
just build-release

%install
just rootdir=%{buildroot} prefix=%{_prefix} install

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
