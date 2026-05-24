// Server-side session helpers. Used by the (dashboard) layout to gate
// authenticated routes — the real check lives in the Rust API, this just
// surfaces the cookie value and a yes/no for callers.

import { cache } from "react";
import { cookies } from "next/headers";
import { redirect } from "next/navigation";
import { api, ApiError, SESSION_COOKIE, type UsageResponse } from "./api";

/// Fetch the session's usage payload, memoised per request via React
/// `cache()`. The (dashboard) layout probes it for the session guard and the
/// /usage and /billing pages need it too; keying on the forwarded cookie
/// string means all of them share a single backend round-trip per render.
export const fetchUsage = cache(
  (cookie: string | null): Promise<UsageResponse> =>
    api.get<UsageResponse>(
      `${apiBase()}/dashboard/usage`,
      cookie ? { cookie } : undefined,
    ),
);

/// Return the raw session cookie value, or null if the user has none.
export async function getSessionCookieValue(): Promise<string | null> {
  const c = await cookies();
  return c.get(SESSION_COOKIE)?.value ?? null;
}

/// Build a `Cookie:` header line that forwards the session to the backend
/// from a server component (the browser does this automatically for
/// browser-issued fetches, but server-side fetches need it explicit).
export async function sessionCookieHeader(): Promise<string | null> {
  const v = await getSessionCookieValue();
  return v ? `${SESSION_COOKIE}=${v}` : null;
}

/// Probe the backend for a valid session by hitting a session-authed
/// endpoint. Returns the usage payload on success (handy because the
/// dashboard shell shows the plan), or null on 401.
///
/// Non-401 errors (backend 500, network, DB outage) are intentionally
/// re-thrown so they surface in Next's error boundary. The alternative —
/// falling through to /login — would mask a real outage as an auth
/// failure and send users into a confusing redirect loop.
export async function probeSession(): Promise<UsageResponse | null> {
  const cookie = await sessionCookieHeader();
  if (!cookie) return null;
  try {
    return await fetchUsage(cookie);
  } catch (e) {
    if (e instanceof ApiError && e.status === 401) return null;
    throw e;
  }
}

/// Server-component guard: returns the usage payload, or redirects to
/// /login if the session is missing or invalid.
export async function requireSession(): Promise<UsageResponse> {
  const u = await probeSession();
  if (!u) redirect("/login");
  return u;
}

/// Absolute URL to the Rust API from the Next.js server. Server-side
/// fetches can't use the /api/* rewrite (no proxy in the server runtime),
/// so they hit the backend directly via the same env var the rewrite uses.
export function apiBase(): string {
  return process.env.NEXT_PUBLIC_API_BASE ?? "http://localhost:3000";
}
