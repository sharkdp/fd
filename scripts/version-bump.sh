#!/usr/bin/bash

set -eu

# This script automates the "Version bump" section

version="$1"

if [[ -z $version ]]; then
  echo "Usage: must supply version as first argument" >&2
  exit 1
fi

git switch -C "release-$version"
sed -i -e "0,/^\[badges/{s/^version =.*/version = \"$version\"/}" Cargo.toml

msrv="$(grep -F rust-version Cargo.toml | sed -e 's/^rust-version= "\(.*\)"/\1/')"

sed -i -e "s/Note that rust version \*[0-9.]+\* or later/Note that rust version *$msrv* or later/" README.md

sed -i -e "s/^# Upcoming release/# $version/" CHANGELOG.md

