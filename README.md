# minhypr

A window minimization manager for Hyprland.

## 📋 Description

Hyprland doesn't natively support window minimization. minhypr solves this problem by moving windows to a special workspace when they are "minimized" and allowing you to restore them later through a visual menu.

## ✨ Features

- Window minimization in Hyprland
- View minimized windows with thumbnails
- Beautiful and intuitive Rofi menu
- Waybar integration
- Automatic window screenshots
- Application icon support

## 📦 Installation

### Dependencies

- Rust and Cargo
- Hyprland
- grim (for screenshots)
- ImageMagick (for image processing)
- Rofi (for the restoration menu)

### Compilation and Installation

```bash
git clone https://github.com/otaviosoaresp/minhypr.git
cd minhypr
make install
```

### Rofi Configuration

To configure Rofi integration:

```bash
make setup-rofi
```

This will create the necessary configuration files in `~/.config/minhypr/`.

## 🚀 Usage

### Hyprland Configuration

Add these lines to your Hyprland configuration file (`~/.config/hypr/hyprland.conf`):

```
# minhypr - Window minimization manager
bind = ALT, M, exec, minhypr minimize            # Minimizes the current window
bind = ALT SHIFT, M, exec, minhypr restore       # Opens menu to restore
```

### With Rofi (recommended)

To use the beautiful Rofi menu:

```
bind = ALT SHIFT, M, exec, ~/.config/minhypr/launch-menu.sh
```

### Commands

- `minhypr minimize` - Minimizes the active window
- `minhypr restore` - Shows menu to restore windows
- `minhypr restore <id>` - Restores a specific window
- `minhypr restore-all` - Restores all windows
- `minhypr restore-last` - Restores the last minimized window
- `minhypr show` - Shows status for waybar
- `minhypr setup-rofi` - Configures Rofi integration

## 🖥️ Waybar Integration

Add this snippet to your Waybar configuration file:

```json
{
    "custom/minhypr": {
        "exec": "minhypr show",
        "return-type": "json",
        "interval": 1,
        "format": "{}",
        "on-click": "~/.config/minhypr/launch-menu.sh",
        "on-click-right": "minhypr restore-all"
    }
}
```

Then add `"custom/minhypr"` to your modules list.

## 🔧 Customization

You can customize the appearance and behavior of minhypr by editing the configuration files in `~/.config/minhypr/`.

## 🤝 Contributing

Contributions, issues, and feature requests are welcome!

## 📝 License

This project is licensed under the MIT License.