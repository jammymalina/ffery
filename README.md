# ffery 🦀

**ffery** (short for *file f✨ery*, use with caution!) is a small command-line utility written in Rust designed for performing bulk operations on files within a directory.

⚠️ **Warning:** This tool modifies files directly on your filesystem based on the commands given. Operations might be irreversible. **Always back up your data before using `ffery` or test it in a safe, non-critical directory first.**

## Features

Currently, `ffery` supports the following command:

*   **`remove-prefix`**: Removes a specified prefix from filenames matching a given extension within a target directory.

## Installation

1.  **Prerequisites:** Ensure you have the Rust toolchain installed (see [rustup.rs](https://rustup.rs/)).
2.  **Clone the repository:**
    ```bash
    git clone https://github.com/jammymalina/ffery
    cd ffery
    ```
3.  **Build and Install:**
    ```bash
    cargo install --path .
    ```
    This will build the `ffery` binary and place it in your Cargo binary path (`~/.cargo/bin/` by default), making it available system-wide.

## Usage

`ffery` is run from the command line.

### General Help

To see the available commands and general options:
```bash
ffery --help
