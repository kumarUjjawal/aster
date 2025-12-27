<div align="center">
<tr style="border: none;">
<td width="180px" style="border: none;">
<img src="./logo.png" alt="Aster logo" width="160"/>
</td>
<td style="border: none; padding-left: 20px;">
<h1 align="center" style="font-size: 3em;">Aster</h1>
</td>
</tr>
</table>
</div>

**Aster** is a Markdown editor built in Rust on top of [GPUI](https://www.gpui.rs/), the GPU-accelerated UI framework from the [Zed](https://zed.dev/) team. It delivers a live split view: rope-backed editing on the left, formatted preview on the right. The goal is to create a fast and efficient markdown editor using GPU rendering. Why am I using `Rope` data structure? Because it is fast and provides several benefits over the regular `String`. If you want to learn more, check out this brilliant article by the `Zed` team: [Rope & SumTree](https://zed.dev/blog/zed-decoded-rope-sumtree). `Zed` uses their own version of `Rope` which isn't really a rope but a `SumTree`. Currently, I am using the regular `Rope` and if there are needs, I might switch to the `Zed` version. Why `gpui`? Because it's GPU accelerated (Metal on macOS) and it provides precise control over the layout, saving us from the hell that is virtual DOM.

---

## Download

Pre-built macOS applications are available for direct download:

| Architecture | Download |
|--------------|----------|
| **Apple Silicon** (M1/M2/M3/M4) | [Aster-arm64.dmg](https://github.com/kumarujjawal/aster/releases/latest/download/Aster-arm64.dmg) |
| **Intel** (Core i5/i7/i9) | [Aster-x86_64.dmg](https://github.com/kumarujjawal/aster/releases/latest/download/Aster-x86_64.dmg) |

### Installation

1. Download the `.dmg` file for your Mac's architecture
2. Double-click the DMG to mount it
3. Drag **Aster.app** to your **Applications** folder
4. Eject the DMG

> **Note**: On first launch, you may need to right-click → **Open** to bypass Gatekeeper (the app is not notarized yet).

---

## Highlights

- Native macOS windowing with Metal rendering via GPUI
- Rope-backed text model (`ropey`) for fast inserts/deletes
- Live Markdown parse and render (CommonMark + GFM extensions)
- Support for tables, footnotes, strikethrough, and task lists
- Image loading (local)
- File explorer sidebar with folder navigation
- Atomic file saves with dirty-state tracking; open/save dialogs via `rfd`

---

## Building from Source

### Requirements

| Requirement | Details |
|-------------|---------|
| **macOS** | 11.0 (Big Sur) or later |
| **Xcode Command Line Tools** | `xcode-select --install` |
| **Rust (Nightly)** | Edition 2024 requires nightly toolchain |

### Step 1: Install Rust (if not already installed)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```


```bash
cargo run
```

Open a file on launch:

```bash
cargo run -- path/to/file.md
```

### Step 4: Build a macOS `.app` bundle (optional)

Install `cargo-bundle`:

```bash
cargo install cargo-bundle
```

Build the app bundle:

```bash
cargo bundle --release
```

The `.app` will be created at `target/release/bundle/osx/Aster.app`.

---

## Cross-Architecture Builds

To build for both Apple Silicon and Intel Macs:

### For Apple Silicon (M1/M2/M3/M4):

```bash
rustup target add aarch64-apple-darwin
cargo bundle --release --target aarch64-apple-darwin
```

### For Intel Macs:

```bash
rustup target add x86_64-apple-darwin
cargo bundle --release --target x86_64-apple-darwin
```

---

## Architecture (brief)

- `src/app.rs` / `src/main.rs`: window bootstrap, logging
- `src/model/`: `DocumentState` (rope, cursor, dirty tracking), `PreviewState` (rendered blocks)
- `src/services/`: Markdown parsing into structured blocks; file I/O helpers
- `src/ui/`: editor view (rope-backed input), preview view (styled blocks), root layout (split panes)
- `src/ui/theme.rs`: light palette and typography tokens

---

## Current Status

- Split edit/preview layout renders headings, italics, bold, code blocks, list items, quotes, and tables
- Light theme applied across panels and preview components
- Keyboard shortcuts for text input and file operations (Cmd+O / Cmd+S)
- File explorer sidebar for navigating markdown files
- Image rendering (local)
- Footnotes support with navigation

---

## License

MIT License - see [LICENSE](LICENSE) for details.
