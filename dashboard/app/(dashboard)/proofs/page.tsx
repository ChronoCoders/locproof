import { api, type ListProofsResponse } from "@/lib/api";
import { sessionCookieHeader, apiBase } from "@/lib/auth";
import { ProofsClient } from "./proofs-client";

// Server component: fetches the first page of proofs with the forwarded
// session cookie (the (dashboard) layout has already guarded the session).
// Pagination and the detail modal are handled client-side from here.
export default async function ProofsPage() {
  const cookie = await sessionCookieHeader();
  const initial = await api.get<ListProofsResponse>(
    `${apiBase()}/dashboard/proofs`,
    cookie ? { cookie } : undefined,
  );

  return (
    <div className="mx-auto max-w-4xl space-y-6">
      <header className="space-y-1">
        <h1 className="text-2xl font-semibold tracking-tight">Proofs</h1>
        <p className="text-sm text-muted-foreground">
          Proximity proofs issued for your account, newest first.
        </p>
      </header>
      <ProofsClient initial={initial} />
    </div>
  );
}
