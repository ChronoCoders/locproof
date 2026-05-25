"use client";

import { useRef, useState } from "react";
import { Loader2, FileSearch } from "lucide-react";

import {
  api,
  ApiError,
  type ListProofsResponse,
  type ProofSummary,
  type ProofDetail,
} from "@/lib/api";
import { fmtDateTime, scoreTone, scoreToneClass } from "@/lib/format";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

function ScoreBadge({ score }: { score: number }) {
  const tone = scoreTone(score);
  return (
    <Badge variant="outline" className={cn("tabular-nums", scoreToneClass(tone))}>
      {score.toFixed(2)}
    </Badge>
  );
}

export function ProofsClient({ initial }: { initial: ListProofsResponse }) {
  const [proofs, setProofs] = useState<ProofSummary[]>(initial.proofs);
  const [cursor, setCursor] = useState<string | null>(initial.next_cursor);
  const [loadingMore, setLoadingMore] = useState(false);
  const [listError, setListError] = useState<string | null>(null);

  // Detail modal state. `detailReq` tags each fetch so a slow earlier
  // response can't overwrite the modal after the user has clicked a
  // different row (stale-response race).
  const [selected, setSelected] = useState<ProofSummary | null>(null);
  const [detail, setDetail] = useState<ProofDetail | null>(null);
  const [detailError, setDetailError] = useState<string | null>(null);
  const detailReq = useRef(0);

  async function loadMore() {
    if (!cursor) return;
    setLoadingMore(true);
    setListError(null);
    try {
      const page = await api.get<ListProofsResponse>(
        `/api/dashboard/proofs?cursor=${encodeURIComponent(cursor)}`,
      );
      setProofs((prev) => [...prev, ...page.proofs]);
      setCursor(page.next_cursor);
    } catch (e) {
      setListError(
        e instanceof ApiError ? e.message : "Could not load more proofs.",
      );
    } finally {
      setLoadingMore(false);
    }
  }

  async function openDetail(p: ProofSummary) {
    const reqId = ++detailReq.current;
    setSelected(p);
    setDetail(null);
    setDetailError(null);
    try {
      const d = await api.get<ProofDetail>(
        `/api/dashboard/proofs/${encodeURIComponent(p.proof_id)}`,
      );
      if (detailReq.current === reqId) setDetail(d);
    } catch (e) {
      if (detailReq.current === reqId) {
        setDetailError(
          e instanceof ApiError ? e.message : "Could not load this proof.",
        );
      }
    }
  }

  if (proofs.length === 0) {
    return (
      <div className="flex flex-col items-center gap-3 rounded-lg border border-dashed py-16 text-center">
        <FileSearch
          aria-hidden
          className="h-8 w-8 text-muted-foreground"
          strokeWidth={1.5}
        />
        <p className="text-sm text-muted-foreground">
          No proofs yet. They&apos;ll appear here once your devices start
          submitting attestations.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {listError ? (
        <Alert variant="destructive">
          <AlertDescription>{listError}</AlertDescription>
        </Alert>
      ) : null}

      <div className="rounded-lg border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Proof ID</TableHead>
              <TableHead className="w-28">Score</TableHead>
              <TableHead className="text-right">Issued</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {proofs.map((p) => (
              <TableRow
                key={p.proof_id}
                role="button"
                tabIndex={0}
                onClick={() => openDetail(p)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    openDetail(p);
                  }
                }}
                className="cursor-pointer"
              >
                <TableCell className="font-mono text-xs">
                  {p.proof_id}
                </TableCell>
                <TableCell>
                  <ScoreBadge score={p.proximity_score} />
                </TableCell>
                <TableCell className="text-right text-sm text-muted-foreground">
                  {fmtDateTime(p.created_at)}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      {cursor ? (
        <div className="flex justify-center">
          <Button variant="outline" onClick={loadMore} disabled={loadingMore}>
            {loadingMore ? (
              <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
            ) : null}
            Load more
          </Button>
        </div>
      ) : null}

      <Dialog
        open={selected !== null}
        onOpenChange={(open) => {
          if (!open) {
            detailReq.current++;
            setSelected(null);
            setDetail(null);
            setDetailError(null);
          }
        }}
      >
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Proof detail</DialogTitle>
            <DialogDescription className="font-mono text-xs">
              {selected?.proof_id}
            </DialogDescription>
          </DialogHeader>
          {detailError ? (
            <Alert variant="destructive">
              <AlertDescription>{detailError}</AlertDescription>
            </Alert>
          ) : detail ? (
            <pre className="max-h-[60vh] overflow-auto rounded-md bg-muted p-4 text-xs leading-relaxed">
              {JSON.stringify(detail, null, 2)}
            </pre>
          ) : (
            <div className="flex items-center gap-2 py-8 text-sm text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
              Loading proof…
            </div>
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}
