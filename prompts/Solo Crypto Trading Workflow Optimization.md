# **Automated DevOps and Version Control Management Architecture for AI-Assisted Algorithmic Trading Systems**

## **Commit Cadence Optimization**

The development of high-performance cryptocurrency trading systems requires a version control system that maintains a high-fidelity record of changes while minimizing developer overhead. The active developer and the artificial intelligence coding assistant, Vera, currently maintain a high commit frequency of 50 commits per week.1 To optimize this workflow, the developer must adopt a highly structured commit cadence tied directly to the state transitions of the Feature-Implementation-Document (FID) Perfection Loop State Machine.

       ┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐  
       │   1\. ANALYZED   │──────\>│     2\. FIXED    │──────\>│   3\. VERIFIED   │  
       └─────────────────┘       └─────────────────┘       └─────────────────┘  
                │                         │                         │  
                ▼                         ▼                         ▼  
         commit \-m "feat"          commit \-m "fix"           commit \-m "test"  
         (Staged Changes)          (Logic Resolved)          (Pass assertions)

The optimal commit frequency requires 3 to 5 commits per active FID. The developer must commit code at every state transition within the local compilation loop. Feature logic and documentation archives must be isolated into distinct commits to prevent history pollution. The commit sequence must follow a strict, logical progression:

| Step | Scope of Change | Conventional Commit Prefix | Purpose |
| :---- | :---- | :---- | :---- |
| 1 | Logic Implementation | feat(FID-XXX) | Introducing new functional blocks or trading algorithms.3 |
| 2 | Bug Resolution | fix(FID-XXX) | Correcting computational or operational anomalies.3 |
| 3 | Automated Verification | test(FID-XXX) | Adding unit or integration test assertions to the cargo suite.1 |
| 4 | State Archival | docs(FID-XXX) | Archiving the completed FID and documenting operational lessons learned.4 |

This granular approach ensures that the repository history remains clear and audit-ready.

PowerShell  
\# Example of staging and committing logic isolated from documentation  
git add src/execution/websocket.rs  
git commit \-m "feat(FID-182): implement WebSocket v2 orderbook streaming channel"

This strategy becomes counterproductive if the developer is engaged in highly unstructured, rapid prototyping where the codebase changes multiple times per hour and rarely compiles. Under these conditions, the overhead of managing 5-state transitions per change will slow down development.  
To migrate to this optimized cadence, the developer must configure Vera's local system prompts to restrict code generation outputs to a single file per iteration. The developer must use git add \-p to review and stage changes interactively before committing.

## **Push Schedule Optimization**

Pushes to the remote repository on GitHub must remain manual and occur strictly at the end of structured development sessions or upon reaching stable verification milestones, rather than utilizing automated file-watchers or cron triggers. For an active algorithmic trading engine with real capital at risk, pushing code serves as a conscious gating mechanism to protect the production codebase.

                  Local Workspace Verification Loop  
              ┌──────────────────────────────────────┐  
              ▼                                      │  
       ┌──────────────┐       ┌──────────────┐       │  
       │  Cargo Lint  │──────\>│  Unit Tests  │───────┘  
       └──────────────┘       └──────────────┘  
              │  
              ▼ (Passes)  
       ┌──────────────┐       ┌──────────────┐  
       │ Manual Push  │──────\>│  Git Push    │ (Only 06:00 \- 00:00)  
       └──────────────┘       └──────────────┘

Pushes must occur exactly 2 to 3 times per 24-hour cycle. To minimize operational risk, the developer must implement a strict push lockout window between 12:00 AM and 6:00 AM local time. This timing boundary prevents fatigue-induced regressions from being deployed to the repository, ensuring that any code entering the upstream branch has undergone daylight validation.

PowerShell  
\# PowerShell gatekeeper function integrated into the local profile  
function Push-SavantGuard {  
    $currentTime \= Get-Date  
    if ($currentTime.Hour \-ge 0 \-and $currentTime.Hour \-lt 6) {  
        Write-Error "Push blocked by Savant Guard: Fatigue lockout active between 00:00 and 06:00."  
        return $false  
    }  
      
    Write-Host "Running local test suite..." \-ForegroundColor Cyan  
    cargo test \-\-workspace \-\-all-targets  
    if ($LASTEXITCODE \-ne 0) {   
        Write-Error "Local test suite failed. Push aborted."; return $false   
    }  
      
    Write-Host "Verification passed. Executing secure push to main..." \-ForegroundColor Green  
    git push origin main  
}

This restricted push schedule is unnecessary if the engine is running in a fully decoupled simulation mode with no live exchange access and zero capital risk. Under those safe conditions, continuous, automated pushes are highly desirable.  
To transition to this model, the developer should add the Push-SavantGuard helper to their PowerShell profile ($PROFILE) and use it as the default command to deploy code.

## **Release Cadence Optimization**

Releasing 4 production versions in a 2-day span is highly suboptimal and indicates a failure to differentiate between internal tag milestones and formal production releases. For a solo developer operating a crypto trading engine, formal releases must represent stable, thoroughly backtested, and paper-validated operational baselines.  
Production releases must follow a structured, bi-weekly cadence or be tied directly to the resolution of an entire operational milestone, such as a major version epic. Standard bug fixes and individual feature completions must be handled via lightweight local tags, while formal GitHub Releases should be drafted only when the software achieves a stable epoch, defined by the completion of a minimum threshold of 10 to 15 FIDs.  
![][image1]  
This approach structures the project's release history, as shown in the comparison below:

| Feature | Current Release Pattern | Optimized Release Pattern |
| :---- | :---- | :---- |
| **Frequency** | Irregular (4 releases in 48 hours) | Bi-weekly or Milestone-driven (1 release every 14 days) |
| **Trigger** | Single FID resolution or immediate fix | Completion of ![][image2] FIDs and stable paper-trading runs |
| **Metadata** | Copy-pasted changelogs | Structured, automated release notes 5 |
| **Stability** | High risk of deployment regressions | Verified stability on live exchange forks |

