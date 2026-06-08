# AGENTS.md — Savant Trading

## Release Workflow (MANDATORY)

Every time you push code:

1. **Update CHANGELOG.md** — add entries for all changes since last push
2. **Review README.md** — update test counts, FID counts, version references, any stale data
3. **Commit docs** — `git add CHANGELOG.md README.md && git commit -m "docs: ..."`
4. **Push** — `git push`
5. **Create/update GitHub release** — `gh release create v{VERSION} --title "..." --notes "..."`

Never push code without updating CHANGELOG + README first. Never skip the GitHub release.

## Build & Test

```bash
cargo clippy -- -D warnings   # Zero warnings
cargo test                     # 264 tests
cargo build --release          # Release build
cd dashboard && npm run build  # Dashboard TypeScript
```

## Protocol

- ECHO Protocol v0.1.0, strict_mode: true
- 15 laws enforced (4 core + 11 extended)
- FIDs in `dev/fids/`, archived to `dev/fids/archive/`
- Session summaries in `dev/session-summaries/`
- Learnings in `dev/LEARNINGS.md`
