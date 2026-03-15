import { invoke } from "@tauri-apps/api/core";

// ── Types ────────────────────────────────────────────────────────────

export interface TabInfo {
  id: string;
  title: string;
  url: string;
  isLoading: boolean;
  faviconUrl: string | null;
}

export interface TabsResponse {
  tabs: TabInfo[];
  activeTabId: string | null;
}

// ── Tab commands ─────────────────────────────────────────────────────

export async function createTab(url?: string): Promise<TabInfo> {
  return invoke("create_tab", { url: url ?? null });
}

export async function closeTab(tabId: string): Promise<void> {
  return invoke("close_tab", { tabId });
}

export async function switchTab(tabId: string): Promise<void> {
  return invoke("switch_tab", { tabId });
}

export async function getTabs(): Promise<TabsResponse> {
  return invoke("get_tabs");
}

export async function reorderTabs(tabIds: string[]): Promise<void> {
  return invoke("reorder_tabs", { tabIds });
}

// ── Navigation commands ──────────────────────────────────────────────

export async function navigateTo(tabId: string, input: string): Promise<void> {
  return invoke("navigate_to", { tabId, input });
}

export async function goBack(): Promise<void> {
  return invoke("go_back");
}

export async function goForward(): Promise<void> {
  return invoke("go_forward");
}

export async function reloadPage(): Promise<void> {
  return invoke("reload_page");
}

export async function getCurrentUrl(): Promise<string> {
  return invoke("get_current_url");
}
