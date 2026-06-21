#!/usr/bin/env python3
"""FID-212 Fix B: Swap the FID-155 chain-sync block and FID-147 heartbeat block
in src/engine/mod.rs. Preserves CRLF exactly.

Reads 4 slice files (already extracted via sed):
  tmp/fid212/head.txt — lines 1-1344 (everything before FID-155 chain block)
  tmp/fid212/chain.txt — lines 1345-1466 (FID-155 / DECISION-015 chain sync block)
  tmp/fid212/heart.txt — lines 1469-1592 (FID-147 heartbeat block)
  tmp/fid212/tail.txt — lines 1593-1622 (FID-093 midnight reset onwards)

Reassembles in NEW order: head + heart + chain + tail
Writes tmp/fid212/new_engine_mod.rs (CRLF-preserved).
"""
import sys, os

ROOT = "."  # invoked from project root
TSLICE = f"{ROOT}/tmp/fid212"

def read_bytes(p):
    with open(p, "rb") as f:
        return f.read()

def crlf_stats(b):
    crlf = b.count(b"\r\n")
    cr = b.count(b"\r") - crlf
    lf_only = b.count(b"\n") - crlf
    return len(b), crlf, lf_only, cr

# Verify each slice is clean CRLF before reassembling
slices = ["head.txt", "chain.txt", "heart.txt", "tail.txt"]
print("=== Slice CRLF stats (all should be LF-only=0, CR-only=0) ===")
for s in slices:
    p = f"{TSLICE}/{s}"
    bs = read_bytes(p)
    b, crlf, lf, cr = crlf_stats(bs)
    flag = "OK" if (lf == 0 and cr == 0) else "LEAK"
    print(f"  {s}: bytes={b} CRLF={crlf} LF-only={lf} CR-only={cr} [{flag}]")

# Reassemble in new order: head + heart + chain + tail
parts_in_order = ["head.txt", "heart.txt", "chain.txt", "tail.txt"]
out_path = f"{TSLICE}/new_engine_mod.rs"
total_bytes = 0
with open(out_path, "wb") as out:
    for p in parts_in_order:
        bs = read_bytes(f"{TSLICE}/{p}")
        out.write(bs)
        total_bytes += len(bs)

print(f"=== Reassembled {out_path}: {total_bytes} bytes ===")
bs = read_bytes(out_path)
b, crlf, lf, cr = crlf_stats(bs)
flag = "OK" if (lf == 0 and cr == 0) else "LEAK"
print(f"  CRLF={crlf} LF-only={lf} CR-only={cr} [{flag}]")

# Compare byte-counts to confirm zero loss
orig_size = os.stat("src/engine/mod.rs").st_size
new_size = os.path.getsize(out_path)
print(f"=== Size comparison: old={orig_size} new={new_size} delta={new_size - orig_size} ===")
print("  (delta should be exactly 0 — pure swap, no additions or deletions)")
