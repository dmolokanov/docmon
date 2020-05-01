.PHONY = all release install uninstall clean deb

TARGET=target/release

VERSION?=0.1.0
REVISION?=1

DEB_VERSION?=$(VERSION)
DEB_REVISION?=$(REVISION)

PACKAGE_NAME=docmon
PACKAGE?="$(PACKAGE_NAME)-$(DEB_VERSION)"

CARGO=cargo
INSTALL=install
INSTALL_DATA=$(INSTALL) -m 644
INSTALL_PROGRAM=$(INSTALL)
SED=sed

prefix?=/usr
exec_prefix?=$(prefix)
sysconfdir?=/etc
unitdir?=/lib/systemd/system
bindir?=$(exec_prefix)/bin
srcdir?=.

DPKGFLAGS=-b -rfakeroot -us -uc -i

all:
	$(CARGO) build --all

release:
	$(CARGO) build -p docmond --release
	strip $(TARGET)/docmond

install: release
	$(INSTALL_PROGRAM) -D $(TARGET)/docmond $(DESTDIR)$(bindir)/docmond
	$(INSTALL_DATA) -D $(srcdir)/contrib/config.toml $(DESTDIR)$(sysconfdir)/docmon/config.toml
	$(INSTALL_DATA) -D $(srcdir)/contrib/docmon.service $(DESTDIR)$(unitdir)/docmon.service

deb: release
	$(INSTALL_PROGRAM) -D $(TARGET)/docmond $(TARGET)/$(PACKAGE)/docmond
	$(INSTALL_DATA) -D $(srcdir)/contrib/config.toml $(TARGET)/$(PACKAGE)/etc/docmon/config.toml.template
	$(INSTALL_DATA) -D $(srcdir)/contrib/docmon.service $(TARGET)/$(PACKAGE)/debian/docmon.service
	cp -R $(srcdir)/contrib/debian $(TARGET)/$(PACKAGE)
	$(SED) "s/@version@/${DEB_VERSION}/g; s/@revision@/${DEB_REVISION}/g;" $(srcdir)/contrib/debian/changelog > $(TARGET)/$(PACKAGE)/debian/changelog
	cd $(TARGET)/$(PACKAGE) && dpkg-buildpackage $(DPKGFLAGS)

uninstall:
	rm -rf $(DESTDIR)$(bindir)/docmond
	-rm $(DESTDIR)$(sysconfdir)/docmon/config.toml
	-rm $(DESTDIR)$(unitdir)/docmon.service

clean:
	cargo clean
