use crate::{Error, Mode};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

const EFIVARS: &str = "/sys/firmware/efi/efivars";
const VAR_PREFIX: &str = "MsiDCVarData-";

// efivarfs prepends the 4-byte attribute word, so offsets are 4 higher than in the firmware.
const NV_BS_RT: [u8; 4] = [0x07, 0, 0, 0];
const MODE_BYTE: usize = 9;
const KNOWN_LEN: usize = 24;
const KNOWN_HIGH_NIBBLE: u8 = 0x3;

const FS_IMMUTABLE_FL: libc::c_int = 0x10;

#[derive(Debug, Clone)]
pub struct ModeVar {
    path: PathBuf,
    raw: Vec<u8>,
    applied: Mode,
}

impl ModeVar {
    pub fn load() -> Result<Self, Error> {
        let dir = fs::read_dir(EFIVARS).map_err(|_| Error::NoEfi)?;
        let path = dir
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with(VAR_PREFIX))
            })
            .ok_or(Error::VarMissing)?;
        Self::read(path)
    }

    fn read(path: PathBuf) -> Result<Self, Error> {
        let raw = fs::read(&path)?;
        let applied = parse(&raw).ok_or_else(|| Error::UnknownLayout(hex(&raw)))?;
        Ok(Self { path, raw, applied })
    }

    pub fn applied(&self) -> Mode {
        self.applied
    }

    pub fn requested(&self) -> Option<Mode> {
        Mode::from_code(self.raw[MODE_BYTE] & 0b11)
    }

    pub fn name(&self) -> &str {
        self.path.file_name().and_then(|n| n.to_str()).unwrap_or_default()
    }

    pub fn hex(&self) -> String {
        hex(&self.raw)
    }

    pub fn mode_byte(&self) -> u8 {
        self.raw[MODE_BYTE]
    }

    pub fn is_known_layout(&self) -> bool {
        self.raw.len() == KNOWN_LEN && self.raw[MODE_BYTE] >> 4 == KNOWN_HIGH_NIBBLE
    }

    pub fn encode(&self, mode: Mode) -> u8 {
        (self.raw[MODE_BYTE] & 0xfc) | mode.code()
    }

    // efivarfs marks the variable immutable.
    pub fn write(&mut self, mode: Mode) -> Result<(), Error> {
        if unsafe { libc::geteuid() } != 0 {
            return Err(Error::NotRoot);
        }
        let mut raw = self.raw.clone();
        raw[MODE_BYTE] = self.encode(mode);

        let was_immutable = set_immutable(&self.path, false)?;
        let written = write_once(&self.path, &raw);
        if was_immutable {
            set_immutable(&self.path, true)?;
        }
        written?;

        *self = Self::read(self.path.clone())?;
        if self.raw != raw {
            return Err(Error::VerifyFailed);
        }
        Ok(())
    }
}

fn parse(raw: &[u8]) -> Option<Mode> {
    if raw.len() <= MODE_BYTE || raw[..4] != NV_BS_RT {
        return None;
    }
    // Bits 0-1 hold the request, bits 2-3 the mode the firmware applied at boot.
    Mode::from_code((raw[MODE_BYTE] >> 2) & 0b11)
}

fn hex(raw: &[u8]) -> String {
    raw.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
}

// SetVariable takes attributes and payload together, so it has to be a single write(2).
fn write_once(path: &Path, raw: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).open(path)?;
    if file.write(raw)? != raw.len() {
        return Err(io::Error::new(io::ErrorKind::WriteZero, "short write to efivarfs"));
    }
    Ok(())
}

fn set_immutable(path: &Path, immutable: bool) -> io::Result<bool> {
    let file = File::open(path)?;
    let fd = file.as_raw_fd();
    let mut flags: libc::c_int = 0;
    if unsafe { libc::ioctl(fd, libc::FS_IOC_GETFLAGS, &mut flags) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let was = flags & FS_IMMUTABLE_FL != 0;
    if was != immutable {
        flags ^= FS_IMMUTABLE_FL;
        if unsafe { libc::ioctl(fd, libc::FS_IOC_SETFLAGS, &flags) } != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(was)
}
