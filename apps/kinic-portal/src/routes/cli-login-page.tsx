// Where: browser route used by icp-cli's Internet Identity link flow.
// What: receives CLI session details, asks II for a delegation, and posts it to localhost.
// Why: terminal auth must derive from memory.kinic.xyz so CLI and portal principals match.

import { useMemo, useState } from "react";
import { AuthClient } from "@dfinity/auth-client";
import { DelegationChain, DelegationIdentity } from "@dfinity/identity";
import type { DerEncodedPublicKey } from "@dfinity/agent";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";

const IDENTITY_PROVIDER = "https://id.ai";
const DELEGATION_EXPIRATION_MS = 8 * 60 * 60 * 1000;
const NANOS_PER_MILLISECOND = 1_000_000n;

type CliLoginParams = {
  publicKey: string;
  callback: string;
};

export type CliLoginParseResult =
  | { kind: "empty" }
  | { kind: "ok"; params: CliLoginParams }
  | { kind: "error"; message: string };

type LoginState = "ready" | "signing-in" | "sending" | "finished" | "error";

export function CliLoginPage() {
  const loginRequest = useMemo(() => parseCliLoginHash(readHash()), []);
  const [state, setState] = useState<LoginState>("ready");
  const [error, setError] = useState<string | null>(null);

  async function signIn() {
    if (loginRequest.kind !== "ok") {
      return;
    }
    setError(null);
    setState("signing-in");
    try {
      const authClient = await AuthClient.create({ keyType: "Ed25519" });
      await login(authClient);
      setState("sending");
      const { params } = loginRequest;
      const delegation = await createCliDelegation(authClient, params.publicKey);
      const response = await fetch(params.callback, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(delegation),
        redirect: "error",
      });
      if (!response.ok) {
        throw new Error(`Callback failed: ${response.status} ${response.statusText}`);
      }
      await authClient.logout();
      setState("finished");
      window.setTimeout(() => window.close(), 2000);
    } catch (cause) {
      setState("error");
      setError(cause instanceof Error ? cause.message : "Sign-in failed");
    }
  }

  if (loginRequest.kind === "empty") {
    return (
      <main className="mx-auto flex min-h-screen w-full max-w-3xl flex-col px-5 pb-20 pt-6 md:px-6 md:pb-24">
        <Card>
          <CardHeader>
            <CardTitle>Start from your terminal.</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4 text-sm leading-7 text-muted-foreground">
            <p>Run the command below and icp-cli will reopen this page with the required session parameters.</p>
            <pre className="overflow-x-auto rounded-2xl border border-border bg-muted px-4 py-3 text-foreground">
              <code>icp identity link ii &lt;identity-name&gt; --host https://memory.kinic.xyz</code>
            </pre>
          </CardContent>
        </Card>
      </main>
    );
  }

  if (loginRequest.kind === "error") {
    return (
      <main className="mx-auto flex min-h-screen w-full max-w-3xl flex-col px-5 pb-20 pt-6 md:px-6 md:pb-24">
        <Card>
          <CardHeader>
            <CardTitle>Terminal login link is invalid.</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4 text-sm leading-7 text-muted-foreground">
            <Alert variant="destructive">
              <AlertTitle>Cannot authorize terminal</AlertTitle>
              <AlertDescription>{loginRequest.message}</AlertDescription>
            </Alert>
            <pre className="overflow-x-auto rounded-2xl border border-border bg-muted px-4 py-3 text-foreground">
              <code>icp identity link ii &lt;identity-name&gt; --host https://memory.kinic.xyz</code>
            </pre>
          </CardContent>
        </Card>
      </main>
    );
  }

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-3xl flex-col px-5 pb-20 pt-6 md:px-6 md:pb-24">
      <Card>
        <CardHeader>
          <CardTitle>Authorize your terminal.</CardTitle>
        </CardHeader>
        <CardContent className="space-y-5 text-sm leading-7 text-muted-foreground">
          <p>Sign in with Internet Identity. Kinic will return a short-lived delegation to icp-cli on localhost.</p>
          {state === "ready" ? (
            <Alert>
              <AlertTitle>Local callback required</AlertTitle>
              <AlertDescription>Approve the browser request to connect to the local address opened by icp-cli.</AlertDescription>
            </Alert>
          ) : null}
          {state === "signing-in" ? <p className="text-foreground">Waiting for Internet Identity...</p> : null}
          {state === "sending" ? <p className="text-foreground">Sending delegation to your terminal...</p> : null}
          {state === "finished" ? <p className="text-foreground">Done. Return to your terminal.</p> : null}
          {state === "error" && error ? (
            <Alert variant="destructive">
              <AlertTitle>Sign-in failed</AlertTitle>
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          ) : null}
          <Button type="button" onClick={signIn} disabled={state === "signing-in" || state === "sending" || state === "finished"}>
            {state === "error" ? "Try again" : "Sign in with Internet Identity"}
          </Button>
        </CardContent>
      </Card>
    </main>
  );
}

export function parseCliLoginHash(hash: string): CliLoginParseResult {
  const normalized = hash.startsWith("#") ? hash.slice(1) : hash;
  if (!normalized) {
    return { kind: "empty" };
  }
  const params = new URLSearchParams(normalized);
  const publicKey = params.get("public_key");
  const callback = params.get("callback");
  if (!publicKey) {
    return { kind: "error", message: "Missing public_key in the terminal login URL." };
  }
  if (!callback) {
    return { kind: "error", message: "Missing callback in the terminal login URL." };
  }
  if (!isAllowedLocalCallback(callback)) {
    return { kind: "error", message: "The callback must use http://127.0.0.1 or http://[::1]." };
  }
  return { kind: "ok", params: { publicKey, callback } };
}

export function isAllowedLocalCallback(value: string): boolean {
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    return false;
  }
  return url.protocol === "http:"
    && !url.username
    && !url.password
    && (url.hostname === "127.0.0.1" || url.hostname === "::1" || url.hostname === "[::1]");
}

async function login(authClient: AuthClient): Promise<void> {
  return new Promise((resolve, reject) => {
    authClient.login({
      identityProvider: IDENTITY_PROVIDER,
      maxTimeToLive: BigInt(DELEGATION_EXPIRATION_MS) * NANOS_PER_MILLISECOND,
      onSuccess: resolve,
      onError: reject,
    });
  });
}

async function createCliDelegation(authClient: AuthClient, publicKey: string) {
  const identity = authClient.getIdentity();
  if (!(identity instanceof DelegationIdentity)) {
    throw new Error("Expected a delegated Internet Identity session.");
  }
  const key = decodeBase64Url(publicKey);
  const sessionPublicKey = key.buffer.slice(key.byteOffset, key.byteOffset + key.byteLength) as DerEncodedPublicKey;
  const delegation = await DelegationChain.create(
    identity,
    { toDer: () => sessionPublicKey },
    new Date(Date.now() + DELEGATION_EXPIRATION_MS),
    { previous: identity.getDelegation() },
  );
  return delegation.toJSON();
}

function decodeBase64Url(value: string): Uint8Array {
  const base64 = value.replaceAll("-", "+").replaceAll("_", "/");
  const padded = base64.padEnd(base64.length + ((4 - (base64.length % 4)) % 4), "=");
  return Uint8Array.from(globalThis.atob(padded), (char) => char.charCodeAt(0));
}

function readHash(): string {
  if (typeof window === "undefined") {
    return "";
  }
  const hash = window.location.hash;
  if (hash) {
    window.history.replaceState(null, "", window.location.pathname);
  }
  return hash;
}
