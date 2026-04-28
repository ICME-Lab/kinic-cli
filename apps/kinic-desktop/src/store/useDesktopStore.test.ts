// Where: desktop global store tests.
// What: verifies selection state transitions around memory detail loading.
// Why: stale details must not survive failed memory selection.

import { useDesktopStore } from "./useDesktopStore";
import type { DesktopMemoryDetails, DesktopMemorySummary } from "@/desktop/types";

const commandMocks = vi.hoisted(() => ({
  createMemory: vi.fn(),
  getMemoryDetails: vi.fn(),
  listMemories: vi.fn(),
}));

vi.mock("@/desktop/commands", () => ({
  desktopCommands: commandMocks,
}));

describe("useDesktopStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useDesktopStore.setState({
      memories: [],
      selectedMemoryId: null,
      selectedDetails: null,
      searchResults: [],
      loading: false,
      error: null,
      toast: null,
      insertMemoryId: "",
      insertMemoryIdTouched: false,
    });
  });

  it("clears stale details when selecting a memory whose details fail", async () => {
    useDesktopStore.setState({
      selectedMemoryId: "old-id",
      selectedDetails: memoryDetails("old-id"),
      insertMemoryId: "",
      insertMemoryIdTouched: false,
    });
    commandMocks.getMemoryDetails.mockRejectedValueOnce(new Error("detail unavailable"));

    await useDesktopStore.getState().selectMemory("new-id");

    expect(useDesktopStore.getState().selectedMemoryId).toBe("new-id");
    expect(useDesktopStore.getState().selectedDetails).toBeNull();
    expect(useDesktopStore.getState().insertMemoryId).toBe("new-id");
    expect(useDesktopStore.getState().error).toBe("detail unavailable");
  });

  it("syncs insert memory while it has not been manually edited", async () => {
    commandMocks.listMemories.mockResolvedValueOnce([memorySummary("first-id")]);
    commandMocks.getMemoryDetails.mockResolvedValueOnce(memoryDetails("first-id"));

    await useDesktopStore.getState().refreshMemories();

    expect(useDesktopStore.getState().insertMemoryId).toBe("first-id");

    commandMocks.getMemoryDetails.mockResolvedValueOnce(memoryDetails("second-id"));
    await useDesktopStore.getState().selectMemory("second-id");

    expect(useDesktopStore.getState().selectedMemoryId).toBe("second-id");
    expect(useDesktopStore.getState().insertMemoryId).toBe("second-id");
  });

  it("keeps a manually entered insert memory when selection changes", async () => {
    useDesktopStore.getState().setInsertMemoryId("manual-id");
    commandMocks.getMemoryDetails.mockResolvedValueOnce(memoryDetails("new-id"));

    await useDesktopStore.getState().selectMemory("new-id");

    expect(useDesktopStore.getState().selectedDetails?.memory_id).toBe("new-id");
    expect(useDesktopStore.getState().insertMemoryId).toBe("manual-id");
    expect(useDesktopStore.getState().insertMemoryIdTouched).toBe(true);
  });

  it("ignores stale memory details that resolve after another selection", async () => {
    let resolveOldDetails: (details: DesktopMemoryDetails) => void = () => {};
    commandMocks.getMemoryDetails
      .mockReturnValueOnce(new Promise<DesktopMemoryDetails>((resolve) => {
        resolveOldDetails = resolve;
      }))
      .mockResolvedValueOnce(memoryDetails("new-id"));

    const oldSelection = useDesktopStore.getState().selectMemory("old-id");
    await useDesktopStore.getState().selectMemory("new-id");
    resolveOldDetails(memoryDetails("old-id"));
    await oldSelection;

    expect(useDesktopStore.getState().selectedMemoryId).toBe("new-id");
    expect(useDesktopStore.getState().selectedDetails?.memory_id).toBe("new-id");
  });

  it("clears stale details after creating a memory", async () => {
    useDesktopStore.setState({
      selectedMemoryId: "old-id",
      selectedDetails: memoryDetails("old-id"),
      insertMemoryId: "manual-id",
      insertMemoryIdTouched: true,
      createName: "Created",
      createDescription: "Created memory",
    });
    commandMocks.createMemory.mockResolvedValueOnce({
      id: "created-id",
      memories: null,
      refresh_warning: null,
    });

    await useDesktopStore.getState().submitCreate();

    expect(useDesktopStore.getState().selectedMemoryId).toBe("created-id");
    expect(useDesktopStore.getState().selectedDetails).toBeNull();
    expect(useDesktopStore.getState().insertMemoryId).toBe("created-id");
    expect(useDesktopStore.getState().insertMemoryIdTouched).toBe(false);
  });

  it("clears selected memory details when refresh returns no memories", async () => {
    useDesktopStore.setState({
      memories: [memorySummary("old-id")],
      selectedMemoryId: "old-id",
      selectedDetails: memoryDetails("old-id"),
      searchResults: [{ memory_id: "old-id", score: 0.9, tag: "docs", sentence: "old" }],
      insertMemoryId: "old-id",
      insertMemoryIdTouched: false,
    });
    commandMocks.listMemories.mockResolvedValueOnce([]);

    await useDesktopStore.getState().refreshMemories();

    expect(useDesktopStore.getState().memories).toEqual([]);
    expect(useDesktopStore.getState().selectedMemoryId).toBeNull();
    expect(useDesktopStore.getState().selectedDetails).toBeNull();
    expect(useDesktopStore.getState().searchResults).toEqual([]);
    expect(useDesktopStore.getState().insertMemoryId).toBe("");
  });

  it("keeps manually entered insert memory when refresh returns no memories", async () => {
    useDesktopStore.setState({
      memories: [memorySummary("old-id")],
      selectedMemoryId: "old-id",
      selectedDetails: memoryDetails("old-id"),
      insertMemoryId: "manual-id",
      insertMemoryIdTouched: true,
    });
    commandMocks.listMemories.mockResolvedValueOnce([]);

    await useDesktopStore.getState().refreshMemories();

    expect(useDesktopStore.getState().selectedMemoryId).toBeNull();
    expect(useDesktopStore.getState().selectedDetails).toBeNull();
    expect(useDesktopStore.getState().insertMemoryId).toBe("manual-id");
  });
});

function memorySummary(id: string): DesktopMemorySummary {
  return {
    id,
    status: "running",
    detail: "ready",
    searchable_memory_id: id,
    name: id,
    version: "1.0.0",
    dim: 384,
  };
}

function memoryDetails(memoryId: string): DesktopMemoryDetails {
  return {
    memory_id: memoryId,
    display_name: memoryId,
    metadata_name: memoryId,
    version: "1.0.0",
    dim: 384,
    owners: [],
    stable_memory_size: 0,
    cycle_amount: 0,
    users: [],
    users_load_error: null,
  };
}
