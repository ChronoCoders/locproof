import { requireSession } from "@/lib/auth";
import { Sidebar } from "@/components/dashboard/sidebar";

// Server-side guard: requireSession() probes the backend and redirects to
// /login when there's no valid session, so every (dashboard) route is gated
// before it renders. The usage payload doubles as the plan badge source.
export default async function DashboardLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  const usage = await requireSession();

  return (
    <div className="flex min-h-svh">
      <Sidebar plan={usage.plan} />
      <main className="flex-1 overflow-x-hidden px-8 py-8">{children}</main>
    </div>
  );
}
