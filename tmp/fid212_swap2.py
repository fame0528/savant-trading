#!/usr/bin/env python3
"""FID-212 Fix B v2: CRLF-preserving block swap in src/engine/mod.rs.

Reads the entire file in binary mode (preserves CRLF), locates the two blocks
by exact byte-anchored markers, swaps them, writes back.

Anchors (verified from grep):
  FID-155 chain-sync block: starts at `        // FID-155 / DECISION-015:`
                             ends at `                last_chain_sync = now;\n            }\n        }\n` (followed by blank line + heartbeat block)

  FID-147 heartbeat block: starts at `        // FID-147: Wallet Reconciliation Heartbeat`
                           ends at the matching `        }` that closes the
                           heartbeat block (followed by an empty line + the
                           FID-093 midnight reset block).

Strategy: find the chain block boundaries by exact byte string matching,
then the heartbeat block, by exact byte string matching. Both blocks are
unique byte sequences in the file, so we can match them precisely.

Order swap: heartbeat FIRST, then chain block.
"""
import shutil, os, sys

SRC = "src/engine/mod.rs"
BACKUP = "tmp/fid212/engine_mod_BACKUP.rs"

# 1. Backup first
shutil.copy2(SRC, BACKUP)
print(f"[backup] {SRC} -> {BACKUP} (bytes={os.path.getsize(BACKUP)})")

# 2. Read full file as bytes
with open(SRC, "rb") as f:
    raw = f.read()

# Verify CRLF state
crlf_count = raw.count(b"\r\n")
lf_only = raw.count(b"\n") - crlf_count
cr_only = raw.count(b"\r") - crlf_count
print(f"[orig]  bytes={len(raw)} CRLF={crlf_count} LF-only={lf_only} CR-only={cr_only}")
assert lf_only == 0 and cr_only == 0, "Source has mixed line endings — abort"

# 3. Anchors
CHAIN_START = b"        // FID-155 / DECISION-015: Periodic chain-driven reconciliation."
CHAIN_END   = b"                last_chain_sync = now;\n            }\n        }\n"

HEART_START = b"        // FID-147: Wallet Reconciliation Heartbeat (runs at start of every cycle)."
# HEART_END: the heartbeat block has a `        }` close that is preceded by
# `                }\n            }\n        }` near line ~250 of the block content. We
# find this by locating the string `DivergenceType::None => {\n                        // halted=true but type=None shouldn't happen, but be safe.\n                        break;\n                    }\n                }\n            }\n        }\n` plus the trailing blank line — the SINGLE unique key. We will find it as the
# region AFTER HEART_START until we have balanced { and }, then trim trailing
# blank line(s).
HEART_END_PADDED = (
    b"                    DivergenceType::None => {\n"
    b"                        // halted=true but type=None shouldn't happen, but be safe.\n"
    b"                        break;\n"
    b"                    }\n"
    b"                }\n"
    b"            }\n"
    b"        }\n"
)

def find_or_fail(needle, haystack, label):
    idx = haystack.find(needle)
    if idx < 0:
        print(f"[FAIL] {label} not found in file", file=sys.stderr)
        sys.exit(2)
    return idx

chain_start = find_or_fail(CHAIN_START, raw, "CHAIN_START")
chain_end_marker = find_or_fail(CHAIN_END, raw, "CHAIN_END (last_chain_sync marker)")
# CHAIN_END is the 3-line tail; chunk_end is the byte position right after it.
chunk_end = chain_end_marker + len(CHAIN_END)

heart_start = find_or_fail(HEART_START, raw, "HEART_START")
heart_end_padded = find_or_fail(HEART_END_PADDED, raw, "HEART_END_PADDED")
# HEART_END_PADDED ends with the closing `}\n`. We want the chunk to include up
# through that line, no trailing blank. chunk_end2 = end of HEART_END_PADDED.
heart_chunk_end = heart_end_padded + len(HEART_END_PADDED)

assert heart_start > chunk_end, "Heartbeat must come AFTER chain block in original"
print(f"[anchors] chain: bytes[{chain_start}..{chunk_end}] (length={chunk_end - chain_start})")
print(f"[anchors] heart: bytes[{heart_start}..{heart_chunk_end}] (length={heart_chunk_end - heart_start})")

# 4. Extract the three parts. Between chain_end and heart_start there are
#    blank line(s) that we want to keep as the post-chain separator.
between_start = chunk_end
between_end = heart_start
between_region = raw[between_start:between_end]

chain_block = raw[chain_start:chunk_end]
heart_block = raw[heart_start:heart_chunk_end]

print(f"[chunks] chain_block bytes={len(chain_block)} heart_block bytes={len(heart_block)} between bytes={len(between_region)}")

# 5. Rebuild: head + between (which becomes the seam between heart and chain) +
#    heart_block + chain_block + tail
head = raw[:chain_start]
tail = raw[heart_chunk_end:]

# between_region currently has structure: `\n\n` (blank lines from chain-close)
# and `\n\n` (blank lines before heartbeat-start). After swap, we want a single
# blank-line gap between heart and chain: keep both, since they're symmetric.
new_content = head + heart_block + between_region + chain_block + tail

# Sanity
new_bytes = len(new_content)
new_crlf = new_content.count(b"\r\n")
new_lf_only = new_content.count(b"\n") - new_crlf
new_cr_only = new_content.count(b"\r") - new_crlf
print(f"[new]   bytes={new_bytes} CRLF={new_crlf} LF-only={new_lf_only} CR-only={new_cr_only}")
assert new_crlf == crlf_count, f"CRLF count mismatch: orig={crlf_count} new={new_crlf}"
assert new_lf_only == 0 and new_cr_only == 0, "Mixed line endings in output"
assert new_bytes == len(raw), f"Byte-count drift: orig={len(raw)} new={new_bytes}"

# 6. Write
with open(SRC, "wb") as f:
    f.write(new_content)
print(f"[write] {SRC} updated (head+heart+chain+tail order, CRLF preserved)")
print(f"[verify] Spot-check: heartbeat block now appears before chain block")
post_content = open(SRC, "rb").read()
h_pos = post_content.find(b"        // FID-147: Wallet Reconciliation Heartbeat")
c_pos = post_content.find(b"        // FID-155 / DECISION-015:")
print(f"          HEARTSTART at byte {h_pos}, CHAINSTART at byte {c_pos}")
assert h_pos < c_pos, "Heartbeat should now appear BEFORE chain block"
print("[OK] HEAD+HEART+CHAIN+TAIL swap verified — heartbeat above chain — CRLF intact")