This bi-weekly release rule is invalidated if a critical, high-severity vulnerability is discovered while live funds are exposed on the exchange order book. Such regressions require immediate emergency hotfix releases to protect capital.  
To implement this model, the developer must stop tagging every single completed FID, accumulate stable changes on the main branch, and run extensive backtesting suites before executing the consolidated weekly release.

## **Branch Strategy Optimization**

The optimal branching model for a solo developer utilizing an AI partner is a modified trunk-based development strategy. Maintaining complex multi-branch hierarchies (such as git-flow with long-running develop, release, and feature branches) introduces severe administrative overhead, increases merge complexity, and provides no functional value when there are no peer-review loops.  
All code resides on a single primary branch (main). However, to isolate the code state while working on highly complex or disruptive FIDs (defined as FIDs with "Impact: High" or those requiring extensive refactoring), the developer must utilize short-lived, local-only feature branches. These branches must exist for less than 4 hours, must never be pushed to the remote GitHub repository, and must be squashed directly back into main once local validation passes.3

PowerShell  
\# PowerShell helper functions for local branch isolation  
function Start-FidBranch {  
    param(\[string\]$FidId)  
    git checkout \-b "local-fid-$FidId"  
    Write-Host "Initialized local isolated branch for FID-$FidId." \-ForegroundColor Green  
}

function Merge-FidBranch {  
    param(\[string\]$FidId)  
    git checkout main  
    git merge \-\-squash "local-fid-$FidId"  
    git commit \-m "feat(FID-$FidId): squash merged features from isolated branch"  
    git branch \-D "local-fid-$FidId"  
    Write-Host "Branch local-fid-$FidId squashed and deleted." \-ForegroundColor Green  
}

This strategy is unsuitable if the developer begins collaborating with external developers or auditing firms, where formal Pull Requests, branch protection rules, and remote reviews are required.  
To migrate to this optimized model, the developer should update the repository configuration on GitHub to restrict branches, configure the PowerShell helper scripts locally, and use short-lived branches only for high-risk modifications.

## **Pre-Push Validation Optimization**

Automating pre-push validation via native git hooks ensures that broken builds or failing tests can never be pushed to the remote repository, preventing the need for public hotfix commits. Since the platform operates on Windows 11 without the gh CLI, the git hook must be constructed using a native Bash-to-PowerShell bridge that executes within the Git Bash terminal emulator distributed with Git for Windows.6  
The validation hook must execute formatting checks (cargo fmt), strict linter evaluations (cargo clippy), and the full test suite.1 To prevent the hook from blocking development velocity, testing must be handled by cargo-nextest, which runs unit tests in parallel processes.9 This process-per-test isolation model drastically reduces execution times on modern multi-core processors 10:  
![][image3]  
On a typical Windows 11 development machine, this transition reduces test execution overhead by up to 60%, maintaining a rapid feedback loop.10  
Save the following executable file directly to .git/hooks/pre-push inside the repository root 6:

Bash  
\#\!/bin/sh  
\# Bash wrapper to execute the PowerShell 7+ pre-push validation script on Windows 11  
echo "Executing Savant local gatekeeper validations..."  
exec pwsh.exe \-NoProfile \-ExecutionPolicy Bypass \-File "./scripts/pre-push-validation.ps1"

Save the corresponding target validation script to scripts/pre-push-validation.ps1 8:

PowerShell  
\# scripts/pre-push-validation.ps1  
$ErrorActionPreference \= "Stop"

Write-Host "Executing strict formatting check..." \-ForegroundColor Cyan  
cargo fmt \-\-all \-- \-\-check  
if ($LASTEXITCODE \-ne 0) {  
    Write-Error "Code formatting violations detected. Run 'cargo fmt'."  
    exit 1  
}

Write-Host "Executing strict compiler clippy checks..." \-ForegroundColor Cyan  
cargo clippy \-\-all-targets \-- \-D warnings  
if ($LASTEXITCODE \-ne 0) {  
    Write-Error "Clippy warnings or compiler lints detected. Fix code issues."  
    exit 1  
}

Write-Host "Running test suite via cargo-nextest..." \-ForegroundColor Cyan  
cargo nextest run \-\-workspace \-\-all-targets  
if ($LASTEXITCODE \-ne 0) {  
    Write-Error "Tests failed. Build is unstable."  
    exit 1  
}

Write-Host "All local validations passed successfully." \-ForegroundColor Green  
exit 0

Ensure the shell wrapper script is marked as executable within Git Bash or a similar environment 6:

Bash  
chmod \+x.git/hooks/pre-push

This hook is counterproductive if the developer's workstation is running restrictive, real-time enterprise antivirus software that heavily throttles the spawning of hundreds of processes by cargo-nextest, causing tests to take minutes.11 On such restricted hosts, standard thread-based cargo test is faster and should be used instead.11  
To migrate, the developer must install cargo-nextest globally 3, save the validation scripts to the codebase, and verify hook execution by attempting a test push.6

## **CHANGELOG Discipline Automation**

Relying on manual tracking of CHANGELOG.md files violates the core principles of continuous optimization and often leads to skipped updates during late-night sessions. The developer must transition to fully automated changelog generation driven by Conventional Commits, utilizing git-cliff to parse git history and extract structural releases.3  
The CHANGELOG.md will be automatically compiled at the release boundary. The tool must parse the standard Conventional Commit headers and map them to clean categories. Crucially, the parser must read the custom FID format (feat: FID-182...) and convert those references into hyperlinked local files and remote GitHub Issues.4  
Install git-cliff locally:

PowerShell  
cargo install git\-cliff

