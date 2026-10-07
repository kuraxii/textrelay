#!/bin/bash
# Build the fcitx5-textrelay RPM. Run under a normal shell.
# NOTE: must be bash — ZCode's zsh snapshot environment can shadow gcc.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
fcitx5_tar="/tmp/fcitx5-src/fcitx5.tar.gz"
version="0.3.0"

if [ ! -f "$fcitx5_tar" ]; then
    echo "Downloading fcitx5 source (headers only needed)..."
    curl -sSL -o "$fcitx5_tar" \
        https://github.com/fcitx/fcitx5/archive/refs/tags/5.1.22.tar.gz
fi

srcdir="$here/.rpmbuild-src"
rm -rf "$srcdir"
mkdir -p "$srcdir"
cp "$here/textrelay.cpp" "$here/textrelay.conf" "$here/LICENSE" "$srcdir/"
cp -r "$here/compat" "$srcdir/"
tar -C "$srcdir" -czf "$here/fcitx5-textrelay-${version}.tar.gz" .

rpmdir="$HOME/rpmbuild"
mkdir -p "$rpmdir/SOURCES" "$rpmdir/SPECS"
cp "$here/fcitx5-textrelay-${version}.tar.gz" "$rpmdir/SOURCES/"
cp "$fcitx5_tar" "$rpmdir/SOURCES/fcitx5-5.1.22.tar.gz"
cp "$here/fcitx5-textrelay.spec" "$rpmdir/SPECS/"

rpmbuild -ba --define "_topdir $rpmdir" "$rpmdir/SPECS/fcitx5-textrelay.spec"

echo
ls -la "$rpmdir/RPMS/x86_64/"
echo "Install with:"
echo "  sudo dnf install $rpmdir/RPMS/x86_64/fcitx5-textrelay-${version}-1.*.rpm"