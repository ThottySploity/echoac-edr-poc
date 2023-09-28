use crate::driver::edrs::Edrs;

use log::{error, warn};

use windows_sys::Win32::Storage::FileSystem::VerQueryValueW;
use windows_sys::Win32::Storage::FileSystem::VS_FIXEDFILEINFO;
use windows_sys::Win32::Storage::FileSystem::GetFileVersionInfoW;   
use windows_sys::Win32::Storage::FileSystem::GetFileVersionInfoSizeW;
use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows_sys::Win32::System::Memory::MEM_RELEASE;
use windows_sys::Win32::System::Memory::VirtualFree;
use windows_sys::Win32::System::Memory::PAGE_READWRITE;
use windows_sys::Win32::System::Memory::MEM_RESERVE;
use windows_sys::Win32::System::Memory::MEM_COMMIT;
use windows_sys::Win32::System::Memory::VirtualAlloc;
use windows_sys::Win32::System::ProcessStatus::GetDeviceDriverBaseNameW;
use windows_sys::Win32::System::ProcessStatus::EnumDeviceDrivers;

use ntapi::ntexapi::NtQuerySystemInformation;
use ntapi::ntldr::RTL_PROCESS_MODULES;


use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use std::os::raw::c_void;
use std::os::raw::c_char;
use std::ffi::CStr;

pub struct Utils;

impl Utils {

    pub fn u16_array_to_utf8_string(arr: &[u16]) -> String {
        let utf16_string: Vec<u16> = arr.to_vec();
        String::from_utf16_lossy(&utf16_string)
    }

    // Convert a String to a PWCH
    pub unsafe fn convert_string_to_mut_u16(s: String) -> *mut u16 {
        let utf16_data: Vec<u16> = s.encode_utf16().collect();
        let len = utf16_data.len();
        let ptr =
            std::alloc::alloc(std::alloc::Layout::from_size_align(len * 2, 2).unwrap()) as *mut u16;

        std::ptr::copy_nonoverlapping(utf16_data.as_ptr(), ptr, len);

        *ptr.add(len) = 0;

        ptr
    }

    pub unsafe fn get_notify_address(kernel_base: usize, offset: usize) -> usize {
        return kernel_base + offset
    }

    pub unsafe fn make_psp_routine(routine: &str) -> String {
        format!("Psp{}NotifyRoutine", routine)
    }

    pub unsafe fn is_driver_name_edr(driver_name: String) -> bool {
        let edrs = Edrs::get_drivers();

        for edr in edrs {
            if driver_name.to_lowercase() == edr.to_lowercase() {
                return true;
            }
        }
        false
    }

    pub unsafe fn get_driver(address: u64, offset: &mut u64) -> String {
        let mut drivers: [usize; 1024] = [0; 1024];
        let mut needed = 0u32;
        let mut c_drivers = 0u32;

        let mut diff = 0;
        let mut min_diff = 4294967295u64;
        
        if EnumDeviceDrivers(drivers.as_mut_ptr() as *mut *mut c_void, (std::mem::size_of::<usize>() * drivers.len()) as u32, &mut needed) != 0 {
            c_drivers = (needed as usize / std::mem::size_of_val(&drivers[0])) as u32;
            for i in 0..c_drivers {
                if (drivers[i as usize]) as u64 <= address {
                    diff = address - drivers[i as usize] as u64;
                    if diff < min_diff {
                        min_diff = diff;
                    }
                }
            }
        } else {
            if address != 0 {
                warn!("Could not enumerate device driver at: 0x{:x}, an EDR driver might be missing", address);
            }
        }

        let mut sz_driver: [u16; 1024] = [0; 1024];
        if GetDeviceDriverBaseNameW((address - min_diff) as *const c_void, sz_driver.as_mut_ptr(), 1024) != 0 {
            let driver_name = format!("{}", Utils::u16_array_to_utf8_string(&sz_driver).replace("\0", ""));
            *offset = min_diff;
            return driver_name;
        } else {
            warn!("Could not enumerate device driver at: 0x{:x}, an EDR driver might be missing", address);
        }
        "".to_string()
    }

    //https://www.unknowncheats.me/forum/general-programming-and-reversing/427419-getkernelbase.html
    pub unsafe fn get_kernel_base_address() -> usize {
        // Leak ntoskrnl base address using NtQuerySystemInformation
        let module_info: *mut RTL_PROCESS_MODULES = VirtualAlloc(
            null_mut(),
            1024 * 1024,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        ) as *mut RTL_PROCESS_MODULES;

        if NtQuerySystemInformation(11, module_info as *mut c_void, 1024 * 1024, 0 as *mut u32) != 0 {
            error!("NtQuerySystemInformation didn't work");
            VirtualFree(module_info as *mut c_void, 0, MEM_RELEASE);
            return 0;
        }

        let module_slice =
            std::slice::from_raw_parts_mut(module_info, (*module_info).NumberOfModules as usize);

        for module in module_slice {
            let module_file =
                CStr::from_ptr((*module).Modules[0].FullPathName.as_ptr() as *const c_char);
            let string = module_file.to_string_lossy().into_owned();
            if string.contains("ntoskrnl") {
                return (*module).Modules[0].ImageBase as usize;
            }
        }
        0
    }

    pub unsafe fn get_ntoskrnl_version() -> String {
        let mut system_dir = [0u16; 260];
        GetSystemDirectoryW(system_dir.as_mut_ptr(), system_dir.len() as u32);
        let binding = OsString::from_wide(&system_dir);
        let system_dir = binding.to_string_lossy();

        let file_path = format!("{}\\ntoskrnl.exe", system_dir).replace("\0", "");
        let file_path_wide: Vec<u16> = file_path.encode_utf16().chain(Some(0)).collect();

        let size = GetFileVersionInfoSizeW(file_path_wide.as_ptr(), std::ptr::null_mut());

        if size == 0 {
            println!("Error occurred while getting version information size.");
            return "".to_string();
        }

        let mut ver_buffer = Vec::<u8>::with_capacity(size as usize);
        let ver_buffer_ptr = ver_buffer.as_mut_ptr() as *mut c_void;
        GetFileVersionInfoW(file_path_wide.as_ptr(), 0, size, ver_buffer_ptr);

        // Retrieve the version information
        let mut ver_info: *mut c_void = null_mut();
        let mut ver_info_size: u32 = 0;
        let ver_info_query = OsStr::new("\\").encode_wide().chain(Some(0).into_iter()).collect::<Vec<u16>>();
        VerQueryValueW(ver_buffer_ptr, ver_info_query.as_ptr(), &mut ver_info, &mut ver_info_size);

        let file_info = ver_info as *const VS_FIXEDFILEINFO;

        // Extract version numbers
        let build_number = hiword((*file_info).dwFileVersionLS);
        let revision_number = loword((*file_info).dwFileVersionLS);

        format!("ntoskrnl_{}-{}.exe", build_number, revision_number)
    }
}

fn hiword(l: u32) -> u16 {
    ((l >> 16) & 0xFFFF) as u16
}

fn loword(l: u32) -> u16 {
    (l & 0xFFFF) as u16
}