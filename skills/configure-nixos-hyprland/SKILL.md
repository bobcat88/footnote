---
name: configure-nixos-hyprland
description: Use when changing a NixOS or Home Manager configuration, a flake, or a Hyprland desktop (keybinds, monitors, waybar, services, startup), or debugging a Linux desktop that won't build, log in or render. Match the installed versions, build before switching, keep a rollback, one change at a time.
license: MIT
---

# Configure NixOS and Hyprland

## Run
- State: `nixos-version` / `hyprctl version` / `nix flake metadata` / `nix-env --list-generations -p /nix/var/nix/profiles/system`
- Check: `nix flake check` → `nixos-rebuild build --flake .#<host>` → `nvd diff /run/current-system result` (if installed)
- Try: `sudo nixos-rebuild test --flake .#<host>` (no boot entry) → `switch` only when it works and you're allowed
- Back: `sudo nixos-rebuild switch --rollback` / pick an older generation in the boot menu
- Hyprland live: `hyprctl reload` / `hyprctl monitors` / `hyprctl configerrors`

## Before editing
- Find the entry points: `flake.nix`, host module, Home Manager module, `hyprland.conf` or its Nix module
- Read docs for the installed version; Hyprland syntax changes between releases, don't paste the latest into an older install
- Don't migrate formats or channels as a side effect

## Editing
- One change, build, test; then the next
- Pin inputs in `flake.lock`; update one input at a time (`nix flake update <input>`) and note why
- Secrets never in the Nix store (world-readable): sops-nix or agenix, or a file outside the store
- Services: `systemctl status <unit>` / `journalctl -u <unit> -b` after a switch; user units with `--user`
- Keep a working TTY login and a keybind to a terminal; a broken compositor must not lock you out

## Hyprland
- Monitors: name from `hyprctl monitors`, explicit resolution@rate, position, scale
- Env vars for Wayland toolkits set once (Hyprland `env =` or the Nix module), not in several places
- Startup: `exec-once` for daemons, `exec` only for what must rerun on reload
- Changing animations, blur or shadows: measure idle CPU and GPU before and after, on the real display

## Done
- Builds / switches / logs in / keyboard, monitors, audio, network, suspend and resume work as before / rollback tested once
- A build on another machine or a parsed file isn't a desktop test; without the host, acceptance is pending
