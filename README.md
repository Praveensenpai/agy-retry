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

## Install

### Download binary

Grab the latest release for your platform:

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

---

## Usage

```bash
# start a new session
agy-retry

# resume a specific conversation
agy-retry -c <conversation-id>
```

All arguments are forwarded to `agy` as-is.

---

## Configuration

Tune the behavior with environment variables — no config file needed.

| Variable | Default | Description |
|---|---|---|
| `AGY_BIN` | `~/.local/bin/agy` | Path to your `agy` binary |
| `AGY_AUTO_MAX_RETRIES` | `3` | How many times to retry before giving up |
| `AGY_AUTO_RETRY_DELAY` | `1.0` | Seconds to wait before retrying |
| `AGY_AUTO_COOLDOWN` | `4.0` | Seconds to ignore output noise after retrying |
| `AGY_AUTO_EXTRA_PATTERNS` | _(empty)_ | Extra error strings to watch for, pipe-separated |

```bash
# retry up to 5 times with a 2s delay
AGY_AUTO_MAX_RETRIES=5 AGY_AUTO_RETRY_DELAY=2.0 agy-retry

# watch for additional error messages
AGY_AUTO_EXTRA_PATTERNS="rate limit exceeded|quota reached" agy-retry
```

---

## License

MIT
