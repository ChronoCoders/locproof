// Small presentation helpers shared across dashboard views.

/// Render an ISO-8601 timestamp as a compact local date-time. Falls back to
/// the raw string if the input doesn't parse, so a malformed value is visible
/// rather than silently blank.
export function fmtDateTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/// Render a `YYYY-MM-DD` month bucket (first of the month) as "MMM" for a
/// compact chart axis (e.g. "May"), or "MMM YYYY" when `withYear` is set.
/// Parsed as UTC so the day-1 date can't slip to the previous month in
/// negative-offset timezones.
export function fmtMonth(iso: string, withYear = false): string {
  const d = new Date(`${iso.slice(0, 10)}T00:00:00Z`);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(undefined, {
    month: "short",
    year: withYear ? "numeric" : undefined,
    timeZone: "UTC",
  });
}

/// The backend encodes an unlimited (enterprise) quota as u32::MAX. Detect it
/// so the UI shows "Unlimited" instead of a meaningless 4.29-billion ceiling.
export const UNLIMITED_QUOTA = 4_294_967_295;

export function isUnlimited(quota: number): boolean {
  return quota >= UNLIMITED_QUOTA;
}

/// Map a proximity score in [0, 1] to a semantic tone for the score badge.
/// >= 0.8 reads as a confident match, 0.5–0.8 as borderline, below as weak.
export type ScoreTone = "high" | "medium" | "low";

export function scoreTone(score: number): ScoreTone {
  if (score >= 0.8) return "high";
  if (score >= 0.5) return "medium";
  return "low";
}

const TONE_CLASS: Record<ScoreTone, string> = {
  high: "bg-emerald-500/15 text-emerald-400 border-emerald-500/30",
  medium: "bg-amber-500/15 text-amber-400 border-amber-500/30",
  low: "bg-red-500/15 text-red-400 border-red-500/30",
};

export function scoreToneClass(tone: ScoreTone): string {
  return TONE_CLASS[tone];
}
