# Roadmap

## Completed v1.x Summary

LocalAccessGuard v1.x is complete as a Windows network diagnostic CLI for proxy/PAC/WinHTTP cleanup, profile-based DNS/TCP diagnostics, report generation, report comparison, network info inspection, UDP checks, and simple `config.json` support.

## Planned Skip

Versions v1.4.0 through v1.9.0 are intentionally skipped so the project can move directly from the finalized v1.x line to the next major architecture milestone.

## Current Milestone

v2.0.0 is the current milestone and focuses on modular code organization. The large `src/main.rs` has been split into smaller source modules for easier maintenance.

## Planned v2.x Ideas

- Cleaner error handling
- Easier testing
- Future logging
- Additional report formats
- Profile groups
- Firewall inspection
