# clp

A fast, cross-platform CLI tool for copying exact parts of files and streams directly to your clipboard.

## Why `clp`?

I built `clp` because I frequently needed to copy specific contents from files and paste them elsewhere. My previous workflow was frustrating: I either had to open an editor just to copy a few lines, or `cat` the file in the terminal and awkwardly highlight and copy text with the mouse—often capturing unwanted line breaks, line numbers, or terminal artifacts. 

`clp` solves this by giving you a simple, precise way to select, slice, and send content straight to the system clipboard from the command line.

> **Note:** This tool was entirely built and reviewed with AI—all I did was specify the features to implement and perform a brief scan of the implementation.

---

## Features

* **Flexible Input:** Copy entire files or pipe content directly from `stdin`.
* **Line Slicing:**
  * Take the first $N$ lines (`-l N`).
  * Copy inclusive line ranges (`-r START:END`), including open-ended ranges like `5:` (line 5 to end) or `:10` (start to line 10).
* **Delimiter Slicing:**
  * Copy up to delimiters (`-u DELIM...` exclusive, `-i DELIM...` inclusive).
  * Supports multiple delimiters, cutting at whichever matches first.
* **Composable Modifiers:**
  * **Append (`-a`):** Append to existing clipboard content with smart newline handling.
  * **Trim (`-t`):** Strip leading and trailing whitespace before copying.
  * **Verbose (`-v`):** Inspect line counts, byte counts, and a clean preview of copied content.
* **Clipboard Utilities:**
  * Paste directly to stdout (`-p`).
  * Clear clipboard history/content (`-c`).
* **Content Fidelity:** Preserves original line endings (CRLF and LF) and trailing newlines exactly as they appear in the source.
* **Cross-Platform:** Works on Linux (Wayland via data-control and X11 via pure Rust x11rb/xclip), macOS, and Windows.

---

## Installation

### Prerequisites

* [Rust](https://rustup.rs/) (1.85 or newer)
* **Linux Users:**
  * **Wayland:** Compositor clipboard support or tools like `wl-clipboard` (`wl-copy`/`wl-paste`), `clipman`, or `cliphist`.
  * **X11:** `xclip` (recommended) or a clipboard manager like `copyq`, `gpaste`, or `clipit`.

### Install from Source

Clone the repository and install the binary with Cargo:

```bash
git clone https://github.com/ajaka/clp.git
cd clp
cargo install --path . --force
```

This compiles an optimized release build and installs the `clp` executable into `$HOME/.cargo/bin` (make sure `~/.cargo/bin` is in your `PATH`).

---

## Usage

```bash
clp [OPERATION] [MODIFIERS] [FILE]
```

### Operations (Mutually Exclusive)

| Flag | Description |
|---|---|
| *(none)* | Copy the entire file or stdin (default) |
| `-l <N>` | Copy the first `N` lines |
| `-r <START:END>` | Copy inclusive 1-indexed line range (`5:15`, `5:`, `:10`) |
| `-u <DELIM...>` | Copy up to the first delimiter (delimiter excluded) |
| `-i <DELIM...>` | Copy up to the first delimiter (delimiter included) |
| `-p` | Write clipboard content to stdout |
| `-c` | Clear the clipboard |
| `--help` | Display usage information |

### Modifiers (Composable)

| Flag | Description |
|---|---|
| `-a` | Append to existing clipboard content instead of replacing it |
| `-t` | Trim surrounding whitespace before copying |
| `-v` | Display a summary report of what was copied |

---

## Examples

### Basic Copying
```bash
# Copy an entire file
clp src/main.rs

# Copy directly from a pipeline
curl -s https://example.com | clp
```

### Copying Lines and Ranges
```bash
# Copy the first 20 lines of a file
clp -l 20 server.log

# Copy lines 15 through 30
clp -r 15:30 config.toml

# Copy from line 50 to the end of the file
clp -r 50: script.py

# Copy from the start up to line 10
clp -r :10 script.py
```

### Delimiters & Composing Modifiers
```bash
# Copy content until the string "BEGIN_TESTS"
clp -u "BEGIN_TESTS" test_suite.rs

# Copy until "STOP" or "END", trim whitespace, and append to clipboard
clp -u "STOP" "END" -t -a notes.txt

# Copy trimmed lines and report statistics
clp -r 1:50 -t -v Cargo.lock
```

### Pasting and Clearing
```bash
# Paste current clipboard to terminal or file
clp -p > pasted.txt

# Clear clipboard
clp -c
```

---

## License

This project is licensed under the [MIT License](LICENSE).
