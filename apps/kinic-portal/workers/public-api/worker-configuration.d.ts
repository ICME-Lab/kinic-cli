declare namespace Cloudflare {
  interface GlobalProps {
    mainModule: typeof import("./src/index");
  }

  interface KVNamespace {
    get(key: string, type: "json"): Promise<unknown>;
    put(key: string, value: string, options: { expirationTtl: number }): Promise<void>;
  }

  interface Env {
    IC_HOST: "https://ic0.app";
    EMBEDDING_API_ENDPOINT: string;
    SUMMARY_CACHE?: KVNamespace;
    SUMMARY_CACHE_TTL_SECONDS?: string;
  }
}

interface Env extends Cloudflare.Env {}

type StringifyValues<EnvType extends Record<string, unknown>> = {
  [Binding in keyof EnvType]: EnvType[Binding] extends string ? EnvType[Binding] : string;
};

declare namespace NodeJS {
  interface ProcessEnv
    extends StringifyValues<
      Pick<Cloudflare.Env, "IC_HOST" | "EMBEDDING_API_ENDPOINT" | "SUMMARY_CACHE_TTL_SECONDS">
    > {}
}
