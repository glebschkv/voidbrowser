import { goBack, goForward, reloadPage, navigateTo } from "../../lib/ipc";

export function NavigationControls() {
  const handleBack = () => {
    goBack().catch(console.error);
  };

  const handleForward = () => {
    goForward().catch(console.error);
  };

  const handleReload = () => {
    reloadPage().catch(console.error);
  };

  const handleHome = () => {
    navigateTo("https://duckduckgo.com").catch(console.error);
  };

  return (
    <div class="flex items-center gap-1 mr-2">
      <button
        class="w-8 h-8 flex items-center justify-center rounded hover:bg-neutral-600 text-neutral-300 hover:text-neutral-100 transition-colors"
        onClick={handleBack}
        title="Back"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M10 3L5 8L10 13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
      <button
        class="w-8 h-8 flex items-center justify-center rounded hover:bg-neutral-600 text-neutral-300 hover:text-neutral-100 transition-colors"
        onClick={handleForward}
        title="Forward"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M6 3L11 8L6 13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
      <button
        class="w-8 h-8 flex items-center justify-center rounded hover:bg-neutral-600 text-neutral-300 hover:text-neutral-100 transition-colors"
        onClick={handleReload}
        title="Reload"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M13 8A5 5 0 1 1 8 3M13 3V8H8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
      <button
        class="w-8 h-8 flex items-center justify-center rounded hover:bg-neutral-600 text-neutral-300 hover:text-neutral-100 transition-colors"
        onClick={handleHome}
        title="Home"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M3 8L8 3L13 8M4 7V13H7V10H9V13H12V7" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
    </div>
  );
}
