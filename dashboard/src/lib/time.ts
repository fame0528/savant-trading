/**
 * Timestamp formatting utilities for the Savant dashboard.
 * Handles multiple input formats and produces consistent 12h AM/PM output.
 *
 * Input formats handled:
 *   - ISO 8601: "2026-06-05T19:55:42Z", "2026-06-05T19:55:42.123Z"
 *   - Space-separated: "2026-06-05 19:55:42"
 *   - Time-only: "19:55:42", "19:55"
 *   - Unix seconds: 1749139542
 *   - Unix milliseconds: 1749139542000
 */

function parseTime(timestamp: string | number): { h: number; m: number; s: number } | null {
  if (timestamp === null || timestamp === undefined) return null;

  // Unix timestamp (number or numeric string)
  if (typeof timestamp === "number" || /^\d{10,13}$/.test(String(timestamp))) {
    const num = typeof timestamp === "number" ? timestamp : parseInt(timestamp, 10);
    const ms = num > 1e12 ? num : num * 1000; // seconds vs milliseconds
    const d = new Date(ms);
    if (isNaN(d.getTime())) return null;
    return { h: d.getHours(), m: d.getMinutes(), s: d.getSeconds() };
  }

  const str = String(timestamp).trim();
  if (!str) return null;

  // Extract time portion — handles both "T" and space separators
  // "2026-06-05T19:55:42Z" → "19:55:42"
  // "2026-06-05 19:55:42" → "19:55:42"
  // "19:55:42" → "19:55:42"
  const timeMatch = str.match(/(\d{1,2}):(\d{2})(?::(\d{2}))?/);
  if (!timeMatch) return null;

  const h = parseInt(timeMatch[1], 10);
  const m = parseInt(timeMatch[2], 10);
  const s = timeMatch[3] ? parseInt(timeMatch[3], 10) : 0;

  if (h < 0 || h > 23 || m < 0 || m > 59 || s < 0 || s > 59) return null;
  return { h, m, s };
}

/** Format as "7:55:42 PM" (12h, no leading zero on hour) */
export function formatTime12h(timestamp: string | number): string {
  const t = parseTime(timestamp);
  if (!t) return "—";
  const period = t.h >= 12 ? "PM" : "AM";
  const h12 = t.h % 12 || 12;
  return `${h12}:${String(t.m).padStart(2, "0")}:${String(t.s).padStart(2, "0")} ${period}`;
}

/** Format as "19:55:42" (24h, zero-padded) */
export function formatTime24h(timestamp: string | number): string {
  const t = parseTime(timestamp);
  if (!t) return "—";
  return `${String(t.h).padStart(2, "0")}:${String(t.m).padStart(2, "0")}:${String(t.s).padStart(2, "0")}`;
}

/** Format as "7:55 PM" (short, no seconds) */
export function formatTimeShort(timestamp: string | number): string {
  const t = parseTime(timestamp);
  if (!t) return "—";
  const period = t.h >= 12 ? "PM" : "AM";
  const h12 = t.h % 12 || 12;
  return `${h12}:${String(t.m).padStart(2, "0")} ${period}`;
}
