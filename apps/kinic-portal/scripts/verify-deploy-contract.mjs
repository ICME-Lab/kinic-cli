// Where: repo-local deploy guard for the portal and public-api Workers.
// What: checks the small set of config values that must stay aligned across Workers.
// Why: deploy drift should fail before Wrangler/network steps, not during production rollout.

import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const PORTAL_ROOT = path.resolve(SCRIPT_DIR, "..");
const PORTAL_WRANGLER_PATH = path.join(PORTAL_ROOT, "wrangler.jsonc");
const PUBLIC_API_WRANGLER_PATH = path.join(PORTAL_ROOT, "workers/public-api/wrangler.jsonc");
const REMOTE_MCP_WRANGLER_PATH = path.join(PORTAL_ROOT, "workers/remote-mcp/wrangler.jsonc");
const REQUIRED_SECRET = "EMBEDDING_API_ENDPOINT";
const SUMMARY_CACHE_BINDING = "SUMMARY_CACHE";

export function loadWranglerConfig(configPath) {
  const raw = readFileSync(configPath, "utf8");
  return JSON.parse(stripJsonComments(raw));
}

export function verifyDeployContract(
  portalConfig,
  publicApiConfig,
  remoteMcpConfig = null,
  options = {},
) {
  const errors = [];
  const portalSummaryCache = findKvBinding(portalConfig, SUMMARY_CACHE_BINDING);
  const publicApiSummaryCache = findKvBinding(publicApiConfig, SUMMARY_CACHE_BINDING);
  const portalRequiredSecrets = listRequiredSecrets(portalConfig);
  const publicApiRequiredSecrets = listRequiredSecrets(publicApiConfig);
  const remoteMcpRequiredSecrets = remoteMcpConfig ? listRequiredSecrets(remoteMcpConfig) : [];
  const portalVars = portalConfig.vars ?? {};
  const publicApiVars = publicApiConfig.vars ?? {};

  if (!portalConfig.name) {
    errors.push(`portal wrangler config is missing "name": ${PORTAL_WRANGLER_PATH}`);
  }
  if (!publicApiConfig.name) {
    errors.push(`public-api wrangler config is missing "name": ${PUBLIC_API_WRANGLER_PATH}`);
  }
  if (remoteMcpConfig && !remoteMcpConfig.name) {
    errors.push(`remote-mcp wrangler config is missing "name": ${REMOTE_MCP_WRANGLER_PATH}`);
  }

  if (!portalSummaryCache) {
    errors.push(`portal is missing kv binding "${SUMMARY_CACHE_BINDING}"`);
  }
  if (!publicApiSummaryCache) {
    errors.push(`public-api is missing kv binding "${SUMMARY_CACHE_BINDING}"`);
  }

  if (portalSummaryCache && publicApiSummaryCache) {
    compareField(errors, "SUMMARY_CACHE.id", portalSummaryCache.id, publicApiSummaryCache.id);
    compareField(errors, "SUMMARY_CACHE.preview_id", portalSummaryCache.preview_id, publicApiSummaryCache.preview_id);
  }

  compareField(
    errors,
    "SUMMARY_CACHE_TTL_SECONDS",
    portalVars.SUMMARY_CACHE_TTL_SECONDS,
    publicApiVars.SUMMARY_CACHE_TTL_SECONDS,
  );

  if (!portalRequiredSecrets.includes(REQUIRED_SECRET)) {
    errors.push(`portal secrets.required must include "${REQUIRED_SECRET}"`);
  }
  if (!publicApiRequiredSecrets.includes(REQUIRED_SECRET)) {
    errors.push(`public-api secrets.required must include "${REQUIRED_SECRET}"`);
  }
  if (remoteMcpConfig && !remoteMcpRequiredSecrets.includes(REQUIRED_SECRET)) {
    errors.push(`remote-mcp secrets.required must include "${REQUIRED_SECRET}"`);
  }

  validateAbsoluteOrigin(errors, "KINIC_PORTAL_ORIGIN", portalVars.KINIC_PORTAL_ORIGIN);
  validateAbsoluteOrigin(errors, "KINIC_PUBLIC_API_ORIGIN", portalVars.KINIC_PUBLIC_API_ORIGIN);
  validateAbsoluteOrigin(errors, "KINIC_REMOTE_MCP_ORIGIN", portalVars.KINIC_REMOTE_MCP_ORIGIN);

  return {
    ok: errors.length === 0,
    errors,
  };
}

