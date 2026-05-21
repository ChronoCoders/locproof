import Link from "next/link";
import { Shield } from "lucide-react";
import { Button } from "@/components/ui/button";

export default function Home() {
  return (
    <main className="flex flex-1 items-center justify-center px-6">
      <div className="flex flex-col items-center gap-8 text-center max-w-xl">
        <Shield
          aria-hidden
          className="h-14 w-14 text-primary"
          strokeWidth={1.5}
        />
        <div className="space-y-3">
          <h1 className="text-4xl font-semibold tracking-tight">LocProof</h1>
          <p className="text-lg text-muted-foreground leading-relaxed">
            Tamper-proof digital witness. Cryptographic evidence that two
            parties were physically present at the same location.
          </p>
        </div>
        <div className="flex gap-3">
          <Button asChild>
            <Link href="/login">Sign in</Link>
          </Button>
          <Button asChild variant="outline">
            <Link href="/register">Create account</Link>
          </Button>
        </div>
      </div>
    </main>
  );
}
