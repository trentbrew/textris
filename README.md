# Textris

A terminal-based Tetris clone written in Rust using [ratatui](https://github.com/ratatui-org/ratatui).

<img width="1930" height="2458" alt="CleanShot 2026-01-07 at 09 38 56@2x" src="https://github.com/user-attachments/assets/bb75a2f1-883a-4b2d-8c4d-8d079add1b0b" />

## Overview

-   **Classic Tetris Gameplay**: Complete with 7 tetromino shapes, rotation, and line clearing.
-   **Modern Mechanics**:
    -   **Ghost Piece**: See exactly where your piece will land.
    -   **Hold Piece**: Save a piece for later (`c` key).
    -   **Lock Delay**: Grace period to slide pieces into place.
    -   **Wall Kicks**: Basic rotation handling near walls.
    -   **7-Bag Randomizer**: True random distribution of pieces.
-   **Sound Effects**: Synthesized retro sound effects (no external assets required).
-   **Combo System**: Score bonuses for consecutive line clears.
-   **Dynamic Resizing**: Game board adapts to your terminal window height.
-   **Smooth Controls**: Input handling designed to prevent stuttering.

## Controls

| Key | Action |
| :--- | :--- |
| `←` / `h` | Move Left |
| `→` / `l` | Move Right |
| `↓` / `j` | Soft Drop |
| `↑` / `k` | Rotate Clockwise |
| `z` | Rotate Counter-Clockwise |
| `Space` | Hard Drop |
| `c` | Hold Piece |
| `p` | Pause |
| `q` | Quit |
| `r` | Restart |

## Installation

### From Source

Ensure you have Rust installed (get it at [rustup.rs](https://rustup.rs)).

```bash
git clone https://github.com/trentbrew/textris.git
cd textris
cargo run --release
```

## License

MIT
