// Where: top-level desktop UI.
// What: switches between startup and authenticated app shell.
// Why: avoid network commands until the user chooses identity and network.

import { StartupScreen } from "./startup-screen";
import { DesktopShell } from "./desktop-shell";
import { useDesktopStore } from "@/store/useDesktopStore";

export function App() {
  const session = useDesktopStore((state) => state.session);
  return session ? <DesktopShell /> : <StartupScreen />;
}
