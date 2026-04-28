// Where: desktop shell smoke tests.
// What: verifies startup gating before any Tauri network command is invoked.
// Why: identity/network selection must happen before canister operations.

import { render, screen } from "@testing-library/react";
import { App } from "./app";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

describe("App", () => {
  it("starts on the session form", () => {
    render(<App />);

    expect(screen.getByRole("heading", { name: "Kinic Memory" })).toBeTruthy();
    expect(screen.getByLabelText("Identity")).toBeTruthy();
    expect(screen.getByRole<HTMLButtonElement>("button", { name: "Start Session" }).disabled).toBe(true);
  });
});
