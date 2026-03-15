# CLAUDE.md — VoidBrowser

## CURRENT PHASE: 7

> Read this file top to bottom before writing any code. Build ONLY the current phase. Do not skip ahead. Do not add features from later phases. When a phase is complete, the human will update CURRENT PHASE.

---

## WHAT IS THIS

VoidBrowser is a zero-tracking privacy browser for Windows. It collects nothing — no telemetry, no analytics, no accounts, no cloud. The browser binary is the entire product. All user data lives on the user's machine, encrypted, and dies when they say so.

**Target platform (MVP):** Windows 10/11 only. Uses WebView2 (Chromium-based).
**Later:** macOS (WKWebView) and Linux (WebKitGTK) come after MVP ships.

---

## LINUX BUILD DEPENDENCIES (CI / Dev Environment)

Before building on Linux, install these packages (required by Tauri/wry even though VoidBrowser targets Windows):

```bash
sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev libsoup-3.0-dev
```

A SessionStart hook in `.claude/hooks.json` auto-installs these in Claude Code web sessions.

---

## TECH STACK (DO NOT CHANGE)

| Layer | Technology | Version |
|-------|-----------|---------|
| App framework | Tauri v2 | latest stable 2.x |
| Backend | Rust | stable toolchain |
| Frontend | SolidJS + TypeScript | solid-js 1.9.x |
| CSS | Tailwind CSS | 4.x |
| Build tool | Vite | 6.x |
| Ad blocking | adblock (Brave's crate) | 0.12.x |
| Encrypted DB | rusqlite + bundled-sqlcipher | 0.32+ |
| Encryption | chacha20poly1305 | 0.10.x |
| Key derivation | argon2 | 0.5.x |
| DNS-over-HTTPS | hickory-resolver | 0.25.x |
| Package manager | pnpm | latest |
| Testing | vitest + cargo test | latest |

---

## THREE RULES (NEVER BREAK THESE)

1. **NEVER add code that makes outbound requests to any server we control.** We have no servers. The ONLY outbound requests are: user-initiated navigation, DNS-over-HTTPS to chosen provider, and filter list updates from EasyList/EasyPrivacy CDNs.
2. **NEVER add telemetry, analytics, crash reporting, or any form of data collection.** Not even "anonymous" analytics.
3. **NEVER use `unwrap()` in production code.** Use `?` or explicit error handling. Panics across FFI = segfault. Wrap all webview callbacks in `catch_unwind` + `AssertUnwindSafe`.

---

## PROJECT STRUCTURE

```
void-browser/
├── CLAUDE.md
├── README.md
├── LICENSE                      # MPL-2.0
├── PRIVACY.md
│
├── src-tauri/
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   └── default.json
│   ├── icons/
│   ├── src/
│   │   ├── main.rs              # Entry: call lib::run()
│   │   ├── lib.rs               # Tauri builder setup, plugin registration
│   │   │
│   │   ├── browser/
│   │   │   ├── mod.rs
│   │   │   ├── webview.rs       # Webview creation and config
│   │   │   ├── tabs.rs          # Tab state machine
│   │   │   └── navigation.rs    # URL handling, search query detection
│   │   │
│   │   ├── privacy/
│   │   │   ├── mod.rs
│   │   │   ├── ad_blocker.rs    # adblock-rust integration
│   │   │   ├── fingerprint.rs   # Generates anti-fingerprint JS
│   │   │   ├── cookie_policy.rs # Cookie blocking/isolation
│   │   │   ├── https_only.rs    # HTTPS enforcement
│   │   │   └── dns_resolver.rs  # DoH via hickory-dns
│   │   │
│   │   ├── storage/
│   │   │   ├── mod.rs
│   │   │   ├── database.rs      # SQLCipher setup
│   │   │   ├── bookmarks.rs     # Encrypted bookmarks
│   │   │   ├── settings.rs      # User preferences
│   │   │   ├── history.rs       # Session-only history (in-memory)
│   │   │   └── crypto.rs        # Key derivation helpers
│   │   │
│   │   └── commands.rs          # ALL #[tauri::command] functions
│   │
│   └── resources/
│       ├── filter_lists/
│       │   ├── easylist.txt
│       │   └── easyprivacy.txt
│       └── fingerprint_shield.js
│
├── src/                         # SolidJS frontend
│   ├── index.html
│   ├── App.tsx                  # Root layout
│   ├── main.tsx                 # SolidJS mount
│   │
│   ├── components/
│   │   ├── browser/
│   │   │   ├── TabBar.tsx
│   │   │   ├── Tab.tsx
│   │   │   ├── AddressBar.tsx
│   │   │   ├── NavigationControls.tsx
│   │   │   └── WebviewContainer.tsx
│   │   │
│   │   ├── privacy/
│   │   │   ├── ShieldIcon.tsx
│   │   │   ├── TrackerList.tsx
│   │   │   └── SitePermissions.tsx
│   │   │
│   │   ├── sidebar/
│   │   │   ├── Sidebar.tsx
│   │   │   ├── BookmarkPanel.tsx
│   │   │   └── HistoryPanel.tsx
│   │   │
│   │   ├── settings/
│   │   │   └── SettingsPage.tsx
│   │   │
│   │   └── shared/
│   │       ├── Modal.tsx
│   │       ├── Tooltip.tsx
│   │       └── ContextMenu.tsx
│   │
│   ├── stores/
│   │   ├── tabStore.ts
│   │   ├── privacyStore.ts
│   │   ├── settingsStore.ts
│   │   ├── bookmarkStore.ts
│   │   └── navigationStore.ts
│   │
│   ├── lib/
│   │   ├── ipc.ts               # Typed Tauri invoke wrappers
│   │   ├── constants.ts
│   │   └── utils.ts
│   │
│   └── styles/
│       └── global.css           # Tailwind directives + base
│
└── .github/
    └── workflows/
        └── build.yml
```

---

## CODING PATTERNS

### Rust

```rust
// Every Tauri command follows this pattern:
#[tauri::command]
async fn navigate_to(
    app: tauri::AppHandle,
    tab_id: String,
    url: String,
) -> Result<(), String> {
    // ... implementation
    Ok(())
}

// Error handling: use thiserror for custom errors
#[derive(Debug, thiserror::Error)]
enum BrowserError {
    #[error("Tab not found: {0}")]
    TabNotFound(String),
    #[error("Navigation failed: {0}")]
    NavigationFailed(String),
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
}

// Make errors serializable for Tauri IPC
impl serde::Serialize for BrowserError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::ser::Serializer {
        serializer.serialize_str(self.to_string().as_str())
    }
}

// FFI safety: ALWAYS wrap webview callbacks
use std::panic::{catch_unwind, AssertUnwindSafe};
let result = catch_unwind(AssertUnwindSafe(|| {
    // callback code that might panic
}));
```

### TypeScript (SolidJS)

```tsx
// Tauri IPC wrapper — every backend call goes through this
import { invoke } from "@tauri-apps/api/core";

export async function navigateTo(tabId: string, url: string): Promise<void> {
  return invoke("navigate_to", { tabId, url });
}

// Components: functional with explicit props
interface TabProps {
  id: string;
  title: string;
  isActive: boolean;
  onClose: (id: string) => void;
  onSelect: (id: string) => void;
}

export function Tab(props: TabProps) {
  return (
    <div
      class={`tab ${props.isActive ? "tab-active" : ""}`}
      onClick={() => props.onSelect(props.id)}
    >
      <span class="tab-title">{props.title}</span>
      <button class="tab-close" onClick={(e) => {
        e.stopPropagation();
        props.onClose(props.id);
      }}>×</button>
    </div>
  );
}

// State: SolidJS stores, never external state libraries
import { createStore } from "solid-js/store";

interface TabState {
  tabs: TabInfo[];
  activeTabId: string | null;
}

const [tabState, setTabState] = createStore<TabState>({
  tabs: [],
  activeTabId: null,
});
```

### Git

- Conventional commits: `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`
- Never commit secrets, API keys, or user data

### Security

- All unsafe blocks MUST have a `// SAFETY:` comment
- Use `clippy` with default lints
- Use `rustfmt` for formatting
- TypeScript strict mode, no `any` types

---

## SEARCH ENGINE URLS

```
DuckDuckGo:    https://duckduckgo.com/?q=%s
Brave Search:  https://search.brave.com/search?q=%s
Startpage:     https://www.startpage.com/do/dsearch?query=%s
Google:        https://www.google.com/search?q=%s
```

Default: DuckDuckGo

---

## DEFAULT SETTINGS

```toml
[privacy]
shield_enabled = true
third_party_cookies = "block"
first_party_cookies = "session_only"
fingerprint_resistance = true
https_only = true

[behavior]
default_search_engine = "duckduckgo"
restore_tabs_on_start = false
history_mode = "session_only"
download_location = "~/Downloads"

[appearance]
theme = "dark"
accent_color = "#6366f1"
font_size = "medium"
sidebar_position = "left"
```

---

## PHASE 1: Skeleton — A Window That Browses

**Goal:** Launch a window with a webview. Type a URL, hit enter, it navigates. Back/forward/reload work.

### Files to create (in this order):

**1. Initialize the project:**
```bash
# Use pnpm create tauri-app with SolidJS + TypeScript template
pnpm create tauri-app void-browser --template ts-solid --manager pnpm
cd void-browser
pnpm install
```

**2. Configure `src-tauri/tauri.conf.json`:**
- App name: "VoidBrowser"
- Window title: "VoidBrowser"
- Window default size: 1280×800
- Window min size: 800×600
- Decorations: true
- Resizable: true
- Disable the default Tauri menu (no File/Edit/Help menu)
- Set identifier: "com.voidbrowser.app"

**3. `src-tauri/Cargo.toml` — add dependencies:**
```toml
[dependencies]
tauri = { version = "2", features = ["unstable"] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
url = "2"
uuid = { version = "1", features = ["v4"] }
tokio = { version = "1", features = ["full"] }
```
Note: `unstable` feature enables multi-webview support needed for tabs in Phase 2.

**4. `src-tauri/src/main.rs`:**
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    void_browser_lib::run();
}
```

**5. `src-tauri/src/lib.rs`:**
- Register Tauri commands: `navigate_to`, `go_back`, `go_forward`, `reload_page`, `get_current_url`
- Create the main window
- The main window renders the SolidJS UI (toolbar, address bar)
- On app setup, create a default "content" webview inside the main window for browsing

**6. `src-tauri/src/commands.rs`:**
Implement these Tauri commands:
- `navigate_to(url: String)` — If input looks like a URL (contains "." or starts with a protocol), navigate to it. Otherwise, treat it as a search query and navigate to `https://duckduckgo.com/?q={encoded_query}`.
- `go_back()` — webview back
- `go_forward()` — webview forward
- `reload_page()` — webview reload
- `get_current_url()` — return the current URL string

