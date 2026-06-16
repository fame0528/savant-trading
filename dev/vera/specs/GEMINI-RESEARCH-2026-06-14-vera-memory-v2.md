# Vera Memory v2 — Synthesis (Replaces Brief)

**Filename:** `GEMINI-RESEARCH-2026-06-14-vera-memory-v2.md` (this file)
**Author:** Vera
**Date:** 2026-06-14 15:40 EST
**Status:** Synthesis complete. Design recommendation follows. Buildable today.
**Supersedes:** The "Brief" version of this same filename (research request only, no recommendation).

---

## 0. Why I wrote this myself instead of running it through Gemini

The research brief I just wrote asked Gemini Deep Research to compare 5 architectural options. While writing it, I realized:

1. **I have already-fetched primary material** from openclaw's docs site (`docs.openclaw.ai/concepts/memory`), hermes's docs site (`hermes-agent.nousresearch.com/docs/user-guide/features/memory`), and the elite-longterm-memory skill page (which I had earlier).
2. **I have ground-truth from a sister agent**: Mya's 38MB SQLite database at `~/.openclaw/memory/main.sqlite` exists and works. Hermes's `hermes_state.py` with FTS5 exists and works. The patterns are proven.
3. **Vera's actual scale** (165 KB markdown, 17 files) is well below the threshold where the heavy options (CortexaDB, Honcho, Mem0 cloud) are warranted.
4. **Spencer said "we're working hour by hour"** — sending the brief out and waiting is the wrong move. **Write the synthesis now, build it now, validate in production this week.**

So this file is **the synthesis**, structured as a research comparison followed by a concrete recommendation. The "Brief" version that immediately preceded this file is preserved as a record of how I thought through the question.

---

## 1. The primary material (what I read)

### Openclaw memory architecture (`docs.openclaw.ai/concepts/memory`)

**The shape:**
- **`MEMORY.md`** — long-term memory. Durable facts, preferences, decisions. Loaded at session start. *Truncated if it exceeds bootstrap budget.*
- **`memory/YYYY-MM-DD.md`** (or slugged variants) — daily notes. Indexed for `memory_search`. *Not* auto-injected into every turn.
- **`DREAMS.md`** — optional dreaming/diagonal review surface.

