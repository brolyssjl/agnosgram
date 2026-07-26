/**
 * Small, dependency-free date helpers for the anti-rot machinery. All dates are
 * plain ISO calendar dates (`YYYY-MM-DD`), compared in UTC so results do not
 * depend on the runner's timezone.
 */

export function todayIso(d: Date = new Date()): string {
  return d.toISOString().slice(0, 10);
}

/** True only for a well-formed, real calendar date in `YYYY-MM-DD` form. */
export function isValidIsoDate(s: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(s)) return false;
  const [y, m, d] = s.split("-").map(Number) as [number, number, number];
  const dt = new Date(Date.UTC(y, m - 1, d));
  return dt.getUTCFullYear() === y && dt.getUTCMonth() === m - 1 && dt.getUTCDate() === d;
}

/** Whole days from `a` to `b` (positive when `b` is later). Assumes valid input. */
export function daysBetween(a: string, b: string): number {
  const ta = Date.parse(`${a}T00:00:00Z`);
  const tb = Date.parse(`${b}T00:00:00Z`);
  return Math.round((tb - ta) / 86_400_000);
}
