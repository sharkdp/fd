#!/bin/bash
COPYRIGHT_YEARS="2018 - "$(date "+%Y")
MAINTAINER="David Peter <mail@david-peter.de>"
REPO="https://github.com/sharkdp/fd"
DPKG_STAGING="${CICD_INTERMEDIATES_DIR:-.}/debian-package"
DPKG_DIR="${DPKG_STAGING}/dpkg"
mkdir -p "${DPKG_DIR}"

if [[ -z "$TARGET" ]]; then
  TARGET="$(rustc -vV | sed -n 's|host: \(.*\)|\1|p')"
fi

case "$TARGET" in
  *-musl*)
    DPKG_BASENAME=fd-musl
    DPKG_CONFLICTS=fd
    ;;
  *)
    DPKG_BASENAME=fd
    DPKG_CONFLICTS=fd-musl
    ;;
esac

if [[ -z "$DPKG_VERSION" ]]; then
  DPKG_VERSION=$(cargo metadata --no-deps --format-version 1 | jq -r .packages[0].version)
fi

unset DPKG_ARCH
case "${TARGET}" in
  aarch64-*-linux-*) DPKG_ARCH=arm64 ;;
  arm-*-linux-*hf) DPKG_ARCH=armhf ;;
  i686-*-linux-*) DPKG_ARCH=i686 ;;
  x86_64-*-linux-*) DPKG_ARCH=amd64 ;;
  *) DPKG_ARCH=notset ;;
esac;

DPKG_NAME="${DPKG_BASENAME}_${DPKG_VERSION}_${DPKG_ARCH}.deb"

BIN_PATH=${BIN_PATH:-target/${TARGET}/release/fd}

# Binary
install -Dm755 "${BIN_PATH}" "${DPKG_DIR}/usr/bin/fd"

# Man page
install -Dm644 'doc/fd.1' "${DPKG_DIR}/usr/share/man/man1/fd.1"
gzip -n --best "${DPKG_DIR}/usr/share/man/man1/fd.1"

# Autocompletion files
install -Dm644 'autocomplete/fd.bash' "${DPKG_DIR}/usr/share/bash-completion/completions/fd"
install -Dm644 'autocomplete/fd.fish' "${DPKG_DIR}/usr/share/fish/vendor_completions.d/fd.fish"
install -Dm644 'autocomplete/_fd' "${DPKG_DIR}/usr/share/zsh/vendor-completions/_fd"

# README and LICENSE
install -Dm644 "README.md" "${DPKG_DIR}/usr/share/doc/${DPKG_BASENAME}/README.md"
install -Dm644 "LICENSE-MIT" "${DPKG_DIR}/usr/share/doc/${DPKG_BASENAME}/LICENSE-MIT"
install -Dm644 "LICENSE-APACHE" "${DPKG_DIR}/usr/share/doc/${DPKG_BASENAME}/LICENSE-APACHE"
install -Dm644 "CHANGELOG.md" "${DPKG_DIR}/usr/share/doc/${DPKG_BASENAME}/changelog"
gzip -n --best "${DPKG_DIR}/usr/share/doc/${DPKG_BASENAME}/changelog"

cat > "${DPKG_DIR}/usr/share/doc/${DPKG_BASENAME}/copyright" <<EOF
Format: http://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: fd
Source: ${REPO}

Files: *
Copyright: ${MAINTAINER}
Copyright: $COPYRIGHT_YEARS ${MAINTAINER}
License: Apache-2.0 or MIT

License: Apache-2.0
  On Debian systems, the complete text of the Apache-2.0 can be found in the
  file /usr/share/common-licenses/Apache-2.0.

License: MIT
  Permission is hereby granted, free of charge, to any
  person obtaining a copy of this software and associated
  documentation files (the "Software"), to deal in the
  Software without restriction, including without
  limitation the rights to use, copy, modify, merge,
  publish, distribute, sublicense, and/or sell copies of
  the Software, and to permit persons to whom the Software
  is furnished to do so, subject to the following
  conditions:
  .
  The above copyright notice and this permission notice
  shall be included in all copies or substantial portions
  of the Software.
  .
  THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
  ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
  TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
  PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
  SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
  CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
  OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
  IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
  DEALINGS IN THE SOFTWARE.
EOF
  chmod 644 "${DPKG_DIR}/usr/share/doc/${DPKG_BASENAME}/copyright"

  # control file
  mkdir -p "${DPKG_DIR}/DEBIAN"
  cat > "${DPKG_DIR}/DEBIAN/control" <<EOF
Package: ${DPKG_BASENAME}
Version: ${DPKG_VERSION}
Section: utils
Priority: optional
Maintainer: ${MAINTAINER}
Homepage: ${REPO}
Architecture: ${DPKG_ARCH}
Provides: fd
Conflicts: ${DPKG_CONFLICTS}
Description: simple, fast and user-friendly alternative to find
  fd is a program to find entries in your filesystem.
  It is a simple, fast and user-friendly alternative to find.
  While it does not aim to support all of finds powerful functionality, it provides
  sensible (opinionated) defaults for a majority of use cases.
EOF

DPKG_PATH="${DPKG_STAGING}/${DPKG_NAME}"

if [[ -n $GITHUB_OUTPUT ]]; then
  echo "DPKG_NAME=${DPKG_NAME}" >> "$GITHUB_OUTPUT"
  echo "DPKG_PATH=${DPKG_PATH}" >> "$GITHUB_OUTPUT"
fi

# build dpkg
fakeroot dpkg-deb --build "${DPKG_DIR}" "${DPKG_PATH}"
