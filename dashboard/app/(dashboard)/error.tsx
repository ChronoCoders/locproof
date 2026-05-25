"use client";

import { useEffect } from "react";
import { AlertTriangle, RotateCw } from "lucide-react";

import { Button } from "@/components/ui/button";

// Segment-level error boundary for authenticated routes. The server-side
// data fetches in child pages (e.g. proofs/page.tsx) rethrow non-401 errors
// (backend 500, DB outage, network) on purpose — they land here rather than
// masquerading as an auth failure. 401s are handled upstream by a redirect
// to /login, so they never reach this boundary. (A segment's error.tsx does
// not wrap its own layout.tsx, so a throw from requireSession() in the
// (dashboard) layout bubbles to a parent boundary, not this one.)
//
// `unstable_retry` (Next 16.2) re-fetches and re-renders the boundary's
// children — unlike `reset`, which only re-renders the cached, still-failed
// RSC payload and so can't recover a transient server-fetch failure.
export default function DashboardError({
  error,
  unstable_retry,
}: {
  error: Error & { digest?: string };
  unstable_retry: () => void;
}) {
  useEffect(() => {
    console.error(error);
  }, [error]);

  return (
    <div className="mx-auto flex max-w-md flex-col items-center gap-4 py-24 text-center">
      <AlertTriangle
        aria-hidden
        className="h-8 w-8 text-muted-foreground"
        strokeWidth={1.5}
      />
      <div className="space-y-1">
        <h2 className="text-lg font-semibold tracking-tight">
          Something went wrong
        </h2>
        <p className="text-sm text-muted-foreground">
          We couldn&apos;t load this page. This is usually temporary — try
          again in a moment.
        </p>
      </div>
      <Button variant="outline" onClick={() => unstable_retry()}>
        <RotateCw className="h-4 w-4" aria-hidden strokeWidth={1.5} />
        Try again
      </Button>
    </div>
  );
}
