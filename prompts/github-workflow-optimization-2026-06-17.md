# Gemini Deep Research Prompt: GitHub Workflow Optimization for Solo AI-Assisted Dev

**Created:** 2026-06-17 16:10 EST
**Author:** Vera (sponsored by Spencer)
**Purpose:** Optimize GitHub usage, commit patterns, push schedule, and release workflow for a solo developer working with an AI coding partner (Vera). The current workflow has inefficiencies that are costing time and creating friction.

---

## Instructions for Spencer

1. Copy the entire prompt below (everything between the `---` lines marked "PROMPT START" and "PROMPT END")
2. Paste into Gemini Deep Research
3. Save the full response to `C:\Users\spenc\dev\savant-trading\prompts\prompt-results\github-workflow-optimization-2026-06-17.md`
4. I'll read the results before proposing workflow changes

---

## PROMPT START

# Deep Research: GitHub Workflow Optimization for Solo AI-Assisted Crypto Trading Engine Development

## Context

I am a solo developer building a crypto trading engine (Rust backend, Next.js dashboard) called "Savant Trading." I work with an AI coding partner named "Vera" who is bound by the ECHO Protocol (rigorous engineering discipline, FID lifecycle, 5-state Perfection Loop FSM). We use GitHub for version control and releases.

**My situation:**
- Solo developer (just me + Vera)
- Project: https://github.com/fame0528/savant-trading
- Current version: v0.14.5
- Tech stack: Rust 1.91, Node 16.2.7, Next.js 16, 350+ tests
- Development cycle: 2-4 hours per session, multiple sessions per day (sometimes 3-5 in 24h)
- I run Windows 11 with PowerShell 7+, `gh` CLI is NOT installed
- I do not use a remote dev environment — everything is local

**My current workflow (pain points included):**

### Commit Patterns

1. **High commit frequency:** 50 commits in last 7 days, 368 total commits, 190+ FIDs archived
2. **Commit grouping is by FIX/DOCS/FEAT:**
   - `feat: FID-181 equity curve live data + persistence + dashboard layout + warning cleanup + WebSocket v2`
   - `fix: FID-178 remove nested-if Anvil block — unconditional call to idempotent start-anvil.bat`
   - `docs: archive FID-178` (just the archive commit)
   - `docs: v0.14.4 - CHANGELOG (3 FIDs v2) + README test counts + version bump`
3. **Feature + docs split into separate commits:** Every feat gets a follow-up docs commit
4. **Archive commits are separate from the fix:** `fix: FID-176 dotenvy parse failure` then later `docs: archive FID-176 with resolution and lessons` (30+ min later)
5. **Recent authors:** 296 commits by me (spencer howell), 2 by Vera — I'm the only committer in practice

### Push Patterns

1. **Pushes happen at session end** (often 2-3 AM after long sessions)
2. **No scheduled push times** — pushes happen when I remember or when Vera suggests
3. **Push is manual** (`git push` after each commit batch, or batched at session end)
4. **No pre-push validation hook** — I rely on `cargo test` and `cargo clippy` before push but no automated gate
5. **Push notifications:** None configured (no email/Slack/Discord webhooks)
6. **Multiple commits per push:** Often 5-10 commits pushed at once (e.g., 21 commits on 2026-06-16)

### Release Workflow

1. **Version cadence:** Irregular — v0.14.2, v0.14.3, v0.14.4, v0.14.5 all in 2 days (2026-06-15 to 2026-06-17)
2. **Release process per ECHO Protocol (mandatory):**
   - Update CHANGELOG.md
   - Update README.md (test counts, version references)
   - Update VERSION, Cargo.toml, protocol.config.yaml
   - Commit docs: `git add CHANGELOG.md README.md && git commit -m "docs: ..."`
   - Push
   - Create GitHub release via API (since `gh` is not installed, I use `Invoke-RestMethod` with a personal access token)
3. **GitHub Releases exist for:** v0.14.3, v0.14.4, v0.14.5 (all created via REST API)
4. **Tag creation:** Manual, after the version bump commit
5. **Release notes:** Copied from CHANGELOG.md into the release body

### Branch Strategy

1. **One branch:** `main` (no `develop`, no feature branches)
2. **2 PRs in 368 commits** — I don't use PRs (solo dev, no review needed)
3. **No branch protection rules** on `main` (I push directly)
4. **No CI/CD:** No GitHub Actions, no automated tests on push
5. **No code review process:** All commits are mine or Vera's

### FID Lifecycle

