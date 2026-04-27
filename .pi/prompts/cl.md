---
description: Audit changelog entries before release
---
Audit changelog entries for all commits since the last release.

## Process

1. **Find the last release tag:**
   ```bash
   git tag --sort=-version:refname | head -1
   ```

2. **List all commits since that tag:**
   ```bash
   git log <tag>..HEAD --oneline
   ```

3. **Read each crate's `[Unreleased]` section:**
   - evtide-core/CHANGELOG.md
   - evtide/CHANGELOG.md
   - evtide-codec/CHANGELOG.md (if exists)
   - evtide-raw/CHANGELOG.md (if exists)
   - evtide-usb/CHANGELOG.md (if exists)

4. **For each commit, check:**
   - Skip: changelog updates, doc-only changes, release housekeeping
   - Determine which crate(s) the commit affects (use `git show --stat`)
   - Verify a changelog entry exists in the affected crate(s)

5. **Report:**
   - List commits with missing entries
   - Add any missing entries directly

## Changelog Format Reference

Sections (in order, see AGENTS.md for full rules):
- `### Breaking Changes` - API changes requiring migration
- `### Added` - New features
- `### Changed` - Changes to existing functionality
- `### Fixed` - Bug fixes
- `### Removed` - Removed features
