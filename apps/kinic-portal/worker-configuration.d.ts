declare namespace Cloudflare {
  interface KVNamespace {
    get(key: string, type: "json"): Promise<unknown>;
    put(key: string, value: string, options: { expirationTtl: number }): Promise<void>;
  }

  interface Env {
    ASSETS: Fetcher;
    IC_HOST?: string;
    EMBEDDING_API_ENDPOINT: string;
    SUMMARY_CACHE?: KVNamespace;
    SUMMARY_CACHE_TTL_SECONDS?: string;
    KINIC_PORTAL_ORIGIN: string;
    KINIC_PUBLIC_API_ORIGIN: string;
    KINIC_REMOTE_MCP_ORIGIN?: string;
  }
}

interface Env extends Cloudflare.Env {}

type StringifyValues<EnvType extends Record<string, unknown>> = {
  [Binding in keyof EnvType]: EnvType[Binding] extends string ? EnvType[Binding] : string;
};

declare namespace NodeJS {
  interface ProcessEnv
    extends StringifyValues<
      Pick<Cloudflare.Env, "KINIC_PORTAL_ORIGIN" | "KINIC_PUBLIC_API_ORIGIN" | "EMBEDDING_API_ENDPOINT" | "SUMMARY_CACHE_TTL_SECONDS">
    > {}
}
