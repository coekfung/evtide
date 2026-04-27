# Development Rules

## Crate Layout

```
evtide/                         ← virtual manifest workspace
├── evtide-core/                ← EventCd, EventExtTrigger (domain types, zero deps)
├── evtide-codec/               ← sans-IO EVT stream format decoders (→ core)
├── evtide-raw/                 ← .raw file reader (→ core, codec)
├── evtide-usb/                 ← USB camera comms [planning] (→ core, codec)
└── evtide/                     ← umbrella re-export (→ all above)
```

## Conversational Style

- Keep answers short and concise
- No emojis in commits, issues, PR comments, or code
- No fluff or cheerful filler text
- Technical prose only, be kind but direct

## Code Quality

- `#![forbid(unsafe_code)]` — unsafe code is never allowed in this crate
- All public APIs must be documented with `///` doc comments
- Follow Rust API guidelines where applicable
- Do not preserve backward compatibility unless the user explicitly asks for it
- Always ask before removing functionality or code that appears to be intentional

## Commands

- After code changes (not documentation-only changes): `cargo check` (get full output, no tail). Fix all errors before committing.
- After code changes: `cargo clippy -- -D warnings`. If clippy reports warnings, ask the user for confirmation before fixing them.
- Format code before committing: `cargo fmt`
- Run tests: `cargo test` (from crate root)
- If you create or modify a test, you MUST run that test and iterate until it passes.
- NEVER commit unless the user explicitly asks

## Changelog

Location: `<crate>/CHANGELOG.md` (each sub-crate has its own).

### Format

Use these sections under `## [Unreleased]`:

- `### Breaking Changes`
- `### Added`
- `### Changed`
- `### Fixed`
- `### Removed`

### Rules

- New entries ALWAYS go under `## [Unreleased]`
- Append to existing subsections, do not create duplicates
- NEVER modify already-released version sections
- Before adding entries, read the full `[Unreleased]` section to see which subsections already exist

## Releasing

**Lockstep versioning**: All sub-crates always share the same version number.

**Version semantics**:

- `patch`: Bug fixes, new features, non-breaking changes
- `minor`: API breaking changes

### Steps

1. Update `CHANGELOG.md` in each affected crate: finalize `[Unreleased]` with the new version and date
2. Bump `version = "X.Y.Z"` in the `[package]` section of every sub-crate's `Cargo.toml`
3. Bump `version = "X.Y.Z"` in every `[dependencies]` entry that references a workspace crate
   (e.g., `evtide-core = { path = "../evtide-core", version = "X.Y.Z" }` in `evtide/Cargo.toml`)
4. Commit: `git commit -m "chore: release vX.Y.Z"`
5. Tag: `git tag vX.Y.Z`
6. Publish in dependency order: `cargo publish -p evtide-core && cargo publish -p evtide`
7. Add a new `## [Unreleased]` section to each crate's `CHANGELOG.md`

## Branch Workflow

- Work in feature branches off `main`
- Once a feature is complete and tests/checks pass, merge into `main`
- Never open PRs yourself — work in feature branches until the user's requirements are met, then merge into `main` and push

## Commitment Messages

- Use conventional commits: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`
- Include `fixes #<number>` or `closes #<number>` when there is a related issue or PR

## **CRITICAL** Git Rules for Parallel Agents **CRITICAL**

Multiple agents may work on different files in the same worktree simultaneously. You MUST follow these rules:

### Committing

- **ONLY commit files YOU changed in THIS session**
- NEVER use `git add -A` or `git add .` — these sweep up changes from other agents
- ALWAYS use `git add <specific-file-paths>` listing only files you modified
- Before committing, run `git status` and verify you are only staging YOUR files
- Track which files you created/modified/deleted during the session

### Forbidden Git Operations

These commands can destroy other agents' work:

- `git reset --hard` — destroys uncommitted changes
- `git checkout .` — destroys uncommitted changes
- `git clean -fd` — deletes untracked files
- `git stash` — stashes ALL changes including other agents' work
- `git add -A` / `git add .` — stages other agents' uncommitted work

### Safe Workflow

```bash
# 1. Check status first
git status

# 2. Add ONLY your specific files
git add src/foo.rs
git add Cargo.toml

# 3. Commit
git commit -m "feat: description"

# 4. Push (pull --rebase if needed, but NEVER reset/checkout)
git pull --rebase && git push
```

### If Rebase Conflicts Occur

- Resolve conflicts in YOUR files only
- If conflict is in a file you didn't modify, abort and ask the user
- NEVER force push

### User Override

If the user instructions conflict with rules set out here, ask for confirmation that they want to override the rules. Only then execute their instructions.
