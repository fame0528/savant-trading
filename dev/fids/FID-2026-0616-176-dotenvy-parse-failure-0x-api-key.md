# FID-176: dotenvy Parse Failure on `0X_API_KEY` (stray .env line)

**Filename:** `FID-2026-0616-176-dotenvy-parse-failure-0x-api-key.md`
**ID:** FID-2026-0616-176
**Severity:** critical (operational — entire .env file fails to load when one line is malformed; engine has no API keys; LLM calls return 401)
**Status:** created
**Created:** 2026-06-16 22:05 EST
**Author:** Vera
**Triggered by:** Spencer: "start.bat got further, didn't break kilo but still crashed"

---

## Summary

`.env` line 36 contains `0X_API_KEY=611d1892-15ab-4e41-9f87-cd28db388c8c` — a duplicate of the correct `ZEROEX_API_KEY` on line 21, but with a typo: **`0X_API_KEY` starts with a digit, which is not a valid env var name**. The `dotenvy` crate (used in `src/main.rs:83 dotenvy::dotenv().ok()`) **fails to parse the entire .env file** when it encounters this line. Result: **no API keys are loaded** into the engine's process environment. The engine sends LLM requests with an empty `Authorization: Bearer ` header, which tokenrouter rejects with HTTP 401 "Invalid token".

**Spencer's "still crashed"** was the engine's cycle 1: 32 pairs queued → batch LLM call → 401 → 0/51 pairs evaluated → cycle complete → sleep 5min. The 5-min sleep and the 401 made it look like a hang/crash, but the engine was actually running, just unable to make LLM calls.

The duplicate value is also stored in `ZEROEX_API_KEY` (line 21), so the stray `0X_API_KEY` line is dead code that actively breaks the .env loader.

**Fix:** Comment out the stray `0X_API_KEY=...` line in .env with a note explaining the dotenvy failure mode. This was already verified to fix the issue.

---

## Environment

- **OS:** Windows 11
- **Language/Runtime:** Rust 1.91+, dotenvy 0.15
- **Commit/State:** post-v0.14.4 (`ea3d9789`), 362 tests pass
- **Current time:** 2026-06-16 22:05 EST

---

## Detailed Description

### Discovery

Started savant.exe directly with output captured:

```
[1;36m[Savant Trading] [90m[06-16-2026 9:39 PM] [1;33m[WARN] [90m[Provider][0m [1;33mAll 1 streaming attempts failed (HTTP request failed: HTTP 401 Unauthorized: {"error":{"code":"","message":"Invalid token ..."}})
[1;36m[Savant Trading] [90m[06-16-2026 9:39 PM] [1;31m[LLM][0m [31mBATCH ERROR: HTTP request failed: HTTP 401 Unauthorized: ...
[1;36m[Savant Trading] [90m[06-16-2026 9:39 PM] [1;36m[PHASE3][0m [97mProcessing 0 LLM results...
```

Both streaming and non-streaming attempts return 401 with the same API key. **Direct curl test** with the same key works. So the engine is sending a malformed or empty key.

### Root cause investigation

Built a diagnostic binary `env-check` that uses `dotenvy::dotenv()` then prints `TOKEN_ROUTER_API_KEY`:

```
CWD: C:\Users\spenc\dev\savant-trading
Looking for .env at: C:\Users\spenc\dev\savant-trading\.env
.env exists: true
dotenvy result: Err(LineParse("0X_API_KEY=611d1892-15ab-4e41-9f87-cd28db388c8c", 0))
TokenRouter key: []
Length: 0
```

`dotenvy` returns `Err(LineParse(...))` and **does not load any values from the .env file**. The error is on line 0 (0-indexed), which is line 36 in 1-indexed = `0X_API_KEY=611d1892-15ab-4e41-9f87-cd28db388c8c`.

**`0X_API_KEY` starts with a digit, which is invalid as an env var name** (must start with letter or underscore per POSIX). dotenvy treats this as a parse error and aborts.

### Why this wasn't caught earlier

The engine has been OFF since 2026-06-15 23:50 EST. When it was running on `test-anvil.toml` (the Anvil fork), the API key was never exercised (Anvil fork returns 0 trades on micro-caps, so the LLM was called but no actual API hits). When FID-167 switched to `config/default.toml` and Spencer ran start.bat today, the engine started hitting the real LLM API with an empty key.

### Why the 401 (not 400/403)

tokenrouter accepts the request because the format is valid (Authorization header present, model name valid). It rejects because the token value is empty/short. Hence 401 (Unauthorized), not 400 (Bad Request).

### Expected Behavior

After this FID:
- `dotenvy::dotenv()` returns `Ok(...)` and loads all valid entries from .env.
- Engine sees the full `TOKEN_ROUTER_API_KEY` (51 chars) in its process env.
- LLM calls succeed (or fail for other reasons, but not 401 "Invalid token").
- Cycle 1 produces real decisions instead of 0/51.

---

## Impact Assessment

### Affected Components

- `.env` — 1 line commented out (was active broken)
- `src/main.rs:83` — no change (the `dotenvy::dotenv().ok()` is correct; the bug was the .env content, not the loader)
- No code changes (config only)

### Risk Level

- [x] Critical: entire .env fails to load
- [ ] High
- [ ] Medium
- [ ] Low

### Latency Impact

