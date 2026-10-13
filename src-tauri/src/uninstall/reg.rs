//! Kayıt defterini okuma (yalnız okuma; silme PowerShell betiğinde, önce .reg yedekle).
//! Yollar "HKLM\SOFTWARE\…" / "HKCU\Software\…" biçiminde.

#[cfg(windows)]
mod imp {
    use windows::core::{HSTRING, PWSTR};
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegEnumValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER,
        HKEY_LOCAL_MACHINE, KEY_READ, REG_DWORD, REG_EXPAND_SZ, REG_SZ, REG_VALUE_TYPE,
    };

    struct Key(HKEY);
    impl Drop for Key {
        fn drop(&mut self) {
            unsafe {
                let _ = RegCloseKey(self.0);
            }
        }
    }

    fn open(path: &str) -> Option<Key> {
        let (root, sub) = path.split_once('\\').unwrap_or((path, ""));
        let hive = match root.to_ascii_uppercase().as_str() {
            "HKLM" | "HKEY_LOCAL_MACHINE" => HKEY_LOCAL_MACHINE,
            "HKCU" | "HKEY_CURRENT_USER" => HKEY_CURRENT_USER,
            _ => return None,
        };
        let mut h = HKEY::default();
        let r = unsafe { RegOpenKeyExW(hive, &HSTRING::from(sub), None, KEY_READ, &mut h) };
        (r == ERROR_SUCCESS).then_some(Key(h))
    }

    pub fn exists(path: &str) -> bool {
        open(path).is_some()
    }

    pub fn subkeys(path: &str) -> Vec<String> {
        let Some(k) = open(path) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for i in 0.. {
            let mut buf = [0u16; 512];
            let mut len = buf.len() as u32;
            let r = unsafe {
                RegEnumKeyExW(
                    k.0,
                    i,
                    Some(PWSTR(buf.as_mut_ptr())),
                    &mut len,
                    None,
                    None,
                    None,
                    None,
                )
            };
            if r != ERROR_SUCCESS {
                break;
            }
            out.push(String::from_utf16_lossy(&buf[..len as usize]));
        }
        out
    }

    /// Metin ve sayı değerleri (ad, veri). Ad boşsa varsayılan değer.
    pub fn values(path: &str) -> Vec<(String, String)> {
        let Some(k) = open(path) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for i in 0.. {
            let mut name = [0u16; 512];
            let mut nlen = name.len() as u32;
            let mut data = vec![0u8; 16 * 1024];
            let mut dlen = data.len() as u32;
            let mut kind = 0u32;
            let r = unsafe {
                RegEnumValueW(
                    k.0,
                    i,
                    Some(PWSTR(name.as_mut_ptr())),
                    &mut nlen,
                    None,
                    Some(&mut kind),
                    Some(data.as_mut_ptr()),
                    Some(&mut dlen),
                )
            };
            if r != ERROR_SUCCESS {
                // Çok uzun veri: atla, sonrakine geç (ERROR_MORE_DATA); başka hata: bitti.
                if r.0 == 234 {
                    continue;
                }
                break;
            }
            let n = String::from_utf16_lossy(&name[..nlen as usize]);
            let bytes = &data[..dlen as usize];
            let v = match REG_VALUE_TYPE(kind) {
                REG_SZ | REG_EXPAND_SZ => {
                    let w: Vec<u16> = bytes
                        .chunks_exact(2)
                        .map(|c| u16::from_le_bytes([c[0], c[1]]))
                        .collect();
                    String::from_utf16_lossy(&w)
                        .trim_end_matches('\0')
                        .to_string()
                }
                REG_DWORD if bytes.len() >= 4 => {
                    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]).to_string()
                }
                _ => continue,
            };
            out.push((n, v));
        }
        out
    }
}

#[cfg(windows)]
pub use imp::{exists, subkeys, values};

#[cfg(not(windows))]
pub fn exists(_path: &str) -> bool {
    false
}
#[cfg(not(windows))]
pub fn subkeys(_path: &str) -> Vec<String> {
    Vec::new()
}
#[cfg(not(windows))]
pub fn values(_path: &str) -> Vec<(String, String)> {
    Vec::new()
}
