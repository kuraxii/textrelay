# RPM spec for fcitx5-textrelay
#
# Build: ./build-rpm.sh
# Result: ~/rpmbuild/RPMS/x86_64/fcitx5-textrelay-<ver>-<rel>.<arch>.rpm

%global __provides_exclude ^libtextrelay\\.so$

Name:           fcitx5-textrelay
Version:        0.3.0
Release:        1%{?dist}
Summary:        Fcitx5 addon for phone-to-desktop text relay

Group:          System Environment/Libraries
License:        MIT
URL:            https://example.invalid/fcitx5-textrelay
Source0:        fcitx5-textrelay-%{version}.tar.gz
Source1:        fcitx5-5.1.22.tar.gz

BuildRequires:  gcc-c++
BuildRequires:  cmake
BuildRequires:  make

%description
A lightweight Fcitx5 addon that listens on a local unix socket
($XDG_RUNTIME_DIR/textrelay.sock). When text is received via the socket,
it commits the text to the currently focused input context using
InputContext::commitString(). This allows a phone browser or any remote
client to send text directly into the focused text field on the Linux
desktop, without clipboard round-trips, DBus integration, or key-event
simulation.

%prep
%setup -q -n . -T -b 0
# Source0 is a flat tar with just source files (no subdir).
mkdir -p build/src
cp -a textrelay.cpp textrelay.conf LICENSE compat build/src/

%build
# fcitx5 headers from Fedora's fcitx5-5.1.22 source package (only the
# src/lib/fcitx/ subtree is needed). Fedora does not ship fcitx5-devel.
tar -C /tmp -xzf %{SOURCE1}
export FCITX5_SRC=/tmp/fcitx5-5.1.22

cd build
g++ -std=c++17 -O2 -fPIC -shared \
    -o libtextrelay.so src/textrelay.cpp \
    -I src/compat \
    -I $FCITX5_SRC/src/lib \
    -I $FCITX5_SRC/build/src/lib \
    $(pkg-config --cflags fcitx5-core fcitx5-utils) \
    -lfcitx5-core -lfcitx5-utils

%install
install -m 0755 -d %{buildroot}%{_libdir}/fcitx5
install -m 0755 -d %{buildroot}%{_datadir}/fcitx5/addon
install -m 0755 build/libtextrelay.so %{buildroot}%{_libdir}/fcitx5/libtextrelay.so
install -m 0644 build/src/textrelay.conf %{buildroot}%{_datadir}/fcitx5/addon/textrelay.conf

%files
%{_libdir}/fcitx5/libtextrelay.so
%{_datadir}/fcitx5/addon/textrelay.conf

%changelog
* Mon Oct  7 2024  - 0.3.0-1
- Rename to textrelay (was fcitx5-voiceinput)
- Add unix socket interface ($XDG_RUNTIME_DIR/textrelay.sock), keep DBus.
- Remove DBus dependency, data plane via unix socket only.
- Initial version.