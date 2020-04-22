.PHONY = all release install uninstall clean

TARGET=target/release
CARGO=cargo
INSTALL=install
INSTALL_DATA=$(INSTALL) -m 644
INSTALL_PROGRAM=$(INSTALL)

prefix?=/usr
exec_prefix?=$(prefix)
sysconfdir?=/etc
bindir?=$(exec_prefix)/bin
srcdir?=.

all:
	$(CARGO) build --all

release:
	$(CARGO) build -p docmond --release
	strip $(TARGET)/docmond

install: release
	$(INSTALL_PROGRAM) -D $(TARGET)/docmond $(DESTDIR)$(bindir)/docmond
	$(INSTALL_DATA) -D $(srcdir)/contrib/config.toml $(DESTDIR)$(sysconfdir)/docmon/config.toml

uninstall:
	rm -rf $(DESTDIR)$(bindir)/docmond
	-rm $(DESTDIR)$(sysconfdir)/docmon/config.toml

clean:
	cargo clean