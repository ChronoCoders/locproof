import { Check } from "lucide-react";

import { sessionCookieHeader, fetchUsage } from "@/lib/auth";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

// Plan catalogue. `quota` mirrors api/src/plan.rs (free 100, starter 5k,
// growth 25k, enterprise unlimited) — keep in sync with the backend.
const PLANS = [
  {
    id: "free",
    name: "Free",
    price: "$0",
    cadence: "/mo",
    quota: "100 proofs / month",
    features: ["Up to 100 proofs/mo", "API key auth", "Community support"],
  },
  {
    id: "starter",
    name: "Starter",
    price: "$49",
    cadence: "/mo",
    quota: "5,000 proofs / month",
    features: ["Up to 5,000 proofs/mo", "Email support", "Usage analytics"],
  },
  {
    id: "growth",
    name: "Growth",
    price: "$199",
    cadence: "/mo",
    quota: "25,000 proofs / month",
    features: [
      "Up to 25,000 proofs/mo",
      "Priority support",
      "Multiple API keys",
    ],
  },
  {
    id: "enterprise",
    name: "Enterprise",
    price: "Custom",
    cadence: "",
    quota: "Unlimited proofs",
    features: ["Unlimited proofs", "SLA + SSO", "Dedicated support"],
  },
] as const;

export default async function BillingPage() {
  const cookie = await sessionCookieHeader();
  const usage = await fetchUsage(cookie);
  const current = usage.plan;

  return (
    <div className="mx-auto max-w-5xl space-y-6">
      <header className="space-y-1">
        <h1 className="text-2xl font-semibold tracking-tight">Billing</h1>
        <p className="text-sm text-muted-foreground">
          You&apos;re on the{" "}
          <span className="font-medium capitalize text-foreground">
            {current}
          </span>{" "}
          plan. Self-serve upgrades are coming soon.
        </p>
      </header>

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {PLANS.map((plan) => {
          const isCurrent = plan.id === current;
          const isEnterprise = plan.id === "enterprise";
          return (
            <Card
              key={plan.id}
              className={cn(
                "flex flex-col",
                isCurrent && "border-foreground/40 ring-1 ring-foreground/20",
              )}
            >
              <CardHeader className="space-y-1">
                <div className="flex items-center justify-between">
                  <CardTitle className="text-base">{plan.name}</CardTitle>
                  {isCurrent ? <Badge variant="secondary">Current</Badge> : null}
                </div>
                <CardDescription>
                  <span className="text-2xl font-semibold text-foreground">
                    {plan.price}
                  </span>
                  {plan.cadence}
                </CardDescription>
              </CardHeader>
              <CardContent className="flex-1">
                <ul className="space-y-2 text-sm">
                  {plan.features.map((f) => (
                    <li key={f} className="flex items-start gap-2">
                      <Check
                        aria-hidden
                        className="mt-0.5 h-4 w-4 shrink-0 text-emerald-400"
                        strokeWidth={2}
                      />
                      <span className="text-muted-foreground">{f}</span>
                    </li>
                  ))}
                </ul>
              </CardContent>
              <CardFooter>
                {/* Upgrade/checkout is wired in Phase 4c (Stripe). For now the
                    CTA is a disabled placeholder so the layout is final. */}
                <Button
                  variant={isCurrent ? "outline" : "default"}
                  className="w-full"
                  disabled
                >
                  {isCurrent
                    ? "Current plan"
                    : isEnterprise
                      ? "Contact sales"
                      : "Upgrade"}
                </Button>
              </CardFooter>
            </Card>
          );
        })}
      </div>

      <p className="text-xs text-muted-foreground">
        Plan changes and payment are handled manually for now — reach out and
        we&apos;ll move you over. Self-serve checkout lands in a later release.
      </p>
    </div>
  );
}
