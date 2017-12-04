#!/usr/bin/env bash
# Building and packaging for release

set -ex

build() {
    cargo build --target "$TARGET" --release --verbose
}

pack() {
    local tempdir
    local out_dir
    local package_name

    tempdir=$(mktemp -d 2>/dev/null || mktemp -d -t tmp)
    out_dir=$(pwd)
    package_name="$PROJECT_NAME-$TRAVIS_TAG-$TARGET"

    # create a "staging" directory
    mkdir "$tempdir/$package_name"
    mkdir "$tempdir/$package_name/autocomplete"

    # copying the main binary
    cp "target/$TARGET/release/$PROJECT_NAME" "$tempdir/$package_name/"
    strip "$tempdir/$package_name/$PROJECT_NAME"

    # manpage, readme and license
    cp "doc/$PROJECT_NAME.1" "$tempdir/$package_name"
    cp README.md "$tempdir/$package_name"
    cp LICENSE-MIT "$tempdir/$package_name"
    cp LICENSE-APACHE "$tempdir/$package_name"

    # various autocomplete
    cp target/"$TARGET"/release/build/"$PROJECT_NAME"-*/out/"$PROJECT_NAME".bash-completion "$tempdir/$package_name/autocomplete"
    cp target/"$TARGET"/release/build/"$PROJECT_NAME"-*/out/"$PROJECT_NAME".fish "$tempdir/$package_name/autocomplete"
    cp target/"$TARGET"/release/build/"$PROJECT_NAME"-*/out/_"$PROJECT_NAME" "$tempdir/$package_name/autocomplete"

    # archiving
    pushd "$tempdir"
    tar czf "$out_dir/$package_name.tar.gz" "$package_name"/*
    popd
    rm -r "$tempdir"
}

make_deb() {
    local tempdir
    local architecture
    local version
    local dpkgname
    local conflictname

    case $TARGET in
        x86_64*)
            architecture=amd64
            ;;
        i686*)
            architecture=i386
            ;;
        *)
            echo "ERROR: unknown target" >&2
            return 1
            ;;
    esac
    version=${TRAVIS_TAG#v}
    if [[ $TARGET = *musl* ]]; then
      dpkgname=$PACKAGE_NAME-musl
      conflictname=$PACKAGE_NAME
    else
      dpkgname=$PACKAGE_NAME
      conflictname=$PACKAGE_NAME-musl
    fi

    tempdir=$(mktemp -d 2>/dev/null || mktemp -d -t tmp)

    # copy the main binary
    install -Dm755 "target/$TARGET/release/build/$PROJECT_NAME" "$tempdir/usr/bin/$PROJECT_NAME"
    strip "$tempdir/usr/bin/$PROJECT_NAME"

    # manpage
    install -Dm644 "doc/$PROJECT_NAME.1" "$tempdir/usr/share/man/man1/$PROJECT_NAME.1"

    # readme and license
    install -Dm644 README.md "$tempdir/usr/share/doc/$PACKAGE_NAME/README.md"
    install -Dm644 LICENSE-MIT "$tempdir/usr/share/doc/$PACKAGE_NAME/LICENSE-MIT"
    install -Dm644 LICENSE-APACHE "$tempdir/usr/share/doc/$PACKAGE_NAME/LICENSE-APACHE"

    # completions
    install -Dm644 target/$TARGET/release/build/$PROJECT_NAME-*/out/$PROJECT_NAME.bash_completion "$tempdir/usr/share/bash-completion/completions/$PROJECT_NAME.bash_completion"
    install -Dm644 target/$TARGET/release/build/$PROJECT_NAME-*/out/$PROJECT_NAME.fish "$tempdir/usr/share/fish/vendor_completions.d/$PROJECT_NAME.fish"
    install -Dm644 target/$TARGET/release/build/$PROJECT_NAME-*/out/_$PROJECT_NAME "$tempdir/usr/share/zsh/site-functions/_$PROJECT_NAME"

    # Control file
    mkdir "$tempdir/DEBIAN"
    cat > "$tempdir/DEBIAN/control" <<EOF
Package: $dpkgname
Version: $version
Section: utils
Priority: optional
Maintainer: David Peter
Architecture: $architecture
Provides: $PACKAGE_NAME
Conflicts: $conflictname
Description: Simple, fast and user-friendly alternative to find
EOF

    dpkg-deb --build "$tempdir" "${dpkgname}_${version}_${architecture}.deb"
}


main() {
    build
    pack
    if [[ $TARGET = *linux* ]]; then
      make_deb
    fi
}

main
