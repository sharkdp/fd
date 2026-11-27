#!/bin/bash
#
# Create a .tar.gz archive of the fd project for distribution
#
# Expected environment variables:
#
# TARGET: rust target to publish. Defaults to default rustc target
# BIN_PATH: path to fd binary
# PKG_STAGING: directory to use for staging files to include in the archive
# ARCHIVE_NAME: Name of output file, minus extension, defaults to "fd" with a version appended
#   if VERSION is specified
# VERSION: version to publish
# GITHUB_OUTPUT: file to store github output variables in

if [[ -z "$TARGET" ]]; then
  TARGET="$(rustc -vV | sed -n 's/host: //p')"
fi

PKG_suffix=".tar.gz"
EXE_suffix=""
case ${TARGET} in
*-pc-windows-*)
  PKG_suffix=".zip"
  EXE_suffix=".exe"
  ;;
esac

if [[ -z $BIN_PATH ]]; then
  BIN_PATH=target/${PROFILE:-release}/fd
fi

if [[ -z "$ARCHIVE_NAME" ]]; then
  ARCHIVE_NAME=fd
  [[ -n "$VERSION" ]] && ARCHIVE_NAME+="-v$VERSION"
fi

PKG_NAME=${ARCHIVE_NAME}${PKG_suffix}

staging_dir="${PKG_STAGING:-package}"
ARCHIVE_DIR="${staging_dir}/${ARCHIVE_NAME}/"
mkdir -p "${ARCHIVE_DIR}"

# Binary
cp "${BIN_PATH}" "$ARCHIVE_DIR"

# README, LICENSE and CHANGELOG files
cp "README.md" "LICENSE-MIT" "LICENSE-APACHE" "CHANGELOG.md" "$ARCHIVE_DIR"

# Man page
cp "doc/fd.1" "$ARCHIVE_DIR"

# Autocompletion files
cp -r autocomplete "${ARCHIVE_DIR}"

# base compressed package
pushd "${staging_dir}/" >/dev/null
case ${TARGET} in
*-pc-windows-*) 7z -y a "${PKG_NAME}" "${ARCHIVE_NAME}"/* | tail -2 ;;
*) tar czf "${PKG_NAME}" "${ARCHIVE_NAME}"/* ;;
esac;
popd >/dev/null

if [[ -n "$GITHUB_OUTPUT" ]]; then
  echo "PKG_NAME=${PKG_NAME}" >> $GITHUB_OUTPUT
  # Let subsequent steps know where to find the compressed package
  echo "PKG_PATH=${PKG_STAGING}/${PKG_NAME}" >> $GITHUB_OUTPUT
fi
