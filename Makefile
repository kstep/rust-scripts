reinstall:
	@cargo uninstall --root /usr/local scripts
	cargo install --root /usr/local --path $(CURDIR)
