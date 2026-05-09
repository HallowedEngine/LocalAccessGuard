# Changelog
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