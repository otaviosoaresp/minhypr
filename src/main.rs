/*
 * Minhypr - A window minimization manager for Hyprland
 */

 use std::{
    collections::HashMap,
    env,
    fs::{self},
    io::{self, Result, Write},
    path::Path,
    process::Command,
};
use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;

// Get HOME and define constants based on it
fn get_base_dirs() -> (String, String, String) {
    let cache_dir = String::from("/tmp/minhypr-state");
    let cache_file = format!("{}/windows.json", cache_dir);
    let preview_dir = String::from("/tmp/minhypr-previews");
    
    (cache_dir, cache_file, preview_dir)
}

// Global constants
lazy_static! {
    static ref DIRS: (String, String, String) = get_base_dirs();
}

// Access constants
fn cache_dir() -> &'static str {
    &DIRS.0
}

fn cache_file() -> &'static str {
    &DIRS.1
}

fn preview_dir() -> &'static str {
    &DIRS.2
}

const ICONS: &[(&str, &str)] = &[
    ("firefox", ""),
    ("Alacritty", ""),
    ("kitty", ""),
    ("discord", "󰙯"),
    ("Steam", ""),
    ("chromium", ""),
    ("chrome", ""),
    ("code", "󰨞"),
    ("spotify", ""),
    ("default", "󰖲"),
];

#[derive(Clone, Serialize, Deserialize)]
struct MinimizedWindow {
    address: String,
    display_title: String,
    class: String,
    original_title: String,
    preview_path: Option<String>,
    icon: String,
    workspace: i32,
}

fn get_app_icon(class_name: &str) -> String {
    ICONS
        .iter()
        .find(|(name, _)| class_name.to_lowercase().contains(&name.to_lowercase()))
        .map(|(_, icon)| *icon)
        .unwrap_or(ICONS.last().unwrap().1)
        .to_string()
}

fn capture_window_preview(window_id: &str, geometry: &str) -> Result<String> {
    let preview_path = format!("{}/{}.png", preview_dir(), window_id);
    let thumb_path = format!("{}/{}.thumb.png", preview_dir(), window_id);
    let icon_path = format!("{}/{}.icon.png", preview_dir(), window_id);

    // Capture screenshot with grim
    Command::new("grim")
        .args(["-g", geometry, &preview_path])
        .output()?;

    // Create a thumbnail for the menu
    Command::new("convert")
        .args([
            &preview_path,
            "-resize",
            "200x150^",
            "-gravity",
            "center",
            "-extent",
            "200x150",
            "-quality", "90",
            &thumb_path,
        ])
        .output()?;
        
    // Create a smaller icon for Rofi
    Command::new("convert")
        .args([
            &preview_path,
            "-resize",
            "64x64^",
            "-gravity",
            "center",
            "-extent",
            "64x64",
            "-quality", "90",
            &icon_path,
        ])
        .output()?;

    // Save storage space by removing the original
    fs::remove_file(&preview_path)?;

    // Return path to thumbnail
    Ok(thumb_path)
}

fn read_windows_from_cache() -> Result<Vec<MinimizedWindow>> {
    if !Path::new(cache_file()).exists() {
        return Ok(Vec::new());
    }
    
    let content = fs::read_to_string(cache_file())?;
    match serde_json::from_str::<Vec<MinimizedWindow>>(&content) {
        Ok(windows) => {
            // Additional validation to ensure that windows still exist
            validate_cached_windows(windows)
        },
        Err(_) => Ok(Vec::new()),
    }
}

fn validate_cached_windows(windows: Vec<MinimizedWindow>) -> Result<Vec<MinimizedWindow>> {
    if windows.is_empty() {
        return Ok(Vec::new());
    }

    // Get all Hyprland windows
    let window_check = Command::new("hyprctl")
        .args(["clients", "-j"])
        .output()?;
    
    let windows_json = String::from_utf8(window_check.stdout).unwrap_or_default();
    
    // Check each window in the special workspace: special:minimized
    let special_check = Command::new("hyprctl")
        .args(["workspaces", "-j"])
        .output()?;
    
    let workspaces_json = String::from_utf8(special_check.stdout).unwrap_or_default();
    
    // Filter only valid windows
    let mut valid_windows = Vec::new();
    let mut need_update = false;
    
    for window in windows {
        // Double check: the window must exist in the system AND be in the special:minimized workspace
        if (windows_json.contains(&window.address.clone()) &&
           (workspaces_json.contains("special:minimized") &&
            workspaces_json.contains(&window.address))) {
            valid_windows.push(window);
        } else {
            need_update = true;
        }
    }
    
    // If we found invalid windows, update the cache
    if need_update {
        save_windows_to_cache(&valid_windows)?;
        signal_waybar();
    }
    
    Ok(valid_windows)
}

