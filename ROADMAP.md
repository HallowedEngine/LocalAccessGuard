# Roadmap

## Completed v1.x Summary

LocalAccessGuard v1.x is complete as a Windows network diagnostic CLI for proxy/PAC/WinHTTP cleanup, profile-based DNS/TCP diagnostics, report generation, report comparison, network info inspection, UDP checks, and simple `config.json` support.

## Planned Skip

Versions v1.4.0 through v1.9.0 are intentionally skipped so the project can move directly from the finalized v1.x line to the next major architecture milestone.

## Next Major Milestone

v2.0.0 will focus on a modular architecture refactor.

## Planned v2.x Ideas

- `src/config.rs`
- `src/profiles.rs`
- `src/network.rs`
- `src/report.rs`
- `src/diagnostics.rs`
- `src/cli.rs`
- Cleaner error handling
- Easier testing
- Future logging, report export, profile groups, and firewall inspection
