# Changelog

## v1.1.0

### Added
- `help`, `--help`, and `-h` help aliases
- Clearer CLI usage and command examples
- Plain status prefixes for terminal output

### Changed
- Improved terminal output readability
- Improved help screen formatting
- Updated project version to v1.1.0

## v1.0.0

### Added
- First stable CLI release
- Finalized command set for v1.0.0
- Stable release documentation

### Changed
- Updated project version to v1.0.0
- Updated README version history
- Prepared the project for portfolio/release usage

### Verified
- `status`
- `restore`
- `profiles`
- `validate`
- `doctor <profile>`
- `report`
- `compare <old_report> <new_report>`
- `netinfo`
- `udpcheck`

## v0.9.0

### Added
- `udpcheck` command for basic UDP diagnostics
- UDP socket bind test
- UDP connect test
- UDP diagnostics section in generated reports
- UDP capability notes for Discord voice and Roblox gameplay troubleshooting

### Changed
- Report output now includes basic UDP diagnostic information

## v0.8.0

### Added
- `netinfo` command for Windows network adapter inspection
- Active network adapter information
- IPv4 address reporting
- Default gateway reporting
- DNS server reporting
- DHCP status reporting
- Network info section in generated reports

### Changed
- Report output now includes network adapter and DNS configuration details

## v0.7.0

### Added
- `compare <old_report> <new_report>` command
- Report summary comparison
- Overall status comparison
- Warning count comparison
- DNS failure count comparison
- TCP failure count comparison
- Reason list diffing between two reports

### Changed
- Reports can now be used in before/after troubleshooting workflows

## v0.6.0

### Added
- Report summary section
- Basic risk scoring for generated reports
- Warning count in report output
- DNS failure count in report output
- TCP failure count in report output
- Overall status field in report output
- Human-readable reasons for warnings and failures

### Changed
- Report output now starts with a summary before detailed diagnostics

## v0.5.0

### Added
- `profiles` command for listing valid JSON profiles
- `validate` command for validating profile JSON files
- Validation for empty profile names
- Validation for empty domain lists
- Validation for empty domain entries
- Validation for empty TCP test domains

### Changed
- Profile handling is now easier to inspect from the CLI

## v0.4.0

### Added
- JSON profile system
- `profiles/discord.json`
- `profiles/roblox.json`
- `doctor <profile>` now loads profile data from JSON
- `report` now includes diagnostics for all JSON profiles

### Changed
- Discord and Roblox domains are no longer hardcoded in command routing
- Service diagnostics are now profile-driven

## v0.3.0

### Added
- Working `report` command
- Automatic `reports/` directory creation
- Timestamped diagnostic report files
- Windows proxy status in report output
- AutoConfigURL / PAC proxy status in report output
- WinHTTP proxy status in report output
- Known network tool process status in report output
- Discord DNS and TCP 443 diagnostics in report output
- Roblox DNS and TCP 443 diagnostics in report output

### Changed
- `report` is now prepared for before/after network comparison workflows

## v0.2.0

### Added
- Working `restore` command
- AutoConfigURL / PAC proxy detection
- Warning for background Cloudflare WARP service
- Warning for active DPI/proxy tool processes

### Changed
- `status` now checks PAC proxy configuration
- Process checker now prints warnings for network tools

### Fixed
- Stale ProxyServer entries can now be removed by `restore`
- WinHTTP proxy can now be reset by `restore`


## v0.1.0

### Added
- Initial CLI structure
- `status` command
- `doctor discord` command
- `doctor roblox` command
- Windows proxy status check
- WinHTTP proxy status check
- Known process detection
- DNS resolution test
- TCP 443 connectivity test
