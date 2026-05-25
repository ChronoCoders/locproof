// Typed fetch wrapper. All calls hit the Next /api/* rewrite, which proxies
// to the Rust API. Same-origin so the lp_session cookie flows automatically
// (and SameSite=Strict is satisfied).

export const SESSION_COOKIE = "lp_session";

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

type Options = {
  /** When set, prepend the cookie header manually (server-side fetches that
   * don't get the cookie injected by the browser). */
  cookie?: string;
};

async function request<R>(
  method: string,
  path: string,
  body?: unknown,
  opts?: Options,
): Promise<R> {
  const headers: Record<string, string> = {};
  if (body !== undefined) headers["Content-Type"] = "application/json";
  if (opts?.cookie) headers["Cookie"] = opts.cookie;

  const res = await fetch(path, {
    method,
    headers,
    body: body !== undefined ? JSON.stringify(body) : undefined,
    credentials: "include",
    cache: "no-store",
  });

  if (!res.ok) {
    const data = (await res.json().catch(() => null)) as
      | { error?: string }
      | null;
    throw new ApiError(res.status, data?.error ?? res.statusText);
  }
  if (res.status === 204) return undefined as R;
  return (await res.json()) as R;
}

export const api = {
  get: <R>(path: string, opts?: Options) => request<R>("GET", path, undefined, opts),
  post: <R>(path: string, body?: unknown, opts?: Options) =>
    request<R>("POST", path, body, opts),
  del: <R = void>(path: string, opts?: Options) =>
    request<R>("DELETE", path, undefined, opts),
};

// Shared response shapes mirroring the Rust API. Keep in sync with
// api/src/routes/*.rs.

export type RegisterResponse = {
  user_id: string;
  customer_id: string;
  email: string;
  session_expires_at: string;
};

export type LoginResponse = {
  user_id: string;
  customer_id: string;
  session_expires_at: string;
};

export type ProofSummary = {
  proof_id: string;
  proximity_score: number;
  created_at: string;
};

export type ListProofsResponse = {
  proofs: ProofSummary[];
  next_cursor: string | null;
};

export type UsageResponse = {
  plan: string;
  current_month: { month: string; count: number; quota: number };
  history: { month: string; count: number }[];
};

export type KeySummary = {
  id: string;
  name: string;
  created_at: string;
  last_used_at: string | null;
  is_active: boolean;
};

export type CreateKeyResponse = {
  id: string;
  name: string;
  api_key: string;
  created_at: string;
};

// Full stored proof, returned by GET /dashboard/proofs/:id. The nested device
// attestations carry raw byte arrays (Ed25519 keys/signatures) plus the signal
// snapshot, so the shape is deep and partly opaque — the UI just pretty-prints
// it as JSON rather than rendering every field, hence the permissive typing.
export type ProofDetail = {
  id: string;
  timestamp: number;
  proximity_score: number;
} & Record<string, unknown>;
