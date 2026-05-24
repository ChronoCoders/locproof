import { sessionCookieHeader, fetchUsage } from "@/lib/auth";
import { UsageClient } from "./usage-client";

// Server component: fetches usage with the forwarded session cookie (the
// (dashboard) layout has already guarded the session). fetchUsage is
// request-memoised, so this reuses the layout's round-trip. The chart itself
// is client-side (Recharts), so the data is handed down to UsageClient.
export default async function UsagePage() {
  const cookie = await sessionCookieHeader();
  const usage = await fetchUsage(cookie);

  return (
    <div className="mx-auto max-w-4xl space-y-6">
      <header className="space-y-1">
        <h1 className="text-2xl font-semibold tracking-tight">Usage</h1>
        <p className="text-sm text-muted-foreground">
          Proofs issued this month and over the past year.
        </p>
      </header>
      <UsageClient usage={usage} />
    </div>
  );
}
