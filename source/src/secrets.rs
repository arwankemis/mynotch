//! Per-user Windows DPAPI. No secret values appear in diagnostic messages.
use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN};
use windows::core::w;

pub fn protect(value: &str) -> Result<String, String> {
    if value.is_empty() { return Ok(String::new()); }
    let mut input = value.as_bytes().to_vec();
    let blob = CRYPT_INTEGER_BLOB { cbData: input.len() as u32, pbData: input.as_mut_ptr() };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptProtectData(&blob, w!("mynotch"), None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output)
            .map_err(|_| "Windows tidak dapat melindungi secret untuk pengguna ini".to_owned())?;
        let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize);
        let result = bytes.iter().map(|v| format!("{v:02x}")).collect();
        let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
        input.fill(0);
        Ok(result)
    }
}

pub fn unprotect(value: &str) -> Result<String, String> {
    if value.is_empty() { return Ok(String::new()); }
    if value.len() > 131072 || value.len() % 2 != 0 || !value.is_ascii() { return Err("Format secret tidak valid".into()); }
    let mut data = (0..value.len()).step_by(2).map(|i| u8::from_str_radix(&value[i..i+2],16))
        .collect::<Result<Vec<_>, _>>().map_err(|_| "Format secret tidak valid".to_owned())?;
    let blob = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_mut_ptr() };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptUnprotectData(&blob, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output)
            .map_err(|_| "Secret hanya dapat dibuka oleh akun Windows yang menyimpannya".to_owned())?;
        let bytes = std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize);
        let result = String::from_utf8(bytes.to_vec()).map_err(|_| "Data secret tidak valid".to_owned());
        bytes.fill(0);
        let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
        data.fill(0);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn rejects_invalid_ciphertext() { assert!(unprotect("x").is_err()); assert!(unprotect("zz").is_err()); }
    #[test] fn roundtrip_user_secret() { let text="test-only-key-\u{2600}"; let cipher=protect(text).unwrap(); assert!(!cipher.contains(text)); assert_eq!(unprotect(&cipher).unwrap(),text); }
}
