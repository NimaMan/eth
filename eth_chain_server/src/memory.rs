#[cfg(all(target_os = "linux", target_env = "gnu"))]
use std::os::raw::c_int;

#[cfg(all(target_os = "linux", target_env = "gnu"))]
extern "C" {
    fn malloc_trim(pad: usize) -> c_int;
}

pub fn trim_allocator() -> bool {
    trim_allocator_impl()
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn trim_allocator_impl() -> bool {
    unsafe { malloc_trim(0) != 0 }
}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
fn trim_allocator_impl() -> bool {
    false
}