fn save_windows_to_cache(windows: &[MinimizedWindow]) -> Result<()> {
    let json = serde_json::to_string(windows)?;
    fs::write(cache_file(), json)
}

fn parse_window_info(info: &str) -> Result<HashMap<String, String>> {
    match serde_json::from_str::<HashMap<String, String>>(info) {
        Ok(map) => Ok(map),
        Err(_) => {
            // Fallback parsing for simpler formats
            let mut result = HashMap::new();
            let content = info.trim_matches(|c| c == '{' || c == '}');

            for pair in content.split(',') {
                if let Some((key, value)) = pair.split_once(':') {
                    let clean_key = key.trim().trim_matches('"');
                    let clean_value = value.trim().trim_matches('"');
                    result.insert(clean_key.to_string(), clean_value.to_string());
                }
            }

            Ok(result)
        }
    }
}

fn restore_specific_window(window_id: &str) -> Result<()> {
    println!("Restoring window: {}", window_id);
    
    // Get the specific window from cache
    let windows = read_windows_from_cache()?;
    
    // Find the window we want to restore
    let mut found = false;
    let mut updated_windows = Vec::new();
    
    // Move the window back to its original workspace
    for window in &windows {
        if window.address == window_id {
            Command::new("hyprctl")
                .args([
                    "dispatch",
                    "movetoworkspace",
                    &format!("{},address:{}", window.workspace, window_id),
                ])
                .output()?;
            
            // Focus on the window
            Command::new("hyprctl")
                .args(["dispatch", "focuswindow", &format!("address:{}", window_id)])
                .output()?;
            
            // Remove only this window from the minimized list
            found = true;
        } else {
            updated_windows.push(window.clone());
        }
    }
    
    if !found {
        println!("Window not found in cache: {}", window_id);
        return Ok(());
    }
    
    // Update cache with remaining windows
    save_windows_to_cache(&updated_windows)?;
    
    Ok(())
}

fn restore_all_windows() -> Result<()> {
    let windows = read_windows_from_cache()?;

    for window in windows {
        restore_specific_window(&window.address)?;
    }

    Ok(())
}

