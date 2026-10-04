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

The firmware applies the mode at boot. Switching means leaving it a request and telling it to store that request.

1. The request is a 2-bit code (0 hybrid, 1 discrete, 2 integrated) in bits 0 and 1 of byte 9 of the UEFI variable `MsiDCVarData`.
2. The firmware stores it when software SMI `0x11` runs. The tool raises that SMI by writing `0x11` to I/O port `0xB2` through `/dev/port`.
3. On the next boot the firmware sets up the display mux and GPU power, and reports the mode it applied in bits 2 and 3 of the same byte.

Only the two request bits are changed. The rest of the variable is written back as it was read.

## Supported hardware

| Laptop | Board | BIOS | Status |
| --- | --- | --- | --- |
| Vector 16 HX A13VHG | MS-15M1 | E15M1IMS.90D | Switch to discrete confirmed. Hybrid and integrated not tried yet. |

Other MSI laptops with a GPU switch in MSI Center may work the same way, but they are untested. On an untested board the tool only writes with `--force` (CLI) or the confirmation checkbox (GUI). It never writes if the variable does not parse.

Keep a way to switch back (MSI Center, or a BIOS reset) until you have seen it work on your machine.

To help add a model, run `msi-gpu-mode report` in each mode and open an issue with the output.

## Requirements

- Linux booted in UEFI mode
- Secure Boot off, because kernel lockdown blocks `/dev/port`
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
active:     dgpu (NVIDIA)
requested:  dgpu
```

## GUI

Run `msi-gpu-mode-gui`, pick a mode and press Apply. The GUI itself runs unprivileged and asks for your password through polkit when it writes the request.

## Risks

This writes a firmware variable and runs a firmware SMI handler. A wrong value could leave the laptop booting with no usable display output until the BIOS settings are reset. Use it at your own risk. I only have one MSI laptop to test it on.

In integrated only mode, display outputs that are wired to the discrete GPU stop working.
