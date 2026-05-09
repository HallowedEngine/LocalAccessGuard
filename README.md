# LocalAccessGuard

LocalAccessGuard is a Rust-based Windows network diagnostic, cleanup, profile-checking, and reporting CLI utility.

It checks Windows proxy-related settings, detects known network tools, runs profile-based DNS/TCP diagnostics, inspects network adapter and DNS configuration, runs basic UDP diagnostics, exports timestamped reports, and can safely clean stale proxy configuration.

## Current Version

v2.1.0

## Stable Scope

LocalAccessGuard is a stable Windows network diagnostic CLI focused on proxy/PAC/WinHTTP cleanup, profile-based DNS/TCP diagnostics, report generation, and report comparison.

It is:

- a Windows network diagnostic CLI
- a cleanup utility for proxy/PAC/WinHTTP settings
- a profile-based DNS/TCP diagnostic tool
- a report generator and comparison utility

It is not:

- a VPN
- a proxy engine
- a DPI bypass tool
- a packet manipulation tool

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
- Unique report filenames with millisecond timestamps
- Plain text, JSON, and Markdown report formats
- Simple append-only local logging
- Profile groups for grouped doctor checks
- Clear CLI help aliases and readable terminal status prefixes

## Commands

```text
cargo run -- status
cargo run -- restore
cargo run -- profiles
cargo run -- validate
cargo run -- doctor <profile>
cargo run -- doctor-group <group>
cargo run -- report
cargo run -- report --format <txt|json|md>
cargo run -- groups
cargo run -- compare <old_report> <new_report>
cargo run -- netinfo
cargo run -- udpcheck
cargo run -- config
cargo run -- help
```

Profiles, groups, reports, and logs use the effective configuration. By default, profiles are loaded from `profiles/*.json`, groups from `groups/*.json`, reports are saved under `reports/`, and logs are appended under `logs/`.

## Config

LocalAccessGuard reads `config.json` from the project root when it exists. If the file is missing or invalid, built-in defaults are used so existing commands continue to work.

Configurable values include profile, group, report, and log directories, default report profiles, UDP test target, whether Cloudflare WARP warnings are shown and counted, and whether local logging is enabled.

Show the effective configuration with:

```bash
cargo run -- config
```

## Report Formats

The default report command still writes the normal plain text report:

```bash
cargo run -- report
```

Reports can also be generated explicitly as text, JSON, or Markdown:

```bash
cargo run -- report --format txt
cargo run -- report --format json
cargo run -- report --format md
```

Report filenames include millisecond timestamps and add a numeric suffix if a matching file already exists.

## Logs

When `enable_logging` is true, LocalAccessGuard appends basic command and warning events to:

```text
logs/local_access_guard.log
```

Logging is local only and avoids secrets, tokens, packet payloads, and private traffic content. Logging failures print a warning but do not stop the command.

## Profile Groups

Groups are JSON files under `groups/*.json`:

```json
{
  "name": "gaming",
  "profiles": ["discord", "roblox"]
}
```

List groups and run doctor checks for a group with:

```bash
cargo run -- groups
cargo run -- doctor-group gaming
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

- v2.1.0: Added unique report filenames, local logging, report formats, and profile groups.
- v2.0.0: Split code into smaller source modules for easier maintenance.
- v1.3.0: Final v1 polish before v2.0 modular refactor.
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
