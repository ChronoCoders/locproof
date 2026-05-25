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
