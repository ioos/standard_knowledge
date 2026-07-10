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
RUN cd /home/vscode/mise-build && mise trust mise.toml && mise install
