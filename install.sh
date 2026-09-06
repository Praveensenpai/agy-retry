#!/usr/bin/env bash
# ==============================================================================
#  agy-retry - Remote Binary Bootstrapper
# ==============================================================================
set -euo pipefail

REPO="Praveensenpai/agy-retry"
BINARY="agy-retry"
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

echo "⚡ ========================================= ⚡"
echo "           agy-retry Installer                "
echo "⚡ ========================================= ⚡"

# Detect OS
OS="$(uname -s)"
if [ "$OS" != "Linux" ]; then
    echo "⚠️  Pre-compiled binaries are currently only available for Linux."
    echo "Attempting to build from source via Cargo..."
    FORCE_BUILD=1
else
    FORCE_BUILD=0
fi

# Detect Architecture (x86_64 or aarch64)
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64)
        ASSET_ARCH="linux-x86_64"
        ;;
    aarch64|arm64)
        ASSET_ARCH="linux-aarch64"
        ;;
    *)
        echo "❌ Architecture $ARCH is not supported for pre-compiled binaries."
        FORCE_BUILD=1
        ;;
esac

INSTALLED=0

# 1. Fetch latest release tag from GitHub and download binary
if [ "$FORCE_BUILD" -eq 0 ]; then
    echo "==> Checking latest release on GitHub..."
    TAG=$(curl -4 -sSL -H "Cache-Control: no-cache" -H "Pragma: no-cache" "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' || true)

    if [ -n "$TAG" ]; then
        DOWNLOAD_URL="https://github.com/$REPO/releases/download/$TAG/${BINARY}-${ASSET_ARCH}.tar.gz"
        echo "==> Downloading pre-compiled ${BINARY} (${ASSET_ARCH} - $TAG)..."
        TMP_DIR=$(mktemp -d)
        trap 'rm -rf "$TMP_DIR"' EXIT

        if curl -4 -fsSL "$DOWNLOAD_URL" | tar -xz -C "$TMP_DIR" 2>/dev/null; then
            if [ -f "$TMP_DIR/$BINARY" ]; then
                install -m 755 "$TMP_DIR/$BINARY" "$INSTALL_DIR/$BINARY"
                INSTALLED=1
            elif [ -f "$TMP_DIR/${BINARY}-${ASSET_ARCH}" ]; then
                install -m 755 "$TMP_DIR/${BINARY}-${ASSET_ARCH}" "$INSTALL_DIR/$BINARY"
                INSTALLED=1
            fi
        fi

        if [ "$INSTALLED" -eq 1 ]; then
            echo "✔ Successfully installed ${BINARY} to $INSTALL_DIR/$BINARY"
        else
            echo "⚠️  Failed to download release binary. Falling back to building from source..."
        fi
    fi
fi

# 2. Fallback: Build from source if binary installation didn't succeed
if [ "$INSTALLED" -eq 0 ]; then
    echo "==> Building from source..."
    TARGET_DIR="$(mktemp -d)"
    trap 'rm -rf "$TARGET_DIR"' EXIT

    if ! command -v git &>/dev/null; then
        echo "❌ git is required to build from source. Please install git."
        exit 1
    fi

    git clone --depth 1 "https://github.com/$REPO.git" "$TARGET_DIR"
    cd "$TARGET_DIR"

    if ! command -v cargo &>/dev/null; then
        echo "==> Cargo not found. Installing minimal Rust toolchain..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable --no-modify-path
        if [ -f "$HOME/.cargo/env" ]; then
            # shellcheck source=/dev/null
            source "$HOME/.cargo/env"
        fi
    fi

    echo "==> Compiling ${BINARY} with Cargo..."
    cargo build --release
    install -m 755 "target/release/$BINARY" "$INSTALL_DIR/$BINARY"
    echo "✔ Compiled and installed ${BINARY} to $INSTALL_DIR/$BINARY"
fi

# Ensure ~/.local/bin is in PATH for this session and persistent in shell profiles
case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *) export PATH="$INSTALL_DIR:$PATH" ;;
esac

add_path_to_rc() {
    local rc_file="$1"
    if [ -f "$rc_file" ] && ! grep -q '\.local/bin' "$rc_file"; then
        printf '\n# User local binaries\nexport PATH="$HOME/.local/bin:$PATH"\n' >> "$rc_file"
    fi
}

add_path_to_rc "$HOME/.bashrc"
add_path_to_rc "$HOME/.zshrc"

# Optional: Symlink to /usr/local/bin if root or passwordless sudo
if [ "${EUID:-$(id -u)}" -eq 0 ]; then
    ln -sf "$INSTALL_DIR/$BINARY" "/usr/local/bin/$BINARY" 2>/dev/null || true
elif command -v sudo &>/dev/null && sudo -n true 2>/dev/null; then
    sudo ln -sf "$INSTALL_DIR/$BINARY" "/usr/local/bin/$BINARY" 2>/dev/null || true
fi

echo ""
echo "🎉 Installation complete!"
echo "Run '${BINARY}' or '${BINARY} -c <conversation-id>' to start."