Create the following configuration file as cliff.toml in the repository root directory 3:

Ini, TOML  
\# cliff.toml  
\[changelog\]  
header \= """  
\# Savant Trading Engine Changelog  
All notable changes to this project are documented automatically.  
"""  
body \= """  
{% if version %}\\  
\#\# \[{{ version | trim\_start\_matches(pat="v") }}\] \- {{ timestamp | date(format="%Y-%m-%d") }}  
{% else %}\\  
\#\# \[Unreleased\]  
{% endif %}\\  
{% for group, commits in commits | group\_by(attribute="group") %}  
\#\#\# {{ group | upper\_first }}  
{% for commit in commits %}  
\- {% if commit.scope %}\*{{ commit.scope }}\*: {% endif %}{{ commit.message | split(pat="\\n") | first | upper\_first | trim }} (\[{{ commit.id | truncate(length=7, end="") }}\](https://github.com/fame0528/savant-trading/commit/{{ commit.id }}))  
{%- if commit.footers %}  
  {% for footer in commit.footers %}  
  \* {{ footer.token }}: {{ footer.value }}  
  {% endfor %}  
{%- endif %}  
{% endfor %}  
{% endfor %}  
"""  
footer \= ""  
trim \= true

\[git\]  
conventional\_commits \= true  
filter\_unconventional \= false  
split\_commits \= false  
commit\_parsers \=+\\\\)", group \= "🚀 Features (FID Core)" },  
    { message \= "^feat", group \= "🚀 General Features" },  
    { message \= "^fix\\\\(FID-\[0\-9\]+\\\\)", group \= "🐛 Bug Fixes (FID Resolved)" },  
    { message \= "^fix", group \= "🐛 General Bug Fixes" },  
    { message \= "^docs\\\\(FID-\[0\-9\]+\\\\)", group \= "📝 FID Lifecycle Archives" },  
    { message \= "^docs", group \= "📚 Documentation Update" },  
    { message \= "^perf", group \= "⚡ Performance Optimization" },  
    { message \= "^refactor", group \= "🚜 Refactoring" },  
    { message \= ".\*", group \= "⚙️ Miscellaneous Development" }  
\]  
commit\_preprocessors \=+)', replace \= "(https://github.com/fame0528/savant-trading/issues/${1})" }  
\]  
filter\_commits \= false  
topo\_order \= false  
sort\_commits \= "oldest"

To output the changelog during a release cycle, execute the command:

PowerShell  
git\-cliff \-o CHANGELOG.md

Automated changelog generation is ineffective if the developer writes unstructured, non-conventional commits, which results in omitted entries or a cluttered "Other" catch-all section.4  
To migrate, the developer must install git-cliff 3, configure cliff.toml, run a baseline check using git-cliff \--context to verify output generation, and commit the configuration file.15

## **Version Management Automation**

Manual tracking of software versions across multiple files (Cargo.toml, package.json, Cargo.lock, config files, etc.) is highly error-prone. The developer must automate this process using cargo-release, a cargo subcommand designed to handle workspace version propagation, pre-release checks, file version search-and-replace actions, and post-release version bumps.16  
Every formal version bump must execute atomically. The tool must be configured to locate and replace the version definition across five target files. The developer must run a dry-run validation (cargo release \--dry-run) before every production release to preview the planned changes and ensure file substitutions will execute successfully.17  
Install cargo-release locally:

PowerShell  
cargo install cargo\-release

Create a release.toml configuration file in the repository root directory 16:

Ini, TOML  
\# release.toml  
sign-commit \= true  
sign-tag \= true  
push \= false  
tag \= true  
tag-name \= "v{{version}}"  
pre-release-commit-message \= "chore: release version {{version}}"

\# List of files requiring structured version string substitution  
pre-release-replacements \=+\\\\.\[0\-9\]+\\\\.\[0\-9\]+', replace \= "Savant Engine v{{version}}" }  
\]

\[package.metadata.release\]  
shared-version \= true

Execute a dry-run release bump using:

PowerShell  
cargo release patch \-\-dry-run

Execute the actual file updates and tagging action using:

PowerShell  
cargo release patch \-\-execute

This utility is unsafe to use if the developer is working in a complex workspace with cyclic or unmanaged dependencies, where automated bumps can break compiler validation.19  
To migrate, the developer must install cargo-release 16, save the configuration properties inside release.toml 16, and run a dry-run execution to verify version propagation across the five target files.17

## **Release Automation**

Since the local Windows environment lacks the gh CLI, the developer should delegate the creation of GitHub releases to GitHub Actions rather than using manual PowerShell scripts with raw Personal Access Tokens (PATs).20 This approach removes the risk of exposing PATs in the local PowerShell terminal history.21 The release pipeline must trigger automatically whenever a new semver tag is pushed to the remote repository.20  
The workflow triggers on the push of tags matching the semantic pattern v\*.\*.\*.20 It executes the engine compilation within a secure container, verifies all assertions, generates release assets, and builds a formal GitHub Release containing the changelog notes.20  
Create .github/workflows/release.yml in the repository directory 21:

YAML  
\#.github/workflows/release.yml  
name: Savant Automated Release

on:  
  push:  
    tags:  
      \- 'v\*.\*.\*'

permissions:  
  contents: write

