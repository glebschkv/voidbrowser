import { createSignal } from "solid-js";

const [sidebarOpen, setSidebarOpen] = createSignal(false);
const [settingsOpen, setSettingsOpen] = createSignal(false);

function toggleSidebar(): void {
  setSidebarOpen((prev) => !prev);
  if (settingsOpen()) setSettingsOpen(false);
}

function toggleSettings(): void {
  setSettingsOpen((prev) => !prev);
  if (sidebarOpen()) setSidebarOpen(false);
}

export { sidebarOpen, setSidebarOpen, settingsOpen, setSettingsOpen, toggleSidebar, toggleSettings };
