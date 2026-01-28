# OmarchyISO

A TUI app for creating customized [Omarchy Linux](https://omarchy.org) ISO images with your personal dotfiles, configs and packages.

## Features

- Choose AUR and official Arch packages from your system to include
- Inject your dotfiles into the new ISO for a personalized setup
- Remove unwanted base Omarchy packages and apps

## Requirements

- Omarchy Linux (of course!)

## Installation

```bash
yay -S omarchyiso

```

Or use the install script:

```bash
curl -fsSL https://raw.githubusercontent.com/tahayvr/omarchyiso/main/install.sh | bash
```

## License

MIT

## Acknowledgements

- **'omarchyiso'** uses a fork of [omarchy-iso](https://github.com/omacom-io/omarchy-iso) for the core ISO building process.
- Made with [Rust](https://rust-lang.org/) & [Ratatui](https://github.com/ratatui/ratatui)
