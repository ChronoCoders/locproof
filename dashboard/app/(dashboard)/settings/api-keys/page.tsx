import { api, type KeySummary } from "@/lib/api";
import { sessionCookieHeader, apiBase } from "@/lib/auth";
import { ApiKeysClient } from "./api-keys-client";

// Server component: fetches the customer's API keys with the forwarded
// session cookie (the (dashboard) layout has already guarded the session).
// Create/deactivate happen client-side from here.
export default async function ApiKeysPage() {
  const cookie = await sessionCookieHeader();
  const keys = await api.get<KeySummary[]>(
    `${apiBase()}/dashboard/keys`,
    cookie ? { cookie } : undefined,
  );

  return (
    <div className="mx-auto max-w-3xl space-y-6">
      <header className="space-y-1">
        <h1 className="text-2xl font-semibold tracking-tight">API keys</h1>
        <p className="text-sm text-muted-foreground">
          Keys authenticate requests to the proximity API. Treat them like
          passwords — they&apos;re shown in full only once, at creation.
        </p>
      </header>
      <ApiKeysClient initial={keys} />
    </div>
  );
}
