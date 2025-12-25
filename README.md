<p align="center">
  <img src="./logo.png" alt="Aster logo" width="160" style="vertical-align: middle; margin-right: 12px;">
  <span style="font-size: 28px; font-weight: 700; vertical-align: middle;">Aster</span>
</p>

**Aster** is a Markdown editor built in Rust on top of [GPUI](https://www.gpui.rs/), the GPU-accelerated UI framework from the Zed team. It delivers a live split view: rope-backed editing on the left, formatted preview on the right. The goal is to create a fast and effiecient markdown editor using the gpu rendering. Why am I using `Rope` data structure; because it is fast and it provides several benefit over the regular `String`. If you want to learn more how, you can check out this brilliant article by the `Zed` team [Rope & SumTree](https://zed.dev/blog/zed-decoded-rope-sumtree). `Zed` uses their own version of `Rope` which isn't really a rope but a `SumTree`. Currenlty I am using the regular `Rope` and if there are needs, I might switch to the `Zed` version. Why `gpui`? Because it's GPU accelerated (Metal on MacOs) and it provides precise control over the layout and save us from the hell that is virtual DOM. 


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
