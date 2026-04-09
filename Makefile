PROFILE?=release
EXE=target/$(PROFILE)/fd
prefix=/usr/local
bindir=$(prefix)/bin
datadir=$(prefix)/share
exe_name=fd
ifdef VERSION
	ARCHIVE_NAME = fd-v$(VERSION)
else
	ARCHIVE_NAME = fd
endif
export ARCHIVE_NAME
archive_path=package/$(ARCHIVE_NAME).tar.gz

$(EXE): Cargo.toml src/**/*.rs
	cargo build --profile $(PROFILE) --locked

.PHONY: completions
completions: autocomplete/fd.bash autocomplete/fd.fish autocomplete/_fd.ps1 autocomplete/_fd

comp_dir=@mkdir -p autocomplete

autocomplete/fd.bash: $(EXE)
	$(comp_dir)
	$(EXE) --gen-completions bash > $@

autocomplete/fd.fish: $(EXE)
	$(comp_dir)
	$(EXE) --gen-completions fish > $@

autocomplete/_fd.ps1: $(EXE)
	$(comp_dir)
	$(EXE) --gen-completions powershell > $@

autocomplete/_fd: contrib/completion/_fd
	$(comp_dir)
	cp $< $@

archive: $(archive_path)

$(archive_path): completions $(EXE)
	bash scripts/create-archive.sh

install: $(EXE) completions
	install -Dm755 $(EXE) $(DESTDIR)$(bindir)/fd
	install -Dm644 autocomplete/fd.bash $(DESTDIR)/$(datadir)/bash-completion/completions/$(exe_name)
	install -Dm644 autocomplete/fd.fish $(DESTDIR)/$(datadir)/fish/vendor_completions.d/$(exe_name).fish
	install -Dm644 autocomplete/_fd $(DESTDIR)/$(datadir)/zsh/site-functions/_$(exe_name)
	install -Dm644 doc/fd.1 $(DESTDIR)/$(datadir)/man/man1/$(exe_name).1
