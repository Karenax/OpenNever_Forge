use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "camelCase")]
pub struct ResourceKey {
    pub resref: String,
    pub resource_type: u16,
}

impl ResourceKey {
    pub fn new(resref: impl Into<String>, resource_type: u16) -> Self {
        Self {
            resref: resref.into().trim_matches('\0').trim().to_ascii_lowercase(),
            resource_type,
        }
    }

    pub fn extension(&self) -> Option<&'static str> {
        resource_extension(self.resource_type)
    }

    pub fn file_name(&self) -> String {
        match self.extension() {
            Some(extension) => format!("{}.{}", self.resref, extension),
            None => format!("{}.#{}", self.resref, self.resource_type),
        }
    }
}

impl fmt::Display for ResourceKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.file_name())
    }
}

pub fn resource_extension(resource_type: u16) -> Option<&'static str> {
    match resource_type {
        0 => Some("res"),
        1 => Some("bmp"),
        3 => Some("tga"),
        4 => Some("wav"),
        6 => Some("plt"),
        7 => Some("ini"),
        8 => Some("bmu"),
        10 => Some("txt"),
        2002 => Some("mdl"),
        2005 => Some("fnt"),
        2007 => Some("lua"),
        2009 => Some("nss"),
        2010 => Some("ncs"),
        2011 => Some("mod"),
        2012 => Some("are"),
        2013 => Some("set"),
        2014 => Some("ifo"),
        2015 => Some("bic"),
        2016 => Some("wok"),
        2017 => Some("2da"),
        2018 => Some("tlk"),
        2022 => Some("txi"),
        2023 => Some("git"),
        2025 => Some("uti"),
        2027 => Some("utc"),
        2029 => Some("dlg"),
        2030 => Some("itp"),
        2032 => Some("utt"),
        2033 => Some("dds"),
        2035 => Some("uts"),
        2036 => Some("ltr"),
        2037 => Some("gff"),
        2038 => Some("fac"),
        2040 => Some("ute"),
        2042 => Some("utd"),
        2044 => Some("utp"),
        2045 => Some("dft"),
        2046 => Some("gic"),
        2047 => Some("gui"),
        2051 => Some("utm"),
        2052 => Some("dwk"),
        2053 => Some("pwk"),
        2055 => Some("utg"),
        2056 => Some("jrl"),
        2058 => Some("utw"),
        2059 => Some("4pc"),
        2060 => Some("ssf"),
        2061 => Some("hak"),
        2062 => Some("nwm"),
        2063 => Some("bik"),
        2064 => Some("ndb"),
        2065 => Some("ptm"),
        2066 => Some("ptt"),
        2067 => Some("bak"),
        2068 => Some("dat"),
        2069 => Some("shd"),
        2070 => Some("xbc"),
        2071 => Some("wbm"),
        2072 => Some("mtr"),
        2073 => Some("ktx"),
        2074 => Some("ttf"),
        2075 => Some("sql"),
        2076 => Some("tml"),
        2077 => Some("sq3"),
        2078 => Some("lod"),
        2079 => Some("gif"),
        2080 => Some("png"),
        2081 => Some("jpg"),
        2082 => Some("caf"),
        2083 => Some("jui"),
        2084 => Some("cdb"),
        9997 => Some("erf"),
        9998 => Some("bif"),
        9999 => Some("key"),
        _ => None,
    }
}

pub fn resource_type_for_extension(extension: &str) -> Option<u16> {
    let extension = extension.trim_start_matches('.').to_ascii_lowercase();
    const TYPES: &[u16] = &[
        0, 1, 3, 4, 6, 7, 8, 10, 2002, 2005, 2007, 2009, 2010, 2011, 2012, 2013, 2014, 2015, 2016,
        2017, 2018, 2022, 2023, 2025, 2027, 2029, 2030, 2032, 2033, 2035, 2036, 2037, 2038, 2040,
        2042, 2044, 2045, 2046, 2047, 2051, 2052, 2053, 2055, 2056, 2058, 2059, 2060, 2061, 2062,
        2063, 2064, 2065, 2066, 2067, 2068, 2069, 2070, 2071, 2072, 2073, 2074, 2075, 2076, 2077,
        2078, 2079, 2080, 2081, 2082, 2083, 2084, 9997, 9998, 9999,
    ];
    TYPES
        .iter()
        .copied()
        .find(|value| resource_extension(*value) == Some(extension.as_str()))
}

pub fn decode_nwn_text(bytes: &[u8]) -> String {
    if let Ok(value) = std::str::from_utf8(bytes) {
        return value.to_owned();
    }
    bytes
        .iter()
        .map(|byte| match *byte {
            0x80 => '€',
            0x82 => '‚',
            0x83 => 'ƒ',
            0x84 => '„',
            0x85 => '…',
            0x86 => '†',
            0x87 => '‡',
            0x88 => 'ˆ',
            0x89 => '‰',
            0x8A => 'Š',
            0x8B => '‹',
            0x8C => 'Œ',
            0x8E => 'Ž',
            0x91 => '‘',
            0x92 => '’',
            0x93 => '“',
            0x94 => '”',
            0x95 => '•',
            0x96 => '–',
            0x97 => '—',
            0x98 => '˜',
            0x99 => '™',
            0x9A => 'š',
            0x9B => '›',
            0x9C => 'œ',
            0x9E => 'ž',
            0x9F => 'Ÿ',
            value => char::from(value),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_normalized_and_extensions_are_bidirectional() {
        assert_eq!(ResourceKey::new(" Module ", 2014).file_name(), "module.ifo");
        assert_eq!(resource_type_for_extension(".DLG"), Some(2029));
        assert_eq!(decode_nwn_text(b"Caf\xE9"), "Café");
    }
}