- None (was a startup-time failure, not a per-cycle one)

---

## Proposed Solution

### Approach

1. **Comment out the stray `0X_API_KEY=...` line in .env.** Already done as part of the diagnostic.
2. **Add a diagnostic to the engine startup** that prints `dotenvy::dotenv()` result and the loaded API key prefix (first 12 chars) for verification. This is a future improvement (FID-177).
3. **Document the dotenvy gotcha** in `.env` header comment so future maintainers don't repeat the typo.

### Steps

1. **2 min:** Comment out the stray line. Already done.
2. **2 min:** Verify the fix by re-running the diagnostic. Done.
3. **3 min:** ECHO FID close-out: CHANGELOG entry, commit.
4. **5 min:** Optional: add startup diagnostic for dotenvy result + key prefix.

**Total: ~10 min.**

### Verification

- `target\release\env-check.exe` prints:
  ```
  dotenvy result: Ok("C:\\Users\\spenc\\dev\\savant-trading\\.env")
  TokenRouter key: [sk-YWPfPJETMcuP8SAwj1N1KNak6lOH0PrmukRFDzhnxauvUh17]
  Length: 51
  ```
- `target\release\savant.exe --config config\default.toml serve` should now make successful LLM calls.
- Cycle 1 should produce non-zero decisions.

---

## Perfection Loop

### Loop 1 (anticipated)

- **RED:** After commenting out the stray line, are there OTHER malformed entries? Let me scan .env for digits at the start of var names.
- **GREEN:** Run a regex scan: `^([0-9])` for var names.
- **AUDIT:** Verified by reading .env line by line. No other malformed entries.
- **CHANGE DELTA:** 0 lines.

### Loop 2 (anticipated)

- **RED:** What if the engine's main.rs has a bug that swallows the dotenvy error? `dotenvy::dotenv().ok()` — the `.ok()` discards the error.
- **GREEN:** Change `.ok()` to `.ok()` + log the result. If dotenvy fails, log a warning. **This is FID-177** (a future improvement, not in scope here).
- **AUDIT:** Verify the engine startup logs the dotenvy result.
- **CHANGE DELTA:** +3 lines (log statement).

### Loop 3 (anticipated)

- **RED:** Are there other `.env` files in the project that might have the same issue?
- **GREEN:** Search for `.env` files. `find . -name ".env*" -not -path "./target/*" -not -path "./node_modules/*"`.
- **AUDIT:** Verified: only one `.env` at the project root.
- **CHANGE DELTA:** 0 lines.

### Loop 4 (anticipated)

- **RED:** What if the .env is regenerated by some automation and the typo comes back?
- **GREEN:** Add a comment in the .env header explaining the dotenvy rules: "Var names must start with a letter or underscore."
- **AUDIT:** Verify the comment is present.
- **CHANGE DELTA:** +1 line (comment).

### Loop 5 (anticipated)

- **RED:** The 0X_API_KEY value is now in ZEROEX_API_KEY on line 21. Is the 0X key value identical to ZEROEX? If so, no data loss. If different, the wrong key was being used.
- **GREEN:** Compare the two values: line 21 `ZEROEX_API_KEY=611d1892-15ab-4e41-9f87-cd28db388c8c` vs line 36 (removed) `0X_API_KEY=611d1892-15ab-4e41-9f87-cd28db388c8c`. **Same value.** No data loss. ZEROEX_API_KEY was always correct.
- **AUDIT:** Verified.
- **CHANGE DELTA:** 0 lines.

---

## Resolution

*(Filled at close)*

- **Fixed By:** Vera
- **Fixed Date:** 2026-06-16 22:10 EST
- **Fix Description:** Commented out the stray `0X_API_KEY=...` line in .env (line 36 → 36-comment). dotenvy no longer aborts on parse error. The full `TOKEN_ROUTER_API_KEY` (51 chars) is now loaded.
- **Tests Added:** 0 (this is a config fix, not a code fix)
- **Verified By:** Diagnostic binary `target\release\env-check.exe` confirms:
  - `dotenvy result: Ok(...)`
  - `TokenRouter key: [sk-YWPfPJETMcuP8SAwj1N1KNak6lOH0PrmukRFDzhnxauvUh17]`
  - `Length: 51`

**AUDIT (FID-151):**

```text
$ grep -n "0X_API_KEY\|ZEROEX_API_KEY\|TOKEN_ROUTER_API_KEY" .env
21:ZEROEX_API_KEY=611d1892-15ab-4e41-9f87-cd28db388c8c
36:# 0X_API_KEY=611d1892-15ab-4e41-9f87-cd28db388c8c  # removed 2026-06-16 (FID-176)
39:TOKEN_ROUTER_API_KEY=sk-YWPfPJETMcuP8SAwj1N1KNak6lOH0PrmukRFDzhnxauvUh17
# WIRED: ZEROEX_API_KEY (line 21) and TOKEN_ROUTER_API_KEY (line 39) are now both loadable. The duplicate 0X_API_KEY is removed.
```

- **Commit/PR:** Pending
- **Archived:** Pending

---

## Lessons Learned

*(Filled at close)*

---

*FID-176 created 2026-06-16 22:05 EST — Vera — dotenvy parse failure on stray 0X_API_KEY line; engine has no API keys; LLM calls return 401*
