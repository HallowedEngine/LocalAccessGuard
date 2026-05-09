# LocalAccessGuard

LocalAccessGuard is a Rust-based Windows CLI network diagnostic and cleanup utility.

It checks Windows proxy-related settings, detects known network tools, runs profile-based DNS/TCP diagnostics, exports timestamped reports, and can safely clean stale proxy configuration.

## Current Version

v0.5.0

## Features

- Windows Proxy inspection
- AutoConfigURL / PAC proxy inspection
- WinHTTP proxy inspection
- Known network tool process detection
- Cloudflare WARP service warning
- GoodbyeDPI / BypaxDPI process detection
- Profile-based diagnostics with JSON files
- DNS resolution tests
- TCP 443 connectivity tests
- Safe restore / cleanup command
- Timestamped diagnostic report export
- Profile listing
- Profile validation

## Commands

### Show system status

```bash
cargo run -- status

Checks:

Windows Proxy
AutoConfigURL / PAC proxy
WinHTTP proxy
Known network tool processes
Run diagnostics for a profile
cargo run -- doctor discord
cargo run -- doctor roblox

Profiles are loaded from:

profiles/*.json
List available profiles
cargo run -- profiles

Example output:

Available profiles:
- discord: Discord
- roblox: Roblox
Validate profiles
cargo run -- validate

Validation checks:

JSON syntax
Empty profile name
Empty domain list
Empty domain entries
Empty TCP test domain
Generate report
cargo run -- report

Reports are saved under:

reports/

Example:

reports/local_access_report_2026-05-09_15-42-34.txt
Restore proxy settings
cargo run -- restore

Restore action:

Disables Windows user proxy
Deletes stale ProxyServer entry
Deletes AutoConfigURL / PAC proxy entry
Resets WinHTTP proxy only when needed
Profile Format

Example profiles/discord.json:

{
  "name": "Discord",
  "domains": [
    "discord.com",
    "discord.gg"
  ],
  "tcp_test_domain": "discord.com"
}

Example profiles/roblox.json:

{
  "name": "Roblox",
  "domains": [
    "roblox.com",
    "www.roblox.com"
  ],
  "tcp_test_domain": "roblox.com"
}
Release Build
cargo build --release

Release executable path:

target/release/LocalAccessGuard.exe

Versioned release binaries are kept locally under:

releases/
Version History
v0.1.0

Initial diagnostic prototype.

v0.2.0

Added restore and proxy cleanup.

v0.3.0

Added timestamped diagnostic report export.

v0.4.0

Added JSON profile system.

v0.5.0

Added profile listing and profile validation.

Notes

LocalAccessGuard does not act as a VPN, proxy, packet manipulation engine, or DPI bypass engine.

It is currently a local diagnostic, cleanup, and reporting utility.