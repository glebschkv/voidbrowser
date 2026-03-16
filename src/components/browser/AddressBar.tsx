import { createSignal, createEffect, onCleanup, onMount, Show, For } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { navigateTo, getCurrentUrl, setSettingsOpen } from "../../lib/ipc";
import { tabState } from "../../stores/tabStore";
import { ShieldIcon } from "../privacy/ShieldIcon";
import {
  suggestions,
  showSuggestions,
  fetchSuggestions,
  clearSuggestions,
  type Suggestion,
} from "../../stores/navigationStore";

// On Windows, WebView2 rewrites void://X to http://void.X/ — normalize back.
function normalizeVoidUrl(rawUrl: string): string {
  const match = rawUrl.match(/^https?:\/\/void\.(\w+)\/?$/);
  if (match) return `void://${match[1]}`;
  return rawUrl;
}

export function AddressBar() {
  const [url, setUrl] = createSignal("");
  const [isEditing, setIsEditing] = createSignal(false);
  const [editValue, setEditValue] = createSignal("");
  const [selectedIndex, setSelectedIndex] = createSignal(-1);
  let inputRef: HTMLInputElement | undefined;
  let unlistenUrlChanged: (() => void) | undefined;

  const isHttps = () => url().startsWith("https://");

  onMount(async () => {
    const unlisten = await listen<{ tabId: string; url: string }>(
      "tab-url-changed",
      (event) => {
        if (event.payload.tabId === tabState.activeTabId && !isEditing()) {
          setUrl(normalizeVoidUrl(event.payload.url));
        }
      }
    );
    unlistenUrlChanged = unlisten;
  });

  onCleanup(() => {
    unlistenUrlChanged?.();
  });

  createEffect(() => {
    const activeId = tabState.activeTabId;
    if (!activeId || isEditing()) return;

    const activeTab = tabState.tabs.find((t) => t.id === activeId);
    if (activeTab) {
      setUrl(normalizeVoidUrl(activeTab.url));
    }

    getCurrentUrl()
      .then((u) => {
        if (!isEditing()) setUrl(normalizeVoidUrl(u));
      })
      .catch(() => {});
  });

  const handleFocus = () => {
    setIsEditing(true);
    setEditValue(url());
    setSelectedIndex(-1);
    setTimeout(() => inputRef?.select(), 0);
    // Hide the content webview so the autocomplete dropdown is visible
    setSettingsOpen(true).catch(console.error);
  };

  const handleBlur = () => {
    // Delay to allow click on suggestion to fire first
    setTimeout(() => {
      setIsEditing(false);
      clearSuggestions();
      setSelectedIndex(-1);
      // Show the content webview again
      setSettingsOpen(false).catch(console.error);
    }, 150);
  };

  const navigateToValue = (value: string) => {
    if (value && tabState.activeTabId) {
      navigateTo(tabState.activeTabId, value).catch(console.error);
    }
    setIsEditing(false);
    clearSuggestions();
    setSelectedIndex(-1);
    inputRef?.blur();
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    const items = suggestions();

    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((prev) => Math.min(prev + 1, items.length - 1));
      return;
    }

    if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((prev) => Math.max(prev - 1, -1));
      return;
    }

    if (e.key === "Enter") {
      const idx = selectedIndex();
      if (idx >= 0 && idx < items.length) {
        navigateToValue(items[idx].url);
      } else {
        navigateToValue(editValue().trim());
      }
      return;
    }

    if (e.key === "Escape") {
      setIsEditing(false);
      clearSuggestions();
      setSelectedIndex(-1);
      inputRef?.blur();
    }
  };

  const handleInput = (e: InputEvent) => {
    const value = (e.target as HTMLInputElement).value;
    setEditValue(value);
    setSelectedIndex(-1);
    fetchSuggestions(value);
  };

  const handleSuggestionClick = (suggestion: Suggestion) => {
    navigateToValue(suggestion.url);
  };

  return (
    <div class="flex-1 flex items-center h-8 bg-neutral-700 rounded-md px-3 text-sm font-mono relative">
      {isHttps() && !isEditing() && (
        <svg
          class="w-4 h-4 mr-2 text-green-400 flex-shrink-0"
          viewBox="0 0 16 16"
          fill="none"
        >
          <rect
            x="3"
            y="7"
            width="10"
            height="7"
            rx="1"
            stroke="currentColor"
            stroke-width="1.5"
          />
          <path
            d="M5 7V5a3 3 0 0 1 6 0v2"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
          />
        </svg>
      )}
      <input
        ref={inputRef}
        data-address-bar
        type="text"
        class="flex-1 bg-transparent outline-none text-neutral-200 placeholder-neutral-500"
        placeholder="Search or enter URL"
        value={isEditing() ? editValue() : url()}
        onFocus={handleFocus}
        onBlur={handleBlur}
        onInput={handleInput}
        onKeyDown={handleKeyDown}
        spellcheck={false}
      />
      <ShieldIcon />

      {/* Autocomplete dropdown */}
      <Show when={isEditing() && showSuggestions()}>
        <div class="absolute left-0 right-0 top-[34px] bg-neutral-800 border border-neutral-700 rounded-md shadow-lg z-50 overflow-hidden">
          <For each={suggestions()}>
            {(suggestion, index) => (
              <div
                class={`flex items-center gap-2 px-3 py-2 cursor-pointer text-xs ${
                  index() === selectedIndex()
                    ? "bg-neutral-600"
                    : "hover:bg-neutral-700"
                }`}
                onMouseDown={() => handleSuggestionClick(suggestion)}
              >
                {suggestion.type === "bookmark" ? (
                  <svg
                    class="w-3.5 h-3.5 text-accent flex-shrink-0"
                    viewBox="0 0 16 16"
                    fill="currentColor"
                  >
                    <path d="M8 1.5l1.76 3.57 3.94.57-2.85 2.78.67 3.93L8 10.42l-3.52 1.93.67-3.93L2.3 5.64l3.94-.57L8 1.5z" />
                  </svg>
                ) : (
                  <svg
                    class="w-3.5 h-3.5 text-neutral-500 flex-shrink-0"
                    viewBox="0 0 16 16"
                    fill="none"
                  >
                    <circle
                      cx="8"
                      cy="8"
                      r="5.5"
                      stroke="currentColor"
                      stroke-width="1.5"
                    />
                    <path
                      d="M8 4.5V8l2 1.5"
                      stroke="currentColor"
                      stroke-width="1.5"
                      stroke-linecap="round"
                    />
                  </svg>
                )}
                <div class="flex-1 min-w-0">
                  <span class="text-neutral-200 truncate block">
                    {suggestion.title || suggestion.url}
                  </span>
                </div>
                <span class="text-neutral-500 truncate max-w-[200px]">
                  {suggestion.url}
                </span>
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}
