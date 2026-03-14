import { invoke } from "@tauri-apps/api/core";

export async function navigateTo(input: string): Promise<void> {
  return invoke("navigate_to", { input });
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
