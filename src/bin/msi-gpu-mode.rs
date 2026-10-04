use clap::{Parser, Subcommand};
use msi_gpu_mode::{Error, Machine, Mode, ModeVar, system};
use std::process::ExitCode;

#[derive(Parser)]
#[command(version, about = "Switch the GPU mode of MSI laptops")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Show the active mode and the mode stored for the next boot
    Status,
    /// Store a mode for the next boot: hybrid, dgpu or igpu
    Set {
        mode: Mode,
        /// Write even if this board or variable layout is untested
        #[arg(long)]
        force: bool,
        /// Show what would be written without touching the firmware
        #[arg(long)]
        dry_run: bool,
    },
    /// Print machine details to attach to a bug report
    Report,
}

fn main() -> ExitCode {
    let result = match Cli::parse().command.unwrap_or(Command::Status) {
        Command::Status => status(),
        Command::Set { mode, force, dry_run } => set(mode, force, dry_run),
        Command::Report => report(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn status() -> Result<(), Error> {
    let machine = Machine::detect();
    let var = ModeVar::load()?;
    let gpus = system::gpus();

    println!("{}", machine.describe());
    let concerns = system::concerns(&machine, &var);
    if concerns.is_empty() {
        println!("support:    verified");
    } else {
        println!("support:    untested ({})", concerns.join("; "));
    }
    let names: Vec<_> = gpus.iter().map(|g| g.vendor_name()).collect();
    match system::active_mode(&gpus) {
        Some(mode) => println!("active:     {mode} ({})", names.join(" + ")),
        None => println!("active:     unknown, no GPU found on the PCI bus"),
    }
    println!("next boot:  {}", var.mode());
    Ok(())
}

fn set(mode: Mode, force: bool, dry_run: bool) -> Result<(), Error> {
    let before = ModeVar::load()?;
    if dry_run {
        println!(
            "would change byte 9 of {} from 0x{:02x} ({}) to 0x{:02x} ({mode})",
            before.name(),
            before.mode_byte(),
            before.mode(),
            before.encode(mode),
        );
        return Ok(());
    }
    let after = system::set_mode(mode, force)?;
    if before.mode_byte() == after.mode_byte() {
        println!("next boot is already {mode}");
    } else {
        println!("next boot: {mode} (was {}). Reboot to apply.", before.mode());
    }
    Ok(())
}

fn report() -> Result<(), Error> {
    let machine = Machine::detect();
    println!("vendor:   {}", machine.vendor);
    println!("product:  {}", machine.product);
    println!("board:    {}", machine.board);
    println!("bios:     {}", machine.bios);
    println!("ec:       {}", machine.ec_firmware.as_deref().unwrap_or("msi-ec not loaded"));
    for gpu in system::gpus() {
        let kind = if gpu.integrated { "integrated" } else { "discrete" };
        println!("gpu:      {} {:04x} {kind}", gpu.address, gpu.vendor);
    }
    let var = ModeVar::load()?;
    println!("variable: {}", var.name());
    println!("data:     {}", var.hex());
    println!("decoded:  {}", var.mode());
    Ok(())
}
