import { createSignal } from "solid-js";
import {
  setSidebarOpen as setSidebarOpenIPC,
  setSettingsOpen as setSettingsOpenIPC,
} from "../lib/ipc";

const [sidebarOpen, setSidebarOpen] = createSignal(false);
const [settingsOpen, setSettingsOpen] = createSignal(false);

function toggleSidebar(): void {
  const newOpen = !sidebarOpen();
  setSidebarOpen(newOpen);
  if (settingsOpen()) {
    setSettingsOpen(false);
    setSettingsOpenIPC(false).catch(console.error);
  }
  setSidebarOpenIPC(newOpen).catch(console.error);
}

function toggleSettings(): void {
  const newOpen = !settingsOpen();
  setSettingsOpen(newOpen);
  if (sidebarOpen()) {
    setSidebarOpen(false);
    setSidebarOpenIPC(false).catch(console.error);
  }
  setSettingsOpenIPC(newOpen).catch(console.error);
}

function closeSidebar(): void {
  if (sidebarOpen()) {
    setSidebarOpen(false);
    setSidebarOpenIPC(false).catch(console.error);
  }
}

function closeSettings(): void {
  if (settingsOpen()) {
    setSettingsOpen(false);
    setSettingsOpenIPC(false).catch(console.error);
  }
}

export {
  sidebarOpen,
  setSidebarOpen,
  settingsOpen,
  setSettingsOpen,
  toggleSidebar,
  toggleSettings,
  closeSidebar,
  closeSettings,
};
