# agy-retry

Ever walked away from a long `agy` session, came back, and found it sitting dead on an error screen — having done nothing for the last 30 minutes?

`agy-retry` fixes that. It watches your `agy` session and the moment it hits an error, it automatically picks back up and keeps going. You don't have to watch it. You don't have to restart it. It just works.

## The problem

`agy` sometimes fails mid-task:

```
There was a network issue connecting to the server.
```

```
Agent execution terminated due to error.
```

When that happens, you normally have to be there, notice it, and manually type `.` to resume. If you're away, the session is dead until you get back.

## The fix

Run `agy-retry` instead of `agy`. That's it.

It detects the failure, waits a moment, and sends the resume signal automatically — up to 3 times before giving up and letting you know.

## Installation

### Download binary

Grab the latest from the [Releases](https://github.com/Praveensenpai/agy-retry/releases) page.

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

Drop-in replacement for `agy`:

```bash
agy-retry
agy-retry -c <conversation-id>
```

## Configuration

| Variable | Default | What it does |
|---|---|---|
| `AGY_BIN` | `~/.local/bin/agy` | Path to your `agy` binary |
| `AGY_AUTO_MAX_RETRIES` | `3` | How many times to retry before stopping |
| `AGY_AUTO_RETRY_DELAY` | `1.0` | Seconds to wait before retrying |
| `AGY_AUTO_COOLDOWN` | `4.0` | Seconds to ignore output noise after a retry |
| `AGY_AUTO_EXTRA_PATTERNS` | _(empty)_ | Extra error strings to watch for, pipe-separated |

```bash
# Retry up to 5 times, wait 2s between each
AGY_AUTO_MAX_RETRIES=5 AGY_AUTO_RETRY_DELAY=2.0 agy-retry -c my-session

# Watch for additional error messages
AGY_AUTO_EXTRA_PATTERNS="rate limit exceeded|quota reached" agy-retry
```

## License

MIT
