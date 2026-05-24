"use client";

import { Bar, BarChart, CartesianGrid, XAxis, YAxis } from "recharts";

import type { UsageResponse } from "@/lib/api";
import { fmtMonth, isUnlimited } from "@/lib/format";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from "@/components/ui/chart";

const chartConfig = {
  count: { label: "Proofs", color: "var(--chart-1)" },
} satisfies ChartConfig;

export function UsageClient({ usage }: { usage: UsageResponse }) {
  const { current_month, history } = usage;
  const unlimited = isUnlimited(current_month.quota);
  const pct = unlimited
    ? 0
    : Math.min(100, Math.round((current_month.count / current_month.quota) * 100));

  // History is oldest-first and excludes the current month; append the
  // current month so the chart ends on "now".
  const data = [
    ...history.map((h) => ({ month: fmtMonth(h.month), count: h.count })),
    { month: fmtMonth(current_month.month), count: current_month.count },
  ];

  // `data` is never empty (the current month is always appended), so a true
  // "no usage" state is zero history *and* zero proofs this month.
  const noUsage = history.length === 0 && current_month.count === 0;

  const fmtNum = (n: number) => n.toLocaleString();

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="text-base font-medium">This month</CardTitle>
          <CardDescription>
            {fmtMonth(current_month.month, true)}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex items-baseline justify-between">
            <span className="text-3xl font-semibold tabular-nums">
              {fmtNum(current_month.count)}
            </span>
            <span className="text-sm text-muted-foreground">
              of{" "}
              {unlimited ? "Unlimited" : `${fmtNum(current_month.quota)} included`}
            </span>
          </div>
          {unlimited ? null : (
            <>
              <Progress value={pct} aria-label={`${pct}% of monthly quota used`} />
              <p className="text-xs text-muted-foreground">
                {pct}% of your monthly quota used
              </p>
            </>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-base font-medium">
            Proofs over time
          </CardTitle>
          <CardDescription>Last 12 months, including this one.</CardDescription>
        </CardHeader>
        <CardContent>
          {noUsage ? (
            <p className="py-8 text-center text-sm text-muted-foreground">
              No usage recorded yet.
            </p>
          ) : (
            <ChartContainer config={chartConfig} className="h-64 w-full">
              <BarChart accessibilityLayer data={data} margin={{ left: -16 }}>
                <CartesianGrid vertical={false} />
                <XAxis
                  dataKey="month"
                  tickLine={false}
                  axisLine={false}
                  tickMargin={8}
                />
                <YAxis
                  tickLine={false}
                  axisLine={false}
                  allowDecimals={false}
                  width={48}
                />
                <ChartTooltip content={<ChartTooltipContent />} />
                <Bar dataKey="count" fill="var(--color-count)" radius={4} />
              </BarChart>
            </ChartContainer>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
