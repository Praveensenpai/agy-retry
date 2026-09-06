# agy-retry

A transparent terminal wrapper for [`agy`](https://github.com/google-deepmind/antigravity) that automatically detects errors and retries the session — so your agent keeps running without you babysitting it.

## How it works

`agy-retry` forks `agy` inside a PTY, monitors its output in real-time, and when it detects a failure message (e.g. network error, agent execution terminated), it automatically sends `.` + Enter to resume — just like you would manually.

```
┌─────────────┐        PTY        ┌──────────────┐
│   Terminal  │ ◄────────────────► │  agy-retry   │
└─────────────┘                    └──────┬───────┘
                                          │ monitors output
                                          ▼
                                   ┌──────────────┐
                                   │     agy      │
                                   └──────────────┘
```

## Features

- 🔁 **Auto-retry** — detects errors and resumes with `.` + Enter automatically
- 🪟 **Transparent PTY** — fully interactive, passes through all input/output
- 📐 **Terminal resize** — syncs `SIGWINCH` / window size to the child process
- ⚙️ **Configurable** — tunable via environment variables
- 🔗 **Conversation forwarding** — supports `-c` / `--conversation` flags

## Installation

### Download binary (recommended)

Grab the latest release for your platform from the [Releases](https://github.com/Praveensenpai/agy-retry/releases) page.

```bash
# Linux x86_64
curl -L https://github.com/Praveensenpai/agy-retry/releases/latest/download/agy-retry-linux-x86_64.tar.gz | tar -xz
chmod +x agy-retry
mv agy-retry ~/.local/bin/
```

### Build from source

```bash
git clone https://github.com/Praveensenpai/agy-retry.git
cd agy-retry
cargo build --release
cp target/release/agy-retry ~/.local/bin/
```

## Usage

Use `agy-retry` as a drop-in replacement for `agy`:

```bash
# Start a new session
agy-retry

# Resume a specific conversation
agy-retry -c <conversation-id>
agy-retry --conversation <conversation-id>
```

## Configuration

| Environment Variable       | Default                    | Description                                      |
|---------------------------|----------------------------|--------------------------------------------------|
| `AGY_BIN`                 | `~/.local/bin/agy`         | Path to the `agy` binary                         |
| `AGY_AUTO_MAX_RETRIES`    | `3`                        | Max consecutive retries before pausing           |
| `AGY_AUTO_RETRY_DELAY`    | `1.0`                      | Seconds to wait before sending retry             |
| `AGY_AUTO_COOLDOWN`       | `4.0`                      | Seconds to ignore redraws after a retry          |
| `AGY_AUTO_EXTRA_PATTERNS` | _(empty)_                  | Pipe-separated extra error patterns to detect    |

### Example

```bash
AGY_AUTO_MAX_RETRIES=5 AGY_AUTO_RETRY_DELAY=2.0 agy-retry -c my-session
```

### Custom error patterns

```bash
AGY_AUTO_EXTRA_PATTERNS="rate limit exceeded|quota reached" agy-retry
```

## License

MIT
