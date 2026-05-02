PROFILE=release
EXE=target/$(PROFILE)/fd
prefix=/usr/local
bindir=$(prefix)/bin
datadir=$(prefix)/share
exe_name=fd

$(EXE): Cargo.toml src/**/*.rs
	cargo build --profile $(PROFILE) --locked

.PHONY: completions
completions: autocomplete/fd.bash autocomplete/fd.fish autocomplete/_fd.ps1 autocomplete/_fd autocomplete/_fdfind autocomplete/fdfind.bash autocomplete/fdfind.fish

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

autocomplete/_fdfind: contrib/completion/_fdfind
	$(comp_dir)
	cp $< $@

autocomplete/fdfind.bash: contrib/completion/fdfind.bash
	$(comp_dir)
	cp $< $@

autocomplete/fdfind.fish: contrib/completion/fdfind.fish
	$(comp_dir)
	cp $< $@

install: $(EXE) completions
	install -Dm755 $(EXE) $(DESTDIR)$(bindir)/fd
	install -Dm644 autocomplete/fd.bash $(DESTDIR)/$(datadir)/bash-completion/completions/$(exe_name)
	install -Dm644 autocomplete/fd.fish $(DESTDIR)/$(datadir)/fish/vendor_completions.d/$(exe_name).fish
	install -Dm644 autocomplete/_fd $(DESTDIR)/$(datadir)/zsh/site-functions/_fd
	install -Dm644 autocomplete/fdfind.bash $(DESTDIR)/$(datadir)/zsh/site-functions/fdfind.bash
	install -Dm644 autocomplete/fdfind.fish $(DESTDIR)/$(datadir)/zsh/site-functions/fdfind.fish
	install -Dm644 autocomplete/_fdfind $(DESTDIR)/$(datadir)/zsh/site-functions/_fdfind
	install -Dm644 doc/fd.1 $(DESTDIR)/$(datadir)/man/man1/$(exe_name).1
