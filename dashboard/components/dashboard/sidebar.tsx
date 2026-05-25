"use client";

import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { useState } from "react";
import {
  FileCheck2,
  BarChart3,
  CreditCard,
  KeyRound,
  LogOut,
  Loader2,
  Shield,
} from "lucide-react";

import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";

const NAV = [
  { href: "/proofs", label: "Proofs", icon: FileCheck2 },
  { href: "/usage", label: "Usage", icon: BarChart3 },
  { href: "/billing", label: "Billing", icon: CreditCard },
  { href: "/settings/api-keys", label: "API keys", icon: KeyRound },
] as const;

export function Sidebar({ plan }: { plan: string }) {
  const pathname = usePathname();
  const router = useRouter();
  const [signingOut, setSigningOut] = useState(false);

  async function signOut() {
    setSigningOut(true);
    try {
      await api.post("/api/auth/logout");
    } catch {
      // Even if the call fails, drop the user back to /login — the session
      // cookie is httpOnly so we can't clear it here, but the backend
      // invalidates on logout and an invalid cookie just redirects anyway.
    } finally {
      router.push("/login");
      router.refresh();
    }
  }

  return (
    <aside className="flex w-60 shrink-0 flex-col border-r bg-card/40">
      <Link
        href="/proofs"
        className="flex items-center gap-2 px-6 py-5 text-foreground"
      >
        <Shield aria-hidden className="h-5 w-5" strokeWidth={1.5} />
        <span className="font-semibold tracking-tight">LocProof</span>
      </Link>

      <nav className="flex-1 space-y-1 px-3">
        {NAV.map(({ href, label, icon: Icon }) => {
          const active = pathname === href || pathname.startsWith(`${href}/`);
          return (
            <Link
              key={href}
              href={href}
              aria-current={active ? "page" : undefined}
              className={cn(
                "flex items-center gap-3 rounded-md px-3 py-2 text-sm transition-colors",
                active
                  ? "bg-accent text-accent-foreground"
                  : "text-muted-foreground hover:bg-accent/50 hover:text-foreground",
              )}
            >
              <Icon aria-hidden className="h-4 w-4" strokeWidth={1.5} />
              {label}
            </Link>
          );
        })}
      </nav>

      <div className="space-y-3 border-t p-3">
        <div className="px-3 text-xs text-muted-foreground">
          Plan
          <span className="ml-1 font-medium capitalize text-foreground">
            {plan}
          </span>
        </div>
        <Button
          variant="ghost"
          size="sm"
          className="w-full justify-start text-muted-foreground"
          onClick={signOut}
          disabled={signingOut}
        >
          {signingOut ? (
            <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
          ) : (
            <LogOut className="h-4 w-4" aria-hidden strokeWidth={1.5} />
          )}
          Sign out
        </Button>
      </div>
    </aside>
  );
}
