#!/usr/bin/env python3
"""FID-212 Fix B v3: text-mode Python regex swap.

Source file is LF-only (verified: CRLF=0, LF-only=6230, CR-only=0).
Text mode read/write preserves \n line endings and avoids sed/head CRLF
stripping entirely.

Strategy:
  1. Read src/engine/mod.rs as text.
  2. Use re.DOTALL to capture two blocks:
        FID-155 chain-sync block: from `        // FID-155 / DECISION-015:`
                                  to its closing `        }\n`
        FID-147 heartbeat block: from `        // FID-147:`` to its closing `        }\n`
  3. Swap the two blocks. Both end with `\n`; the only separator between them
     is a single blank-line `\n`, which we preserve verbatim.
  4. Write back. Verify byte count, line count, and content ordering.

Backup: tmp/fid212/engine_mod_BACKUP.rs is the pre-swap verbatim copy.
"""
import re, os, sys, shutil

SRC = "src/engine/mod.rs"
BACKUP = "tmp/fid212/engine_mod_BACKUP.rs"

# 1. Refresh backup from current source (in case v2 ran partially)
shutil.copy2(SRC, BACKUP)

# 2. Read
with open(SRC, "r", encoding="utf-8") as f:
    content = f.read()

orig_lines = content.count("\n") + (0 if content.endswith("\n") else 1)
print(f"[orig]  bytes={len(content)} lines={orig_lines}")

# 3. Capture each block starting at its comment, ending at `        }\n`
#    The closing brace line is 8-space-indented `        }` followed by `\n`.
#    We capture everything from the comment to (and including) that final `\n`.
chain_re = re.compile(
    r'        // FID-155 / DECISION-015: Periodic chain-driven reconciliation\..*?        \}\n',
    re.DOTALL,
)
heart_re = re.compile(
    r'        // FID-147: Wallet Reconciliation Heartbeat \(runs at start of every cycle\)\..*?        \}\n',
    re.DOTALL,
)

chain_m = chain_re.search(content)
heart_m = heart_re.search(content)
assert chain_m is not None, "CHAIN block not found"
assert heart_m is not None, "HEART block not found"
assert chain_m.start() < heart_m.start(), "chain block must appear before heart in orig"

chain_full = content[chain_m.start():chain_m.end()]
heart_full = content[heart_m.start():heart_m.end()]
separator  = content[chain_m.end():heart_m.start()]

print(f"[chunks] chain_block lines={chain_full.count(chr(10))} bytes={len(chain_full)}")
print(f"[chunks] heart_block lines={heart_full.count(chr(10))} bytes={len(heart_full)}")
print(f"[chunks] separator repr={separator!r}")

# Sanity: separator is exactly "\n" (a single blank line) — the original
# structure was `        }\n\n        // FID-147`, where the LAST `\n` of `\n        }\n` ends
# the chain block, and the `\n\n` after includes that trailing newline +
# the empty-line `\n`. After end of chain_match (right after chain's closing
# `\n`), there should be ONE more `\n` for the blank line, then heart text.
# In text-mode, this is exactly `\n`.
assert separator == "\n", f"Unexpected separator between blocks: {separator!r}"

# 4. Build new content: head + heart_full + separator + chain_full + tail
head = content[:chain_m.start()]
tail = content[heart_m.end():]
new = head + heart_full + separator + chain_full + tail

# 5. Verify invariants
new_lines = new.count("\n") + (0 if new.endswith("\n") else 1)
print(f"[new]   bytes={len(new)} lines={new_lines}")
assert new_lines == orig_lines, f"Line count drift: orig={orig_lines} new={new_lines}"
assert len(new) == len(content), f"Byte drift: orig={len(content)} new={len(new)}"

# Verify swap: HEART comment must now appear BEFORE CHAIN comment
heart_pos_new = new.find("        // FID-147: Wallet Reconciliation Heartbeat")
chain_pos_new = new.find("        // FID-155 / DECISION-015:")
print(f"[verify] new[HEART at byte {heart_pos_new}, CHAIN at byte {chain_pos_new}]")
assert heart_pos_new < chain_pos_new, "Heartbeat must now precede chain block"
assert heart_pos_new != -1 and chain_pos_new != -1, "Both markers must exist"

# 6. Write
with open(SRC, "w", encoding="utf-8") as f:
    f.write(new)
print(f"[write] {SRC} updated")
print(f"[ok] HL+CL swap verified; bytes={len(new)}, lines={new_lines}, LF-only preserved")
