// Where: TypeScript mirror of Rust desktop DTOs.
// What: defines command request/response shapes used by Zustand and views.
// Why: keep the Tauri IPC surface explicit and isolated from portal types.

export type DesktopNetwork = "local" | "mainnet";
export type DesktopSearchScope = "all" | "selected";
export type DesktopInsertMode = "file" | "inline_text" | "manual_embedding";

export type DesktopSessionRequest = {
  identity: string;
  network: DesktopNetwork;
};

export type DesktopSessionView = {
  identity: string;
  network: string;
  auth_mode: string;
  principal_id: string;
  embedding_api_endpoint: string;
  balance_base_units: string | null;
  balance_kinic: string | null;
  fee_base_units: string | null;
  price_base_units: string | null;
  required_total_base_units: string | null;
  required_total_kinic: string | null;
  sufficient_balance: boolean | null;
  errors: string[];
};

export type DesktopMemorySummary = {
  id: string;
  status: string;
  detail: string;
  searchable_memory_id: string | null;
  name: string;
  version: string;
  dim: number | null;
};

export type DesktopMemoryDetails = {
  memory_id: string;
  display_name: string;
  metadata_name: string;
  version: string;
  dim: number | null;
  owners: string[];
  stable_memory_size: number | null;
  cycle_amount: number | null;
  users: DesktopMemoryUser[];
  users_load_error: string | null;
};

export type DesktopMemoryUser = {
  principal_id: string;
  role: string;
};

export type DesktopSearchRequest = {
  query: string;
  scope: DesktopSearchScope;
  selected_memory_id: string | null;
};

export type DesktopSearchResponse = {
  items: DesktopSearchResult[];
};

export type DesktopSearchResult = {
  memory_id: string;
  score: number;
  tag: string | null;
  sentence: string;
};

export type DesktopCreateMemoryRequest = {
  name: string;
  description: string;
};

export type DesktopCreateMemoryResponse = {
  id: string;
  memories: DesktopMemorySummary[] | null;
  refresh_warning: string | null;
};

export type DesktopInsertRequest = {
  mode: DesktopInsertMode;
  memory_id: string;
  tag: string | null;
  text: string | null;
  file_path: string | null;
};

export type DesktopInsertResponse = {
  memory_id: string;
  tag: string;
  inserted_count: number;
  source_name: string | null;
};

export type DesktopPreferencesView = {
  default_memory_id: string | null;
  saved_tags: string[];
  manual_memory_ids: string[];
  chat_overall_top_k: number;
  chat_per_memory_cap: number;
  chat_mmr_lambda: number;
};

export type DesktopPreferencesUpdate = {
  default_memory_id: string | null;
};
