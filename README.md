# Synctui

[![GitHub License](https://img.shields.io/github/license/hertelukas/synctui)](./LICENSE-MIT)
[![CI](https://github.com/hertelukas/synctui/workflows/CI/badge.svg)](https://github.com/hertelukas/synctui/actions?query=workflow%3ACI)

> [!WARNING]
> Synctui is under active development. Everything that works, should be correct,
> but it's not heavily tested yet.

> [!NOTE]
> - Not affiliated with the Syncthing Foundation.
> - Contributions are welcome!

---

**Synctui** lets you control [Syncthing](https://syncthing.net) from your terminal — no need to open a browser. Perfect for headless setups like servers or Raspberry Pis. Skip the port forwarding and get syncing.

It already supports most essential features, so you can manage devices and folders pretty comfortably. That said, don’t uninstall the Syncthing GUI just yet — advanced features are still on the roadmap.

## 🚀 Installation
1. **Install Rust and Cargo** (if you haven't already):
``` bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. **Install Synctui**
```bash
cargo install --git https://github.com/hertelukas/synctui
```

3. **Configure Synctui:**

Create a config.toml in your system's default config directory.
On Linux, for example:

``` bash
~/.config/synctui/config.toml
```

With this content:
``` toml
api-key="your-api-key"
```

To find your API key (on Linux):

``` bash
cat ~/.config/syncthing/config.xml | grep apikey
```

4. **Run the app:**

``` bash
synctui
```

## 📌 Roadmap
- [x] Accept incoming devices
- [x] Accept incoming folders
- [x] Share new folders
- [x] Show device ID as a QR code
- [x] Modify/delete folders
- [ ] Modify/delete devices
- [ ] Ignore folders/devices
- [ ] Live sync status & updates (WIP)

