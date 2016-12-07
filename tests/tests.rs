extern crate memsec;
extern crate quickcheck;
#[cfg(not(unix))] extern crate libc;
#[cfg(unix)] extern crate libsodium_sys;
#[cfg(unix)] extern crate nix;

use std::{ mem, cmp };
use quickcheck::quickcheck;


#[test]
fn memzero_test() {
    let mut x: [usize; 16] = [1; 16];
    unsafe { memsec::memzero(x.as_mut_ptr(), mem::size_of_val(&x)) };
    assert_eq!(x, [0; 16]);
    x.clone_from_slice(&[1; 16]);
    assert_eq!(x, [1; 16]);
    unsafe { memsec::memzero(x[1..11].as_mut_ptr(), 10 * mem::size_of_val(&x[0])) };
    assert_eq!(x, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1]);
}

#[test]
fn memcmp_test() {
    fn memcmp(x: Vec<u8>, y: Vec<u8>) -> bool {
        #[cfg(unix)] unsafe {
            memsec::memcmp(
                x.as_ptr(),
                y.as_ptr(),
                cmp::min(x.len(), y.len())
            ) == libsodium_sys::sodium_memcmp(
                x.as_ptr() as *const u8,
                y.as_ptr() as *const u8,
                cmp::min(x.len(), y.len())
            )
        }

        #[cfg(not(unix))] unsafe {
            let memsec_result = memsec::memcmp(
                x.as_ptr(),
                y.as_ptr(),
                cmp::min(x.len(), y.len())
            ) == 0;
            let libc_result = libc::memcmp(
                x.as_ptr() as *const libc::c_void,
                y.as_ptr() as *const libc::c_void,
                cmp::min(x.len(), y.len())
            ) == 0;
            memsec_result == libc_result
        }
    }
    quickcheck(memcmp as fn(Vec<u8>, Vec<u8>) -> bool);
}

#[test]
fn mlock_munlock_test() {
    let mut x = [1; 16];

    assert!(unsafe { memsec::mlock(x.as_mut_ptr(), mem::size_of_val(&x)) });
    assert!(unsafe { memsec::munlock(x.as_mut_ptr(), mem::size_of_val(&x)) });
    assert_eq!(x, [0; 16]);
}

#[test]
fn malloc_u64_test() {
    unsafe {
        let p: *mut u64 = memsec::malloc(mem::size_of::<u64>()).unwrap();
        *p = std::u64::MAX;
        assert_eq!(*p, std::u64::MAX);
        memsec::free(p);
    }
}

#[test]
fn malloc_free_test() {
    let memptr: *mut u8 = unsafe { memsec::malloc(1).unwrap() };
    assert!(!memptr.is_null());
    unsafe { memsec::free(memptr) };

    let memptr: *mut u8 = unsafe { memsec::malloc(0).unwrap() };
    assert!(!memptr.is_null());
    unsafe { memsec::free(memptr) };

    let memptr = unsafe { memsec::malloc::<u8>(std::usize::MAX - 1) };
    assert!(memptr.is_none());

    let buf: *mut u8 = unsafe { memsec::allocarray(16).unwrap() };
    unsafe { memsec::memzero(buf, 16) };
    assert_eq!(unsafe { memsec::memcmp(buf, [0; 16].as_ptr(), 16) }, 0);
    unsafe { memsec::free(buf) };
}

#[test]
fn malloc_mprotect_1_test() {
    let x: *mut u8 = unsafe { memsec::malloc(16).unwrap() };

    unsafe { memsec::memset(x, 1, 16) };
    assert!(unsafe { memsec::mprotect(x, memsec::Prot::ReadOnly) });
    assert_eq!(unsafe { memsec::memcmp(x, [1; 16].as_ptr(), 16) }, 0);
    assert!(unsafe { memsec::mprotect(x, memsec::Prot::NoAccess) });
    assert!(unsafe { memsec::mprotect(x, memsec::Prot::ReadWrite) });
    unsafe { memsec::memzero(x, 16) };
    unsafe { memsec::free(x) };
}

#[cfg(all(unix, target_os = "linux"))]
#[should_panic]
#[test]
fn malloc_mprotect_2_test() {
    use nix::sys::signal;
    extern fn sigsegv(_: i32) { panic!() }
    let sigaction = signal::SigAction::new(
        signal::SigHandler::Handler(sigsegv),
        signal::SA_SIGINFO,
        signal::SigSet::empty(),
    );
    unsafe { signal::sigaction(signal::SIGSEGV, &sigaction).ok() };

    let x: *mut u8 = unsafe { memsec::allocarray(16).unwrap() };

    unsafe { memsec::memset(x, 1, 16) };
    unsafe { memsec::mprotect(x, memsec::Prot::ReadOnly) };
    unsafe { memsec::memzero(x, 16) }; // SIGSEGV!
}
