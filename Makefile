PROFILE=release
CARGO_CMD=cargo
prefix=/usr/local
bindir=$(prefix)/bin
datadir=$(prefix)/share
exe_name=fd
build_opts=--locked --profile $(PROFILE)

ifeq ($(PROFILE),dev)
	profile_dir=debug
else
	profile_dir=$(PROFILE)
endif
EXE=target/$(profile_dir)/fd

ifdef TARGET
	EXE=target/$(TARGET)/$(profile_dir)/fd
	build_opts+=--target $(TARGET)
endif

$(EXE): Cargo.toml src/**/*.rs
	$(CARGO_CMD) build $(build_opts)

.PHONY: completions
completions: autocomplete/fd.bash autocomplete/fd.fish autocomplete/fd.ps1 autocomplete/_fd

comp_dir=@mkdir -p autocomplete

autocomplete/fd.bash: $(EXE)
	$(comp_dir)
	$(EXE) --gen-completions bash > $@

autocomplete/fd.fish: $(EXE)
	$(comp_dir)
	$(EXE) --gen-completions fish > $@

autocomplete/fd.ps1: $(EXE)
	$(comp_dir)
	$(EXE) --gen-completions powershell > $@

autocomplete/_fd: contrib/completion/_fd
	$(comp_dir)
	cp $< $@

install: $(EXE) completions
	install -Dm755 $(EXE) $(DESTDIR)$(bindir)/fd
	install -Dm644 autocomplete/fd.bash $(DESTDIR)/$(datadir)/bash-completion/completions/$(exe_name)
	install -Dm644 autocomplete/fd.fish $(DESTDIR)/$(datadir)/fish/vendor_completions.d/$(exe_name).fish
	install -Dm644 autocomplete/_fd $(DESTDIR)/$(datadir)/zsh/site-functions/_$(exe_name)
	install -Dm644 doc/fd.1 $(DESTDIR)/$(datadir)/man/man1/$(exe_name).1
