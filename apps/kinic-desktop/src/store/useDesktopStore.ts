// Where: global UI and command state for Kinic Memory.
// What: stores the active session, command results, forms, and toast state.
// Why: keep Tauri command orchestration out of presentational components.

import { create } from "zustand";
import { desktopCommands } from "@/desktop/commands";
import type {
  DesktopInsertMode,
  DesktopMemoryDetails,
  DesktopMemorySummary,
  DesktopNetwork,
  DesktopPreferencesView,
  DesktopSearchResult,
  DesktopSearchScope,
  DesktopSessionView,
} from "@/desktop/types";

export type ActiveTab = "memories" | "insert" | "create" | "settings";

type DesktopStore = {
  identityInput: string;
  networkInput: DesktopNetwork;
  session: DesktopSessionView | null;
  preferences: DesktopPreferencesView | null;
  memories: DesktopMemorySummary[];
  selectedMemoryId: string | null;
  selectedDetails: DesktopMemoryDetails | null;
  searchQuery: string;
  searchScope: DesktopSearchScope;
  searchResults: DesktopSearchResult[];
  activeTab: ActiveTab;
  loading: boolean;
  toast: string | null;
  error: string | null;
  insertMode: DesktopInsertMode;
  insertMemoryId: string;
  insertMemoryIdTouched: boolean;
  insertTag: string;
  insertText: string;
  insertFilePath: string;
  createName: string;
  createDescription: string;
  setIdentityInput: (value: string) => void;
  setNetworkInput: (value: DesktopNetwork) => void;
  setActiveTab: (tab: ActiveTab) => void;
  setSearchQuery: (value: string) => void;
  setSearchScope: (value: DesktopSearchScope) => void;
  setInsertMode: (value: DesktopInsertMode) => void;
  setInsertMemoryId: (value: string) => void;
  setInsertTag: (value: string) => void;
  setInsertText: (value: string) => void;
  setInsertFilePath: (value: string) => void;
  setCreateName: (value: string) => void;
  setCreateDescription: (value: string) => void;
  startSession: () => Promise<void>;
  refreshMemories: () => Promise<void>;
  selectMemory: (memoryId: string) => Promise<void>;
  submitSearch: () => Promise<void>;
  submitInsert: () => Promise<void>;
  submitCreate: () => Promise<void>;
  updateDefaultMemory: (memoryId: string | null) => Promise<void>;
  clearToast: () => void;
};

export const useDesktopStore = create<DesktopStore>((set, get) => ({
  identityInput: "",
  networkInput: "local",
  session: null,
  preferences: null,
  memories: [],
  selectedMemoryId: null,
  selectedDetails: null,
  searchQuery: "",
  searchScope: "all",
  searchResults: [],
  activeTab: "memories",
  loading: false,
  toast: null,
  error: null,
  insertMode: "file",
  insertMemoryId: "",
  insertMemoryIdTouched: false,
  insertTag: "",
  insertText: "",
  insertFilePath: "",
  createName: "",
  createDescription: "",
  setIdentityInput: (identityInput) => set({ identityInput }),
  setNetworkInput: (networkInput) => set({ networkInput }),
  setActiveTab: (activeTab) => set({ activeTab }),
  setSearchQuery: (searchQuery) => set({ searchQuery }),
  setSearchScope: (searchScope) => set({ searchScope }),
  setInsertMode: (insertMode) => set({ insertMode }),
  setInsertMemoryId: (insertMemoryId) => set({ insertMemoryId, insertMemoryIdTouched: true }),
  setInsertTag: (insertTag) => set({ insertTag }),
  setInsertText: (insertText) => set({ insertText }),
  setInsertFilePath: (insertFilePath) => set({ insertFilePath }),
  setCreateName: (createName) => set({ createName }),
  setCreateDescription: (createDescription) => set({ createDescription }),
  clearToast: () => set({ toast: null }),
  startSession: async () => runCommand(set, async () => {
    const session = await desktopCommands.getSession({
      identity: get().identityInput,
      network: get().networkInput,
    });
    const preferences = await desktopCommands.getPreferences();
    set({ session, preferences, toast: "Session started." });
    await get().refreshMemories();
  }),
  refreshMemories: async () => runCommand(set, async () => {
    const memories = await desktopCommands.listMemories();
    const selectedMemoryId = memories[0]?.searchable_memory_id ?? memories[0]?.id ?? null;
    set({
      memories,
      selectedMemoryId,
      selectedDetails: null,
      searchResults: [],
      ...(selectedMemoryId === null && !get().insertMemoryIdTouched ? { insertMemoryId: "" } : {}),
    });
    if (selectedMemoryId) {
      await get().selectMemory(selectedMemoryId);
    }
  }),
  selectMemory: async (memoryId) => runCommand(set, async () => {
    const shouldSyncInsertMemory = !get().insertMemoryIdTouched;
    set({
      selectedMemoryId: memoryId,
      selectedDetails: null,
      ...(shouldSyncInsertMemory ? { insertMemoryId: memoryId } : {}),
    });
    const selectedDetails = await desktopCommands.getMemoryDetails(memoryId);
    if (get().selectedMemoryId === memoryId) {
      set({ selectedDetails });
    }
  }),
  submitSearch: async () => runCommand(set, async () => {
    const response = await desktopCommands.searchMemories({
      query: get().searchQuery,
      scope: get().searchScope,
      selected_memory_id: get().selectedMemoryId,
    });
    set({ searchResults: response.items, toast: `Loaded ${response.items.length} search results.` });
  }),
  submitInsert: async () => runCommand(set, async () => {
    const result = await desktopCommands.insertContent({
      mode: get().insertMode,
      memory_id: get().insertMemoryId,
      tag: emptyToNull(get().insertTag),
      text: emptyToNull(get().insertText),
      file_path: emptyToNull(get().insertFilePath),
    });
    set({
      insertText: "",
      toast: `Inserted ${result.inserted_count} chunk${result.inserted_count === 1 ? "" : "s"} into ${result.memory_id}.`,
    });
  }),
  submitCreate: async () => runCommand(set, async () => {
    const result = await desktopCommands.createMemory({
      name: get().createName,
      description: get().createDescription,
    });
    if (result.memories) {
      set({ memories: result.memories });
    }
    set({
      createName: "",
      createDescription: "",
      selectedMemoryId: result.id,
      selectedDetails: null,
      insertMemoryId: result.id,
      insertMemoryIdTouched: false,
      toast: `Created memory ${result.id}.`,
    });
  }),
  updateDefaultMemory: async (memoryId) => runCommand(set, async () => {
    const preferences = await desktopCommands.updatePreferences({ default_memory_id: memoryId });
    set({ preferences, toast: "Default memory updated." });
  }),
}));

async function runCommand(
  set: (partial: Partial<DesktopStore>) => void,
  command: () => Promise<void>,
) {
  set({ loading: true, error: null });
  try {
    await command();
  } catch (error) {
    set({ error: error instanceof Error ? error.message : String(error) });
  } finally {
    set({ loading: false });
  }
}

function emptyToNull(value: string) {
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}
