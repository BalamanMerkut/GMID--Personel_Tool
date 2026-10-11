//! Valve'in KeyValues (VDF/ACF) metin biçimi: `"anahtar" "değer"` ve `"anahtar" { … }`.
//! Steam'in kütüphane klasörleri, kurulu oyunları ve oynama süreleri bu biçimde tutulur.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Vdf {
    Str(String),
    Map(BTreeMap<String, Vdf>),
}

impl Vdf {
    /// Büyük/küçük harfe bakmadan alt anahtar (Steam aynı anahtarı farklı yazabiliyor).
    pub fn get(&self, key: &str) -> Option<&Vdf> {
        match self {
            Vdf::Map(m) => m.get(key).or_else(|| {
                m.iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(key))
                    .map(|(_, v)| v)
            }),
            Vdf::Str(_) => None,
        }
    }

    pub fn path(&self, keys: &[&str]) -> Option<&Vdf> {
        keys.iter().try_fold(self, |v, k| v.get(k))
    }

    pub fn str(&self) -> Option<&str> {
        match self {
            Vdf::Str(s) => Some(s),
            Vdf::Map(_) => None,
        }
    }

    pub fn map(&self) -> Option<&BTreeMap<String, Vdf>> {
        match self {
            Vdf::Map(m) => Some(m),
            Vdf::Str(_) => None,
        }
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Vdf::str)
    }
}

fn tokens(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                let mut s = String::new();
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => match chars.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some(o) => s.push(o),
                            None => {}
                        },
                        '"' => break,
                        c => s.push(c),
                    }
                }
                out.push(format!("\"{s}"));
            }
            '{' | '}' => out.push(c.to_string()),
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            c if c.is_whitespace() => {}
            c => {
                // Tırnaksız kelime (eski dosyalarda olabilir).
                let mut s = String::from(c);
                while let Some(&n) = chars.peek() {
                    if n.is_whitespace() || n == '{' || n == '}' || n == '"' {
                        break;
                    }
                    s.push(n);
                    chars.next();
                }
                out.push(format!("\"{s}"));
            }
        }
    }
    out
}

fn parse_map(t: &[String], i: &mut usize) -> BTreeMap<String, Vdf> {
    let mut m = BTreeMap::new();
    while *i < t.len() {
        let tok = &t[*i];
        if tok == "}" {
            *i += 1;
            break;
        }
        let Some(key) = tok.strip_prefix('"') else {
            *i += 1;
            continue;
        };
        *i += 1;
        match t.get(*i).map(String::as_str) {
            Some("{") => {
                *i += 1;
                m.insert(key.to_string(), Vdf::Map(parse_map(t, i)));
            }
            Some(v) if v.starts_with('"') => {
                m.insert(key.to_string(), Vdf::Str(v[1..].to_string()));
                *i += 1;
            }
            _ => break,
        }
    }
    m
}

/// Metni ayrıştırır; en üstteki anahtar(lar) bir haritada döner.
pub fn parse(src: &str) -> Vdf {
    let t = tokens(src);
    let mut i = 0;
    Vdf::Map(parse_map(&t, &mut i))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_steam_files() {
        let v = parse(
            r#"
"libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"apps"
		{
			"570"		"123"
		}
	}
	"1"	"D:\\SteamLibrary"   // eski biçim
}"#,
        );
        assert_eq!(
            v.path(&["libraryfolders", "0", "path"]).and_then(Vdf::str),
            Some(r"C:\Program Files (x86)\Steam")
        );
        assert_eq!(
            v.path(&["LIBRARYFOLDERS", "1"]).and_then(Vdf::str),
            Some(r"D:\SteamLibrary")
        );
        assert!(v
            .path(&["libraryfolders", "0", "apps"])
            .and_then(Vdf::map)
            .is_some());
    }
}
