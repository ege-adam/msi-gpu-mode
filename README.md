# msi-gpu-mode

Switch the GPU mode of MSI laptops from Linux, without booting Windows and MSI Center.

It comes as a command line tool (`msi-gpu-mode`) and a small GUI (`msi-gpu-mode-gui`).

## Modes

| Mode | CLI name | Effect |
| --- | --- | --- |
| Hybrid | `hybrid` | Integrated GPU drives the display, discrete GPU on demand |
| Discrete only | `dgpu` | Display wired to the discrete GPU, integrated GPU off |
| Integrated only | `igpu` | Discrete GPU powered off |

A change takes effect after a reboot.

## How it works

MSI Center stores the selected mode in the UEFI variable `MsiDCVarData`. The firmware reads it at boot and sets up the display mux and GPU power accordingly.

The mode is a 2-bit code (0 hybrid, 1 discrete, 2 integrated) kept twice in the low nibble of one byte: `0x30`, `0x35` and `0x3a`. This tool rewrites that nibble and leaves every other byte as it was.

The values were found by switching modes in MSI Center and comparing the firmware variables and the EC memory after each reboot.

## Supported hardware

| Laptop | Board | Status |
| --- | --- | --- |
| Vector 16 HX A13VHG | MS-15M1 | Values confirmed against MSI Center |

Other MSI laptops with a GPU switch in MSI Center probably use the same variable, but they are untested. On an untested board the tool only writes when asked to with `--force` (CLI) or the confirmation checkbox (GUI), and it never writes if the variable does not parse.

Note that a switch written from Linux and then applied by a reboot still needs more testing, also on the verified board. Keep a way to switch back (MSI Center, or a BIOS reset) until you have seen it work on your machine.

To help add a model, run `msi-gpu-mode report` in each mode and open an issue with the output.

## Requirements

- Linux booted in UEFI mode
- Rust 1.85 or newer to build
- `pkexec` (polkit) for the GUI
- Optional: the `msi-ec` kernel module, used to show the EC firmware version

## Build

```
cargo build --release
```

The binaries end up in `target/release`. Keep them in the same directory, or put `msi-gpu-mode` in your `PATH`, so the GUI can find the CLI.

```
sudo install -m 755 target/release/msi-gpu-mode target/release/msi-gpu-mode-gui /usr/local/bin/
```

## CLI

```
msi-gpu-mode status
sudo msi-gpu-mode set dgpu
sudo msi-gpu-mode set igpu --dry-run
msi-gpu-mode report
```

`status` and `report` only read and do not need root.

```
$ msi-gpu-mode status
Vector 16 HX A13VHG (MS-15M1, BIOS E15M1IMS.90D, EC 15M1IMS2.107)
support:    verified
active:     hybrid (Intel + NVIDIA)
next boot:  hybrid
```

## GUI

Run `msi-gpu-mode-gui`, pick a mode and press Apply. The GUI itself runs unprivileged and asks for your password through polkit when it writes the variable.

## Risks

This writes a firmware variable. A wrong value could leave the laptop booting with no usable display output until the BIOS settings are reset. Use it at your own risk.

In integrated only mode, display outputs that are wired to the discrete GPU stop working.
