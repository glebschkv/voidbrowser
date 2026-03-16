import { createSignal, onMount, onCleanup, Show } from "solid-js";
import { findInPage, findNext, findPrevious, stopFindInPage } from "../../lib/ipc";

interface FindBarProps {
  visible: boolean;
  onClose: () => void;
}

export function FindBar(props: FindBarProps) {
  const [query, setQuery] = createSignal("");
  let inputRef: HTMLInputElement | undefined;

  onMount(() => {
    if (props.visible && inputRef) {
      inputRef.focus();
    }
  });

  // Focus input when becoming visible
  const focusInput = () => {
    setTimeout(() => inputRef?.focus(), 0);
  };

  // Watch for visibility changes
  let prevVisible = false;
  const checkVisible = () => {
    if (props.visible && !prevVisible) {
      focusInput();
    }
    prevVisible = props.visible;
  };

  // Use a simple interval to detect visibility changes since we can't use createEffect easily
  const interval = setInterval(checkVisible, 100);
  onCleanup(() => clearInterval(interval));

  const handleSearch = () => {
    const q = query();
    if (q) {
      findInPage(q).catch(console.error);
    }
  };

  const handleNext = () => {
    findNext().catch(console.error);
  };

  const handlePrev = () => {
    findPrevious().catch(console.error);
  };

  const handleClose = () => {
    stopFindInPage().catch(console.error);
    setQuery("");
    props.onClose();
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      if (e.shiftKey) {
        handlePrev();
      } else if (query()) {
        // First enter does the search, subsequent enters go to next
        handleSearch();
        // After initial search, pressing Enter again should go next
        setTimeout(() => findNext().catch(console.error), 50);
      }
    }
    if (e.key === "Escape") {
      e.preventDefault();
      handleClose();
    }
  };

  const handleInput = (e: InputEvent) => {
    const value = (e.target as HTMLInputElement).value;
    setQuery(value);
    if (value) {
      findInPage(value).catch(console.error);
    } else {
      stopFindInPage().catch(console.error);
    }
  };

  return (
    <Show when={props.visible}>
      <div class="fixed top-[82px] right-4 z-50 flex items-center gap-1 bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-1.5 shadow-lg">
        <input
          ref={inputRef}
          type="text"
          class="bg-neutral-700 text-neutral-200 text-sm px-2 py-1 rounded outline-none w-48 placeholder-neutral-500"
          placeholder="Find in page"
          value={query()}
          onInput={handleInput}
          onKeyDown={handleKeyDown}
          spellcheck={false}
        />
        <button
          class="w-7 h-7 flex items-center justify-center rounded hover:bg-neutral-600 text-neutral-400 hover:text-neutral-200"
          onClick={handlePrev}
          title="Previous match (Shift+Enter)"
        >
          <svg class="w-3.5 h-3.5" viewBox="0 0 16 16" fill="none">
            <path d="M4 10L8 6L12 10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
        <button
          class="w-7 h-7 flex items-center justify-center rounded hover:bg-neutral-600 text-neutral-400 hover:text-neutral-200"
          onClick={handleNext}
          title="Next match (Enter)"
        >
          <svg class="w-3.5 h-3.5" viewBox="0 0 16 16" fill="none">
            <path d="M4 6L8 10L12 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
        <button
          class="w-7 h-7 flex items-center justify-center rounded hover:bg-neutral-600 text-neutral-400 hover:text-neutral-200"
          onClick={handleClose}
          title="Close (Escape)"
        >
          <svg class="w-3.5 h-3.5" viewBox="0 0 8 8" fill="none">
            <path d="M1 1L7 7M7 1L1 7" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      </div>
    </Show>
  );
}