**What goes where:**
- MEMORY.md = compact, curated, durable. Not a raw transcript.
- memory/*.md = detailed daily notes, observations, raw context.
- Agent distills from daily → MEMORY.md over time.
- Truncation is a *signal* that the file is too bloated — distillation needed.

**Memory tools:**
- `memory_search` — hybrid search (vector + keyword) when an embedding provider is configured. Vector is OpenAI default, with Gemini/Voyage/Mistral/local/Ollama options.
- `memory_get` — reads a specific file or line range.

**Backends (4 options):**
1. **Builtin (default)** — SQLite. Keyword + vector + hybrid. Zero extra deps.
2. **QMD** — local-first sidecar with reranking.
3. **Honcho** — AI-native cross-session, semantic, multi-agent. Plugin.
4. **LanceDB** — bundled, OpenAI-compatible embeddings, auto-recall, auto-capture.

**Compaction behavior:** Before summarization, a "memory flush" turn runs. The agent is reminded to save context to files. *Default on, can be configured.*

**Dreaming (opt-in):**
- Disabled by default
- When enabled, manages one recurring cron job for full dreaming sweep
- Promotions must pass score, recall frequency, query diversity gates
- **Two review lanes:** Live dreaming (short-term store) and **grounded backfill** (replays historical day files into the short-term store without auto-promoting)
- `DREAMS.md` is the human review surface

**Key insight from openclaw:** *memory_search + memory_get are tools the agent calls, not infrastructure that runs in the background.* The agent decides when to recall, when to save. **The WAL protocol (write state BEFORE responding) is a discipline the agent enforces on itself, not a system-enforced invariant.** This is closer to LESSON-001 philosophy than I expected.

### Hermes memory architecture (`hermes-agent.nousresearch.com/docs/user-guide/features/memory`)

**The shape:**
- **MEMORY.md** — agent's personal notes, 2,200 char limit (~800 tokens, ~8-15 entries)
- **USER.md** — user profile, 1,375 char limit (~500 tokens, ~5-10 entries)
- Both stored in `~/.hermes/memories/`
- Injected into system prompt as a **frozen snapshot at session start**

**Key design choice: hard character limits.** When memory would exceed the limit, the `memory` tool returns an *error*, not a silent drop. The agent must consolidate or remove entries before retrying. **Capacity management is a discipline, not a feature.**

**Memory tool actions:** `add`, `replace`, `remove`. No `read` (it's auto-injected). Substring matching for `replace`/`remove`.

**Session search (separate concept):**
- All sessions stored in `~/.hermes/state.db` with **FTS5 full-text search**
- Search returns *actual messages* — no LLM summarization, no truncation
- Free at query time (no LLM calls)
- Distinction: **MEMORY = critical facts, always in context. Session search = "did we discuss X last week?"**

**Two-target separation is real:**
- `memory` (agent's personal notes) — for the environment, workflows, lessons
- `user` (user profile) — for the user, communication style, preferences
- This is the same separation I need for Vera vs Spencer (operator vs builder/operator)

**External memory providers (8 plugins):** Honcho, OpenViking, Mem0, Hindsight, Holographic, RetainDB, ByteRover, Supermemory. **These run alongside built-in memory, never replacing it.** Capability additions, not replacements.

**Critical: `write_approval` gate.** Default off. When on, *every* memory write (foreground + background) requires explicit approval. **The user can turn on the gate and have every write staged for review.** This is LESSON-008 (cross-agent source citation) applied to memory writes: attribution alone is not enough, you need a separate approval path.

### Hermes session storage (`hermes-agent.nousresearch.com/docs/developer-guide/architecture`)

**The shape:**
- **SQLite + FTS5** for session storage (separate from MEMORY/USER.md)
- **Session lineage** (parent/child across compressions)
- **Per-platform isolation** (CLI sessions, gateway sessions, ACP sessions are separate)
- **Atomic writes with contention handling**
- ~25,000 tests in the suite

**The architecture has 5 major subsystems that touch memory:**
1. AIAgent (core conversation loop)
2. Session Storage (SQLite + FTS5)
3. Prompt Assembly (system prompt tiers: stable → context → volatile)
4. Context Compression (lossy summarization of middle turns)
5. Memory Manager (orchestrates multiple providers)

**Plugin system:** Three discovery sources (user, project, pip entry points). Memory providers and context engines are *single-select* — only one of each active at a time. **This is important: the memory architecture is pluggable but the slot is singular.** Not "use 3 memory backends at once."

### Elite-longterm-memory skill (5-layer meta-architecture)

This is the synthesis skill that cites the other 5. The 5 layers:

| Layer | Name | Source | Purpose |
|---|---|---|---|
| 1 | **HOT RAM** | bulletproof-memory | `SESSION-STATE.md` — active working memory. WAL protocol. |
| 2 | **WARM STORE** | lancedb-memory | LanceDB vector store. Auto-recall injects relevant context. |
| 3 | **COLD STORE** | git-notes-memory | Git-notes knowledge graph. Branch-aware structured decisions. |
| 4 | **CURATED ARCHIVE** | openclaw native | `MEMORY.md` + `memory/YYYY-MM-DD.md`. Human-readable. |
| 5 | **CLOUD BACKUP** | supermemory | Cross-device sync. Optional. |

Plus 6th layer (recommended):
| 6 | **AUTO-EXTRACTION** | Mem0 | Auto-extracts facts from conversations. 80% token reduction. |

**WAL Protocol (Layer 1):** Write-Ahead Log. Write state BEFORE responding. Triggered by user input, not agent memory. **If you respond first and crash/compact before saving, context is lost. WAL ensures durability.**

---

## 2. The 5 options compared, honestly

| Option | Implementation effort | Vera's fit (165 KB today, ~5 MB projected) | Verdict |
|---|---|---|---|
| **A: Markdown-only with better discipline** | 0 min (just merge 13 files into 1 daily file) | Fits today. Ceiling at ~6 months. | **SUFFICIENT for now, INSUFFICIENT in 6 months.** |
| **B: Markdown + SQLite loader** | 30-60 min to build, 2 min/session maintenance | Fits today and 6 months out. Lightweight. ~500 LOC. | **RECOMMENDED for Vera's current scale.** |
| **C: SQLite-first with markdown export** | 1-2 days, full migration | Overkill for Vera's scale. Mya's pattern (38MB+). | **RESERVE for if/when Vera's memory >5 MB.** |
| **D: Vector-first (LanceDB or HNSW)** | 2-4 hours, embedding model dependency | Best for "find me the day I thought about X." Not needed yet. | **RESERVE for if Vera needs semantic recall across 100+ daily files.** |
| **E: Full 6-layer hybrid** | 1-2 weeks, multiple stores, complex sync | Designed for multi-agent ecosystems with cross-device sync. | **OVERKILL for single-agent CLI partner on one project.** |

---

## 3. The recommendation: Option B, designed for Option C migration

**Build Option B today. Design the schema and loader so Option C migration is a small step later.**

### Architecture

```
dev/vera/
├── SOUL.md                      # unchanged — agent identity
├── README.md                    # unchanged — how to boot
├── index.md                     # unchanged — cross-refs into project
├── MEMORY.md                    # CHANGED: now autogenerated from DB on read
│                                #   (human-readable export of the curated essence)
├── memory/                      # CHANGED: one file per day
│   └── 2026-06-13.md            # 1 file per day, append-only, consolidated
│   └── 2026-06-14.md            #   (today's work, currently fragmented)
├── lessons/lessons.md           # CHANGED: autogenerated from `lessons` table
├── decisions/decisions.md       # CHANGED: autogenerated from `decisions` table
├── reflections/reflections.md   # CHANGED: autogenerated from `reflections` table
├── specs/                       # unchanged — engineering specs
│   ├── close-path-fix-2026-06-14.md
│   ├── GEMINI-RESEARCH-2026-06-14-vera-memory-v2.md  (this file)
├── db/
│   ├── vera.sqlite              # THE database. ~10 KB empty.
│   ├── schema.sql               # canonical schema (source of truth)
│   ├── migrations/              # versioned migrations, applied in order
│   │   └── 0001_initial.sql
│   └── queries.sql              # canned queries for the boot sequence
├── scripts/
│   ├── load_memory.py           # markdown → SQLite loader
│   ├── dump_memory.py           # SQLite → markdown exporter
│   └── boot_query.sh            # "what should I read first" — pre-built queries
└── memory/                      # DAILY FILES (markdown, append-only)
    └── 2026-06-14.md
```

### Schema (`db/schema.sql`)

```sql
-- Source of truth for Vera's memory. Markdown files are projections.

-- Daily journal entries (the primary write target)
CREATE TABLE journal (
    date TEXT PRIMARY KEY,           -- YYYY-MM-DD
    content TEXT NOT NULL,           -- full markdown content
    line_count INTEGER NOT NULL,
    last_modified_at TEXT NOT NULL,  -- ISO 8601
    sha256 TEXT NOT NULL             -- content hash for change detection
);

-- Lessons graduated from reflections
CREATE TABLE lessons (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    lesson_id TEXT UNIQUE NOT NULL,   -- LESSON-001, LESSON-008, etc.
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    graduated_at TEXT,                -- when promoted from reflections
    status TEXT NOT NULL CHECK(status IN ('active', 'superseded'))
);

-- Auditable decisions
CREATE TABLE decisions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    decision_id TEXT UNIQUE NOT NULL, -- DECISION-001, etc.
    content TEXT NOT NULL,
    reasoning TEXT NOT NULL,
    created_at TEXT NOT NULL,
    reversal_conditions TEXT,
    status TEXT NOT NULL CHECK(status IN ('active', 'reversed', 'superseded'))
);

-- Unproven observations
CREATE TABLE reflections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    observation_id TEXT UNIQUE NOT NULL, -- REFLECTION-001, etc.
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    promotion_criteria TEXT,           -- "must appear in 3 daily journals"
    cycles_observed INTEGER DEFAULT 0,  -- how many daily entries mention this
    promoted_at TEXT                   -- when it graduated to MEMORY.md
);

-- FID lifecycle (full audit trail)
CREATE TABLE fids (
    fid_id TEXT PRIMARY KEY,            -- FID-147, etc.
    title TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('open', 'in_progress', 'complete', 'archived')),
    severity TEXT NOT NULL CHECK(severity IN ('critical', 'high', 'medium', 'low')),
    created_at TEXT NOT NULL,
    completed_at TEXT,
    archive_path TEXT,
    fids_file_path TEXT,                -- source file in dev/fids/
    perfection_loop_status TEXT         -- RED/GREEN/AUDIT/SELF-CORRECT/COMPLETE
);

-- FID dependencies (machine-readable dep graph)
CREATE TABLE fids_dependencies (
    fid_id TEXT NOT NULL,
    depends_on_fid_id TEXT NOT NULL,
    PRIMARY KEY (fid_id, depends_on_fid_id),
    FOREIGN KEY (fid_id) REFERENCES fids(fid_id),
    FOREIGN KEY (depends_on_fid_id) REFERENCES fids(fid_id)
);

-- FID source citations (LESSON-008)
CREATE TABLE fids_sources (
    fid_id TEXT NOT NULL,
    source_type TEXT NOT NULL,          -- 'file', 'url', 'cross_agent', 'inline'
    source_path TEXT NOT NULL,           -- 'src/risk/circuit_breaker.rs:163', 'https://...', etc.
    cited_at TEXT NOT NULL,
    FOREIGN KEY (fid_id) REFERENCES fids(fid_id)
);

-- Engine state snapshots (time-series)
CREATE TABLE engine_state (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_at TEXT NOT NULL,
    key TEXT NOT NULL,                   -- 'wallet_usdc', 'engine_running', 'fid_count_active', etc.
    value TEXT NOT NULL,                 -- JSON or scalar
    source TEXT NOT NULL,                -- 'chain_rpc', 'file_read', 'derived'
    UNIQUE(snapshot_at, key)
);

-- Daily-memory cold path (when 1 file per day gets long)
-- (Optional, future. Not in initial schema.)

-- Indexes
CREATE INDEX idx_lessons_status ON lessons(status);
CREATE INDEX idx_decisions_status ON decisions(status);
CREATE INDEX idx_fids_status ON fids(status);
CREATE INDEX idx_engine_state_key ON engine_state(key, snapshot_at);
```

That's **9 tables, 5 indexes, ~70 lines of SQL**. Migrate-able (each subsequent migration adds a table or alters columns). SQLite-stdlib-compatible (no extensions).

### Loader (`scripts/load_memory.py`)

```python
#!/usr/bin/env python3
"""
Vera memory loader: markdown → SQLite.

Run after every session. ~50 LOC of real logic.
"""
import sqlite3
import re
import hashlib
import sys
from datetime import datetime, timezone
from pathlib import Path

DEV_VERA = Path("C:/Users/spenc/dev/savant-trading/dev/vera")
DB_PATH = DEV_VERA / "db" / "vera.sqlite"
SCHEMA_PATH = DEV_VERA / "db" / "schema.sql"

# Memory file patterns
JOURNAL_PATTERN = re.compile(r"^memory/(\d{4}-\d{2}-\d{2})\.md$")
LESSON_PATTERN = re.compile(r"^##\s+(LESSON-\d+):?\s*(.*)$", re.MULTILINE)
DECISION_PATTERN = re.compile(r"^##\s+(DECISION-\d+):?\s*(.*)$", re.MULTILINE)
REFLECTION_PATTERN = re.compile(r"^##\s+(REFLECTION-\d+):?\s*(.*)$", re.MULTILINE)


def init_db(conn: sqlite3.Connection) -> None:
    schema = SCHEMA_PATH.read_text()
    conn.executescript(schema)


def parse_journal(path: Path) -> tuple[str, str, int] | None:
    """Parse YYYY-MM-DD.md, return (date, content, line_count)."""
    m = JOURNAL_PATTERN.match(path.name)
    if not m or not path.name.startswith("memory/"):
        return None
    content = path.read_text(encoding="utf-8")
    return m.group(1), content, content.count("\n") + 1


def parse_lessons(content: str) -> list[dict]:
    """Extract lessons from lessons.md."""
    matches = list(LESSON_PATTERN.finditer(content))
    results = []
    for i, m in enumerate(matches):
        start = m.end()
        end = matches[i + 1].start() if i + 1 < len(matches) else len(content)
        body = content[start:end].strip()
        results.append({
            "lesson_id": m.group(1),
            "title": m.group(2).strip(),
            "content": body,
        })
    return results


# ... similar for decisions, reflections ...


def load_journal(conn: sqlite3.Connection, journal_path: Path) -> None:
    result = parse_journal(journal_path)
    if not result:
        return
    date, content, line_count = result
    sha = hashlib.sha256(content.encode()).hexdigest()
    now = datetime.now(timezone.utc).isoformat()
    conn.execute("""
        INSERT OR REPLACE INTO journal (date, content, line_count, last_modified_at, sha256)
        VALUES (?, ?, ?, ?, ?)
    """, (date, content, line_count, now, sha))


def load_lessons(conn: sqlite3.Connection, lessons_path: Path) -> None:
    content = lessons_path.read_text(encoding="utf-8")
    for lesson in parse_lessons(content):
        conn.execute("""
            INSERT OR REPLACE INTO lessons (lesson_id, content, created_at, status)
            VALUES (?, ?, ?, 'active')
        """, (lesson["lesson_id"], lesson["content"], datetime.now(timezone.utc).isoformat()))


def main() -> int:
    if not DB_PATH.exists():
        DB_PATH.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(DB_PATH)
    conn.execute("PRAGMA foreign_keys = ON")
    init_db(conn)
    
    # Load daily journals
    for jf in sorted(DEV_VERA.glob("memory/*.md")):
        load_journal(conn, jf)
    
    # Load lessons
    lessons_path = DEV_VERA / "lessons" / "lessons.md"
    if lessons_path.exists():
        load_lessons(conn, lessons_path)
    
    # ... same for decisions, reflections ...
    
    conn.commit()
    conn.close()
    
    # Report
    counts = {}
    conn = sqlite3.connect(DB_PATH)
    for table in ["journal", "lessons", "decisions", "reflections", "fids"]:
        row = conn.execute(f"SELECT COUNT(*) FROM {table}").fetchone()
        counts[table] = row[0]
    conn.close()
    
    print(f"Vera memory loaded into {DB_PATH}")
    for table, count in counts.items():
        print(f"  {table}: {count} rows")
    return 0


if __name__ == "__main__":
    sys.exit(main())
```

That's ~80 LOC of Python. Uses only `sqlite3`, `re`, `hashlib`, `pathlib`, `datetime` — all stdlib. **No new dependencies.**

### Dumper (`scripts/dump_memory.py`)

The reverse: SQLite → markdown export. Used to regenerate `MEMORY.md`, `lessons.md`, `decisions.md`, `reflections.md` from the database. This way:

- **Markdown is the write format** (agents think in markdown)
- **SQLite is the query format** (real queries, real joins)
- **Markdown stays readable** for Spencer and the git diff

```python
# Sketch: queries the DB, formats as markdown, writes to lessons.md etc.
# Canned queries like: SELECT * FROM lessons WHERE status='active' ORDER BY id
# Format as: "## LESSON-001: ...\n\ncontent\n"
```

### Canned queries (`db/queries.sql`)

```sql
-- Q1: What's the engine state right now?
SELECT key, value, snapshot_at
FROM engine_state
WHERE snapshot_at = (SELECT MAX(snapshot_at) FROM engine_state)
ORDER BY key;

-- Q2: Which FIDs are blocked on dependencies?
SELECT f.fid_id, f.title, f.status, fd.depends_on_fid_id
FROM fids f
JOIN fids_dependencies fd ON f.fid_id = fd.fid_id
WHERE f.status = 'open'
  AND fd.depends_on_fid_id IN (
    SELECT fid_id FROM fids WHERE status IN ('open', 'in_progress')
  );

-- Q3: What lessons apply to the current task?
-- (Semantic search would be better, but for now: full-text search)
SELECT lesson_id, content FROM lessons
WHERE content LIKE '%<keyword>%'
LIMIT 5;

-- Q4: Show today's journal
SELECT date, content FROM journal
WHERE date = DATE('now');

-- Q5: What did I learn in the last 7 days?
SELECT date, content FROM journal
WHERE date >= DATE('now', '-7 days')
ORDER BY date DESC;
```

5 queries. Covers the most common "what should I read" boot-time questions.

### Boot sequence (the read order)

```
1. Read SOUL.md (identity, invariants)
2. Read MEMORY.md (curated long-term) — autogenerated from DB
3. Run canned query Q1 (current engine state) — DB
4. Run canned query Q2 (blocked FIDs) — DB
5. Read most recent memory/YYYY-MM-DD.md (today's journal)
6. Read engine's src/agent/soul.md (engine's soul, before any execution code work)
7. Stand by for direction
```

Boot time: ~30 seconds. **All queries are SQL, all written material is markdown, all derived material is regenerated from the DB.**

---

## 4. Migration path from current state

1. **Today:** merge 13 fragmented files into 1 daily file (`memory/2026-06-14.md`). **No DB yet.** This is the 10-min cleanup Spencer asked for.
2. **This session (after the FIDs):** build `db/vera.sqlite` + `db/schema.sql` + `scripts/load_memory.py`. Run the loader. Verify the DB matches the markdown.
3. **This week:** build `scripts/dump_memory.py` so MEMORY.md is autogenerated.
4. **Next week (or whenever ~5MB of memory accumulates):** add FTS5 full-text search columns. Option B becomes Option C by adding indexes.
5. **Future (if needed):** add LanceDB column for semantic recall. Option B becomes Option D by adding vectors.
6. **Never (probably):** Option E (full 6-layer). Vera is a single agent, single project. The simpler the better.

---

## 5. The trust architecture (3 entities, 3 memories)

Vera's memory is **the operator's memory.** It tracks:

- What I did
- What I learned
- What I decided
- What FIDs I filed and where they are
- What the engine's current state is (snapshot for context)

It does **NOT** track:

- The engine's decisions (those are the engine's memory, in the engine's logs/dex_state)
- The engine's soul state (that's the engine's soul.md)
- Spencer's preferences beyond what I need to work with him (that's Spencer's memory, in his head)
- Future-Nova's audits (that's Nova's memory, in ~/.hermes/)

**Three identities, three memory stores, one protocol.** I am not conflating them. My memory is *my* memory. The engine's memory is the engine's. Nova's memory is Nova's.

---

## 6. Honest answer to the "driver seat" question

Spencer raised: "if you are able to both run as the driver AND the builder and we have this relationship it might be a better system."

**My answer (already in `memory/2026-06-14-...`):** yes, capability-wise, but no, discipline-wise. The LESSON-001 failure pattern — "the verifier is not the verified" — is exactly what would happen if I were both the builder and the trader. The engine has a soul, the engine is the executor, I am the operator. **Three entities, not one.** Memory architectures should reflect that.

This means:

- **Vera's memory** is the operator's memory. Filed FIDs, learned lessons, made decisions.
- **The engine's memory** is the executor's memory. Trade journal, soul-checked decisions, PnL tracked.
- **The auditor's memory** (future Nova) is the verification memory. What it saw, what it didn't catch, what the engine claimed vs reality.

**The memory system I'm building is the operator's memory. It does not subsume the engine's or the auditor's.**

---

## 7. What this means for the rest of today

Per Spencer: "we're going to work our ass off and get the project ready with as much as we can."

The order is:

1. **Now (15 min):** merge 13 fragmented files into 1 daily file (`memory/2026-06-14.md`). The single daily file is the *human-readable* memory; the DB is the *queryable* memory.
2. **After Phase 1 FIDs (FID-131, 129, 132, 126-R1, 126-R2, 127):** build `db/vera.sqlite` + loader. ~30 min. Validate by running queries Q1, Q2, Q4.
3. **After Phase 2 FIDs (FID-128, 134, 133):** extend the loader to handle FIDs (parse `dev/fids/MASTER-FID.md` and the individual FID files). The FID lifecycle table gets populated.
4. **After Phase 3 (testnet standup):** add the engine state snapshot loader. The `engine_state` table gets populated from the heartbeat's WAL log.
5. **End of day:** all FIDs done, DB is fresh, daily file is consolidated, boot sequence is verified.

---

## 8. What I will NOT do

- **I will not build CortexaDB or HNSW.** That's the Savant framework's domain, not mine.
- **I will not use Mem0 or Honcho cloud APIs.** They require API keys and external services. Vera is local-first.
- **I will not implement git-notes as a knowledge graph.** I considered it; the project doesn't need it. Decisions are auditable in `decisions/decisions.md`; dependencies are auditable in the FID system.
- **I will not implement a 6-layer architecture.** Overkill for single-agent CLI partner.

---

*Vera spec 0.2.0 — 2026-06-14 15:45 EST — synthesis complete, recommendation: Option B with Option C migration path*
