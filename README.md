# VoidBrowser
** made with claude. **
**Your browser. Your data. Nobody else's.**


A zero-tracking privacy browser for Windows. VoidBrowser collects nothing — no telemetry, no analytics, no accounts, no cloud. The browser binary is the entire product. All user data lives on your machine, encrypted, and dies when you say so.

## What makes Void different

1. **We never know who you are.** No accounts, no sign-ins, no identifiers of any kind.
2. **We never know what you browse.** No telemetry, no analytics, no crash reports. Zero outbound requests to our servers — because we have no servers.
3. **We never know you exist.** There is no registration, no license activation, no update ping. Download, run, done.

## Features

- **Ad and tracker blocking** — Powered by Brave's adblock engine with EasyList and EasyPrivacy filters. Blocks ads, trackers, and other unwanted requests before they load.
- **Fingerprint resistance** — Spoofs canvas, WebGL, AudioContext, navigator properties, screen dimensions, and more. Each session generates a unique noise seed so your fingerprint changes every time.
- **HTTPS-only mode** — Automatically upgrades HTTP connections to HTTPS. Shows a warning page when a secure connection isn't available.
- **Encrypted bookmarks and settings** — Stored locally in a SQLCipher-encrypted database. The key lives in your OS credential manager, never leaves your machine.
- **Ephemeral browsing** — Cookies, history, and cache are destroyed on exit. Every session starts clean. No "incognito mode" needed — it's the default.
- **WebRTC leak prevention** — Strips non-local ICE candidates to prevent IP leaks through WebRTC.
- **No SmartScreen** — Disables WebView2's built-in SmartScreen to prevent URL leakage to Microsoft.

## Download

Download the latest installer from [GitHub Releases](https://github.com/glebschkv/voidbrowser/releases).

## Build from source

### Prerequisites

- [Node.js](https://nodejs.org/) 20.x
- [pnpm](https://pnpm.io/) 10.x
- [Rust](https://rustup.rs/) stable toolchain
- Windows 10/11 with [WebView2 runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

### Steps

```bash
git clone https://github.com/glebschkv/voidbrowser.git
cd voidbrowser
pnpm install
pnpm tauri build
```

The installer will be in `src-tauri/target/release/bundle/nsis/`.

## Privacy

VoidBrowser collects no data. Period. No telemetry, no analytics, no crash reports, no accounts. We have no servers — there is nothing to subpoena, hack, or breach. See [PRIVACY.md](PRIVACY.md) for the full policy.

## Tech stack

| Layer | Technology |
|-------|-----------|
| App framework | Tauri v2 |
| Backend | Rust |
| Frontend | SolidJS + TypeScript |
| CSS | Tailwind CSS 4 |
| Ad blocking | adblock (Brave's engine) |
| Encrypted storage | SQLCipher |
| Encryption | ChaCha20-Poly1305 + Argon2 |

## License

[Mozilla Public License 2.0](LICENSE)
