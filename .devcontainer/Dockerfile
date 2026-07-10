FROM mcr.microsoft.com/devcontainers/base:debian

# chromium + chromium-driver stay in lockstep in Debian, which is what
# `wasm-pack test --headless --chrome` needs (see CHROMEDRIVER in devcontainer.json).
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get install -y --no-install-recommends \
       build-essential pkg-config chromium chromium-driver \
    && rm -rf /var/lib/apt/lists/*

USER vscode
RUN curl -fsSL https://mise.run | sh
ENV PATH="/home/vscode/.local/bin:/home/vscode/.local/share/mise/shims:${PATH}"
ENV MISE_TRUSTED_CONFIG_PATHS=/workspaces
RUN echo 'eval "$(mise activate bash)"' >> /home/vscode/.bashrc \
    && echo 'eval "$(mise activate zsh)"' >> /home/vscode/.zshrc

# Bake toolchains into the image; this layer only rebuilds when mise.toml changes.
COPY --chown=vscode:vscode mise.toml /home/vscode/mise-build/mise.toml
RUN cd /home/vscode/mise-build && mise trust mise.toml && mise install \
    && eval "$(mise activate bash --shims)" \
    && npm install -g @anthropic-ai/claude-code \
    # mise's shims resolve tools relative to the calling user's home dir, which
    # breaks for anything invoked as a different user (e.g. sudo, or a script
    # that assumes root). Symlink the real install directories to fixed names
    # and put those on PATH so plain `node`/`npm`/`claude`/`cargo`/`uv`/
    # `wasm-pack`/`prek` resolve correctly for any caller. (Symlinking the
    # individual binaries instead of their containing directories breaks npm's
    # own script, which computes sibling paths like `../lib/node_modules`
    # relative to its real directory.)
    && sudo mkdir -p /usr/local/lib/mise-bin \
    && sudo ln -sfn "$(dirname "$(mise which node)")" /usr/local/lib/mise-bin/node \
    && sudo ln -sfn "$(dirname "$(mise which rustc)")" /usr/local/lib/mise-bin/cargo \
    && sudo ln -sfn "$(dirname "$(mise which uv)")" /usr/local/lib/mise-bin/uv \
    && sudo ln -sfn "$(dirname "$(mise which wasm-pack)")" /usr/local/lib/mise-bin/wasm-pack \
    && sudo ln -sfn "$(dirname "$(mise which prek)")" /usr/local/lib/mise-bin/prek
ENV PATH="/usr/local/lib/mise-bin/node:/usr/local/lib/mise-bin/cargo:/usr/local/lib/mise-bin/uv:/usr/local/lib/mise-bin/wasm-pack:/usr/local/lib/mise-bin/prek:${PATH}"
