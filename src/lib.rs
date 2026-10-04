pub mod efivar;
pub mod mode;
pub mod system;

pub use efivar::ModeVar;
pub use mode::Mode;
pub use system::{Gpu, Machine};

use std::fmt;
use std::io;

#[derive(Debug)]
pub enum Error {
    NoEfi,
    NotMsi(String),
    VarMissing,
    UnknownLayout(String),
    NeedsForce(Vec<&'static str>),
    NotRoot,
    VerifyFailed,
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoEfi => write!(f, "efivarfs is not mounted, the system was not booted in UEFI mode"),
            Error::NotMsi(vendor) => write!(f, "not an MSI machine (vendor: {vendor})"),
            Error::VarMissing => write!(f, "this firmware has no MsiDCVarData variable, GPU switching is not supported"),
            Error::UnknownLayout(hex) => write!(f, "MsiDCVarData has an unknown layout: {hex}"),
            Error::NeedsForce(reasons) => {
                write!(f, "refusing to write without --force: {}", reasons.join("; "))
            }
            Error::NotRoot => write!(f, "writing the firmware variable needs root"),
            Error::VerifyFailed => write!(f, "the firmware did not keep the new value"),
            Error::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}
