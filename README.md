<p align="center">
  <img src="./logo.png" alt="Aster logo" width="120">
</p>

# Aster

**Aster** is a macOS-first Markdown editor built in Rust on top of [GPUI](https://www.gpui.rs/), the GPU-accelerated UI framework from the Zed team. It delivers a live split view: rope-backed editing on the left, formatted preview on the right.

## Highlights
- Native macOS windowing with Metal rendering via GPUI.
- Rope-backed text model (`ropey`) for fast inserts/deletes.
- Live Markdown parse and render (CommonMark + basic inline styling).
- Light-mode UI with accent styling for headings, bullets, code blocks, and quotes.
- Atomic file saves with dirty-state tracking; open/save dialogs via `rfd`.

## Architecture (brief)
- `src/app.rs` / `src/main.rs`: window bootstrap, logging.
- `src/model/`: `DocumentState` (rope, cursor, dirty tracking), `PreviewState` (rendered blocks).
- `src/services/`: Markdown parsing into structured blocks; file I/O helpers.
- `src/ui/`: editor view (rope-backed input), preview view (styled blocks), root layout (split panes).
- `src/ui/theme.rs`: light palette and typography tokens.

## Getting Started
```bash
cargo run
```

Requirements: macOS with Metal toolchain installed (`xcode-select --install` or full Xcode).

## Current Status
- Split edit/preview layout renders headings, italics, bold, code blocks, list items, and quotes.
- Light theme applied across panels and preview components.
- Basic keyboard handling for text input and file shortcuts (Cmd+O / Cmd+S).

## Next Steps
- Richer editor UX (caret/selection visuals, IME support).
- Syntax highlighting in code blocks.
- Scroll sync between editor and preview.
- Clickable links and image rendering.
