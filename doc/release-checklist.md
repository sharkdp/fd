# Release checklist

This file can be used as-is, or copied into the GitHub PR description which includes
necessary changes for the upcoming release.

## Version bump

- [ ] Create a new branch for the required changes for this release.
- [ ] Update version in `Cargo.toml`. Run `cargo build` to update `Cargo.lock`.
      Make sure to `git add` the `Cargo.lock` changes as well.
- [ ] Find the current min. supported Rust version by running
      `grep rust-version Cargo.toml`.
- [ ] Update the `fd` version and the min. supported Rust version in `README.md`.
- [ ] Update `CHANGELOG.md`. Change the heading of the *"Upcoming release"* section
      to the version of this release.

## Pre-release checks and updates

- [ ] Install the latest version (`cargo install --locked -f --path .`) and make
      sure that it is available on the `PATH` (`fd --version` should show the
      new version).
- [ ] Review `-h`, `--help`, and the `man` page.
- [ ] Run `fd -h` and copy the output to the *"Command-line options"* section in
      the README
- [ ] Push all changes and wait for CI to succeed (before continuing with the
      next section).
- [ ] Optional: manually test the new features and command-line options described
      in the `CHANGELOG.md`.
- [ ] Run `cargo publish --dry-run` to make sure that it will succeed later
      (after creating the GitHub release).

## Release

- [ ] Merge your release branch (should be a fast-forward merge).
- [ ] Create a tag and push it: `git tag vX.Y.Z; git push origin tag vX.Y.Z`.
      This will trigger the deployment via GitHub Actions.
      REMINDER: If your `origin` is a fork, don't forget to push to e.g. `upstream`
      instead.
- [ ] Go to https://github.com/sharkdp/fd/releases to create the new
      release and wait for the new release to finish (creating the tag will automatically
      create a new release). If necessary, make any adjustments to the release notes.
- [ ] Check if the binary deployment works (archives and Debian packages should
      appear when the CI run *for the Git tag* has finished).
- [ ] Publish to crates.io by running `cargo publish` in a *clean* repository.
      One way to do this is to clone a fresh copy.

## Post-release

- [ ] Prepare a new *"Upcoming release"* section at the top of `CHANGELOG.md`.
      Put this at the top:

      # Upcoming release

      ## Features


      ## Bugfixes


      ## Changes


      ## Other

