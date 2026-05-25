"use client";

import { useMemo, useState } from "react";
import { Loader2, Plus, Copy, Check, KeyRound } from "lucide-react";

import {
  api,
  ApiError,
  type KeySummary,
  type CreateKeyResponse,
} from "@/lib/api";
import { fmtDateTime } from "@/lib/format";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
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
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

export function ApiKeysClient({ initial }: { initial: KeySummary[] }) {
  const [keys, setKeys] = useState<KeySummary[]>(initial);
  const [hideInactive, setHideInactive] = useState(true);

  // Create flow.
  const [createOpen, setCreateOpen] = useState(false);
  const [name, setName] = useState("");
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);
  // The one-time plaintext reveal after a successful create.
  const [revealed, setRevealed] = useState<CreateKeyResponse | null>(null);
  const [copied, setCopied] = useState(false);

  // Deactivate flow.
  const [target, setTarget] = useState<KeySummary | null>(null);
  const [deactivating, setDeactivating] = useState(false);
  const [deactivateError, setDeactivateError] = useState<string | null>(null);

  const visible = useMemo(
    () => (hideInactive ? keys.filter((k) => k.is_active) : keys),
    [keys, hideInactive],
  );
  const hiddenCount = keys.length - keys.filter((k) => k.is_active).length;

  async function createKey() {
    const trimmed = name.trim();
    if (!trimmed) {
      setCreateError("Give the key a name.");
      return;
    }
    setCreating(true);
    setCreateError(null);
    try {
      const created = await api.post<CreateKeyResponse>("/api/dashboard/keys", {
        name: trimmed,
      });
      setKeys((prev) => [
        {
          id: created.id,
          name: created.name,
          created_at: created.created_at,
          last_used_at: null,
          is_active: true,
        },
        ...prev,
      ]);
      setCreateOpen(false);
      setName("");
      setCopied(false);
      setRevealed(created);
    } catch (e) {
      setCreateError(
        e instanceof ApiError ? e.message : "Could not create the key.",
      );
    } finally {
      setCreating(false);
    }
  }

  async function copyKey() {
    if (!revealed) return;
    try {
      await navigator.clipboard.writeText(revealed.api_key);
      setCopied(true);
    } catch {
      // Clipboard can be blocked (no permission / insecure context). The key
      // is visible for manual copy, so just leave the button state unchanged.
    }
  }

  async function deactivateKey() {
    if (!target) return;
    setDeactivating(true);
    setDeactivateError(null);
    try {
      await api.del(`/api/dashboard/keys/${encodeURIComponent(target.id)}`);
      setKeys((prev) =>
        prev.map((k) =>
          k.id === target.id ? { ...k, is_active: false } : k,
        ),
      );
      setTarget(null);
    } catch (e) {
      setDeactivateError(
        e instanceof ApiError ? e.message : "Could not deactivate the key.",
      );
    } finally {
      setDeactivating(false);
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-4">
        <label className="flex items-center gap-2 text-sm text-muted-foreground">
          <Switch
            checked={hideInactive}
            onCheckedChange={setHideInactive}
            aria-label="Hide inactive keys"
          />
          Hide inactive
          {hiddenCount > 0 ? (
            <span className="text-xs">({hiddenCount})</span>
          ) : null}
        </label>
        <Button onClick={() => setCreateOpen(true)}>
          <Plus className="h-4 w-4" aria-hidden strokeWidth={2} />
          Create key
        </Button>
      </div>

      {visible.length === 0 ? (
        <div className="flex flex-col items-center gap-3 rounded-lg border border-dashed py-16 text-center">
          <KeyRound
            aria-hidden
            className="h-8 w-8 text-muted-foreground"
            strokeWidth={1.5}
          />
          <p className="text-sm text-muted-foreground">
            {keys.length === 0
              ? "No API keys yet. Create one to start authenticating requests."
              : "No active keys. Toggle off “Hide inactive” to see deactivated keys."}
          </p>
        </div>
      ) : (
        <div className="rounded-lg border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Created</TableHead>
                <TableHead>Last used</TableHead>
                <TableHead className="w-px" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {visible.map((k) => (
                <TableRow key={k.id}>
                  <TableCell className="font-medium">{k.name}</TableCell>
                  <TableCell>
                    {k.is_active ? (
                      <Badge
                        variant="outline"
                        className="border-emerald-500/30 bg-emerald-500/15 text-emerald-400"
                      >
                        Active
                      </Badge>
                    ) : (
                      <Badge variant="outline" className="text-muted-foreground">
                        Inactive
                      </Badge>
                    )}
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {fmtDateTime(k.created_at)}
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {k.last_used_at ? fmtDateTime(k.last_used_at) : "Never"}
                  </TableCell>
                  <TableCell className="text-right">
                    {k.is_active ? (
                      <Button
                        variant="ghost"
                        size="sm"
                        className="text-destructive hover:text-destructive"
                        onClick={() => {
                          setDeactivateError(null);
                          setTarget(k);
                        }}
                      >
                        Deactivate
                      </Button>
                    ) : null}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}

      {/* Create-key dialog. */}
      <Dialog
        open={createOpen}
        onOpenChange={(open) => {
          setCreateOpen(open);
          if (!open) {
            setName("");
            setCreateError(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create API key</DialogTitle>
            <DialogDescription>
              Name it so you can recognise it later (e.g. “production”,
              “staging”).
            </DialogDescription>
          </DialogHeader>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void createKey();
            }}
            className="space-y-4"
          >
            {createError ? (
              <Alert variant="destructive">
                <AlertDescription>{createError}</AlertDescription>
              </Alert>
            ) : null}
            <div className="space-y-2">
              <Label htmlFor="key-name">Name</Label>
              <Input
                id="key-name"
                value={name}
                autoFocus
                maxLength={120}
                onChange={(e) => setName(e.target.value)}
                placeholder="production"
              />
            </div>
            <DialogFooter>
              <DialogClose asChild>
                <Button type="button" variant="outline">
                  Cancel
                </Button>
              </DialogClose>
              <Button type="submit" disabled={creating}>
                {creating ? (
                  <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
                ) : null}
                Create key
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* One-time plaintext reveal. */}
      <Dialog
        open={revealed !== null}
        onOpenChange={(open) => {
          if (!open) {
            setRevealed(null);
            setCopied(false);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Copy your API key</DialogTitle>
            <DialogDescription>
              This is the only time the full key is shown. Store it securely —
              you won&apos;t be able to see it again.
            </DialogDescription>
          </DialogHeader>
          <div className="flex items-center gap-2">
            <code className="flex-1 overflow-x-auto rounded-md bg-muted px-3 py-2 font-mono text-xs">
              {revealed?.api_key}
            </code>
            <Button
              type="button"
              variant="outline"
              size="icon"
              onClick={copyKey}
              aria-label="Copy API key"
            >
              {copied ? (
                <Check className="h-4 w-4 text-emerald-400" aria-hidden />
              ) : (
                <Copy className="h-4 w-4" aria-hidden />
              )}
            </Button>
          </div>
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button">Done</Button>
            </DialogClose>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Deactivate confirmation. */}
      <Dialog
        open={target !== null}
        onOpenChange={(open) => {
          if (!open && !deactivating) {
            setTarget(null);
            setDeactivateError(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Deactivate “{target?.name}”?</DialogTitle>
            <DialogDescription>
              Requests using this key will stop working immediately. This can&apos;t
              be undone — you&apos;ll need to create a new key.
            </DialogDescription>
          </DialogHeader>
          {deactivateError ? (
            <Alert variant="destructive">
              <AlertDescription>{deactivateError}</AlertDescription>
            </Alert>
          ) : null}
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              disabled={deactivating}
              onClick={() => {
                setTarget(null);
                setDeactivateError(null);
              }}
            >
              Cancel
            </Button>
            <Button
              type="button"
              variant="destructive"
              disabled={deactivating}
              onClick={() => void deactivateKey()}
            >
              {deactivating ? (
                <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
              ) : null}
              Deactivate
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
