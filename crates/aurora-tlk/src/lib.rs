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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum TlkEditAction {
    SetEntry {
        index: u32,
        text: Option<String>,
        sound_resref: Option<String>,
        sound_length: f32,
    },
    AppendEntry {
        text: Option<String>,
    },
}

pub fn apply_tlk_edit(table: &mut TalkTable, action: &TlkEditAction) -> AppResult<()> {
    match action {
        TlkEditAction::SetEntry {
            index,
            text,
            sound_resref,
            sound_length,
        } => {
            let entry = table.entries.get_mut(*index as usize).ok_or_else(|| {
                tlk_error(&table.source, "TLK_ENTRY_OUT_OF_BOUNDS", index.to_string())
            })?;
            validate_sound(sound_resref.as_deref(), *sound_length, &table.source)?;
            entry.text = text.clone();
            entry.sound_resref = sound_resref.clone();
            entry.sound_length = *sound_length;
            entry.flags = (entry.flags & !(TEXT_PRESENT | SOUND_PRESENT))
                | if entry.text.is_some() {
                    TEXT_PRESENT
                } else {
                    0
                }
                | if entry.sound_resref.is_some() {
                    SOUND_PRESENT
                } else {
                    0
                };
        }
        TlkEditAction::AppendEntry { text } => {
            if table.entries.len() >= MAX_ENTRIES as usize {
                return Err(tlk_error(
                    &table.source,
                    "TLK_ENTRY_LIMIT_EXCEEDED",
                    MAX_ENTRIES.to_string(),
                ));
            }
            table.entries.push(TlkEntry {
                index: table.entries.len() as u32,
                flags: if text.is_some() { TEXT_PRESENT } else { 0 },
                text: text.clone(),
                sound_resref: None,
                volume_variance: 0.0,
                pitch_variance: 0.0,
                sound_length: 0.0,
            });
        }
    }
    Ok(())
}

pub fn write_tlk(table: &TalkTable) -> AppResult<Vec<u8>> {
    if table.entries.len() > MAX_ENTRIES as usize {
        return Err(tlk_error(
            &table.source,
            "TLK_ENTRY_LIMIT_EXCEEDED",
            table.entries.len().to_string(),
        ));
    }
    let strings_offset = HEADER_SIZE
        .checked_add(table.entries.len().checked_mul(ENTRY_SIZE).ok_or_else(|| {
            tlk_error(
                &table.source,
                "TLK_SIZE_OVERFLOW",
                "entry table overflow".into(),
            )
        })?)
        .ok_or_else(|| tlk_error(&table.source, "TLK_SIZE_OVERFLOW", "header overflow".into()))?;
    let mut records = vec![0_u8; table.entries.len() * ENTRY_SIZE];
    let mut strings = Vec::new();
    for (position, entry) in table.entries.iter().enumerate() {
        if entry.index != position as u32 {
            return Err(tlk_error(
                &table.source,
                "TLK_ENTRY_INDEX_INVALID",
                format!("entry {} declares index {}", position, entry.index),
            ));
        }
        validate_sound(
            entry.sound_resref.as_deref(),
            entry.sound_length,
            &table.source,
        )?;
        if !entry.volume_variance.is_finite() || !entry.pitch_variance.is_finite() {
            return Err(tlk_error(
                &table.source,
                "TLK_SOUND_VARIANCE_INVALID",
                position.to_string(),
            ));
        }
        let base = position * ENTRY_SIZE;
        let flags = (entry.flags & !(TEXT_PRESENT | SOUND_PRESENT))
            | if entry.text.is_some() {
                TEXT_PRESENT
            } else {
                0
            }
            | if entry.sound_resref.is_some() {
                SOUND_PRESENT
            } else {
                0
            };
        records[base..base + 4].copy_from_slice(&flags.to_le_bytes());
        if let Some(sound) = &entry.sound_resref {
            records[base + 4..base + 4 + sound.len()].copy_from_slice(sound.as_bytes());
        }
        records[base + 20..base + 24].copy_from_slice(&entry.volume_variance.to_le_bytes());
        records[base + 24..base + 28].copy_from_slice(&entry.pitch_variance.to_le_bytes());
        let offset = u32::try_from(strings.len()).map_err(|_| {
            tlk_error(
                &table.source,
                "TLK_SIZE_OVERFLOW",
                "string offset exceeds u32".into(),
            )
        })?;
        let text = entry.text.as_deref().unwrap_or_default().as_bytes();
        let size = u32::try_from(text.len()).map_err(|_| {
            tlk_error(
                &table.source,
                "TLK_SIZE_OVERFLOW",
                "entry text exceeds u32".into(),
            )
        })?;
        records[base + 28..base + 32].copy_from_slice(&offset.to_le_bytes());
        records[base + 32..base + 36].copy_from_slice(&size.to_le_bytes());
        records[base + 36..base + 40].copy_from_slice(&entry.sound_length.to_le_bytes());
        strings.extend_from_slice(text);
    }
    let total = strings_offset.checked_add(strings.len()).ok_or_else(|| {
        tlk_error(
            &table.source,
            "TLK_SIZE_OVERFLOW",
            "output size overflow".into(),
        )
    })?;
    let mut output = Vec::with_capacity(total);
    output.extend_from_slice(b"TLK V3.0");
    output.extend_from_slice(&table.language_id.to_le_bytes());
    output.extend_from_slice(&(table.entries.len() as u32).to_le_bytes());
    output.extend_from_slice(&(strings_offset as u32).to_le_bytes());
    output.extend_from_slice(&records);
    output.extend_from_slice(&strings);
    Ok(output)
}

fn validate_sound(sound: Option<&str>, length: f32, source: &str) -> AppResult<()> {
    if !length.is_finite() || length < 0.0 {
        return Err(tlk_error(
            source,
            "TLK_SOUND_LENGTH_INVALID",
            length.to_string(),
        ));
    }
    if let Some(sound) = sound
        && (sound.is_empty()
            || sound.len() > 16
            || !sound.is_ascii()
            || sound.contains(['/', '\\', '\0']))
    {
        return Err(tlk_error(
            source,
            "TLK_SOUND_RESREF_INVALID",
            sound.to_owned(),
        ));
    }
    Ok(())
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

    #[test]
    fn edits_writes_and_reopens_a_tlk_deterministically() {
        let mut table = TalkTable {
            language_id: 0,
            entries: vec![TlkEntry {
                index: 0,
                flags: 0,
                text: None,
                sound_resref: None,
                volume_variance: 0.0,
                pitch_variance: 0.0,
                sound_length: 0.0,
            }],
            source: "custom.tlk".into(),
        };
        apply_tlk_edit(
            &mut table,
            &TlkEditAction::SetEntry {
                index: 0,
                text: Some("Bonjour".into()),
                sound_resref: Some("voice_01".into()),
                sound_length: 1.5,
            },
        )
        .expect("edit");
        apply_tlk_edit(
            &mut table,
            &TlkEditAction::AppendEntry {
                text: Some("Suite".into()),
            },
        )
        .expect("append");
        let first = write_tlk(&table).expect("write");
        let reopened = parse_tlk(&first, "reopened.tlk").expect("reopen");
        let second = write_tlk(&reopened).expect("rewrite");
        assert_eq!(first, second);
        assert_eq!(reopened.entries[0].text.as_deref(), Some("Bonjour"));
        assert_eq!(
            reopened.entries[0].sound_resref.as_deref(),
            Some("voice_01")
        );
    }
}