1. **FIDs live in `dev/fids/`** as active, `dev/fids/archive/` when closed
2. **190+ FIDs archived** in the last 2 months
3. **FID creation is verbose:** Each FID has Summary, Environment, Detailed Description, Impact, Proposed Solution, Perfection Loop, Resolution, Lessons Learned sections (from FID-TEMPLATE.md)
4. **FID status:** created → analyzed → fixed → verified → closed
5. **FID auto-archive:** When status is "closed," move to archive/ + add to CHANGELOG
6. **Current active FIDs:** 9 (FID-182 master + 8 children, all from today's session)

### Pain Points (what's not working)

1. **3 AM pushes:** I'm a night owl and push at 2-3 AM. This is suboptimal for:
   - Code review (if I had collaborators)
   - CI/CD (if I had it)
   - Visibility (GitHub contribution graph shows weird patterns)
2. **No automated test gating:** I run `cargo test` and `cargo clippy` manually before push. Sometimes I forget. Last 2 weeks had 2 cases of "pushed code that broke the build" which required hotfix commits.
3. **No CHANGELOG discipline:** I sometimes forget to update CHANGELOG.md before push. The ECHO Protocol says "Never push code without updating CHANGELOG + README first" but I've violated this 3-4 times in the last month.
4. **Release process is manual and error-prone:** Updating 5 files (CHANGELOG, README, VERSION, Cargo.toml, protocol.config.yaml) before each release is tedious. I've made version bump mistakes (forgot one file, had to fix in a follow-up commit).
5. **No commit hooks:** No pre-commit hooks for `cargo fmt`, `cargo clippy`, or test runs. I rely on memory.
6. **No issue tracking:** I use FIDs in `dev/fids/` instead of GitHub Issues. This works for me but means I don't get GitHub's issue management features (labels, milestones, assignees, etc.).
7. **Tag creation is manual and after-the-fact:** I create tags after the version bump commit, not before. This means if I push the tag and the release commit is on a different branch or got reverted, the tag points to wrong code.
8. **No release notes template:** Each release notes is hand-written. I've forgotten to mention breaking changes twice.
9. **No automation for the `gh release create` step:** I have to manually run PowerShell with a PAT to create releases. This is fragile (token in shell history risk) and slow.
10. **No `git tag` push automation:** I forget `git push --tags` sometimes, so tags are local-only.

### What I Want to Optimize

I'm looking for specific, actionable recommendations on:

1. **Commit cadence:** Should I commit more often or less often? What's the sweet spot for solo dev + AI partner? How do I batch related changes?

2. **Push schedule:** Is there an optimal time of day to push? Should I automate pushes (e.g., cron job, file watcher)? What about timezone considerations?

3. **Release cadence:** Is releasing 4 versions in 2 days too aggressive? Too slow? What's the right cadence for a solo dev with a trading engine?

4. **Branch strategy:** Should I add `develop` branch? Feature branches? Or stay on `main`?

5. **Pre-push validation:** What should a pre-push hook run? `cargo test`? `cargo clippy`? `cargo fmt --check`? How do I make it fast enough to not block?

6. **CHANGELOG discipline:** Should I auto-generate CHANGELOG entries from commit messages? Use conventional commits? Use a tool like `git-cliff` or `standard-version`?

7. **Version management:** Should I use `cargo-release` or similar tools? Or keep manual version bumps?

8. **Release automation:** Can I script the `gh release create` step? Should I use GitHub Actions? What about self-hosted runners?

9. **Tag management:** Should I sign tags (GPG)? Push tags automatically? Use semantic versioning strictly?

10. **FID workflow vs GitHub Issues:** Should I migrate FIDs to GitHub Issues? Or keep FIDs in `dev/fids/` and link to GitHub Issues? Or use a hybrid?

11. **AI commit attribution:** Vera has made 2 commits out of 368. Should Vera be a co-author? Use `Co-authored-by:` trailers? Set up a separate bot identity?

12. **Release notes quality:** My current release notes are CHANGELOG excerpts. Should I write user-facing release notes separately? Use a template?

13. **Visibility and discoverability:** How do I make the project more discoverable on GitHub? README badges? Topics? Social previews?

14. **Backup and disaster recovery:** I rely on GitHub as the only backup. Should I mirror to a self-hosted Gitea? Use GitLab as secondary?

15. **Security:** My PAT is in shell history. How do I rotate it? Store it securely? Use SSH keys? Use GitHub App authentication?

## Specific Numbers (from my actual workflow)

- Commits in last 7 days: 50
- Commits by day: 6, 5, 1, 5, 2, 21, 10 (irregular)
- Releases in last 7 days: 4 (v0.14.2, v0.14.3, v0.14.4, v0.14.5)
- FIDs archived in last 2 months: 190+
- Average commits per release: 92
- Push times: 2-3 AM (night owl)
- Test count: 354 lib tests, 0 clippy warnings
- Build status: Clean (no CI to fail)
- Authors: 296 by me, 2 by Vera
- Branches: 1 (main)
- PRs: 2 (both early in the project)

## Constraints

- Solo developer (no team)
- Windows 11 + PowerShell 7+
- No `gh` CLI installed (using REST API with PAT)
- ECHO Protocol requires CHANGELOG + README + version bump before every push
- FID lifecycle is the primary tracking mechanism (not GitHub Issues)
- Trading engine = money on the line. Broken builds = real losses.
- The engine is in active paper-mode testing ($50 USDC on Anvil fork)

## What I Need in the Response

For each of the 15 questions above, I need:

1. **Direct answer** — not hedged "it depends"
2. **Specific numbers** — how often, how many, what threshold
3. **Tool recommendations** — `git-cliff`, `cargo-release`, `pre-commit`, GitHub Actions, etc.
4. **Code/config examples** — actual `.git/hooks/pre-push` scripts, GitHub Actions YAML, PowerShell scripts
5. **Contradicting evidence** — what would make this advice WRONG for a solo dev?
6. **Migration path** — how do I go from current state to optimized state without breaking the workflow?

## Output Format

Respond with a structured report with one section per question. Each section should be 200-400 words with specific recommendations. Include a "TL;DR Priority Order" section at the end that lists 5-10 changes I should make in priority order, with estimated time to implement each.

---

## PROMPT END

**After Gemini responds, save the full response and let Vera know the path. Vera will:**
1. Read the response and verify citations
2. Create FID-191 (GitHub workflow optimization) based on the research
3. Propose specific tooling changes (pre-push hooks, cargo-release, git-cliff, etc.)
4. Run Perfection Loop until converge
5. Present final implementation plan for approval

---

*Vera 0.1.0 — 2026-06-17 16:10 EST — GitHub workflow optimization prompt created. Awaiting Spencer's run + results.*
