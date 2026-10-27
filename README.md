# 💥 nuke

A fast, interactive directory-cleaner CLI for developers, written in Rust.

Instead of manually digging into stale project folders or waiting for Windows Explorer / `rm -rf` to choke on 200,000 tiny files inside a `node_modules` directory:

```bash
nuke
```

`nuke` recursively scans from your current path (or a specified directory), matches throwaway build/cache directories against a known roster, calculates their disk footprint in parallel, prints a breakdown table, and asks for confirmation before deleting them.

---

## 📦 Installation

### Pre-built Binaries (No Rust Required)
Zero-dependency, standalone executables (< 1 MB) are available for Windows, macOS, and Linux on the **[Releases](https://github.com/zzz-nayan/nuke/releases)** page.

1. Download the archive for your operating system:
   - **Windows:** `nuke-windows-x86_64.zip`
   - **macOS (Apple Silicon):** `nuke-macos-aarch64.tar.gz`
   - **macOS (Intel):** `nuke-macos-x86_64.tar.gz`
   - **Linux:** `nuke-linux-x86_64.tar.gz`
2. Extract the `nuke` executable and place it anywhere in your system's `PATH`.

### From Source (Requires Rust)
If you have Cargo installed:
```bash
cargo install --git https://github.com/zzz-nayan/nuke.git
```

---

## 🎯 Target Ecosystems

| Ecosystem | Directories | Sibling Safety Checks |
| :--- | :--- | :--- |
| **Node / Web** | `node_modules`, `.next`, `.nuxt`, `.turbo`, `dist`, `build` | Generic folders verified against `package.json` / build configs |
| **C# / .NET** | `bin`, `obj`, `.vs` | Verified against `*.csproj`, `*.sln`, or build props |
| **Unity** | `Library`, `Temp`, `Obj`, `Build`, `Builds`, `Logs`, `MemoryCaptures` | Verified against `Assets/` or `ProjectSettings/` |
| **Python** | `__pycache__`, `.pytest_cache`, `.venv`, `env` | Verified against `pyvenv.cfg` or Python configs |
| **Rust / C++** | `target`, `out` | Verified against `Cargo.toml` or C/C++ build manifests |
| **Custom** | Any folder specified via `-t, --target` | Explicit user override |

---

## 🚀 CLI Options

```text
Usage: nuke [OPTIONS] [PATH]

Arguments:
  [PATH]  Directory to scan (defaults to current directory)

Options:
  -y, --yes              Skip confirmation and delete everything discovered
  -d, --depth <DEPTH>    Maximum directory traversal depth [default: 4]
      --dry-run          Scan and report only; delete nothing
  -t, --target <TARGET>  Extra folder names to target, comma separated
  -f, --force            Force scanning filesystem root or home directory
  -h, --help             Print help
  -V, --version          Print version
```

### Examples

```bash
# Scan current directory interactively
nuke

# Scan a specific directory
nuke D:/Projects

# Preview reclaimable space without deleting anything
nuke --dry-run

# Delete immediately without prompt (useful in CI / scripts)
nuke -y

# Traversal depth of 2 levels
nuke -d 2

# Add custom target directory names
nuke -t "cache,.cache,artifacts"
```

---

## 🛡️ Safety Features
- **Sibling Verification:** Prevents deleting folders named `build`, `bin`, or `Library` unless project indicators (`package.json`, `.csproj`, `Assets/`, etc.) exist.
- **Symlink Protection:** Never follows symlinks or junctions, and refuses to delete links pointing outside the scan root.
- **Root Protection:** Refuses to scan filesystem root (`C:\`, `/`) or user home directory without `--force`.
- **Self-Preservation:** Detects if a directory contains the actively executing `nuke` process and skips it to avoid Windows OS lock errors.
- **Windows Read-Only Overrides:** Automatically clears Windows read-only flags (e.g. Git/package caches) when deleting.
