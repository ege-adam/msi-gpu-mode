use crate::{Error, Mode, ModeVar};
use std::fs;

const DMI: &str = "/sys/class/dmi/id";
const MSI_EC: &str = "/sys/devices/platform/msi-ec";
const PCI: &str = "/sys/bus/pci/devices";

// Boards where switching through MsiDCVarData was confirmed against MSI Center.
const VERIFIED_BOARDS: &[&str] = &["MS-15M1"];

#[derive(Debug, Clone)]
pub struct Machine {
    pub vendor: String,
    pub product: String,
    pub board: String,
    pub bios: String,
    pub ec_firmware: Option<String>,
}

impl Machine {
    pub fn detect() -> Self {
        Self {
            vendor: read(&format!("{DMI}/sys_vendor")).unwrap_or_default(),
            product: read(&format!("{DMI}/product_name")).unwrap_or_default(),
            board: read(&format!("{DMI}/board_name")).unwrap_or_default(),
            bios: read(&format!("{DMI}/bios_version")).unwrap_or_default(),
            ec_firmware: read(&format!("{MSI_EC}/fw_version")),
        }
    }

    pub fn is_msi(&self) -> bool {
        self.vendor.starts_with("Micro-Star")
    }

    pub fn is_verified(&self) -> bool {
        VERIFIED_BOARDS.contains(&self.board.as_str())
    }

    pub fn describe(&self) -> String {
        let mut s = format!("{} ({}, BIOS {}", self.product, self.board, self.bios);
        if let Some(ec) = &self.ec_firmware {
            s.push_str(&format!(", EC {ec}"));
        }
        s.push(')');
        s
    }
}

#[derive(Debug, Clone)]
pub struct Gpu {
    pub address: String,
    pub vendor: u16,
    pub integrated: bool,
}

impl Gpu {
    pub fn vendor_name(&self) -> &'static str {
        match self.vendor {
            0x8086 => "Intel",
            0x10de => "NVIDIA",
            0x1002 => "AMD",
            _ => "unknown",
        }
    }
}

pub fn gpus() -> Vec<Gpu> {
    let Ok(dir) = fs::read_dir(PCI) else {
        return Vec::new();
    };
    let mut found: Vec<Gpu> = dir
        .flatten()
        .filter_map(|entry| {
            let address = entry.file_name().into_string().ok()?;
            let class = read_hex(&format!("{PCI}/{address}/class"))?;
            if class >> 16 != 0x03 {
                return None;
            }
            let vendor = read_hex(&format!("{PCI}/{address}/vendor"))? as u16;
            // The integrated GPU sits on the root bus, a discrete one behind a bridge.
            let integrated = address.split(':').nth(1) == Some("00");
            Some(Gpu { address, vendor, integrated })
        })
        .collect();
    found.sort_by(|a, b| a.address.cmp(&b.address));
    found
}

pub fn active_mode(gpus: &[Gpu]) -> Option<Mode> {
    let integrated = gpus.iter().any(|g| g.integrated);
    let discrete = gpus.iter().any(|g| !g.integrated);
    match (integrated, discrete) {
        (true, true) => Some(Mode::Hybrid),
        (false, true) => Some(Mode::Discrete),
        (true, false) => Some(Mode::Integrated),
        (false, false) => None,
    }
}

// Empty means the machine matches what was tested.
pub fn concerns(machine: &Machine, var: &ModeVar) -> Vec<&'static str> {
    let mut reasons = Vec::new();
    if !machine.is_verified() {
        reasons.push("this board has not been tested");
    }
    if !var.is_known_layout() {
        reasons.push("the firmware variable differs from the tested layout");
    }
    reasons
}

pub fn set_mode(mode: Mode, force: bool) -> Result<ModeVar, Error> {
    let machine = Machine::detect();
    if !machine.is_msi() {
        return Err(Error::NotMsi(machine.vendor));
    }
    let mut var = ModeVar::load()?;
    let reasons = concerns(&machine, &var);
    if !reasons.is_empty() && !force {
        return Err(Error::NeedsForce(reasons));
    }
    if var.mode_byte() != var.encode(mode) {
        var.write(mode)?;
    }
    Ok(var)
}

fn read(path: &str) -> Option<String> {
    let s = fs::read_to_string(path).ok()?;
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_string())
}

fn read_hex(path: &str) -> Option<u32> {
    u32::from_str_radix(read(path)?.trim_start_matches("0x"), 16).ok()
}
