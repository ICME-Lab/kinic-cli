// Where: Tauri IPC wrapper for the desktop frontend.
// What: maps typed TypeScript calls to Rust commands.
// Why: keep command names and request shapes in one place.

import { invoke } from "@tauri-apps/api/core";
import type {
  DesktopCreateMemoryRequest,
  DesktopCreateMemoryResponse,
  DesktopInsertRequest,
  DesktopInsertResponse,
  DesktopMemoryDetails,
  DesktopMemorySummary,
  DesktopPreferencesUpdate,
  DesktopPreferencesView,
  DesktopSearchRequest,
  DesktopSearchResponse,
  DesktopSessionRequest,
  DesktopSessionView,
} from "./types";

export const desktopCommands = {
  getSession(request: DesktopSessionRequest) {
    return invoke<DesktopSessionView>("get_session", { request });
  },
  listMemories() {
    return invoke<DesktopMemorySummary[]>("list_memories");
  },
  searchMemories(request: DesktopSearchRequest) {
    return invoke<DesktopSearchResponse>("search_memories", { request });
  },
  getMemoryDetails(memoryId: string) {
    return invoke<DesktopMemoryDetails>("get_memory_details", { memoryId });
  },
  createMemory(request: DesktopCreateMemoryRequest) {
    return invoke<DesktopCreateMemoryResponse>("create_memory", { request });
  },
  insertContent(request: DesktopInsertRequest) {
    return invoke<DesktopInsertResponse>("insert_content", { request });
  },
  getPreferences() {
    return invoke<DesktopPreferencesView>("get_preferences");
  },
  updatePreferences(update: DesktopPreferencesUpdate) {
    return invoke<DesktopPreferencesView>("update_preferences", { update });
  },
};