jobs:  
  build-and-release:  
    runs-on: ubuntu-latest  
    steps:  
      \- name: Checkout Codebase  
        uses: actions/checkout@v4  
        with:  
          fetch-depth: 0

      \- name: Set up Rust Toolchain  
        uses: dtolnay/rust-toolchain@stable  
        with:  
          toolchain: 1.91

      \- name: Cache Cargo Dependencies  
        uses: actions/cache@v4  
        with:  
          path: |  
            \~/.cargo/bin/  
            \~/.cargo/registry/index/  
            \~/.cargo/registry/cache/  
            \~/.cargo/git/db/  
            target/  
          key: ${{ runner.os }}-cargo-${{ hashFiles('\*\*/Cargo.lock') }}

      \- name: Run Complete Test Suite  
        run: cargo test \--workspace \--all-targets \--all-features

      \- name: Compile Optimized Release Binary  
        run: cargo build \--release \--locked \--bin savant-engine

      \- name: Generate Changelog Excerpt  
        id: changelog  
        run: |  
          cargo install git-cliff  
          git-cliff \--latest \--strip all \-o RELEASE\_NOTES.md

      \- name: Create GitHub Release and Upload Assets  
        uses: softprops/action-gh-release@v2  
        with:  
          body\_path: RELEASE\_NOTES.md  
          draft: false  
          prerelease: false  
          files: |  
            target/release/savant-engine  
        env:  
          GITHUB\_TOKEN: ${{ secrets.GITHUB\_TOKEN }}

Remote release pipelines are unsuitable if the trading engine contains highly proprietary, closed-source mathematical models that cannot be uploaded to, or compiled on, public cloud infrastructure.21  
To migrate, the developer must create the .github/workflows directory, save the YAML file locally, and push a test tag to verify that the remote pipeline compiles the binary and creates the release.21

## **Tag Management Optimization**

Manual tag creation is highly prone to mistakes, such as pointing to the wrong commit if executed after a branch reset or rollback. To guarantee the cryptographic integrity of the software baseline, all git tags must be automatically created during the release process and signed using local SSH keys instead of GPG.17 SSH keys are much simpler to manage on Windows 11 than GPG.23  
All Git tags must be configured to use SSH signatures.22 When the developer triggers a release via cargo-release, the tool will automatically generate the correct tag pointing to the current commit, sign it, and push it to the remote repository, ensuring tag authenticity.17  
First, verify that the local SSH agent is running and configured to start automatically on Windows 11\. Open PowerShell 7+ as an Administrator and execute the following commands 23:

PowerShell  
\# Set SSH Agent to automatic start  
Set-Service \-Name ssh\-agent \-StartupType Automatic  
Start-Service ssh\-agent

Next, configure git to sign all commits and tags using the local SSH public key 22:

PowerShell  
\# Instruct Git to use SSH instead of GPG  
git config \-\-global gpg.format ssh  
git config \-\-global commit.gpgsign true  
git config \-\-global tag.gpgsign true

\# Set the path to the Windows OpenSSH executable  
git config \-\-global gpg.ssh.program "C:\\Windows\\System32\\OpenSSH\\ssh-keygen.exe"

\# Point Git to the public SSH signing key  
git config \-\-global user.signingkey "$env:USERPROFILE\\.ssh\\id\_ed25519.pub"

Configure local verification by setting up an allowed\_signers file to map signatures to the developer's email address 22:

