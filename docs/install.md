# Install OptiDock

OptiDock currently installs as a Rust CLI binary.

## Quick install

From the repository root:

```bash
chmod +x scripts/install.sh
./scripts/install.sh
```

The installer will:

- build the CLI with `cargo build --release`
- copy the `optidock` binary into `~/.local/bin`
- create `~/.optidock/.env` from `.env.example`
- add `~/.local/bin` to `~/.zshrc` if needed

## Requirements

- Rust and Cargo installed
- a working shell profile such as `~/.zshrc`

## Configure providers

Edit:

```bash
~/.optidock/.env
```

Supported provider styles:

- OpenAI
- Anthropic / Claude
- Gemini
- OpenRouter
- Ollama
- local OpenAI-compatible endpoints

## Verify installation

```bash
optidock providers
optidock analyze .
optidock pipeline .
```

## Uninstall

```bash
chmod +x scripts/uninstall.sh
./scripts/uninstall.sh
```
