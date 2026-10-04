use std::fs::OpenOptions;
use std::io;
use std::os::unix::fs::FileExt;

const APM_CONTROL_PORT: u64 = 0xb2;
const COMMIT_GPU_MODE: u8 = 0x11;

// The firmware stores the request only when its handler for this SMI runs.
pub fn commit_gpu_mode() -> io::Result<()> {
    let port = OpenOptions::new().write(true).open("/dev/port")?;
    port.write_all_at(&[COMMIT_GPU_MODE], APM_CONTROL_PORT)
}