PowerShell  
git config \-\-global gpg.ssh.allowedSignersFile "$env:USERPROFILE/.ssh/allowed\_signers"  
$email \= git config user.email  
$publicKey \= Get-Content "$env:USERPROFILE/.ssh/id\_ed25519.pub"  
"$email namespaces=\`"git\`" $publicKey" | Out-File "$env:USERPROFILE/.ssh/allowed\_signers" \-Encoding utf8 \-Append

SSH signing is unusable if the developer must commit from restricted machines where private keys cannot be stored or loaded into an active agent session.22  
To migrate, the developer must start the Windows 11 OpenSSH service 23, configure global Git settings to sign commits and tags with the public signing key 23, and verify the setup using the allowed\_signers configuration.23

## **FID Workflow vs. GitHub Issues**

The custom Feature Implementation Document (FID) filesystem structure inside dev/fids/ is highly effective for local AI context mapping, but it lacks team visibility, labels, status boards, and search functionality. Rather than abandoning filesystem tracking or manually double-entering data, the developer should implement a hybrid strategy using gh-issue-sync.26 This tool mirrors local markdown files with YAML frontmatter into GitHub Issues using bidirectional synchronization.26  
The active FIDs (currently 9\) reside in dev/fids/. A local PowerShell script runs at the start and end of every development session to synchronize the status of these files with GitHub Issues.26 This maintains the filesystem-based workflow as the source of truth, while mirroring states to GitHub to enable advanced issue tracking.  
Save the following synchronization script to scripts/Sync-Fids.ps1:

PowerShell  
\# scripts/Sync-Fids.ps1  
$ErrorActionPreference \= "Stop"

\# Ensure the sync tool is installed locally  
if (\-not (Get-Command "gh-issue-sync" \-ErrorActionPreference SilentlyContinue)) {  
    Write-Host "Installing gh-issue-sync..." \-ForegroundColor Yellow  
    \# Installs the sync binary via Go compiler  
    go install github.com/mitsuhiko/gh\-issue-sync/cmd/gh\-issue-sync@latest  
}

\# Run the bidirectional sync between dev/fids and GitHub  
Write-Host "Executing bidirectional FID synchronization..." \-ForegroundColor Cyan  
$env:GH\_ISSUE\_SYNC\_DIR \= "dev/fids"  
gh\-issue-sync sync \-\-all

Write-Host "Synchronization complete. Local files and GitHub Issues are aligned." \-ForegroundColor Green

To structure local FIDs for correct parsing, format the file headers with YAML frontmatter 26:

YAML  
\---  
title: "FID-182: WebSocket Live Data Stream Stability"  
state: open  
labels:  
  \- engine  
  \- live-trading  
  \- high-priority  
\---

This synchronization fails if Vera or the developer modifies local markdown formats with unstructured headers that violate the strict YAML frontmatter required by the parser.26  
To migrate, the developer must install gh-issue-sync 26, format local FID templates with YAML headers 26, and run the synchronization script to populate the GitHub Issues page.26

## **AI Commit Attribution**

Out of 368 total commits, Vera has only 2 direct attributions. This lacks accurate project history tracking and makes it difficult to audit AI-generated code changes. The developer must configure Vera's system prompt to append the standardized Co-authored-by: trailer to all commit message recommendations, ensuring accurate project attribution on GitHub.28  
Any commit containing code generated, refactored, or optimized by Vera must append a Co-authored-by trailer at the end of the commit message.29 This trailer must be separated from the main commit body by a blank line.29 When pushed, GitHub reads this metadata and displays the avatar of both the developer and the AI assistant, providing accurate contribution history.29

Code snippet  
\# Example of a co-authored commit message structure  
feat(FID-182): optimize orderbook memory allocation footprint

Reduces memory allocations within the local L2 orderbook cache by   
reusing vector buffers instead of reallocating on every packet.

Co-authored-by: Vera \<vera@savant.trading\>

This attribution fails if the developer is working offline or with private commits, as GitHub cannot map the no-reply email to a verified user account, showing the signature as unverified.  
To migrate, the developer must append the co-authorship instructions to Vera's system configuration, run a test commit locally to confirm that Git parses the trailing metadata 29, and verify that both authors appear on GitHub once pushed.

## **Release Notes Quality**

Excerpts from raw changelogs often contain low-level developer commits, such as formatting cleanups, spelling fixes, or variable renames. These entries are distracting in formal release descriptions. The developer must split the automated release notes into a clean, two-tiered structure: a user-facing operational summary and an automated technical changelog.  
Release notes must be generated using a structured markdown template. The template must group changes into three distinct sections: Operational Status, Key Features, and Bug Resolutions. This is implemented directly within cliff.toml to automate compilation 30:

Ini, TOML  
\# cliff.toml template configurations  
\[changelog\]  
body \= """  
\# Savant Engine Release {{ version }} ({{ timestamp | date(format="%Y-%m-%d") }})

\#\# 📊 Operational Status Update  
This release has passed all paper-trading safety validations on the Anvil live fork.

\#\# 🚀 Key Deliverables & Features  
{% for commit in commits | filter(attribute="group", value="🚀 Features (FID Core)") %}  
\- \*\*{{ commit.scope }}\*\*: {{ commit.message | split(pat="\\n") | first | upper\_first }}  
{% endfor %}

\#\# 🐛 Resolved Anomalies & Bug Fixes  
{% for commit in commits | filter(attribute="group", value="🐛 Bug Fixes (FID Resolved)") %}  
\- \*\*{{ commit.scope }}\*\*: {{ commit.message | split(pat="\\n") | first | upper\_first }}  
{% endfor %}

\#\# ⚙️ Engineering & Infrastructure Update  
{% for commit in commits | filter(attribute="group", value="⚙️ Miscellaneous Development") %}  
\- {{ commit.message | split(pat="\\n") | first | upper\_first }}  
{% endfor %}  
"""

Structured notes are unnecessary if the repository remains strictly private, where high-level, human-readable formatting adds no functional value.5  
To migrate, the developer must update the cliff.toml file with the custom body template, test the formatting output using git-cliff \--latest 19, and link the output file to the automated release pipeline.21

## **Repository Visibility and Discoverability**

As a proprietary trading engine, high visibility can introduce security and intellectual property risks. However, maintaining a clean, professional public landing page with solid presentation is highly beneficial if the developer plans to open-source non-sensitive portions of the engine, showcase their skills, or attract potential capital allocators.  
The repository must utilize automated, high-fidelity badges, precise technology classifications, and a structured introduction. This provides immediate context to visitors without exposing proprietary algorithmic models or sensitive API routing logic.

# **Savant Trading Engine**

(https://img.shields.io/github/actions/workflow/status/fame0528/savant-trading/release.yml?style=flat-square\&logo=rust\&color=orange)\](https://github.com/fame0528/savant-trading/actions) ([https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square))\]([https://opensource.org/licenses/MIT](https://opensource.org/licenses/MIT))   
Savant is a high-performance, asynchronous cryptocurrency algorithmic trading engine written in Rust, featuring a real-time monitoring dashboard built in Next.js.  
If the codebase contains proprietary alpha-generating trading strategies or custom execution logic, maintaining a highly discoverable public repository increases the risk of reverse-engineering or exploitation. In this scenario, the repository must remain strictly private, rendering public badges and topics unnecessary.  
To migrate, the developer must append the markdown badges to the top of the README, assign the repository topics on GitHub, and generate a secure architectural flowchart to use as the social preview image.

## **Backup and Disaster Recovery**

Relying solely on GitHub as a single point of failure introduces significant operational risk. If the developer's GitHub account is flagged or suspended, access to the trading engine codebase and history could be lost. The developer must establish an automated, local daily task that creates a bare mirror clone of the repository and pushes it to a private secondary GitLab repository or a self-hosted Gitea instance to avoid relying solely on GitHub.31  
The backup process must execute a full mirror clone, which captures all branches, local tags, commits, and historical metadata.31 This is pushed to the secondary backup remote 32:  
![][image4]  
Save the following secure backup script to scripts/Backup-Repository.ps1 32:

PowerShell  
\# scripts/Backup-Repository.ps1  
$ErrorActionPreference \= "Stop"

$BackupPath \= "$env:USERPROFILE/backups/savant-trading.git"  
$PrimarySource \= "git@github.com:fame0528/savant-trading.git"  
$SecondaryRemote \= "git@gitlab.com:savant-backups/savant-trading.git"

Write-Host "Initializing backup mirror cycle..." \-ForegroundColor Cyan

if (Test-Path \-Path $BackupPath) {  
    Set-Location $BackupPath  
    git remote update  
} else {  
    New-Item \-ItemType Directory \-Path (Split-Path $BackupPath) \-Force | Out-Null  
    git clone \-\-mirror $PrimarySource $BackupPath  
    Set-Location $BackupPath  
}

Write-Host "Mirroring backup state to secondary target..." \-ForegroundColor Cyan  
git push \-\-mirror $SecondaryRemote

Write-Host "Backup verification process complete." \-ForegroundColor Green

Register this script inside the Windows 11 Task Scheduler to run automatically 33:

PowerShell  
\# Register the mirror task as a daily Windows service trigger  
$Action \= New-ScheduledTaskAction \-Execute 'pwsh.exe' \-Argument "-NoProfile \-WindowStyle Hidden \-File $env:USERPROFILE/savant-trading/scripts/Backup-Repository.ps1"  
$Trigger \= New-ScheduledTaskTrigger \-Daily \-At 3:00AM  
Register-ScheduledTask \-TaskName "SavantRepositoryBackup" \-Action $Action \-Trigger $Trigger

Remote mirror pushing is unsafe if the developer's internet connection has high upload latency, which can cause network packet queuing and disrupt real-time exchange WebSocket connections during active trading hours.  
To migrate, the developer must create a private repository on GitLab 32, verify local SSH access to the secondary remote, and register the daily execution script inside the Windows Task Scheduler.33

## **Security: Credential Management**

Storing GitHub Personal Access Tokens (PATs) in plain text inside PowerShell shell variables or calling them in command histories introduces severe security risks. On Windows 11, PowerShell command history is written in plain text to a local history file 34:  
![][image5]  
An adversary or malicious script gaining access to this file can extract active credentials.36 The developer must rotate any compromised PATs immediately and transition to Git Credential Manager (GCM), which stores credentials securely inside the encrypted Windows Credential Store.38  
The developer must configure Git to route all authentication requests through Git Credential Manager.39 This allows Git to authenticate securely via OAuth without requiring plain-text tokens in command-line arguments or environmental variables.39

PowerShell  
\# Secure initialization script for local environment  
$ErrorActionPreference \= "Stop"

Write-Host "Purging cleartext credential logs from PowerShell history file..." \-ForegroundColor Cyan  
Remove-Item (Get-PSReadlineOption).HistorySavePath \-ErrorAction SilentlyContinue

Write-Host "Configuring Git to use Git Credential Manager..." \-ForegroundColor Cyan  
git config \-\-global credential.helper manager  
git config \-\-global credential.credentialStore wincred

Write-Host "Enforcing secure session restrictions for current terminal..." \-ForegroundColor Cyan  
Set-PSReadLineOption \-HistorySaveStyle SaveNothing

Using credential helpers is not supported inside headless Docker containers or isolated CI environments, where explicit, short-lived tokens must be supplied via secure environment variables.39  
To migrate, the developer must revoke any exposed PATs on GitHub, configure Git globally to route authentication requests through the Windows helper 39, and execute a manual push to trigger the secure, interactive OAuth authentication flow.39

## **Actionable Workflow Priority Roadmap**

The roadmap below defines the exact sequence of workflow improvements, ordered by their operational risk reduction and implementation speed.

┌────────────────────────┐     ┌────────────────────────┐     ┌────────────────────────┐  
│ 1\. GCM & History Purge │────\>│ 2\. Pre-Push Validation │────\>│ 3\. SSH Tag Signing     │  
│  (Est: 10 Minutes)     │     │  (Est: 20 Minutes)     │     │  (Est: 20 Minutes)     │  
└────────────────────────┘     └────────────────────────┘     └────────────────────────┘  
                                                                          │  
                                                                          ▼  
┌────────────────────────┐     ┌────────────────────────┐     ┌────────────────────────┐  
│ 6\. GHA Automations     │\<────│ 5\. Versioning Tooling  │\<────│ 4\. Git-Cliff Setup     │  
│  (Est: 30 Minutes)     │     │  (Est: 30 Minutes)     │     │  (Est: 25 Minutes)     │  
└────────────────────────┘     └────────────────────────┘     └────────────────────────┘  
            │  
            ▼  
┌────────────────────────┐     ┌────────────────────────┐  
│ 7\. FID Sync to Issues  │────\>│ 8\. Task-Scheduler BK   │  
│  (Est: 20 Minutes)     │     │  (Est: 20 Minutes)     │  
└────────────────────────┘     └────────────────────────┘

The specific implementation steps are detailed in the prioritization matrix below:

| Sequence | Priority Task | Implementation Target | Tooling | Duration |
| :---- | :---- | :---- | :---- | :---- |
| **1** | GCM Configuration & History Purge 39 | Evict cleartext credentials from logs; configure Git Credential Manager.34 | PowerShell 7+, GCM Core 39 | 10 Minutes |
| **2** | Pre-Push Hook Integration 6 | Deploy an automated gate checking lints, formatting, and unit tests.1 | cargo-nextest 9, Bash, PS7 8 | 20 Minutes |
| **3** | Cryptographic SSH Tag Signing 22 | Enable automatic tag signing with SSH verification keys.23 | OpenSSH client 23, Git 22 | 20 Minutes |
| **4** | Git-Cliff Parser Setup 3 | Build custom cliff.toml to automate changelog generation.4 | git-cliff compiler 3 | 25 Minutes |
| **5** | Automated Version Management 16 | Configure release.toml to search and replace versions across files.17 | cargo-release 16 | 30 Minutes |
| **6** | GitHub Actions Pipeline 21 | Automate release drafting and asset compilation on tags.20 | GitHub Actions runner 21 | 30 Minutes |
| **7** | Bidirectional FID Sync 26 | Sync local filesystem-based FIDs with remote boards.26 | gh-issue-sync 26 | 20 Minutes |
| **8** | Task Scheduler Mirror Backup 32 | Automate daily bare repository backups to a secondary host.32 | Git, Windows Task Scheduler 33 | 20 Minutes |

#### **Works cited**

1. cargo-husky \- crates.io: Rust Package Registry, accessed June 17, 2026, [https://crates.io/crates/cargo-husky/0.1.0](https://crates.io/crates/cargo-husky/0.1.0)  
2. rhysd/cargo-husky: Setup Git hooks automatically for cargo projects with :dog \- GitHub, accessed June 17, 2026, [https://github.com/rhysd/cargo-husky](https://github.com/rhysd/cargo-husky)  
3. Getting Started \- git-cliff, accessed June 17, 2026, [https://git-cliff.org/docs/](https://git-cliff.org/docs/)  
4. Using git-cliff for Changelogs and Migrating Away from gitchangelog \- Werner Robitza, accessed June 17, 2026, [https://slhck.info/software/2025/10/13/git-cliff-for-changelogs.html](https://slhck.info/software/2025/10/13/git-cliff-for-changelogs.html)  
5. action-gh-release/RELEASE.md at master · softprops/action-gh-release \- GitHub, accessed June 17, 2026, [https://github.com/softprops/action-gh-release/blob/master/RELEASE.md](https://github.com/softprops/action-gh-release/blob/master/RELEASE.md)  
6. Run Tests Before Push — Git Hook | Tower Git Client, accessed June 17, 2026, [https://www.git-tower.com/git-hooks/pre-push-tests](https://www.git-tower.com/git-hooks/pre-push-tests)  
7. Git hooks, practical uses (yes, even on Windows) \- tygertec, accessed June 17, 2026, [https://www.tygertec.com/git-hooks-practical-uses-windows/](https://www.tygertec.com/git-hooks-practical-uses-windows/)  
8. How to run a powershell script as part of a Git hook \- Justin Bird, accessed June 17, 2026, [https://justinjbird.com/blog/2024/how-to-run-a-powershell-script-as-a-git-hook/](https://justinjbird.com/blog/2024/how-to-run-a-powershell-script-as-a-git-hook/)  
9. Benchmarks \- cargo-nextest, accessed June 17, 2026, [https://nexte.st/docs/benchmarks/](https://nexte.st/docs/benchmarks/)  
10. Faster Rust Tests With cargo-nextest | The RustRover Blog, accessed June 17, 2026, [https://blog.jetbrains.com/rust/2026/05/01/faster-rust-tests-with-cargo-nextest/](https://blog.jetbrains.com/rust/2026/05/01/faster-rust-tests-with-cargo-nextest/)  
11. Why process-per-test? \- cargo-nextest, accessed June 17, 2026, [https://nexte.st/docs/design/why-process-per-test/](https://nexte.st/docs/design/why-process-per-test/)  
12. Why nextest is process-per-test : r/rust \- Reddit, accessed June 17, 2026, [https://www.reddit.com/r/rust/comments/1hwf0ox/why\_nextest\_is\_processpertest/](https://www.reddit.com/r/rust/comments/1hwf0ox/why_nextest_is_processpertest/)  
13. Setting Up the Pre-commit Git Hook on Windows with PowerShell | NimblePros Blog, accessed June 17, 2026, [https://blog.nimblepros.com/blogs/setting-up-pre-commit-git-hook-on-windows-with-powershell/](https://blog.nimblepros.com/blogs/setting-up-pre-commit-git-hook-on-windows-with-powershell/)  
14. git-cliff, accessed June 17, 2026, [https://git-cliff.org/docs/configuration/git/](https://git-cliff.org/docs/configuration/git/)  
15. Automatic Changelog Generation with git-cliff | by A S Pamungkas | Medium, accessed June 17, 2026, [https://aspamungkas.medium.com/automatic-changelog-generation-with-git-cliff-c9a224f4f069](https://aspamungkas.medium.com/automatic-changelog-generation-with-git-cliff-c9a224f4f069)  
16. cargo-release/docs/reference.md at master \- GitHub, accessed June 17, 2026, [https://github.com/crate-ci/cargo-release/blob/master/docs/reference.md](https://github.com/crate-ci/cargo-release/blob/master/docs/reference.md)  
17. cargo-release \- crates.io: Rust Package Registry, accessed June 17, 2026, [https://crates.io/crates/cargo-release/0.11.2](https://crates.io/crates/cargo-release/0.11.2)  
18. cargo-release \- crates.io: Rust Package Registry, accessed June 17, 2026, [https://crates.io/crates/cargo-release/0.12.0-beta.3](https://crates.io/crates/cargo-release/0.12.0-beta.3)  
19. cargo-release/docs/faq.md at master · crate-ci/cargo-release \- GitHub, accessed June 17, 2026, [https://github.com/crate-ci/cargo-release/blob/master/docs/faq.md](https://github.com/crate-ci/cargo-release/blob/master/docs/faq.md)  
20. softprops/action-gh-release \- GitHub, accessed June 17, 2026, [https://github.com/softprops/action-gh-release](https://github.com/softprops/action-gh-release)  
21. Create GitHub release using GitHub Actions \- KodeKloud Docs, accessed June 17, 2026, [https://notes.kodekloud.com/docs/GitHub-Actions-Certification/Custom-Actions/Create-GitHub-release-using-GitHub-Actions/page](https://notes.kodekloud.com/docs/GitHub-Actions-Certification/Custom-Actions/Create-GitHub-release-using-GitHub-Actions/page)  
22. Sign commits and tags with SSH keys \- GitLab Docs, accessed June 17, 2026, [https://docs.gitlab.com/user/project/repository/signed\_commits/ssh/](https://docs.gitlab.com/user/project/repository/signed_commits/ssh/)  
23. Signing commits in Git using SSH keys on Windows \- Meziantou's blog, accessed June 17, 2026, [https://www.meziantou.net/signing-commits-in-git-using-ssh-keys-on-windows.htm](https://www.meziantou.net/signing-commits-in-git-using-ssh-keys-on-windows.htm)  
24. Signing Git commits with SSH keys \- Emmanuel Bernard, accessed June 17, 2026, [https://emmanuelbernard.com/blog/2023/11/27/git-signing-ssh/](https://emmanuelbernard.com/blog/2023/11/27/git-signing-ssh/)  
25. Using git with powershell and ssh key with passphrase \- Stack Overflow, accessed June 17, 2026, [https://stackoverflow.com/questions/48843643/using-git-with-powershell-and-ssh-key-with-passphrase](https://stackoverflow.com/questions/48843643/using-git-with-powershell-and-ssh-key-with-passphrase)  
26. mitsuhiko/gh-issue-sync: A github issue sync tool for working with issues locally, accessed June 17, 2026, [https://github.com/mitsuhiko/gh-issue-sync](https://github.com/mitsuhiko/gh-issue-sync)  
27. jackchuka/gh-md: GitHub issues and PRs as local markdown files you can actually edit, accessed June 17, 2026, [https://github.com/jackchuka/gh-md/](https://github.com/jackchuka/gh-md/)  
28. Our coding agent commits deserve better than Co-Authored-By \- fabiorehm.com, accessed June 17, 2026, [https://fabiorehm.com/blog/2026/03/02/our-coding-agent-commits-deserve-better-than-co-authored-by/](https://fabiorehm.com/blog/2026/03/02/our-coding-agent-commits-deserve-better-than-co-authored-by/)  
29. Creating a commit with multiple authors \- GitHub Docs, accessed June 17, 2026, [https://docs.github.com/articles/creating-a-commit-with-multiple-authors](https://docs.github.com/articles/creating-a-commit-with-multiple-authors)  
30. git-cliff/examples/detailed.toml at main \- GitHub, accessed June 17, 2026, [https://github.com/orhun/git-cliff/blob/main/examples/detailed.toml](https://github.com/orhun/git-cliff/blob/main/examples/detailed.toml)  
31. Backing up GitHub repos properly: git, metadata, and a browsable mirror \- Sam Dumont, accessed June 17, 2026, [https://dropbars.be/blog/backing-up-github-repos-properly/](https://dropbars.be/blog/backing-up-github-repos-properly/)  
32. Backup Github repos onto Gitlab (THE FREE WAY) | by Mike Sun \- Medium, accessed June 17, 2026, [https://mightnent.medium.com/backup-github-repos-onto-gitlab-the-free-way-d666fbe74793](https://mightnent.medium.com/backup-github-repos-onto-gitlab-the-free-way-d666fbe74793)  
33. Local GitHub repository backups with Powershell \- Gist, accessed June 17, 2026, [https://gist.github.com/markashleybell/75c76bddc973d39283d6a23331987d9e](https://gist.github.com/markashleybell/75c76bddc973d39283d6a23331987d9e)  
34. PowerShell's Clear-History doesn't clear history \- Stack Overflow, accessed June 17, 2026, [https://stackoverflow.com/questions/13257775/powershells-clear-history-doesnt-clear-history](https://stackoverflow.com/questions/13257775/powershells-clear-history-doesnt-clear-history)  
35. PowerShell History File | 0xdf hacks stuff \- GitLab, accessed June 17, 2026, [https://0xdf.gitlab.io/2018/11/08/powershell-history-file.html](https://0xdf.gitlab.io/2018/11/08/powershell-history-file.html)  
36. ConsoleHost\_history.txt File Missing | Detection \- Insider Threat Matrix, accessed June 17, 2026, [https://insiderthreatmatrix.org/detections/DT002](https://insiderthreatmatrix.org/detections/DT002)  
37. Clearing Windows Console History | Prebuilt detection rules reference \- Elastic, accessed June 17, 2026, [https://www.elastic.co/docs/reference/security/prebuilt-rules/rules/windows/defense\_evasion\_clearing\_windows\_console\_history](https://www.elastic.co/docs/reference/security/prebuilt-rules/rules/windows/defense_evasion_clearing_windows_console_history)  
38. Credential Manager in Windows \- Microsoft Support, accessed June 17, 2026, [https://support.microsoft.com/en-us/windows/credential-manager-in-windows-1b5c916a-6a16-889f-8581-fc16e8165ac0](https://support.microsoft.com/en-us/windows/credential-manager-in-windows-1b5c916a-6a16-889f-8581-fc16e8165ac0)  
39. git-ecosystem/git-credential-manager \- GitHub, accessed June 17, 2026, [https://github.com/git-ecosystem/git-credential-manager](https://github.com/git-ecosystem/git-credential-manager)  
40. Git Credential Manager for Windows \- Microsoft Open Source, accessed June 17, 2026, [http://microsoft.github.io/Git-Credential-Manager-for-Windows/](http://microsoft.github.io/Git-Credential-Manager-for-Windows/)

[image1]: <data:image/png;base64,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>

[image2]: <data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACUAAAAWCAYAAABHcFUAAAAAuklEQVR4Xu2UIQpCQRRFn4iICBZBsZvFJtg0C1azLsANiBsQm8ld/BUYLAYRF+FCPJcpzi+a/vvy58BJN8ww3DtmiUSFmOENV9jIZa60cI1P3GA7jn3RS+nF7rjDThz7Usc5XvGA3Tj2pYYTvOAJB3HsSxP3+MJhLiuczwFszXkAOlyXeFgJvgqtTavTv7W0UHg3VGAVWYWemvNlxAIzHFlYW+Kv0LL6Fnr1zZ4V1Lcxnn/0aCX71avLG+drGV1wwfoWAAAAAElFTkSuQmCC>

[image3]: <data:image/png;base64,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>

[image4]: <data:image/png;base64,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>

[image5]: <data:image/png;base64,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>