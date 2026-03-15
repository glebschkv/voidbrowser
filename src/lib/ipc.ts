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

// ── Privacy commands ────────────────────────────────────────────────

export async function getBlockedCount(tabId: string): Promise<number> {
  return invoke("get_blocked_count", { tabId });
}

export async function toggleShield(tabId: string): Promise<boolean> {
  return invoke("toggle_shield", { tabId });
}

export async function allowHttpAndNavigate(
  tabId: string,
  url: string
): Promise<void> {
  return invoke("allow_http_and_navigate", { tabId, url });
}

export async function toggleSiteShield(domain: string): Promise<boolean> {
  return invoke("toggle_site_shield", { domain });
}

export async function getSiteShieldStatus(domain: string): Promise<boolean> {
  return invoke("get_site_shield_status", { domain });
}
