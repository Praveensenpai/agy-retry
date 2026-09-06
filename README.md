<div align="center">

# agy-retry

**Stop babysitting your AI agent.**

`agy-retry` watches your `agy` session and automatically recovers from errors —
so you can walk away and come back to a finished job.

[![Release](https://img.shields.io/github/v/release/Praveensenpai/agy-retry?style=flat-square&color=orange)](https://github.com/Praveensenpai/agy-retry/releases)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)

</div>

---

## The problem

You kick off a long `agy` task, step away, and come back to this:

```
There was a network issue connecting to the server.
```

The agent has been sitting dead for 30 minutes. Nothing got done.
You type `.` to resume, step away again, and it happens again.

---

## The fix

```bash
# instead of this:
agy

# use this:
agy-retry
```

That's literally it. `agy-retry` sits between you and `agy`, watches the output,
and the moment it sees an error — it sends the resume signal automatically.

---

## ⚡ Quickstart

Install with a single command:

```bash
curl -fsSL https://raw.githubusercontent.com/Praveensenpai/agy-retry/main/install.sh | bash
```

> [!TIP]
> **Zero build dependencies required:** The script automatically detects your CPU architecture (`x86_64` or `aarch64`), pulls the pre-compiled binary from the latest GitHub Release, and installs it to `~/.local/bin/agy-retry`. If no pre-compiled binary matches, it seamlessly falls back to building from source.

<details>
<summary><b>Manual Installation</b></summary>

### Pre-compiled binaries

```bash
# Linux x86_64
curl -L https://github.com/Praveensenpai/agy-retry/releases/latest/download/agy-retry-linux-x86_64.tar.gz \
  | tar -xz && chmod +x agy-retry && mv agy-retry ~/.local/bin/

# Linux aarch64
curl -L https://github.com/Praveensenpai/agy-retry/releases/latest/download/agy-retry-linux-aarch64.tar.gz \
  | tar -xz && chmod +x agy-retry && mv agy-retry ~/.local/bin/
```

### Build from source

```bash
git clone https://github.com/Praveensenpai/agy-retry.git
cd agy-retry
cargo build --release
cp target/release/agy-retry ~/.local/bin/
```

</details>

---

## Usage

```bash
# start a new session
agy-retry

# continue the most recent conversation (just like agy -c)
agy-retry -c

# resume a specific conversation by ID
agy-retry -c <conversation-id>
agy-retry --conversation <conversation-id>
```

All other arguments and flags are forwarded to `agy` as-is.

---

## Configuration

Configure `agy-retry` via `~/.config/agy-retry/config.toml`:

```toml
# Delay in seconds before sending '.' (default: 1.0)
retry_delay = 1.0

# Path to agy binary (default: ~/.local/bin/agy)
# agy_bin = "~/.local/bin/agy"

# Extra error patterns to detect and auto-retry
extra_patterns = [
    "rate limit exceeded",
    "quota reached",
]
```

Default built-in patterns:
- `There was a network issue connecting to the server`
- `Agent execution terminated due to error`


---

## License

MIT
