import { createStore } from "solid-js/store";
import {
  getAllSettings as ipcGetAllSettings,
  setSetting as ipcSetSetting,
} from "../lib/ipc";

const DEFAULTS: Record<string, string> = {
  search_engine: "duckduckgo",
  theme: "dark",
  accent_color: "#6366f1",
  font_size: "medium",
  sidebar_position: "left",
  shield_enabled: "true",
  third_party_cookies: "block",
  first_party_cookies: "session_only",
  fingerprint_resistance: "true",
  https_only: "true",
  restore_tabs_on_start: "false",
  history_mode: "session_only",
  download_location: "~/Downloads",
};

interface SettingsState {
  settings: Record<string, string>;
  initialized: boolean;
}

const [settingsState, setSettingsState] = createStore<SettingsState>({
  settings: { ...DEFAULTS },
  initialized: false,
});

async function initializeSettingsStore(): Promise<void> {
  if (settingsState.initialized) return;

  try {
    const settings = await ipcGetAllSettings();
    setSettingsState({
      settings: { ...DEFAULTS, ...settings },
      initialized: true,
    });
  } catch (e) {
    console.error("Failed to initialize settings store:", e);
    setSettingsState("initialized", true);
  }
}

async function updateSetting(key: string, value: string): Promise<void> {
  try {
    await ipcSetSetting(key, value);
    setSettingsState("settings", key, value);
  } catch (e) {
    console.error("Failed to update setting:", e);
  }
}

function getSettingValue(key: string): string {
  return settingsState.settings[key] ?? DEFAULTS[key] ?? "";
}

export {
  settingsState,
  initializeSettingsStore,
  updateSetting,
  getSettingValue,
  DEFAULTS,
};
