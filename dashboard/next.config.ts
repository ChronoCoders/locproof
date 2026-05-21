import type { NextConfig } from "next";

// Same-domain proxy: the browser talks to the Next dev server (or the prod
// reverse proxy) which forwards /api/* to the Rust API. Dev defaults to the
// local backend; prod sets NEXT_PUBLIC_API_BASE to the public origin.
const apiBase = process.env.NEXT_PUBLIC_API_BASE ?? "http://localhost:3000";

const nextConfig: NextConfig = {
  async rewrites() {
    return [{ source: "/api/:path*", destination: `${apiBase}/:path*` }];
  },
};

export default nextConfig;
