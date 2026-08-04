use aurora_core::{AppError, AppResult, ErrorSeverity, decode_nwn_text};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const GFF_HEADER_SIZE: u64 = 56;
const STRUCT_SIZE: u64 = 12;
const FIELD_SIZE: u64 = 12;
const LABEL_SIZE: u64 = 16;
const MAX_STRUCTS: u32 = 250_000;
const MAX_FIELDS: u32 = 1_000_000;
const MAX_DEPTH: usize = 128;

const FIELD_BYTE: u32 = 0;
const FIELD_CHAR: u32 = 1;
const FIELD_WORD: u32 = 2;
const FIELD_SHORT: u32 = 3;
const FIELD_DWORD: u32 = 4;
const FIELD_INT: u32 = 5;
const FIELD_DWORD64: u32 = 6;
const FIELD_INT64: u32 = 7;
const FIELD_FLOAT: u32 = 8;
const FIELD_DOUBLE: u32 = 9;
const FIELD_CEXOSTRING: u32 = 10;
const FIELD_RESREF: u32 = 11;
const FIELD_CEXOLOCSTRING: u32 = 12;
const FIELD_VOID: u32 = 13;
const FIELD_STRUCT: u32 = 14;
const FIELD_LIST: u32 = 15;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GenericGff {
    pub file_type: String,
    pub file_version: String,
    pub source: String,
    pub struct_count: u32,
    pub field_count: u32,
    pub root: GenericStruct,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GenericStruct {
    pub index: u32,
    pub struct_type: u32,
    pub fields: Vec<GenericField>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GenericField {
    pub label: String,
    pub field_type: u32,
    pub value: GenericValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum GenericValue {
    Byte(u8),
    Char(i8),
    Word(u16),
    Short(i16),
    Dword(u32),
    Int(i32),
    Dword64(u64),
    Int64(i64),
    Float(f32),
    Double(f64),
    String(String),
    ResRef(String),
    LocalizedString(LocalizedString),
    Void(Vec<u8>),
    Struct(Box<GenericStruct>),
    List(Vec<GenericStruct>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalizedValue {
    pub language_id: u32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalizedString {
    pub string_ref: Option<u32>,
    pub values: Vec<LocalizedValue>,
}

impl LocalizedString {
    pub fn primary_text(&self) -> Option<&str> {
        self.values
            .iter()
            .find(|value| value.language_id == 0)
            .or_else(|| self.values.first())
            .map(|value| value.text.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModuleInfo {
    pub name: LocalizedString,
    pub description: LocalizedString,
    pub tag: String,
    pub minimum_game_version: Option<String>,
    pub custom_tlk: Option<String>,
    pub entry_area: String,
    pub hak_files: Vec<String>,
}

pub fn read_module_info(bytes: &[u8], source: &str) -> AppResult<ModuleInfo> {
    let document = GffDocument::parse(bytes, source)?;
    if document.header.file_type != "IFO " {
        return Err(gff_error(
            source,
            "GFF_UNEXPECTED_FILE_TYPE",
            format!(
                "Expected an IFO resource, found {:?}",
                document.header.file_type
            ),
        ));
    }

    let root = document.struct_fields(0)?;
    let name = document.required_locstring(&root, "Mod_Name")?;
    let description = document
        .optional_locstring(&root, "Mod_Description")?
        .unwrap_or(LocalizedString {
            string_ref: None,
            values: Vec::new(),
        });
    let tag = document.required_string(&root, "Mod_Tag")?;
    let minimum_game_version = document
        .optional_string(&root, "Mod_MinGameVer")?
        .filter(|value| !value.is_empty());
    let custom_tlk = document
        .optional_string(&root, "Mod_CustomTlk")?
        .filter(|value| !value.is_empty());
    let entry_area = document.required_resref(&root, "Mod_Entry_Area")?;
    let hak_files = document.read_string_list(&root, "Mod_HakList", "Mod_Hak")?;

    Ok(ModuleInfo {
        name,
        description,
        tag,
        minimum_game_version,
        custom_tlk,
        entry_area,
        hak_files,
    })
}

pub fn parse_gff(bytes: &[u8], source: &str) -> AppResult<GenericGff> {
    let document = GffDocument::parse(bytes, source)?;
    let mut stack = BTreeSet::new();
    let root = document.generic_struct(0, 0, &mut stack)?;
    Ok(GenericGff {
        file_type: document.header.file_type.clone(),
        file_version: "V3.2".to_owned(),
        source: source.to_owned(),
        struct_count: document.header.struct_count,
        field_count: document.header.field_count,
        root,
    })
}

#[derive(Debug)]
struct GffHeader {
    file_type: String,
    struct_offset: u64,
    struct_count: u32,
    field_offset: u64,
    field_count: u32,
    label_offset: u64,
    label_count: u32,
    field_data_offset: u64,
    field_data_size: u64,
    field_indices_offset: u64,
    field_indices_size: u64,
    list_indices_offset: u64,
    list_indices_size: u64,
}

#[derive(Debug, Clone, Copy)]
struct FieldRecord {
    field_type: u32,
    label_index: u32,
    data: u32,
}

struct GffDocument<'a> {
    bytes: &'a [u8],
    source: &'a str,
    header: GffHeader,
}

impl<'a> GffDocument<'a> {
    fn parse(bytes: &'a [u8], source: &'a str) -> AppResult<Self> {
        ensure_range(source, "GFF_HEADER_TOO_SHORT", bytes, 0, GFF_HEADER_SIZE)?;
        let header = GffHeader {
            file_type: text(&bytes[0..4]),
            struct_offset: u64::from(little_u32(bytes, 8)),
            struct_count: little_u32(bytes, 12),
            field_offset: u64::from(little_u32(bytes, 16)),
            field_count: little_u32(bytes, 20),
            label_offset: u64::from(little_u32(bytes, 24)),
            label_count: little_u32(bytes, 28),
            field_data_offset: u64::from(little_u32(bytes, 32)),
            field_data_size: u64::from(little_u32(bytes, 36)),
            field_indices_offset: u64::from(little_u32(bytes, 40)),
            field_indices_size: u64::from(little_u32(bytes, 44)),
            list_indices_offset: u64::from(little_u32(bytes, 48)),
            list_indices_size: u64::from(little_u32(bytes, 52)),
        };
        if &bytes[4..8] != b"V3.2" {
            return Err(gff_error(
                source,
                "GFF_UNSUPPORTED_VERSION",
                format!(
                    "Unsupported GFF version {:?}; expected V3.2",
                    text(&bytes[4..8])
                ),
            ));
        }
        if header.struct_count == 0 || header.struct_count > MAX_STRUCTS {
            return Err(gff_error(
                source,
                "GFF_STRUCT_LIMIT_INVALID",
                format!(
                    "GFF declares {} structs; accepted range is 1..={MAX_STRUCTS}",
                    header.struct_count
                ),
            ));
        }
        if header.field_count > MAX_FIELDS || header.label_count > MAX_FIELDS {
            return Err(gff_error(
                source,
                "GFF_FIELD_LIMIT_EXCEEDED",
                format!(
                    "GFF declares {} fields and {} labels; limit is {MAX_FIELDS}",
                    header.field_count, header.label_count
                ),
            ));
        }

        validate_table(
            source,
            bytes,
            header.struct_offset,
            header.struct_count,
            STRUCT_SIZE,
            "GFF_STRUCT_TABLE_OUT_OF_BOUNDS",
        )?;
        validate_table(
            source,
            bytes,
            header.field_offset,
            header.field_count,
            FIELD_SIZE,
            "GFF_FIELD_TABLE_OUT_OF_BOUNDS",
        )?;
        validate_table(
            source,
            bytes,
            header.label_offset,
            header.label_count,
            LABEL_SIZE,
            "GFF_LABEL_TABLE_OUT_OF_BOUNDS",
        )?;
        ensure_range(
            source,
            "GFF_FIELD_DATA_OUT_OF_BOUNDS",
            bytes,
            header.field_data_offset,
            header.field_data_size,
        )?;
        ensure_range(
            source,
            "GFF_FIELD_INDICES_OUT_OF_BOUNDS",
            bytes,
            header.field_indices_offset,
            header.field_indices_size,
        )?;
        ensure_range(
            source,
            "GFF_LIST_INDICES_OUT_OF_BOUNDS",
            bytes,
            header.list_indices_offset,
            header.list_indices_size,
        )?;

        Ok(Self {
            bytes,
            source,
            header,
        })
    }

    fn struct_fields(&self, struct_index: u32) -> AppResult<Vec<FieldRecord>> {
        if struct_index >= self.header.struct_count {
            return Err(gff_error(
                self.source,
                "GFF_STRUCT_INDEX_INVALID",
                format!("Struct index {struct_index} is outside the struct table"),
            ));
        }
        let base = self.header.struct_offset + u64::from(struct_index) * STRUCT_SIZE;
        let field_data = self.u32_at(base + 4)?;
        let field_count = self.u32_at(base + 8)?;
        if field_count == 0 {
            return Ok(Vec::new());
        }

        let field_indices = if field_count == 1 {
            vec![field_data]
        } else {
            let byte_count = u64::from(field_count).checked_mul(4).ok_or_else(|| {
                gff_error(
                    self.source,
                    "GFF_FIELD_INDEX_OVERFLOW",
                    format!("Field index count {field_count} overflows"),
                )
            })?;
            let start = self.header.field_indices_offset + u64::from(field_data);
            ensure_subrange(
                self.source,
                "GFF_FIELD_INDICES_OUT_OF_BOUNDS",
                start,
                byte_count,
                self.header.field_indices_offset,
                self.header.field_indices_size,
            )?;
            (0..field_count)
                .map(|index| self.u32_at(start + u64::from(index) * 4))
                .collect::<AppResult<Vec<_>>>()?
        };

        field_indices
            .into_iter()
            .map(|field_index| self.field(field_index))
            .collect()
    }

    fn generic_struct(
        &self,
        struct_index: u32,
        depth: usize,
        stack: &mut BTreeSet<u32>,
    ) -> AppResult<GenericStruct> {
        if depth > MAX_DEPTH {
            return Err(gff_error(
                self.source,
                "GFF_DEPTH_LIMIT_EXCEEDED",
                format!("Struct nesting exceeds {MAX_DEPTH}"),
            ));
        }
        if !stack.insert(struct_index) {
            return Err(gff_error(
                self.source,
                "GFF_STRUCT_CYCLE",
                format!("Struct {struct_index} recursively references itself"),
            ));
        }
        if struct_index >= self.header.struct_count {
            return Err(gff_error(
                self.source,
                "GFF_STRUCT_INDEX_INVALID",
                format!("Struct index {struct_index} is outside the struct table"),
            ));
        }
        let base = self.header.struct_offset + u64::from(struct_index) * STRUCT_SIZE;
        let struct_type = self.u32_at(base)?;
        let mut result = Vec::new();
        for field in self.struct_fields(struct_index)? {
            result.push(GenericField {
                label: self.label(field.label_index)?,
                field_type: field.field_type,
                value: self.generic_value(field, depth + 1, stack)?,
            });
        }
        stack.remove(&struct_index);
        Ok(GenericStruct {
            index: struct_index,
            struct_type,
            fields: result,
        })
    }

    fn generic_value(
        &self,
        field: FieldRecord,
        depth: usize,
        stack: &mut BTreeSet<u32>,
    ) -> AppResult<GenericValue> {
        let data_offset = self.header.field_data_offset + u64::from(field.data);
        match field.field_type {
            FIELD_BYTE => Ok(GenericValue::Byte(field.data as u8)),
            FIELD_CHAR => Ok(GenericValue::Char(field.data as u8 as i8)),
            FIELD_WORD => Ok(GenericValue::Word(field.data as u16)),
            FIELD_SHORT => Ok(GenericValue::Short(field.data as u16 as i16)),
            FIELD_DWORD => Ok(GenericValue::Dword(field.data)),
            FIELD_INT => Ok(GenericValue::Int(field.data as i32)),
            FIELD_DWORD64 => Ok(GenericValue::Dword64(self.field_u64_at(data_offset)?)),
            FIELD_INT64 => Ok(GenericValue::Int64(self.field_u64_at(data_offset)? as i64)),
            FIELD_FLOAT => Ok(GenericValue::Float(f32::from_bits(field.data))),
            FIELD_DOUBLE => Ok(GenericValue::Double(f64::from_bits(
                self.field_u64_at(data_offset)?,
            ))),
            FIELD_CEXOSTRING => Ok(GenericValue::String(self.string_at(data_offset)?)),
            FIELD_RESREF => Ok(GenericValue::ResRef(self.resref_at(data_offset)?)),
            FIELD_CEXOLOCSTRING => Ok(GenericValue::LocalizedString(
                self.locstring_at(data_offset)?,
            )),
            FIELD_VOID => {
                let size = u64::from(self.u32_at(data_offset)?);
                ensure_subrange(
                    self.source,
                    "GFF_VOID_OUT_OF_BOUNDS",
                    data_offset + 4,
                    size,
                    self.header.field_data_offset,
                    self.header.field_data_size,
                )?;
                Ok(GenericValue::Void(
                    self.slice(data_offset + 4, size)?.to_vec(),
                ))
            }
            FIELD_STRUCT => Ok(GenericValue::Struct(Box::new(
                self.generic_struct(field.data, depth, stack)?,
            ))),
            FIELD_LIST => {
                let start = self.header.list_indices_offset + u64::from(field.data);
                ensure_subrange(
                    self.source,
                    "GFF_LIST_INDICES_OUT_OF_BOUNDS",
                    start,
                    4,
                    self.header.list_indices_offset,
                    self.header.list_indices_size,
                )?;
                let count = self.u32_at(start)?;
                let bytes = u64::from(count).checked_mul(4).ok_or_else(|| {
                    gff_error(
                        self.source,
                        "GFF_LIST_SIZE_OVERFLOW",
                        format!("List count {count} overflows"),
                    )
                })?;
                ensure_subrange(
                    self.source,
                    "GFF_LIST_INDICES_OUT_OF_BOUNDS",
                    start + 4,
                    bytes,
                    self.header.list_indices_offset,
                    self.header.list_indices_size,
                )?;
                let mut values = Vec::with_capacity(count as usize);
                for index in 0..count {
                    values.push(self.generic_struct(
                        self.u32_at(start + 4 + u64::from(index) * 4)?,
                        depth,
                        stack,
                    )?);
                }
                Ok(GenericValue::List(values))
            }
            value => Err(gff_error(
                self.source,
                "GFF_FIELD_TYPE_UNSUPPORTED",
                format!("Unknown field type {value}"),
            )),
        }
    }

    fn string_at(&self, start: u64) -> AppResult<String> {
        ensure_subrange(
            self.source,
            "GFF_STRING_OUT_OF_BOUNDS",
            start,
            4,
            self.header.field_data_offset,
            self.header.field_data_size,
        )?;
        let size = u64::from(self.u32_at(start)?);
        ensure_subrange(
            self.source,
            "GFF_STRING_OUT_OF_BOUNDS",
            start + 4,
            size,
            self.header.field_data_offset,
            self.header.field_data_size,
        )?;
        Ok(text(self.slice(start + 4, size)?))
    }

    fn resref_at(&self, start: u64) -> AppResult<String> {
        ensure_subrange(
            self.source,
            "GFF_RESREF_OUT_OF_BOUNDS",
            start,
            1,
            self.header.field_data_offset,
            self.header.field_data_size,
        )?;
        let size = u64::from(self.slice(start, 1)?[0]);
        ensure_subrange(
            self.source,
            "GFF_RESREF_OUT_OF_BOUNDS",
            start + 1,
            size,
            self.header.field_data_offset,
            self.header.field_data_size,
        )?;
        Ok(text(self.slice(start + 1, size)?))
    }

    fn locstring_at(&self, start: u64) -> AppResult<LocalizedString> {
        ensure_subrange(
            self.source,
            "GFF_LOCSTRING_OUT_OF_BOUNDS",
            start,
            12,
            self.header.field_data_offset,
            self.header.field_data_size,
        )?;
        let payload_size = u64::from(self.u32_at(start)?);
        ensure_subrange(
            self.source,
            "GFF_LOCSTRING_OUT_OF_BOUNDS",
            start + 4,
            payload_size,
            self.header.field_data_offset,
            self.header.field_data_size,
        )?;
        let string_ref = self.u32_at(start + 4)?;
        let count = self.u32_at(start + 8)?;
        let end = start + 4 + payload_size;
        let mut cursor = start + 12;
        let mut values = Vec::with_capacity(count as usize);
        for _ in 0..count {
            if cursor + 8 > end {
                return Err(gff_error(
                    self.source,
                    "GFF_LOCSTRING_OUT_OF_BOUNDS",
                    format!("Localized string header at {cursor} exceeds {end}"),
                ));
            }
            let language_id = self.u32_at(cursor)?;
            let size = u64::from(self.u32_at(cursor + 4)?);
            if cursor + 8 + size > end {
                return Err(gff_error(
                    self.source,
                    "GFF_LOCSTRING_OUT_OF_BOUNDS",
                    format!("Localized value exceeds payload ending at {end}"),
                ));
            }
            values.push(LocalizedValue {
                language_id,
                text: text(self.slice(cursor + 8, size)?),
            });
            cursor += 8 + size;
        }
        Ok(LocalizedString {
            string_ref: (string_ref != u32::MAX).then_some(string_ref),
            values,
        })
    }

    fn u64_at(&self, offset: u64) -> AppResult<u64> {
        let bytes = self.slice(offset, 8)?;
        Ok(u64::from_le_bytes(
            bytes.try_into().expect("eight-byte slice"),
        ))
    }

    fn field_u64_at(&self, offset: u64) -> AppResult<u64> {
        ensure_subrange(
            self.source,
            "GFF_FIELD_DATA_OUT_OF_BOUNDS",
            offset,
            8,
            self.header.field_data_offset,
            self.header.field_data_size,
        )?;
        self.u64_at(offset)
    }

    fn field(&self, field_index: u32) -> AppResult<FieldRecord> {
        if field_index >= self.header.field_count {
            return Err(gff_error(
                self.source,
                "GFF_FIELD_INDEX_INVALID",
                format!("Field index {field_index} is outside the field table"),
            ));
        }
        let base = self.header.field_offset + u64::from(field_index) * FIELD_SIZE;
        Ok(FieldRecord {
            field_type: self.u32_at(base)?,
            label_index: self.u32_at(base + 4)?,
            data: self.u32_at(base + 8)?,
        })
    }

    fn field_named(&self, fields: &[FieldRecord], name: &str) -> AppResult<Option<FieldRecord>> {
        for field in fields {
            if self.label(field.label_index)? == name {
                return Ok(Some(*field));
            }
        }
        Ok(None)
    }

    fn label(&self, label_index: u32) -> AppResult<String> {
        if label_index >= self.header.label_count {
            return Err(gff_error(
                self.source,
                "GFF_LABEL_INDEX_INVALID",
                format!("Label index {label_index} is outside the label table"),
            ));
        }
        let start = self.header.label_offset + u64::from(label_index) * LABEL_SIZE;
        let bytes = self.slice(start, LABEL_SIZE)?;
        let end = bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(bytes.len());
        Ok(text(&bytes[..end]))
    }

    fn required_string(&self, fields: &[FieldRecord], name: &str) -> AppResult<String> {
        self.optional_string(fields, name)?.ok_or_else(|| {
            gff_error(
                self.source,
                "GFF_REQUIRED_FIELD_MISSING",
                format!("Required CExoString field {name} is missing"),
            )
        })
    }

    fn optional_string(&self, fields: &[FieldRecord], name: &str) -> AppResult<Option<String>> {
        let Some(field) = self.field_named(fields, name)? else {
            return Ok(None);
        };
        self.expect_type(name, field, FIELD_CEXOSTRING)?;
        let start = self.header.field_data_offset + u64::from(field.data);
        ensure_subrange(
            self.source,
            "GFF_STRING_OUT_OF_BOUNDS",
            start,
            4,
            self.header.field_data_offset,
            self.header.field_data_size,
        )?;
        let length = u64::from(self.u32_at(start)?);
        ensure_subrange(
            self.source,
            "GFF_STRING_OUT_OF_BOUNDS",
            start + 4,
            length,
            self.header.field_data_offset,
            self.header.field_data_size,
        )?;
        Ok(Some(text(self.slice(start + 4, length)?)))
    }

    fn required_resref(&self, fields: &[FieldRecord], name: &str) -> AppResult<String> {
        let field = self.field_named(fields, name)?.ok_or_else(|| {
            gff_error(
                self.source,
                "GFF_REQUIRED_FIELD_MISSING",
                format!("Required ResRef field {name} is missing"),
            )
        })?;
        self.expect_type(name, field, FIELD_RESREF)?;
        let start = self.header.field_data_offset + u64::from(field.data);
        ensure_subrange(
            self.source,
            "GFF_RESREF_OUT_OF_BOUNDS",
            start,
            1,
            self.header.field_data_offset,
            self.header.field_data_size,
        )?;
        let length = u64::from(*self.slice(start, 1)?.first().expect("one-byte slice"));
        ensure_subrange(
            self.source,
            "GFF_RESREF_OUT_OF_BOUNDS",
            start + 1,
            length,
            self.header.field_data_offset,
            self.header.field_data_size,
        )?;
        Ok(text(self.slice(start + 1, length)?))
    }

    fn required_locstring(&self, fields: &[FieldRecord], name: &str) -> AppResult<LocalizedString> {
        self.optional_locstring(fields, name)?.ok_or_else(|| {
            gff_error(
                self.source,
                "GFF_REQUIRED_FIELD_MISSING",
                format!("Required CExoLocString field {name} is missing"),
            )
        })
    }

    fn optional_locstring(
        &self,
        fields: &[FieldRecord],
        name: &str,
    ) -> AppResult<Option<LocalizedString>> {
        let Some(field) = self.field_named(fields, name)? else {
            return Ok(None);
        };
        self.expect_type(name, field, FIELD_CEXOLOCSTRING)?;
        let start = self.header.field_data_offset + u64::from(field.data);
        ensure_subrange(
            self.source,
            "GFF_LOCSTRING_OUT_OF_BOUNDS",
            start,
            12,
            self.header.field_data_offset,
            self.header.field_data_size,
        )?;
        let payload_size = u64::from(self.u32_at(start)?);
        ensure_subrange(
            self.source,
            "GFF_LOCSTRING_OUT_OF_BOUNDS",
            start + 4,
            payload_size,
            self.header.field_data_offset,
            self.header.field_data_size,
        )?;
        let string_ref = self.u32_at(start + 4)?;
        let string_count = self.u32_at(start + 8)?;
        let mut cursor = start + 12;
        let end = start + 4 + payload_size;
        let mut values = Vec::with_capacity(string_count as usize);
        for _ in 0..string_count {
            if cursor + 8 > end {
                return Err(gff_error(
                    self.source,
                    "GFF_LOCSTRING_OUT_OF_BOUNDS",
                    format!("Localized string header at {cursor} exceeds payload ending at {end}"),
                ));
            }
            let language_id = self.u32_at(cursor)?;
            let length = u64::from(self.u32_at(cursor + 4)?);
            let value_start = cursor + 8;
            let value_end = value_start.checked_add(length).ok_or_else(|| {
                gff_error(
                    self.source,
                    "GFF_LOCSTRING_OUT_OF_BOUNDS",
                    "Localized string length overflows".to_owned(),
                )
            })?;
            if value_end > end {
                return Err(gff_error(
                    self.source,
                    "GFF_LOCSTRING_OUT_OF_BOUNDS",
                    format!("Localized string ends at {value_end}, payload ends at {end}"),
                ));
            }
            values.push(LocalizedValue {
                language_id,
                text: text(self.slice(value_start, length)?),
            });
            cursor = value_end;
        }

        Ok(Some(LocalizedString {
            string_ref: (string_ref != u32::MAX).then_some(string_ref),
            values,
        }))
    }

    fn read_string_list(
        &self,
        fields: &[FieldRecord],
        list_name: &str,
        value_name: &str,
    ) -> AppResult<Vec<String>> {
        let Some(field) = self.field_named(fields, list_name)? else {
            return Ok(Vec::new());
        };
        self.expect_type(list_name, field, FIELD_LIST)?;
        let start = self.header.list_indices_offset + u64::from(field.data);
        ensure_subrange(
            self.source,
            "GFF_LIST_INDICES_OUT_OF_BOUNDS",
            start,
            4,
            self.header.list_indices_offset,
            self.header.list_indices_size,
        )?;
        let count = self.u32_at(start)?;
        let byte_count = u64::from(count).checked_mul(4).ok_or_else(|| {
            gff_error(
                self.source,
                "GFF_LIST_SIZE_OVERFLOW",
                format!("List {list_name} count {count} overflows"),
            )
        })?;
        ensure_subrange(
            self.source,
            "GFF_LIST_INDICES_OUT_OF_BOUNDS",
            start + 4,
            byte_count,
            self.header.list_indices_offset,
            self.header.list_indices_size,
        )?;

        let mut values = Vec::with_capacity(count as usize);
        for index in 0..count {
            let struct_index = self.u32_at(start + 4 + u64::from(index) * 4)?;
            let child_fields = self.struct_fields(struct_index)?;
            if let Some(value) = self.optional_string(&child_fields, value_name)?
                && !value.is_empty()
            {
                values.push(value);
            }
        }
        Ok(values)
    }

    fn expect_type(&self, name: &str, field: FieldRecord, expected: u32) -> AppResult<()> {
        if field.field_type != expected {
            return Err(gff_error(
                self.source,
                "GFF_FIELD_TYPE_MISMATCH",
                format!(
                    "Field {name} has type {}, expected {expected}",
                    field.field_type
                ),
            ));
        }
        Ok(())
    }

    fn u32_at(&self, offset: u64) -> AppResult<u32> {
        let bytes = self.slice(offset, 4)?;
        Ok(little_u32(bytes, 0))
    }

    fn slice(&self, offset: u64, size: u64) -> AppResult<&'a [u8]> {
        ensure_range(
            self.source,
            "GFF_READ_OUT_OF_BOUNDS",
            self.bytes,
            offset,
            size,
        )?;
        let start = usize::try_from(offset).map_err(|_| {
            gff_error(
                self.source,
                "GFF_OFFSET_UNREPRESENTABLE",
                format!("Offset {offset} cannot be represented on this platform"),
            )
        })?;
        let end = usize::try_from(offset + size).map_err(|_| {
            gff_error(
                self.source,
                "GFF_OFFSET_UNREPRESENTABLE",
                format!(
                    "End offset {} cannot be represented on this platform",
                    offset + size
                ),
            )
        })?;
        Ok(&self.bytes[start..end])
    }
}

fn validate_table(
    source: &str,
    bytes: &[u8],
    offset: u64,
    count: u32,
    record_size: u64,
    code: &str,
) -> AppResult<()> {
    let size = u64::from(count).checked_mul(record_size).ok_or_else(|| {
        gff_error(
            source,
            code,
            format!("Table count {count} times record size {record_size} overflows"),
        )
    })?;
    ensure_range(source, code, bytes, offset, size)
}

fn ensure_range(source: &str, code: &str, bytes: &[u8], offset: u64, size: u64) -> AppResult<()> {
    let end = offset.checked_add(size).ok_or_else(|| {
        gff_error(
            source,
            code,
            format!("Range offset {offset} plus size {size} overflows"),
        )
    })?;
    if end > bytes.len() as u64 {
        return Err(gff_error(
            source,
            code,
            format!(
                "Range {offset}..{end} exceeds resource size {}",
                bytes.len()
            ),
        ));
    }
    Ok(())
}

fn ensure_subrange(
    source: &str,
    code: &str,
    offset: u64,
    size: u64,
    section_offset: u64,
    section_size: u64,
) -> AppResult<()> {
    let end = offset.checked_add(size).ok_or_else(|| {
        gff_error(
            source,
            code,
            format!("Subrange {offset} plus {size} overflows"),
        )
    })?;
    let section_end = section_offset.checked_add(section_size).ok_or_else(|| {
        gff_error(
            source,
            code,
            format!("Section {section_offset} plus {section_size} overflows"),
        )
    })?;
    if offset < section_offset || end > section_end {
        return Err(gff_error(
            source,
            code,
            format!("Subrange {offset}..{end} exceeds section {section_offset}..{section_end}"),
        ));
    }
    Ok(())
}

fn little_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn text(bytes: &[u8]) -> String {
    decode_nwn_text(bytes)
}

fn gff_error(source: &str, code: &str, detail: String) -> Box<AppError> {
    Box::new(
        AppError::new(
            code,
            "Les informations du module sont invalides ou incomplètes.",
            detail,
            ErrorSeverity::Error,
        )
        .with_source(source)
        .with_import_stage("module_ifo")
        .with_suggestion("Vérifiez la copie du module et consultez les diagnostics techniques."),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_minimal_module_info_contract() {
        let bytes = synthetic_ifo();
        let info = read_module_info(&bytes, "fixture.mod::module.ifo").expect("module info");

        assert_eq!(info.name.primary_text(), Some("Forge Test"));
        assert_eq!(info.description.primary_text(), Some("Synthetic module"));
        assert_eq!(info.tag, "MODULE");
        assert_eq!(info.minimum_game_version.as_deref(), Some("1.69"));
        assert_eq!(info.custom_tlk.as_deref(), Some("forge_dialog"));
        assert_eq!(info.entry_area, "startarea");
        assert_eq!(info.hak_files, vec!["forge_assets"]);
    }

    #[test]
    fn generic_reader_preserves_field_order_types_and_nested_lists() {
        let document = parse_gff(&synthetic_ifo(), "fixture.mod::module.ifo").expect("generic GFF");
        assert_eq!(document.file_type, "IFO ");
        assert_eq!(document.root.fields.len(), 7);
        assert_eq!(document.root.fields[0].label, "Mod_MinGameVer");
        assert_eq!(document.root.fields[0].field_type, FIELD_CEXOSTRING);
        let GenericValue::List(values) = &document.root.fields[6].value else {
            panic!("HAK list")
        };
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].fields[0].label, "Mod_Hak");
    }

    #[test]
    fn rejects_a_section_outside_the_resource() {
        let mut bytes = synthetic_ifo();
        bytes[32..36].copy_from_slice(&u32::MAX.to_le_bytes());

        let error = read_module_info(&bytes, "broken.ifo").expect_err("invalid field data");

        assert_eq!(error.code, "GFF_FIELD_DATA_OUT_OF_BOUNDS");
    }

    #[test]
    fn accepts_legacy_module_info_without_minimum_game_version() {
        let mut bytes = synthetic_ifo();
        let label_offset = little_u32(&bytes, 24) as usize;
        bytes[label_offset..label_offset + 14].copy_from_slice(b"UnusedMinVerxx");

        let info = read_module_info(&bytes, "legacy.mod::module.ifo").expect("legacy module info");

        assert_eq!(info.minimum_game_version, None);
    }

    fn synthetic_ifo() -> Vec<u8> {
        let labels = [
            "Mod_MinGameVer",
            "Mod_Name",
            "Mod_Description",
            "Mod_Tag",
            "Mod_CustomTlk",
            "Mod_Entry_Area",
            "Mod_HakList",
            "Mod_Hak",
        ];
        let mut field_data = Vec::new();
        let min_version = push_string(&mut field_data, "1.69");
        let name = push_locstring(&mut field_data, "Forge Test");
        let description = push_locstring(&mut field_data, "Synthetic module");
        let tag = push_string(&mut field_data, "MODULE");
        let custom_tlk = push_string(&mut field_data, "forge_dialog");
        let entry_area = push_resref(&mut field_data, "startarea");
        let hak = push_string(&mut field_data, "forge_assets");

        let fields = [
            (FIELD_CEXOSTRING, 0_u32, min_version),
            (FIELD_CEXOLOCSTRING, 1, name),
            (FIELD_CEXOLOCSTRING, 2, description),
            (FIELD_CEXOSTRING, 3, tag),
            (FIELD_CEXOSTRING, 4, custom_tlk),
            (FIELD_RESREF, 5, entry_area),
            (FIELD_LIST, 6, 0_u32),
            (FIELD_CEXOSTRING, 7, hak),
        ];

        let struct_offset = 56_u32;
        let field_offset = struct_offset + 2 * 12;
        let label_offset = field_offset + fields.len() as u32 * 12;
        let field_data_offset = label_offset + labels.len() as u32 * 16;
        let field_indices_offset = field_data_offset + field_data.len() as u32;
        let field_indices_size = 7 * 4;
        let list_indices_offset = field_indices_offset + field_indices_size;
        let list_indices_size = 8_u32;
        let mut bytes = vec![0_u8; (list_indices_offset + list_indices_size) as usize];
        bytes[0..4].copy_from_slice(b"IFO ");
        bytes[4..8].copy_from_slice(b"V3.2");
        put_u32(&mut bytes, 8, struct_offset);
        put_u32(&mut bytes, 12, 2);
        put_u32(&mut bytes, 16, field_offset);
        put_u32(&mut bytes, 20, fields.len() as u32);
        put_u32(&mut bytes, 24, label_offset);
        put_u32(&mut bytes, 28, labels.len() as u32);
        put_u32(&mut bytes, 32, field_data_offset);
        put_u32(&mut bytes, 36, field_data.len() as u32);
        put_u32(&mut bytes, 40, field_indices_offset);
        put_u32(&mut bytes, 44, field_indices_size);
        put_u32(&mut bytes, 48, list_indices_offset);
        put_u32(&mut bytes, 52, list_indices_size);

        put_struct(&mut bytes, struct_offset as usize, u32::MAX, 0, 7);
        put_struct(&mut bytes, struct_offset as usize + 12, 0, 7, 1);
        for (index, (field_type, label_index, data)) in fields.iter().enumerate() {
            let base = field_offset as usize + index * 12;
            put_u32(&mut bytes, base, *field_type);
            put_u32(&mut bytes, base + 4, *label_index);
            put_u32(&mut bytes, base + 8, *data);
        }
        for (index, label) in labels.iter().enumerate() {
            let start = label_offset as usize + index * 16;
            bytes[start..start + label.len()].copy_from_slice(label.as_bytes());
        }
        bytes[field_data_offset as usize..field_indices_offset as usize]
            .copy_from_slice(&field_data);
        for index in 0..7_u32 {
            put_u32(
                &mut bytes,
                field_indices_offset as usize + index as usize * 4,
                index,
            );
        }
        put_u32(&mut bytes, list_indices_offset as usize, 1);
        put_u32(&mut bytes, list_indices_offset as usize + 4, 1);
        bytes
    }

    fn push_string(data: &mut Vec<u8>, value: &str) -> u32 {
        let offset = data.len() as u32;
        data.extend_from_slice(&(value.len() as u32).to_le_bytes());
        data.extend_from_slice(value.as_bytes());
        offset
    }

    fn push_resref(data: &mut Vec<u8>, value: &str) -> u32 {
        let offset = data.len() as u32;
        data.push(value.len() as u8);
        data.extend_from_slice(value.as_bytes());
        offset
    }

    fn push_locstring(data: &mut Vec<u8>, value: &str) -> u32 {
        let offset = data.len() as u32;
        let payload_size = 16 + value.len() as u32;
        data.extend_from_slice(&payload_size.to_le_bytes());
        data.extend_from_slice(&u32::MAX.to_le_bytes());
        data.extend_from_slice(&1_u32.to_le_bytes());
        data.extend_from_slice(&0_u32.to_le_bytes());
        data.extend_from_slice(&(value.len() as u32).to_le_bytes());
        data.extend_from_slice(value.as_bytes());
        offset
    }

    fn put_struct(bytes: &mut [u8], offset: usize, kind: u32, fields: u32, count: u32) {
        put_u32(bytes, offset, kind);
        put_u32(bytes, offset + 4, fields);
        put_u32(bytes, offset + 8, count);
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
}