**7. `src-tauri/src/browser/mod.rs`, `webview.rs`, `navigation.rs`:**
- `navigation.rs`: URL parsing logic — determine if input is a URL or search query, normalize URLs (add `https://` if no scheme), encode search queries
- `webview.rs`: Functions to create and manage webview instances. For now, just one content webview.

**8. Frontend — `src/App.tsx`:**
Layout (top to bottom):
```
┌──────────────────────────────────────┐
│ [←] [→] [↻] [🏠]  [ URL BAR        ]│  ← toolbar (SolidJS)
├──────────────────────────────────────┤
│                                      │
│        (webview renders here)        │  ← this is the Tauri webview
│                                      │
└──────────────────────────────────────┘
```
The toolbar is rendered by the SolidJS frontend. The browsing content is a separate Tauri webview positioned below the toolbar.

**9. `src/components/browser/AddressBar.tsx`:**
- Text input that shows the current URL
- On Enter: call `navigate_to` command via IPC
- On focus: select all text
- Shows a lock icon 🔒 for HTTPS pages (later phases add the shield icon)

**10. `src/components/browser/NavigationControls.tsx`:**
- Back button (←): calls `go_back` command
- Forward button (→): calls `go_forward` command
- Reload button (↻): calls `reload_page` command. Show as ✕ while page is loading.
- Home button (🏠): navigates to `about:blank` (will be void://home later)

**11. `src/lib/ipc.ts`:**
Typed wrappers around `invoke()` for every command.

**12. `src/styles/global.css`:**
- Tailwind directives
- Dark theme base: bg-neutral-900, text-neutral-100
- Toolbar: bg-neutral-800, border-b border-neutral-700
- Address bar: bg-neutral-700, rounded, text-sm, monospace for URLs
- Buttons: hover:bg-neutral-600, rounded, icon-sized (32×32)

### Architecture note — how the toolbar + webview coexist:

The SolidJS frontend (App.tsx with toolbar) renders in the **main Tauri webview** that fills the window. The browsing content needs to be **a second webview** created from Rust and positioned below the toolbar area. Use Tauri v2's multi-webview support:

From Rust, create a child webview with a y-offset (e.g., top: 40px) so it sits below the toolbar. The SolidJS toolbar communicates with Rust via IPC commands (`navigate_to`, etc.), and Rust forwards those to the content webview.

If multi-webview positioning is too complex for Phase 1, the fallback is: render the toolbar as a Tauri window decoration/overlay and use the single webview for content, communicating URL changes via IPC. But try multi-webview first — it's the foundation for tabs.

### Acceptance criteria:
- [ ] App launches in under 3 seconds
- [ ] Can type a URL and navigate to it
- [ ] Can type a search query and it goes to DuckDuckGo
- [ ] Back/forward/reload buttons work
- [ ] Window is resizable, minimum 800×600
- [ ] Dark theme looks clean and minimal
- [ ] No errors in console on launch
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo test` passes

---

## PHASE 2: Tab System

**Goal:** Multiple tabs with keyboard shortcuts. New tab, close tab, switch tabs.

### What to build:

**1. `src-tauri/src/browser/tabs.rs`:**
Tab state machine in Rust:
```rust
struct Tab {
    id: String,          // uuid
    title: String,
    url: String,
    is_loading: bool,
    favicon_url: Option<String>,
    // webview_id to reference the Tauri webview
}

struct TabManager {
    tabs: Vec<Tab>,
    active_tab_id: Option<String>,
}
```
Each tab = one content webview. Tab creation creates a new webview from Rust. Tab switching shows/hides webviews (bring active to front, hide others — use z-ordering or `set_visible`).

**2. New Tauri commands:**
- `create_tab(url: Option<String>)` → creates new tab, returns tab_id
- `close_tab(tab_id: String)` → destroys webview, removes tab
- `switch_tab(tab_id: String)` → shows that tab's webview, hides others
- `get_tabs()` → returns list of all tabs with their current state
- `reorder_tabs(tab_ids: Vec<String>)` → update ordering

**3. `src/stores/tabStore.ts`:**
SolidJS store mirroring Rust tab state. Listen for Tauri events from Rust to stay in sync:
- `tab-created` event
- `tab-closed` event
- `tab-updated` event (title change, URL change, loading state)
- `active-tab-changed` event

**4. `src/components/browser/TabBar.tsx`:**
Horizontal tab strip above the address bar:
```
┌──────────────────────────────────────┐
│ [Tab 1] [Tab 2] [Tab 3]         [+] │  ← tab bar
│ [←] [→] [↻] [🏠]  [ URL BAR       ]│  ← toolbar
├──────────────────────────────────────┤
│        (active tab's webview)        │
└──────────────────────────────────────┘
```
- Each tab shows: favicon (if available), title (truncated), close button (×)
- Active tab visually distinct (lighter background, bottom border accent color)
- [+] button creates new tab
- Tabs are horizontally scrollable if too many
- Middle-click on tab = close it
- Double-click empty tab bar space = new tab

**5. `src/components/browser/Tab.tsx`:**
Individual tab component with right-click context menu:
- Close tab
- Close other tabs
- Close tabs to the right
- Duplicate tab
- Pin tab (pinned tabs are smaller, show only favicon)

**6. Keyboard shortcuts (handle in Rust via accelerators or in frontend):**
- `Ctrl+T` → new tab
- `Ctrl+W` → close current tab
- `Ctrl+Tab` → next tab
- `Ctrl+Shift+Tab` → previous tab
- `Ctrl+1` through `Ctrl+9` → go to tab N (Ctrl+9 = last tab)
- `Ctrl+L` → focus address bar

**7. Update `AddressBar.tsx`:**
- Shows URL of the currently active tab
- Navigation commands target the active tab

**8. New tab page:**
When a new tab opens, it loads a simple HTML page (served from Tauri's asset protocol or inline) with:
- "VoidBrowser" heading
- A search bar (focuses on load, Enter navigates to search)
- Clean dark background matching the theme

### Acceptance criteria:
- [ ] Can open new tabs with Ctrl+T
- [ ] Can close tabs with Ctrl+W and the × button
- [ ] Can switch tabs by clicking, Ctrl+Tab, Ctrl+1-9
- [ ] Tab bar shows favicon and title for each tab
- [ ] Address bar updates when switching tabs
- [ ] Can open 15+ tabs without crash
- [ ] Right-click context menu works on tabs
- [ ] New tab page looks clean
- [ ] Closing the last tab closes the app (or opens a new tab — pick one)

---

## PHASE 3: Ad/Tracker Blocking

**Goal:** Block ads and trackers using Brave's adblock-rust engine on Windows via WebView2 request interception.

### What to build:

**1. Add to `Cargo.toml`:**
```toml
adblock = "0.12"
```

**2. Download and bundle filter lists in `src-tauri/resources/filter_lists/`:**
- `easylist.txt` from https://easylist.to/easylist/easylist.txt
- `easyprivacy.txt` from https://easylist.to/easylist/easyprivacy.txt

Download these once and commit them. They'll be bundled into the binary.

**3. `src-tauri/src/privacy/ad_blocker.rs`:**
```rust
use adblock::lists::FilterSet;
use adblock::Engine;

pub struct AdBlocker {
    engine: Engine,
}

impl AdBlocker {
    pub fn new() -> Self {
        let mut filter_set = FilterSet::new(true);

        // Load bundled filter lists
        let easylist = include_str!("../../resources/filter_lists/easylist.txt");
        let easyprivacy = include_str!("../../resources/filter_lists/easyprivacy.txt");

        let rules: Vec<String> = easylist.lines().chain(easyprivacy.lines())
            .map(String::from).collect();
        filter_set.add_filters(&rules, Default::default());

        let engine = Engine::from_filter_set(filter_set, true);
        Self { engine }
    }

    pub fn should_block(&self, url: &str, source_url: &str, resource_type: &str) -> bool {
        let result = self.engine.check_network_urls(url, source_url, resource_type);
        result.matched
    }
}
```

Initialize the `AdBlocker` once at app startup and store it in Tauri's managed state (`app.manage(AdBlocker::new())`).

**4. Request interception on Windows:**
This is the critical part. Tauri's `wry` library provides `with_custom_protocol` and `with_navigation_handler`, but for blocking sub-resource requests (scripts, images, XHR), you need to access WebView2's `AddWebResourceRequestedFilter` API directly.

**Approach A (preferred):** Use wry's `with_web_resource_request_handler` if available in the current version. Check the wry docs — newer versions may expose this.

**Approach B (fallback):** Access the WebView2 COM interface via the `webview2-com` crate:
```toml
[target.'cfg(windows)'.dependencies]
webview2-com = "0.38"
```
After creating the webview, get the underlying `ICoreWebView2` handle and register a `WebResourceRequested` handler that checks each request against `AdBlocker::should_block()`.

**Approach C (simplest, start here):** Inject a JavaScript content script via `with_initialization_script` that overrides `fetch()` and `XMLHttpRequest` to check URLs against a blocklist passed from Rust. Less comprehensive than native interception but works immediately.

**Start with Approach C, then upgrade to A or B.** A working JS-based blocker is better than a broken native one.

**5. `src/components/privacy/ShieldIcon.tsx`:**
Small shield icon in the address bar, right side:
- Shows number of blocked requests for current page (e.g., "12" inside shield)
- Green shield = blocking active, gray = disabled for this site
- Click opens a dropdown showing blocked request count

**6. `src/stores/privacyStore.ts`:**
Track per-tab blocked request counts. Rust emits events when requests are blocked:
```typescript
interface PrivacyState {
  blockedCounts: Record<string, number>; // tabId -> count
}
```

**7. New Tauri commands:**
- `get_blocked_count(tab_id: String)` → number of blocked requests for this tab
- `toggle_shield(tab_id: String)` → enable/disable blocking for this tab

### Acceptance criteria:
- [ ] Ads blocked on CNN, Reddit, YouTube (visible difference vs Chrome)
- [ ] Shield icon shows accurate blocked count per page
- [ ] Blocking does NOT break page functionality (pages still load and work)
- [ ] Filter engine initializes in under 500ms
- [ ] Can toggle shield off for a specific tab

---

## PHASE 4: Fingerprint Resistance

**Goal:** Inject anti-fingerprinting JavaScript into every page load.

### What to build:

**1. `src-tauri/resources/fingerprint_shield.js`:**
A comprehensive script that runs before any page JavaScript. Must cover:

**Canvas fingerprinting:**
- Override `CanvasRenderingContext2D.getImageData()` — add ±5 noise to one RGB channel
- Override `HTMLCanvasElement.toDataURL()` and `toBlob()` — add noise before export
- Use per-session per-origin seed for deterministic noise (same page visit = same fingerprint, different session = different)

**WebGL fingerprinting:**
- Override `getParameter()` for `WEBGL_debug_renderer_info` extension
- Return generic: vendor = "Google Inc. (Intel)", renderer = "ANGLE (Intel, Intel(R) UHD Graphics 630 Direct3D11 vs_5_0 ps_5_0)"

**AudioContext fingerprinting:**
- Override `OfflineAudioContext.prototype.startRendering` → add ±0.0001 noise to output buffer

**Navigator spoofing:**
- `hardwareConcurrency` → 4
- `deviceMemory` → 8
- `platform` → "Win32"
- `languages` → `["en-US", "en"]`
- `getBattery()` → rejected promise
- `getGamepads()` → empty array
- Block bluetooth, usb, serial, hid APIs

**Timing protection:**
- Reduce `performance.now()` resolution to 100μs
- Add jitter to setTimeout/setInterval

**Screen spoofing:**
- `screen.width/height` → 1920×1080
- `devicePixelRatio` → 1
- `colorDepth` → 24

**WebRTC leak prevention:**
- Override RTCPeerConnection — strip non-`.local` ICE candidates, empty iceServers

**Anti-detection:**
- All overridden functions must return `"function X() { [native code] }"` from `.toString()`
- Use `Object.defineProperty` with non-configurable, non-enumerable descriptors

**2. `src-tauri/src/privacy/fingerprint.rs`:**
- Generate a random session seed on app startup
- Inject `fingerprint_shield.js` into every webview via `with_initialization_script()`
- Pass the session seed as a variable the script reads

**3. No UI for this phase.** Fingerprint resistance is always on. (Per-site toggle comes later with site permissions.)

### Acceptance criteria:
- [ ] Script injects successfully on every page navigation including iframes
- [ ] Canvas fingerprint differs between sessions
- [ ] WebGL renderer reports the generic spoofed string
- [ ] `navigator.hardwareConcurrency` returns 4 on all sites
- [ ] Does NOT break Google, YouTube, GitHub, Reddit, Twitter
- [ ] `Function.prototype.toString` on overridden methods returns native-looking string

---

## PHASE 5: HTTPS-Only Mode + Cookie Policy

**Goal:** Enforce HTTPS everywhere. Block third-party cookies. Ephemeral first-party cookies.

### What to build:

**1. `src-tauri/src/privacy/https_only.rs`:**
- In the navigation handler, intercept HTTP URLs
- Attempt HTTPS upgrade: replace `http://` with `https://`
- If HTTPS version fails (timeout, cert error), show a warning page
- Warning page: simple HTML explaining the site isn't available over HTTPS, with a "proceed anyway" button that navigates to the HTTP version

**2. `src-tauri/src/privacy/cookie_policy.rs`:**
- Block ALL third-party cookies by default
- First-party cookies are session-only (cleared on browser close)
- On app exit: wipe all webview data directories

**3. Ephemeral storage cleanup on exit:**
In `lib.rs`, on the `RunEvent::Exit` event:
```rust
// Delete all ephemeral webview data
let data_dir = app_handle.path().app_data_dir().unwrap();
let _ = std::fs::remove_dir_all(data_dir.join("webview_data"));
```

Each webview should use `with_incognito(true)` or a temporary data directory that gets nuked on close.

**4. Per-site shield toggle UI:**
- `src/components/privacy/SitePermissions.tsx` — dropdown from shield icon
- Toggle: "Disable protection for this site"
- When disabled: no ad blocking, no fingerprint resistance, cookies allowed
- Stored in memory only (resets on browser close) for now

### Acceptance criteria:
- [ ] HTTP URLs auto-upgrade to HTTPS
- [ ] Warning page shown for HTTPS-only failures
- [ ] After closing and reopening browser: logged out of every site
- [ ] Third-party cookies never persist
- [ ] Can disable shield per-site (site works normally when disabled)

---

## PHASE 6: Encrypted Storage — Bookmarks + Settings

**Goal:** Persistent encrypted local storage for bookmarks and user preferences.

### What to build:

**1. Add to `Cargo.toml`:**
```toml
rusqlite = { version = "0.32", features = ["bundled-sqlcipher"] }
chacha20poly1305 = "0.10"
argon2 = "0.5"
zeroize = { version = "1.8", features = ["derive"] }
```

**2. `src-tauri/src/storage/database.rs`:**
- On first run, generate a random 32-byte key and store it using the OS keychain (Windows Credential Manager via `keyring` crate) — or derive from machine-specific data
- Open SQLCipher database at `{app_data_dir}/vault.db`
- Run schema migrations
- Tables: `bookmarks` (id, url, title, folder, favicon_data, created_at), `settings` (key, value)

**3. `src-tauri/src/storage/bookmarks.rs`:**
- CRUD: add_bookmark, remove_bookmark, update_bookmark, get_bookmarks, search_bookmarks
- Folder hierarchy support (folder column, root = null)

**4. `src-tauri/src/storage/settings.rs`:**
- get_setting(key) → Option<String>
- set_setting(key, value)
- Defaults defined in code, overridden by DB values

**5. `src-tauri/src/storage/history.rs`:**
- Session-only: `Vec<HistoryEntry>` in memory, never touches disk
- Provides data for address bar autocomplete and back/forward
- Cleared completely on app exit

**6. Tauri commands:**
- `add_bookmark(url, title, folder)`
- `remove_bookmark(id)`
- `get_bookmarks(folder)` → Vec<Bookmark>
- `search_bookmarks(query)` → Vec<Bookmark>
- `get_setting(key)` → Option<String>
- `set_setting(key, value)`

**7. Frontend:**
- `src/components/sidebar/BookmarkPanel.tsx` — list bookmarks, add/remove, folders
- `src/components/sidebar/Sidebar.tsx` — collapsible left sidebar with bookmark and history panels
- `src/components/settings/SettingsPage.tsx` — form for all settings (search engine, theme, privacy toggles)
- Keyboard shortcut: `Ctrl+B` toggles bookmark sidebar
- Keyboard shortcut: `Ctrl+D` bookmarks current page
- Address bar autocomplete: show matching bookmarks and history entries as you type

### Acceptance criteria:
- [ ] Bookmarks persist across browser restarts
- [ ] Database file is encrypted (unreadable without key)
- [ ] Settings persist (changing search engine survives restart)
- [ ] Session history works for address bar autocomplete
- [ ] Session history is gone after browser close
- [ ] Sidebar opens/closes smoothly

---

## PHASE 7: New Tab Page + Privacy Dashboard + Polish

**Goal:** Beautiful new tab page, privacy stats, and UI polish for a shippable product.

### What to build:

**1. New tab page (`void://home`):**
Served via Tauri's custom protocol handler. Dark themed, minimal:
- VoidBrowser logo/wordmark centered
- Search bar (large, centered, autofocused)
- Grid of bookmarks below (top 8 bookmarks as tiles with favicon + title)
- Subtle text: "X trackers blocked this session" at bottom

**2. Privacy dashboard (`void://privacy`):**
Full page showing session stats:
- Big counter: "Trackers blocked this session: [N]"
- Big counter: "Ads blocked: [N]"
- Big counter: "HTTPS upgrades: [N]"
- Top 10 blocked domains list
- All data session-only, resets on close

**3. UI polish pass:**
- Loading spinner in tab while page loads
- Favicon loading for tabs (extract from page)
- Smooth tab animations (open/close)
- Tooltip on hover for all toolbar buttons
- Focus ring styling on all interactive elements
- Proper scrollbar styling (thin, dark theme)
- Window title: "{Page Title} — VoidBrowser"
- `Ctrl+F` find-in-page (use webview's built-in find)
- `Ctrl+Plus/Minus/0` zoom controls
- Right-click context menu on pages: Back, Forward, Reload, Copy Link, Open Link in New Tab

**4. About page (`void://about`):**
- VoidBrowser version
- "Built with Tauri + Rust + SolidJS"
- MPL 2.0 license
- Link to GitHub repo
- "We collect nothing. We never will."

### Acceptance criteria:
- [ ] New tab page loads instantly and looks professional
- [ ] Privacy dashboard shows accurate session stats
- [ ] Find-in-page works (Ctrl+F)
- [ ] Zoom works
- [ ] Context menus work
- [ ] The browser feels polished enough to show other people

---

## PHASE 8: Windows Build, README, and Ship

**Goal:** Produce a downloadable Windows installer and publish to GitHub.

### What to build:

**1. App icons:**
Generate app icons for all required sizes (16, 32, 64, 128, 256, 512, 1024). Use a simple shield/void logo. Place in `src-tauri/icons/`.

**2. `src-tauri/tauri.conf.json` build config:**
- Windows: NSIS installer
- Bundle identifier: com.voidbrowser.app
- Version: 0.1.0
- Sign the binary if possible (optional for MVP)

**3. Build and test:**
```bash
pnpm tauri build
```
Test the resulting `.exe` installer on a clean Windows 10 and Windows 11 VM.

**4. `README.md`:**
- Hero: name, tagline ("Your browser. Your data. Nobody else's."), screenshot
- "What makes Void different" — 3 promises: we never know who you are, what you browse, that you exist
- Feature list: ad blocking, fingerprint resistance, encrypted bookmarks, ephemeral browsing, HTTPS-only
- Download link (GitHub Releases)
- Build from source instructions
- Privacy policy summary
- License (MPL 2.0)
- Link to CONTRIBUTING.md

**5. `PRIVACY.md`:**
Short, direct:
- "VoidBrowser collects no data. Period."
- No telemetry, no analytics, no crash reports, no accounts
- Bookmarks and settings stored locally, encrypted with SQLCipher
- Browsing data (cookies, history, cache) destroyed on exit
- No servers exist. There is nothing to subpoena, hack, or breach.
- WebView2 SmartScreen is disabled to prevent URL leakage to Microsoft

**6. `LICENSE`:**
MPL-2.0 full text

**7. `.github/workflows/build.yml`:**
GitHub Actions workflow:
- Trigger on push to main and on tags
- Build Windows binary with `pnpm tauri build`
- Upload artifacts
- On tag: create GitHub Release with the .exe installer

**8. Create first GitHub Release:**
- Tag v0.1.0
- Upload the NSIS installer .exe
- Write release notes highlighting key features

### Acceptance criteria:
- [ ] NSIS installer works on clean Windows 10 and 11
- [ ] Installed app launches and all features work
- [ ] README looks professional with screenshot
- [ ] GitHub Actions build passes
- [ ] GitHub Release is downloadable

---

## POST-MVP (DO NOT BUILD UNTIL ALL 8 PHASES ARE DONE)

These are future features. Do not implement any of these during the MVP phases:

- DNS-over-HTTPS (hickory-dns integration)
- Container tab isolation (separate WebContext per container)
- Cookie banner auto-rejection
- Browser data import (Chrome/Firefox/Brave)
- Reader mode
- Download manager with progress UI
- Split-view browsing
- Tab suspension/destruction for memory management
- Custom themes (light, midnight)
- First-run onboarding wizard
- Cosmetic ad filtering (CSS element hiding)
- macOS support
- Linux support
- Drag-to-reorder tabs
- Pinned tabs
- Tab search (Ctrl+Shift+A)
- Extensions/plugin system
- Password manager integration
- Update checker (GitHub API)

---

## REFERENCE: WebView2 SmartScreen

**CRITICAL:** WebView2 enables Microsoft Defender SmartScreen by default, which sends every URL the user visits to Microsoft's servers. This fundamentally violates our privacy promises.

Disable it when creating webviews:
```rust
// When accessing WebView2 settings, disable SmartScreen
// via ICoreWebView2Settings → put_IsBuiltInErrorPageEnabled(false)
// and environment options to disable SmartScreen
```

Research the exact API call needed. This MUST be disabled before shipping.

---

## REFERENCE: Known Tauri/WebView2 Issues

1. **Multi-webview is unstable:** Tauri v2 multi-webview requires the `unstable` feature flag. Test thoroughly. If it's too buggy, fall back to a single webview with tab content swapping (less ideal but functional).

2. **WebView2 WebResourceRequested does NOT fire for WebSocket connections.** This is a known Microsoft bug (WebView2Feedback#4303). Workaround: override the `WebSocket` constructor in JavaScript.

3. **Panics across FFI = crash.** Always use `catch_unwind` on webview callbacks. Log the error instead of crashing.

4. **WebView2 auto-updates.** The Evergreen runtime updates via Windows Update. You cannot pin a version. Test against Edge Beta/Dev/Canary for early warning of breaking changes.
