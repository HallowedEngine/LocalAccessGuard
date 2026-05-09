# LocalAccessGuard

LocalAccessGuard is a Rust-based Windows network diagnostic, cleanup, profile-checking, and reporting CLI utility.

It checks Windows proxy-related settings, detects known network tools, runs profile-based DNS/TCP diagnostics, inspects network adapter and DNS configuration, runs basic UDP diagnostics, exports timestamped reports, and can safely clean stale proxy configuration.

## Current Version

v1.2.0

## Features

- Windows proxy, PAC, and WinHTTP proxy inspection
- Known network tool process detection
- Safe proxy restore and cleanup
- JSON profile-based diagnostics
- DNS resolution and TCP 443 connectivity tests
- Profile listing and validation
- Timestamped diagnostic reports with summaries and risk scoring
- Report comparison for before/after troubleshooting
- Network adapter and DNS server inspection
- Basic UDP diagnostics
- Simple `config.json` support
- Clear CLI help aliases and readable terminal status prefixes

## Commands

```bash
cargo run -- status
cargo run -- help
cargo run -- restore
cargo run -- report
cargo run -- doctor discord
cargo run -- doctor roblox
cargo run -- profiles
cargo run -- validate
cargo run -- compare reports\old.txt reports\new.txt
cargo run -- netinfo
cargo run -- udpcheck
cargo run -- config
```

Profiles and reports use the effective configuration. By default, profiles are loaded from `profiles/*.json` and reports are saved under `reports/`.

## Config

LocalAccessGuard reads `config.json` from the project root when it exists. If the file is missing or invalid, built-in defaults are used so existing commands continue to work.

Configurable values include the profile directory, report directory, default report profiles, UDP test target, and whether Cloudflare WARP warnings are shown and counted.

Show the effective configuration with:

```bash
cargo run -- config
```

## Profile Format

```json
{
  "name": "Discord",
  "domains": [
    "discord.com",
    "discord.gg"
  ],
  "tcp_test_domain": "discord.com"
}
```

## Release Build

```bash
cargo build --release
```

The release executable is created at `target/release/LocalAccessGuard.exe`.

## Version History

- v1.2.0: Added config.json support.
- v1.1.0: Improved CLI help and terminal output readability.
- v0.1.0: Initial diagnostic prototype.
- v0.2.0: Added restore and proxy cleanup.
- v0.3.0: Added timestamped diagnostic report export.
- v0.4.0: Added JSON profile system.
- v0.5.0: Added profile listing and profile validation.
- v0.6.0: Added report summary and risk scoring.
- v0.7.0: Added report comparison.
- v0.8.0: Added network adapter and DNS server inspection.
- v0.9.0: Added basic UDP diagnostics.
- v1.0.0: First stable CLI release.

## Notes

LocalAccessGuard is not a VPN, bypass tool, proxy engine, packet manipulation engine, or engine wrapper.

It is a Windows network diagnostic, cleanup, profile-checking, and reporting CLI utility.
