.PHONY: build install uninstall setup-rofi add-hyprland-config

INSTALL_DIR = $(HOME)/.local/bin
CONFIG_DIR = $(HOME)/.config/minhypr
BINARY_NAME = minhypr
HYPRLAND_CONFIG = $(HOME)/.config/hypr/hyprland.conf

build:
	cargo build --release

install: build
	mkdir -p $(INSTALL_DIR)
	cp target/release/$(BINARY_NAME) $(INSTALL_DIR)/
	chmod +x $(INSTALL_DIR)/$(BINARY_NAME)
	@echo "Installed in $(INSTALL_DIR)/$(BINARY_NAME)"
	@echo "To complete the installation, run:"
	@echo "  make setup-rofi"
	@echo "  make add-hyprland-config"

setup-rofi:
	@echo "Setting up Rofi integration..."
	PATH="$(INSTALL_DIR):$(PATH)" $(INSTALL_DIR)/$(BINARY_NAME) setup-rofi

add-hyprland-config:
	@if [ -f $(HYPRLAND_CONFIG) ]; then \
		echo "# MinHypr - Window Minimization Manager" >> $(HYPRLAND_CONFIG); \
		echo "bind = ALT, M, exec, $(BINARY_NAME) minimize" >> $(HYPRLAND_CONFIG); \
		echo "bind = ALT SHIFT, M, exec, $(HOME)/.config/minhypr/launch-menu.sh" >> $(HYPRLAND_CONFIG); \
		echo "bind = ALT CTRL, M, exec, $(HOME)/.config/minhypr/simple-menu.sh" >> $(HYPRLAND_CONFIG); \
		echo "bind = ALT SHIFT, R, exec, $(HOME)/.config/minhypr/restore-all.sh" >> $(HYPRLAND_CONFIG); \
		echo "# Make sure the binary is in PATH" >> $(HYPRLAND_CONFIG); \
		echo "env = PATH,$(INSTALL_DIR):$(PATH)" >> $(HYPRLAND_CONFIG); \
		echo "Configuration added to file $(HYPRLAND_CONFIG)"; \
		echo "To apply changes, run:"; \
		echo "  hyprctl reload"; \
	else \
		echo "Hyprland configuration file not found: $(HYPRLAND_CONFIG)"; \
		echo "Manually add these lines to your configuration file:"; \
		echo "------------------------------------------------------------"; \
		echo "# MinHypr - Window Minimization Manager"; \
		echo "bind = ALT, M, exec, $(BINARY_NAME) minimize"; \
		echo "bind = ALT SHIFT, M, exec, \$$HOME/.config/minhypr/launch-menu.sh"; \
		echo "bind = ALT CTRL, M, exec, \$$HOME/.config/minhypr/simple-menu.sh"; \
		echo "bind = ALT SHIFT, R, exec, \$$HOME/.config/minhypr/restore-all.sh"; \
		echo "# Make sure the binary is in PATH"; \
		echo "env = PATH,$(INSTALL_DIR):\$$PATH"; \
		echo "------------------------------------------------------------"; \
	fi

uninstall:
	rm -f $(INSTALL_DIR)/$(BINARY_NAME)
	@echo "Configuration files in $(CONFIG_DIR) have not been removed."
	@echo "To completely remove, run: rm -rf $(CONFIG_DIR)"
	@echo "Successfully uninstalled"
	@echo "Note: The configuration lines added to Hyprland have not been removed."
	@echo "Edit the file $(HYPRLAND_CONFIG) manually to remove them."