fn show_restore_menu() -> Result<()> {
    println!("Starting restoration menu with Rofi...");
    
    let windows = read_windows_from_cache()?;
    
    if windows.is_empty() {
        println!("No minimized windows");
        return Ok(());
    }

    // Create temporary directory for Rofi script
    let rofi_script_dir = format!("{}/rofi", cache_dir());
    fs::create_dir_all(&rofi_script_dir)?;
    let rofi_script = format!("{}/minhypr-menu.sh", rofi_script_dir);
    
    // Create temporary Rofi configuration file
    let rofi_config = format!("{}/minhypr.rasi", rofi_script_dir);
    let config_content = r#"
configuration {
    modi: "window";
    display-window: "Minimized Windows";
    window-format: "{icon} {t}";
    window-thumbnail: true;
    show-icons: true;
    drun-display-format: "{name}";
    fullscreen: false;
    sidebar-mode: false;
}

* {
    background-color: #2E3440;
    text-color: #ECEFF4;
    border-color: #4C566A;
    selected-background: #3B4252;
    selected-text: #88C0D0;
}

window {
    width: 800px;
    border: 2px;
    border-radius: 6px;
    padding: 12px;
}

element {
    padding: 8px 12px;
    border-radius: 4px;
    spacing: 8px;
}

element selected {
    background-color: @selected-background;
    text-color: @selected-text;
}

element-icon {
    size: 32px;
}

element-text {
    vertical-align: 0.5;
}
"#;
    fs::write(&rofi_config, config_content)?;

    // Generate script for Rofi with images and descriptions
    let mut script_content = String::from("#!/bin/bash\n\n");
    script_content.push_str("function gen_entries() {\n");
    
    for window in &windows {
        let display = window.display_title.replace("\"", "\\\"");
        let address = window.address.replace("\"", "\\\"");
        
        // Add preview if available
        if let Some(preview) = &window.preview_path {
            script_content.push_str(&format!(
                "    echo -en \"{display}\\0icon\\x1f{preview}\\x1finfo\\x1f{address}\\n\"\n",
                display = display,
                preview = preview,
                address = address
            ));
        } else {
            // No preview, use only the application icon
            script_content.push_str(&format!(
                "    echo -en \"{display}\\0icon\\x1f{icon}\\x1finfo\\x1f{address}\\n\"\n",
                display = display,
                icon = window.class.to_lowercase(),
                address = address
            ));
        }
    }
    
    script_content.push_str("}\n\n");
    
    // Add logic for selection
    script_content.push_str("if [ -z \"$@\" ]; then\n");
    script_content.push_str("    gen_entries\n");
    script_content.push_str("else\n");
    script_content.push_str("    # Restore selected window\n");
    script_content.push_str("    WINDOW_ID=\"$(echo \"$@\" | sed 's/.*info\\x1f\\(.*\\)/\\1/')\" \n");
    script_content.push_str("    minhypr restore \"$WINDOW_ID\"\n");
    script_content.push_str("fi\n");
    
    // Make the script executable
    fs::write(&rofi_script, script_content)?;
    Command::new("chmod").args(["+x", &rofi_script]).output()?;
    
    // Execute Rofi with our script
    let output = Command::new("rofi")
        .args([
            "-show", "window",
            "-theme", &rofi_config,
            "-modi", &format!("window:{}", rofi_script),
            "-no-fixed-num-lines",
            "-no-click-to-exit",
            "-no-custom",
            "-window-thumbnail", // Show thumbnails if available
            "-theme-str", "window {width: 600px;}"
        ])
        .output()?;
    
    if !output.status.success() {
        // Fallback to simple Rofi if advanced configuration fails
        let mut items = String::new();
        for window in &windows {
            items.push_str(&format!("{}\n", window.display_title));
        }

        let mut selection = Command::new("rofi")
            .args([
                "-dmenu",
                "-p", "Restore window:",
                "-i", // case insensitive matching
                "-no-custom"
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        if let Some(ref mut stdin) = selection.stdin {
            stdin.write_all(items.as_bytes())?;
        }

        let output = selection.wait_with_output()?;
        let selection = String::from_utf8_lossy(&output.stdout);
        let selection = selection.trim();

        if !selection.is_empty() {
            if let Some(window) = windows.iter().find(|w| w.display_title == selection) {
                restore_specific_window(&window.address)?;
            }
        }
    }
    
    Ok(()) // Added Ok() return to correct the error
}

fn restore_window(window_id: Option<&str>) -> Result<()> {
    match window_id {
        Some(id) => restore_specific_window(id),
        None => show_restore_menu(),
    }
}

fn minimize_window() -> Result<()> {
    // Get active window information
    let output = Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()?;

    if !output.status.success() {
        return Ok(());
    }

    let window_info = String::from_utf8(output.stdout).unwrap_or_default();
    let window_data = parse_window_info(&window_info)?;

    // Do not minimize wofi (menu) windows
    if window_data.get("class").map_or(false, |c| c == "wofi") {
        return Ok(());
    }

    // Get the current workspace
    let workspace_output = Command::new("hyprctl")
        .args(["activeworkspace", "-j"])
        .output()?;

    let current_workspace = if workspace_output.status.success() {
        let workspace_info = String::from_utf8(workspace_output.stdout).unwrap_or_default();
        let workspace_data = parse_window_info(&workspace_info)?;
        workspace_data
            .get("id")
            .and_then(|id| id.parse::<i32>().ok())
            .unwrap_or(1)
    } else {
        1 // Default workspace if unable to get current one
    };

    // Extract window information
    let window_addr = match window_data.get("address") {
        Some(addr) => addr,
        None => return Ok(()),
    };
    
    let short_addr: String = window_addr.chars().rev().take(4).collect();
    
    let class_name = match window_data.get("class") {
        Some(class) => class,
        None => return Ok(()),
    };
    
    let title = match window_data.get("title") {
        Some(title) => title,
        None => return Ok(()),
    };
    
    let icon = get_app_icon(class_name);

    // Capture window preview if possible
    let preview_path = if let (Some(at), Some(size)) = (window_data.get("at"), window_data.get("size")) {
        let geometry = format!("{},{}", at.trim(), size.trim());
        capture_window_preview(window_addr, &geometry).ok()
    } else {
        None
    };

    // Create minimized window object
    let window = MinimizedWindow {
        address: window_addr.to_string(),
        display_title: format!("{} {} - {} [{}]", icon, class_name, title, short_addr),
        class: class_name.to_string(),
        original_title: title.to_string(),
        preview_path,
        icon,
        workspace: current_workspace,
    };

    // Move to special workspace (minimize)
    let output = Command::new("hyprctl")
        .args([
            "dispatch",
            "movetoworkspacesilent",
            &format!("special:minimized,address:{}", window_addr),
        ])
        .output()?;

    if output.status.success() {
        // Update list of minimized windows
        let mut windows = read_windows_from_cache()?;
        windows.push(window);
        save_windows_to_cache(&windows)?;
        signal_waybar();
    }

    Ok(())
}

fn show_status() -> Result<()> {
    let windows = read_windows_from_cache()?;
    let count = windows.len();

    if count > 0 {
        println!(
            "{{\"text\":\"󰘸 {}\",\"class\":\"has-windows\",\"tooltip\":\"{} minimized windows\"}}",
            count, count
        );
    } else {
        println!("{{\"text\":\"󰘸\",\"class\":\"empty\",\"tooltip\":\"No minimized windows\"}}");
    }

    Ok(())
}

fn generate_rofi_config() -> Result<()> {
    let home = env::var("HOME").unwrap_or_else(|_| String::from("/tmp"));
    let config_dir = format!("{}/.config/minhypr", home);
    fs::create_dir_all(&config_dir)?;
    
    // Generate Rofi theme file
    let rofi_theme = format!("{}/minhypr.rasi", config_dir);
    let theme_content = r#"/**
 * MinHypr Rofi Theme
 */

configuration {
    modi: "window";
    display-window: "Minimized Windows";
    window-format: "{icon} {t}";
    window-thumbnail: true;
    show-icons: true;
    drun-display-format: "{name}";
    fullscreen: false;
    sidebar-mode: false;
}

* {
    background:     #2E3440;
    background-alt: #3B4252;
    foreground:     #ECEFF4;
    selected:       #88C0D0;
    active:         #A3BE8C;
    urgent:         #BF616A;
    border:         #4C566A;
}

window {
    width: 650px;
    border: 2px;
    border-color: @border;
    border-radius: 6px;
    padding: 12px;
    background-color: @background;
}

mainbox {
    border: 0;
    padding: 0;
}

message {
    border: 2px 0px 0px;
    border-color: @border;
    padding: 10px;
}

textbox {
    text-color: @foreground;
}

inputbar {
    children: [ prompt, textbox-prompt-colon, entry, case-indicator ];
    padding: 12px;
}

prompt {
    text-color: @selected;
}

textbox-prompt-colon {
    expand: false;
    str: ":";
    margin: 0px 4px 0px 0px;
    text-color: @foreground;
}

entry {
    text-color: @foreground;
}

case-indicator {
    text-color: @foreground;
}

listview {
    fixed-height: 0;
    border: 2px 0px 0px;
    border-color: @border;
    spacing: 4px;
    scrollbar: true;
    padding: 10px 5px 0px;
}

element {
    border: 0;
    border-radius: 4px;
    padding: 8px 12px;
}

element normal.normal {
    background-color: inherit;
    text-color: @foreground;
}

element selected.normal {
    background-color: @background-alt;
    text-color: @selected;
}

element-icon {
    size: 42px;
    margin: 0 8px 0 0;
}

element-text {
    background-color: inherit;
    text-color: inherit;
    vertical-align: 0.5;
}

scrollbar {
    width: 4px;
    border: 0;
    handle-width: 8px;
    padding: 0;
    handle-color: @border;
}

button {
    text-color: @foreground;
    border: 2px 0px 0px;
    border-color: @border;
    border-radius: 4px;
}

button selected {
    background-color: @background-alt;
    text-color: @selected;
}
"#;
    fs::write(&rofi_theme, theme_content)?;
    
    // Generate portable launch script
    let rofi_script = format!("{}/launch-menu.sh", config_dir);
    let script_content = r#"#!/bin/bash

# Script to launch the Rofi menu for minimized windows
# Generated by MinHypr - Portable Version

# Find minhypr executable
if [ -x "$HOME/.local/bin/minhypr" ]; then
    MINHYPR="$HOME/.local/bin/minhypr"
elif [ -x "/usr/local/bin/minhypr" ]; then
    MINHYPR="/usr/local/bin/minhypr"
elif [ -x "/usr/bin/minhypr" ]; then
    MINHYPR="/usr/bin/minhypr"
elif command -v minhypr &> /dev/null; then
    MINHYPR="minhypr"
else
    notify-send "Error" "Unable to find minhypr executable"
    exit 1
fi

# Configure theme
THEME="$HOME/.config/minhypr/minhypr.rasi"

# Execute Rofi with configurations
rofi \
  -show window \
  -theme "$THEME" \
  -modi "window:$MINHYPR show-rofi" \
  -no-fixed-num-lines \
  -window-thumbnail \
  -theme-str "window {width: 650px;}"
"#;

    fs::write(&rofi_script, script_content)?;
    Command::new("chmod").args(["+x", &rofi_script]).output()?;
    
    // Generate simple backup script (in case Rofi fails)
    let simple_script = format!("{}/simple-menu.sh", config_dir);
    let simple_content = r#"#!/bin/bash

# Simple script to show and restore minimized windows
# Works as a backup in case Rofi has problems

# Find minhypr executable
if [ -x "$HOME/.local/bin/minhypr" ]; then
    MINHYPR="$HOME/.local/bin/minhypr"
elif [ -x "/usr/local/bin/minhypr" ]; then
    MINHYPR="/usr/local/bin/minhypr"
elif [ -x "/usr/bin/minhypr" ]; then
    MINHYPR="/usr/bin/minhypr"
elif command -v minhypr &> /dev/null; then
    MINHYPR="minhypr"
else
    notify-send "Error" "Unable to find minhypr executable"
    exit 1
fi

# Check if there are minimized windows
WINDOWS=$($MINHYPR show)
if [[ $WINDOWS == *"empty"* ]]; then
    notify-send "MinHypr" "No minimized windows"
    exit 0
fi

# Use simple Rofi to show the list of windows
$MINHYPR restore
"#;
    
    fs::write(&simple_script, simple_content)?;
    Command::new("chmod").args(["+x", &simple_script]).output()?;
    
    // Generate script to restore all windows
    let restore_script = format!("{}/restore-all.sh", config_dir);
    let restore_content = r#"#!/bin/bash

# Script to restore all minimized windows

# Find minhypr executable
if [ -x "$HOME/.local/bin/minhypr" ]; then
    MINHYPR="$HOME/.local/bin/minhypr"
elif [ -x "/usr/local/bin/minhypr" ]; then
    MINHYPR="/usr/local/bin/minhypr"
elif [ -x "/usr/bin/minhypr" ]; then
    MINHYPR="/usr/bin/minhypr"
elif command -v minhypr &> /dev/null; then
    MINHYPR="minhypr"
else
    notify-send "Error" "Unable to find minhypr executable"
    exit 1
fi

$MINHYPR restore-all
"#;
    
    fs::write(&restore_script, restore_content)?;
    Command::new("chmod").args(["+x", &restore_script]).output()?;
    
    println!("Rofi configuration generated in: {}", config_dir);
    println!("Available scripts:");
    println!("  {}/launch-menu.sh - Full Rofi menu", config_dir);
    println!("  {}/simple-menu.sh - Simple menu (in case Rofi fails)", config_dir);
    println!("  {}/restore-all.sh - Restores all windows", config_dir);
    
    println!("\nYou can add these shortcuts to your Hyprland:");
    println!("  bind = ALT SHIFT, M, exec, $HOME/.config/minhypr/launch-menu.sh");
    println!("  bind = ALT CTRL, M, exec, $HOME/.config/minhypr/simple-menu.sh");
    println!("  bind = ALT SHIFT, R, exec, $HOME/.config/minhypr/restore-all.sh");
    
    Ok(())
}

fn show_rofi_menu() -> Result<()> {
    let windows = read_windows_from_cache()?;
    
    if windows.is_empty() {
        println!("INFO: No minimized windows");
        return Ok(());
    }

    // Get the current workspace
    let workspace_output = Command::new("hyprctl")
        .args(["activeworkspace", "-j"])
        .output()?;

    let current_workspace = if workspace_output.status.success() {
        let workspace_info = String::from_utf8(workspace_output.stdout).unwrap_or_default();
        let workspace_data = parse_window_info(&workspace_info)?;
        workspace_data
            .get("id")
            .and_then(|id| id.parse::<i32>().ok())
            .unwrap_or(1)
    } else {
        1 // Default workspace if unable to get current one
    };

    // Verify closed/restored windows
    let mut updated_windows = Vec::new();
    let mut at_least_one_changed = false;
    
    // Verify which windows actually exist
    let window_check = Command::new("hyprctl")
        .args(["clients", "-j"])
        .output()?;
    
    let windows_json = String::from_utf8(window_check.stdout).unwrap_or_default();
    
    // Show only existing windows - simpler format for parsing
    for window in &windows {
        if windows_json.contains(&window.address) {
            // This window still exists
            updated_windows.push(window.clone());
            
            // Use simpler and more reliable format
            // Short title followed by address with "info" prefix
            let short_title = format!("{} - {}", window.class, window.original_title);
            let short_addr = window.address.chars().rev().take(8).collect::<String>();
            
            // Include workspace information in display
            println!("[WS:{}] {} [{}] info{}", 
                window.workspace, 
                short_title, 
                short_addr, 
                window.address);
        } else {
            // This window no longer exists, we don't include
            at_least_one_changed = true;
        }
    }
    
    // Update list of windows if changes
    if at_least_one_changed {
        save_windows_to_cache(&updated_windows)?;
        signal_waybar();
    }
    
    Ok(())
}

fn signal_waybar() {
    Command::new("pkill")
        .args(["-RTMIN+8", "waybar"])
        .output()
        .ok();
}

fn main() -> Result<()> {
    // Create necessary directories
    fs::create_dir_all(cache_dir())?;
    fs::create_dir_all(preview_dir())?;

    if !Path::new(cache_file()).exists() {
        save_windows_to_cache(&Vec::new())?;
    }

    // Process arguments
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("");

    match command {
        "minimize" => {
            minimize_window()?;
        }
        "restore" => {
            let window_id = args.get(2).map(|s| s.as_str());
            restore_window(window_id)?;
        }
        "restore-all" => {
            restore_all_windows()?;
        }
        "restore-last" => {
            let windows = read_windows_from_cache()?;
            if let Some(window) = windows.first() {
                restore_specific_window(&window.address)?;
            } else {
                println!("No minimized windows to restore");
            }
        }
        "show" => {
            show_status()?;
        }
        "show-rofi" => {
            // Special command for integration with Rofi
            show_rofi_menu()?;
        }
        "setup-rofi" => {
            // Generate Rofi configuration files
            generate_rofi_config()?;
        }
        _ => {
            println!("Unknown command: {}", command);
            println!("Usage: minhypr <command> [window_id]");
            println!("Available commands:");
            println!("  minimize       - Minimize active window");
            println!("  restore        - Show menu to restore windows");
            println!("  restore <id>   - Restore specific window");
            println!("  restore-all    - Restore all windows");
            println!("  restore-last   - Restore last minimized window");
            println!("  show           - Show status for waybar");
            println!("  setup-rofi     - Configure integration with Rofi");
            println!("  show-rofi      - Internal script used by Rofi");
        }
    }
    
    Ok(())
}