export function formatContractErrors(errors) {
  return errors.map((error) => `- ${error}`).join("\n");
}

function findKvBinding(config, bindingName) {
  const namespaces = Array.isArray(config.kv_namespaces) ? config.kv_namespaces : [];
  return namespaces.find((entry) => entry?.binding === bindingName) ?? null;
}

function listRequiredSecrets(config) {
  const required = config.secrets?.required;
  return Array.isArray(required)
    ? required.filter((value) => typeof value === "string")
    : [];
}

function compareField(errors, fieldName, portalValue, publicApiValue) {
  if (portalValue !== publicApiValue) {
    errors.push(
      `${fieldName} must match between portal and public-api: portal="${stringifyValue(portalValue)}", public-api="${stringifyValue(publicApiValue)}"`,
    );
  }
}

function stringifyValue(value) {
  return value === undefined ? "undefined" : String(value);
}

function validateAbsoluteOrigin(errors, fieldName, value) {
  if (typeof value !== "string" || value.trim() === "") {
    errors.push(`portal vars.${fieldName} must be set to an absolute non-localhost URL`);
    return;
  }

  let parsed;
  try {
    parsed = new URL(value);
  } catch {
    errors.push(
      `portal vars.${fieldName} must be an absolute URL but got "${stringifyValue(value)}"`,
    );
    return;
  }

  if (isLocalHostname(parsed.hostname)) {
    errors.push(
      `portal vars.${fieldName} must not use a local loopback host but got "${parsed.toString()}"`,
    );
  }
}

function isLocalHostname(hostname) {
  return hostname === "localhost"
    || hostname === "127.0.0.1"
    || hostname === "::1"
    || hostname === "[::1]";
}

function stripJsonComments(source) {
  let output = "";
  let inString = false;
  let stringQuote = "";
  for (let index = 0; index < source.length; index += 1) {
    const current = source[index];
    const next = source[index + 1];

    if (inString) {
      output += current;
      if (current === "\\" && next) {
        output += next;
        index += 1;
        continue;
      }
      if (current === stringQuote) {
        inString = false;
        stringQuote = "";
      }
      continue;
    }

    if (current === "\"" || current === "'") {
      inString = true;
      stringQuote = current;
      output += current;
      continue;
    }

    if (current === "/" && next === "/") {
      while (index < source.length && source[index] !== "\n") {
        index += 1;
      }
      if (index < source.length) {
        output += "\n";
      }
      continue;
    }

    if (current === "/" && next === "*") {
      index += 2;
      while (index < source.length && !(source[index] === "*" && source[index + 1] === "/")) {
        index += 1;
      }
      index += 1;
      continue;
    }

    output += current;
  }
  return output;
}

function main() {
  const portalConfig = loadWranglerConfig(PORTAL_WRANGLER_PATH);
  const publicApiConfig = loadWranglerConfig(PUBLIC_API_WRANGLER_PATH);
  const remoteMcpConfig = loadWranglerConfig(REMOTE_MCP_WRANGLER_PATH);
  const result = verifyDeployContract(portalConfig, publicApiConfig, remoteMcpConfig);

  if (!result.ok) {
    console.error("deploy contract check failed");
    console.error(formatContractErrors(result.errors));
    process.exitCode = 1;
    return;
  }

  console.log("deploy contract check passed");
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  main();
}
