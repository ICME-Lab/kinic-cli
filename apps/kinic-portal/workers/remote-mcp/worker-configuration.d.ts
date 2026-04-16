declare namespace Cloudflare {
  interface GlobalProps {
    mainModule: typeof import("./src/index");
  }

  interface Env {
    DFX_NETWORK: "mainnet";
    IC_HOST: "https://ic0.app";
    EMBEDDING_API_ENDPOINT: string;
  }
}

interface Env extends Cloudflare.Env {}

type StringifyValues<EnvType extends Record<string, unknown>> = {
  [Binding in keyof EnvType]: EnvType[Binding] extends string ? EnvType[Binding] : string;
};

declare namespace NodeJS {
  interface ProcessEnv
    extends StringifyValues<
      Pick<Cloudflare.Env, "DFX_NETWORK" | "IC_HOST" | "EMBEDDING_API_ENDPOINT">
    > {}
}
