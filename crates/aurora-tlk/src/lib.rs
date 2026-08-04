use aurora_core::{AppError, AppResult, ErrorSeverity, decode_nwn_text};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const HEADER_SIZE: usize = 20;
const ENTRY_SIZE: usize = 40;
const MAX_ENTRIES: u32 = 2_000_000;
const TEXT_PRESENT: u32 = 0x0001;
const SOUND_PRESENT: u32 = 0x0002;
pub const CUSTOM_TLK_BASE: u32 = 0x0100_0000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TlkEntry {
    pub index: u32,
    pub flags: u32,
    pub text: Option<String>,
    pub sound_resref: Option<String>,
    pub volume_variance: f32,
    pub pitch_variance: f32,
    pub sound_length: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TalkTable {
    pub language_id: u32,
    pub entries: Vec<TlkEntry>,
    pub source: String,
}

impl TalkTable {
    pub fn from_file(path: &Path) -> AppResult<Self> {
        let bytes = fs::read(path)
            .map_err(|error| AppError::io("read TLK", path.display().to_string(), &error))?;
        parse_tlk(&bytes, &path.display().to_string())
    }

    pub fn entry(&self, index: u32) -> Option<&TlkEntry> {
        self.entries.get(index as usize)
    }
}

pub fn parse_tlk(bytes: &[u8], source: &str) -> AppResult<TalkTable> {
    ensure_range(bytes, 0, HEADER_SIZE, source, "TLK_HEADER_TOO_SHORT")?;
    if &bytes[0..4] != b"TLK " || &bytes[4..8] != b"V3.0" {
        return Err(tlk_error(
            source,
            "TLK_HEADER_UNSUPPORTED",
            "Expected TLK V3.0".into(),
        ));
    }
    let language_id = u32_at(bytes, 8);
    let count = u32_at(bytes, 12);
    let strings_offset = u32_at(bytes, 16) as usize;
    if count > MAX_ENTRIES {
        return Err(tlk_error(
            source,
            "TLK_ENTRY_LIMIT_EXCEEDED",
            format!("{count} entries exceeds {MAX_ENTRIES}"),
        ));
    }
    ensure_range(
        bytes,
        HEADER_SIZE,
        count as usize * ENTRY_SIZE,
        source,
        "TLK_ENTRY_TABLE_OUT_OF_BOUNDS",
    )?;
    if strings_offset < HEADER_SIZE + count as usize * ENTRY_SIZE || strings_offset > bytes.len() {
        return Err(tlk_error(
            source,
            "TLK_STRING_DATA_OUT_OF_BOUNDS",
            format!("String data offset {strings_offset} is invalid"),
        ));
    }
    let mut entries = Vec::with_capacity(count as usize);
    for index in 0..count as usize {
        let base = HEADER_SIZE + index * ENTRY_SIZE;
        let flags = u32_at(bytes, base);
        let sound = decode_fixed(&bytes[base + 4..base + 20]);
        let volume_variance = f32_at(bytes, base + 20);
        let pitch_variance = f32_at(bytes, base + 24);
        let text_offset = u32_at(bytes, base + 28) as usize;
        let text_size = u32_at(bytes, base + 32) as usize;
        let sound_length = f32_at(bytes, base + 36);
        let text = if flags & TEXT_PRESENT != 0 {
            let start = strings_offset.checked_add(text_offset).ok_or_else(|| {
                tlk_error(
                    source,
                    "TLK_STRING_OUT_OF_BOUNDS",
                    "Text offset overflows".into(),
                )
            })?;
            ensure_range(bytes, start, text_size, source, "TLK_STRING_OUT_OF_BOUNDS")?;
            Some(decode_text(&bytes[start..start + text_size]))
        } else {
            None
        };
        entries.push(TlkEntry {
            index: index as u32,
            flags,
            text,
            sound_resref: (flags & SOUND_PRESENT != 0 && !sound.is_empty()).then_some(sound),
            volume_variance,
            pitch_variance,
            sound_length,
        });
    }
    Ok(TalkTable {
        language_id,
        entries,
        source: source.to_owned(),
    })
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocalizedOrigin {
    Embedded,
    CustomTlk,
    DialogTlk,
    Missing,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocalizedResolutionState {
    Resolved,
    MissingTable,
    MissingEntry,
    MissingText,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedString {
    pub language_id: u32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalizedStringRequest {
    pub string_ref: Option<u32>,
    pub embedded: Vec<EmbeddedString>,
    pub language_id: u32,
    pub gender: Gender,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Gender {
    Male,
    Female,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedLocalizedString {
    pub text: Option<String>,
    pub origin: LocalizedOrigin,
    pub state: LocalizedResolutionState,
    pub string_ref: Option<u32>,
    pub table_index: Option<u32>,
    pub source: Option<String>,
}

pub struct LocalizedStringResolver<'a> {
    pub dialog: Option<&'a TalkTable>,
    pub dialog_female: Option<&'a TalkTable>,
    pub custom: Option<&'a TalkTable>,
    pub custom_female: Option<&'a TalkTable>,
}

impl LocalizedStringResolver<'_> {
    pub fn resolve(&self, request: &LocalizedStringRequest) -> ResolvedLocalizedString {
        let gender_offset = if request.gender == Gender::Female {
            1
        } else {
            0
        };
        let localized_id = request
            .language_id
            .saturating_mul(2)
            .saturating_add(gender_offset);
        if let Some(value) = request
            .embedded
            .iter()
            .find(|value| value.language_id == localized_id)
            .or_else(|| {
                request
                    .embedded
                    .iter()
                    .find(|value| value.language_id / 2 == request.language_id)
            })
            .or_else(|| request.embedded.first())
        {
            return ResolvedLocalizedString {
                text: Some(value.text.clone()),
                origin: LocalizedOrigin::Embedded,
                state: LocalizedResolutionState::Resolved,
                string_ref: request.string_ref,
                table_index: None,
                source: None,
            };
        }
        let Some(string_ref) = request.string_ref else {
            return missing(None, LocalizedResolutionState::MissingEntry);
        };
        let custom = string_ref >= CUSTOM_TLK_BASE;
        let index = if custom {
            string_ref - CUSTOM_TLK_BASE
        } else {
            string_ref
        };
        let table = match (custom, request.gender) {
            (true, Gender::Female) => self.custom_female.or(self.custom),
            (true, Gender::Male) => self.custom,
            (false, Gender::Female) => self.dialog_female.or(self.dialog),
            (false, Gender::Male) => self.dialog,
        };
        let origin = if custom {
            LocalizedOrigin::CustomTlk
        } else {
            LocalizedOrigin::DialogTlk
        };
        let Some(table) = table else {
            return ResolvedLocalizedString {
                text: None,
                origin,
                state: LocalizedResolutionState::MissingTable,
                string_ref: Some(string_ref),
                table_index: Some(index),
                source: None,
            };
        };
        let Some(entry) = table.entry(index) else {
            return ResolvedLocalizedString {
                text: None,
                origin,
                state: LocalizedResolutionState::MissingEntry,
                string_ref: Some(string_ref),
                table_index: Some(index),
                source: Some(table.source.clone()),
            };
        };
        ResolvedLocalizedString {
            text: entry.text.clone(),
            origin,
            state: if entry.text.is_some() {
                LocalizedResolutionState::Resolved
            } else {
                LocalizedResolutionState::MissingText
            },
            string_ref: Some(string_ref),
            table_index: Some(index),
            source: Some(table.source.clone()),
        }
    }
}

fn missing(string_ref: Option<u32>, state: LocalizedResolutionState) -> ResolvedLocalizedString {
    ResolvedLocalizedString {
        text: None,
        origin: LocalizedOrigin::Missing,
        state,
        string_ref,
        table_index: None,
        source: None,
    }
}

fn ensure_range(
    bytes: &[u8],
    offset: usize,
    size: usize,
    source: &str,
    code: &str,
) -> AppResult<()> {
    if offset.checked_add(size).is_none_or(|end| end > bytes.len()) {
        return Err(tlk_error(
            source,
            code,
            format!("{offset}+{size} exceeds {}", bytes.len()),
        ));
    }
    Ok(())
}
fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("bounded u32"))
}
fn f32_at(bytes: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("bounded f32"))
}
fn decode_fixed(bytes: &[u8]) -> String {
    decode_text(
        &bytes[..bytes
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(bytes.len())],
    )
}
fn decode_text(bytes: &[u8]) -> String {
    decode_nwn_text(bytes)
}
fn tlk_error(source: &str, code: &str, detail: String) -> Box<AppError> {
    Box::new(
        AppError::new(
            code,
            "La table de dialogues TLK est invalide.",
            detail,
            ErrorSeverity::Error,
        )
        .with_source(source)
        .with_import_stage("tlk"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_text_and_resolves_custom_strrefs() {
        let mut bytes = vec![0; HEADER_SIZE + ENTRY_SIZE + 5];
        bytes[0..4].copy_from_slice(b"TLK ");
        bytes[4..8].copy_from_slice(b"V3.0");
        bytes[12..16].copy_from_slice(&1_u32.to_le_bytes());
        bytes[16..20].copy_from_slice(&((HEADER_SIZE + ENTRY_SIZE) as u32).to_le_bytes());
        bytes[20..24].copy_from_slice(&TEXT_PRESENT.to_le_bytes());
        bytes[52..56].copy_from_slice(&5_u32.to_le_bytes());
        bytes[HEADER_SIZE + ENTRY_SIZE..].copy_from_slice(b"Hello");
        let table = parse_tlk(&bytes, "custom.tlk").expect("TLK");
        let resolved = LocalizedStringResolver {
            dialog: None,
            dialog_female: None,
            custom: Some(&table),
            custom_female: None,
        }
        .resolve(&LocalizedStringRequest {
            string_ref: Some(CUSTOM_TLK_BASE),
            embedded: Vec::new(),
            language_id: 0,
            gender: Gender::Male,
        });
        assert_eq!(resolved.text.as_deref(), Some("Hello"));
        assert_eq!(resolved.origin, LocalizedOrigin::CustomTlk);
    }
}
