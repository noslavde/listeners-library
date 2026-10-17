use std::{os::windows::ffi::OsStrExt, path::Path};
use windows::core::{w, PCWSTR};
use windows::Win32::{
    Foundation::{LocalFree, HLOCAL},
    Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    },
    Security::{
        Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW, SetFileSecurityW,
        DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
    },
};
use zeroize::{Zeroize, Zeroizing};

pub fn protect(input: &[u8]) -> Result<Vec<u8>, String> {
    transform(input, true)
}
pub fn unprotect(input: &[u8]) -> Result<Zeroizing<Vec<u8>>, String> {
    transform(input, false).map(Zeroizing::new)
}
fn transform(input: &[u8], encrypt: bool) -> Result<Vec<u8>, String> {
    if input.is_empty() || input.len() > 1024 * 1024 {
        return Err("Invalid protected storage size".into());
    }
    let source = CRYPT_INTEGER_BLOB {
        cbData: input.len() as u32,
        pbData: input.as_ptr() as *mut u8,
    };
    let context = if cfg!(debug_assertions) {
        b"AudibleDesktop.Development.v2".as_slice()
    } else {
        b"AudibleDesktop.Release.v2".as_slice()
    };
    let entropy = CRYPT_INTEGER_BLOB {
        cbData: context.len() as u32,
        pbData: context.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        let result = if encrypt {
            CryptProtectData(
                &source,
                PCWSTR::null(),
                Some(&entropy),
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &source,
                None,
                Some(&entropy),
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        result.map_err(|_|"Windows could not unlock protected data for this account. Do not replace or delete its key.".to_string())?;
        let data = std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize);
        let bytes = data.to_vec();
        data.zeroize();
        LocalFree(Some(HLOCAL(output.pbData as *mut _)));
        Ok(bytes)
    }
}
/// Owner and SYSTEM only; child files inherit this protected directory DACL.
pub fn restrict_directory(path: &Path) -> Result<(), String> {
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            w!("D:P(A;OICI;FA;;;OW)(A;OICI;FA;;;SY)"),
            1,
            &mut descriptor,
            None,
        )
        .map_err(|_| "Cannot create private storage permissions")?;
        let result = SetFileSecurityW(
            PCWSTR(path.as_ptr()),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            descriptor,
        )
        .as_bool();
        LocalFree(Some(HLOCAL(descriptor.0)));
        if !result {
            return Err("Cannot restrict private storage permissions".into());
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_protection_roundtrip_and_tamper_rejection() {
        let secret = b"dummy-token-not-a-real-account";
        let mut blob = protect(secret).unwrap();
        assert!(!blob.windows(secret.len()).any(|s| s == secret));
        assert_eq!(unprotect(&blob).unwrap().as_slice(), secret);
        let last = blob.len() - 1;
        blob[last] ^= 1;
        assert!(unprotect(&blob).is_err());
    }
}
