# motia-cli

CLI for scaffolding Motia projects with support for Node.js, Python, or mixed language templates.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/MotiaDev/motia-cli/main/install.sh | sh
```

Or via Homebrew (after the tap is set up):

```bash
brew tap MotiaDev/tap
brew install motia-cli
```

## Usage

```bash
motia-cli create [project-name]
```

Interactive prompts will guide you through:
- Project folder name
- Language selection (Node.js, Python, or Mixed)
- iii installation check

## Prerequisites

- **iii** - The Motia runtime. Install with: `curl -fsSL https://install.iii.dev/iii/main/install.sh | sh`
- **Node.js** (for Node.js or Mixed) - Node.js 18+
- **Python** (for Python or Mixed) - Python 3.10+, [uv](https://docs.astral.sh/uv/)
