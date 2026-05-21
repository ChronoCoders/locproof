import { Shield } from "lucide-react";
import Link from "next/link";

export default function AuthLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <main className="flex flex-1 items-center justify-center px-6 py-12">
      <div className="w-full max-w-sm space-y-8">
        <Link
          href="/"
          className="flex items-center justify-center gap-2 text-foreground"
        >
          <Shield aria-hidden className="h-6 w-6" strokeWidth={1.5} />
          <span className="text-lg font-semibold tracking-tight">LocProof</span>
        </Link>
        {children}
      </div>
    </main>
  );
}
