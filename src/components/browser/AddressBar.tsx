import { createSignal, onCleanup, onMount } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { navigateTo, getCurrentUrl } from "../../lib/ipc";

export function AddressBar() {
  const [url, setUrl] = createSignal("");
  const [isEditing, setIsEditing] = createSignal(false);
  const [editValue, setEditValue] = createSignal("");
  let inputRef: HTMLInputElement | undefined;
  let unlistenFn: (() => void) | undefined;

  const isHttps = () => url().startsWith("https://");

  onMount(async () => {
    // Get initial URL
    try {
      const currentUrl = await getCurrentUrl();
      setUrl(currentUrl);
    } catch {
      // Content webview may not be ready yet
    }

    // Listen for URL changes from the content webview
    const unlisten = await listen<string>("url-changed", (event) => {
      if (!isEditing()) {
        setUrl(event.payload);
      }
    });
    unlistenFn = unlisten;
  });

  onCleanup(() => {
    unlistenFn?.();
  });

  const handleFocus = () => {
    setIsEditing(true);
    setEditValue(url());
    setTimeout(() => inputRef?.select(), 0);
  };

  const handleBlur = () => {
    setIsEditing(false);
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      const value = editValue().trim();
      if (value) {
        navigateTo(value).catch(console.error);
      }
      setIsEditing(false);
      inputRef?.blur();
    } else if (e.key === "Escape") {
      setIsEditing(false);
      inputRef?.blur();
    }
  };

  return (
    <div class="flex-1 flex items-center h-8 bg-neutral-700 rounded-md px-3 text-sm font-mono">
      {isHttps() && !isEditing() && (
        <svg
          class="w-4 h-4 mr-2 text-green-400 flex-shrink-0"
          viewBox="0 0 16 16"
          fill="none"
        >
          <rect x="3" y="7" width="10" height="7" rx="1" stroke="currentColor" stroke-width="1.5"/>
          <path d="M5 7V5a3 3 0 0 1 6 0v2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
      )}
      <input
        ref={inputRef}
        type="text"
        class="flex-1 bg-transparent outline-none text-neutral-200 placeholder-neutral-500"
        placeholder="Search or enter URL"
        value={isEditing() ? editValue() : url()}
        onFocus={handleFocus}
        onBlur={handleBlur}
        onInput={(e) => setEditValue(e.currentTarget.value)}
        onKeyDown={handleKeyDown}
        spellcheck={false}
      />
    </div>
  );
}
