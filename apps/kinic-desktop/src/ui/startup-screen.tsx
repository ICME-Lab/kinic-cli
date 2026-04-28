// Where: first-run session form.
// What: collects dfx identity and network before any canister command runs.
// Why: Keychain-backed identity access should be explicit.

import { Database } from "lucide-react";
import { Button, Card, FieldLabel, Input } from "./primitives";
import { useDesktopStore } from "@/store/useDesktopStore";

export function StartupScreen() {
  const identity = useDesktopStore((state) => state.identityInput);
  const network = useDesktopStore((state) => state.networkInput);
  const loading = useDesktopStore((state) => state.loading);
  const error = useDesktopStore((state) => state.error);
  const setIdentity = useDesktopStore((state) => state.setIdentityInput);
  const setNetwork = useDesktopStore((state) => state.setNetworkInput);
  const startSession = useDesktopStore((state) => state.startSession);

  return (
    <main className="flex min-h-screen items-center justify-center px-6">
      <Card className="w-full max-w-xl p-8">
        <div className="flex items-center gap-3">
          <div className="inline-flex size-11 items-center justify-center rounded-full border border-border bg-muted">
            <Database className="size-5 text-foreground" />
          </div>
          <div>
            <h1 className="text-2xl font-semibold tracking-[-0.02em]">Kinic Memory</h1>
            <p className="text-sm text-muted-foreground">Start a desktop session with a dfx identity.</p>
          </div>
        </div>

        <div className="mt-8 grid gap-5">
          <div className="grid gap-2">
            <FieldLabel htmlFor="identity">Identity</FieldLabel>
            <Input
              id="identity"
              value={identity}
              onChange={(event) => setIdentity(event.target.value)}
              placeholder="alice"
              autoFocus
            />
          </div>
          <div className="grid gap-2">
            <FieldLabel>Network</FieldLabel>
            <div className="grid grid-cols-2 gap-2 rounded-full border border-border bg-muted p-1">
              <button
                type="button"
                className={network === "local" ? activeSegment : inactiveSegment}
                onClick={() => setNetwork("local")}
              >
                Local
              </button>
              <button
                type="button"
                className={network === "mainnet" ? activeSegment : inactiveSegment}
                onClick={() => setNetwork("mainnet")}
              >
                Mainnet
              </button>
            </div>
          </div>
          {error ? <p className="rounded-2xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">{error}</p> : null}
          <Button disabled={loading || !identity.trim()} onClick={() => void startSession()}>
            {loading ? "Starting..." : "Start Session"}
          </Button>
        </div>
      </Card>
    </main>
  );
}

const activeSegment = "rounded-full bg-background px-4 py-2 text-sm font-medium text-foreground shadow-[0_1px_2px_rgba(0,0,0,0.04)]";
const inactiveSegment = "rounded-full px-4 py-2 text-sm font-medium text-muted-foreground hover:text-foreground";
