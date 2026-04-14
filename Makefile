SUPERVISOR_SRC=out/phase6/stratsup
SUPERVISOR_DST=/boot/efi/strat/stratsup

.PHONY: install-supervisor stratterm strat-settings

install-supervisor:
	@if [ ! -f "$(SUPERVISOR_SRC)" ]; then \
		echo "Missing supervisor binary: $(SUPERVISOR_SRC)" >&2; \
		exit 1; \
	fi
	@install -d /boot/efi/strat
	@install -m 0755 "$(SUPERVISOR_SRC)" "$(SUPERVISOR_DST)"
	@echo "Installed supervisor to $(SUPERVISOR_DST)"

stratterm:
	$(MAKE) -C stratterm build

strat-settings:
	$(MAKE) -C stratterm run-settings
