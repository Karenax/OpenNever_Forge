use aurora_core::{AppError, AppResult, ErrorSeverity, decode_nwn_text};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const MAX_ROWS: usize = 1_000_000;
const MAX_COLUMNS: usize = 16_384;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TwoDaRow {
    pub label: String,
    pub cells: Vec<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TwoDaTable {
    pub source: String,
    pub default_value: Option<String>,
    pub columns: Vec<String>,
    pub rows: Vec<TwoDaRow>,
}

impl TwoDaTable {
    pub fn cell(&self, row: usize, column: &str) -> Option<&str> {
        let column = self
            .columns
            .iter()
            .position(|value| value.eq_ignore_ascii_case(column))?;
        self.rows
            .get(row)?
            .cells
            .get(column)?
            .as_deref()
            .or(self.default_value.as_deref())
    }
}

pub fn parse_2da(bytes: &[u8], source: &str) -> AppResult<TwoDaTable> {
    let text = decode_nwn_text(bytes)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let mut lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"));
    let Some(header) = lines.next() else {
        return Err(two_da_error(
            source,
            "TWO_DA_HEADER_MISSING",
            "Empty 2DA resource".into(),
        ));
    };
    if !header.eq_ignore_ascii_case("2DA V2.0") {
        return Err(two_da_error(
            source,
            "TWO_DA_HEADER_UNSUPPORTED",
            format!("Expected 2DA V2.0, got {header:?}"),
        ));
    }
    let mut default_value = None;
    let mut column_line = lines.next().ok_or_else(|| {
        two_da_error(
            source,
            "TWO_DA_COLUMNS_MISSING",
            "No column declaration".into(),
        )
    })?;
    if let Some(value) = column_line
        .strip_prefix("DEFAULT:")
        .or_else(|| column_line.strip_prefix("default:"))
    {
        default_value = normalize_cell(value.trim());
        column_line = lines.next().ok_or_else(|| {
            two_da_error(
                source,
                "TWO_DA_COLUMNS_MISSING",
                "No columns after DEFAULT".into(),
            )
        })?;
    }
    let columns = tokenize(column_line, source)?;
    if columns.is_empty() || columns.len() > MAX_COLUMNS {
        return Err(two_da_error(
            source,
            "TWO_DA_COLUMN_LIMIT_EXCEEDED",
            format!("{} columns", columns.len()),
        ));
    }
    let mut rows = Vec::new();
    for line in lines {
        if rows.len() >= MAX_ROWS {
            return Err(two_da_error(
                source,
                "TWO_DA_ROW_LIMIT_EXCEEDED",
                format!("More than {MAX_ROWS} rows"),
            ));
        }
        let mut values = tokenize(line, source)?;
        if values.is_empty() {
            continue;
        }
        let label = values.remove(0);
        if values.len() > columns.len() {
            let tail = values.split_off(columns.len() - 1);
            values.push(tail.join(" "));
        }
        let mut cells = values
            .into_iter()
            .map(|value| normalize_cell(&value))
            .collect::<Vec<_>>();
        cells.resize(columns.len(), None);
        rows.push(TwoDaRow { label, cells });
    }
    Ok(TwoDaTable {
        source: source.to_owned(),
        default_value,
        columns,
        rows,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum TwoDaEditAction {
    SetCell {
        row_index: usize,
        column_index: usize,
        value: Option<String>,
    },
    AddRow {
        label: String,
    },
    RemoveRow {
        row_index: usize,
    },
    SetDefault {
        value: Option<String>,
    },
}

pub fn apply_2da_edit(table: &mut TwoDaTable, action: &TwoDaEditAction) -> AppResult<()> {
    match action {
        TwoDaEditAction::SetCell {
            row_index,
            column_index,
            value,
        } => {
            let column_count = table.columns.len();
            let row = table.rows.get_mut(*row_index).ok_or_else(|| {
                two_da_error(
                    &table.source,
                    "TWO_DA_ROW_OUT_OF_BOUNDS",
                    row_index.to_string(),
                )
            })?;
            if *column_index >= column_count {
                return Err(two_da_error(
                    &table.source,
                    "TWO_DA_COLUMN_OUT_OF_BOUNDS",
                    column_index.to_string(),
                ));
            }
            row.cells.resize(column_count, None);
            row.cells[*column_index] = value.clone();
        }
        TwoDaEditAction::AddRow { label } => {
            validate_row_label(table, label)?;
            if table
                .rows
                .iter()
                .any(|row| row.label.eq_ignore_ascii_case(label))
            {
                return Err(two_da_error(
                    &table.source,
                    "TWO_DA_ROW_DUPLICATE",
                    label.clone(),
                ));
            }
            table.rows.push(TwoDaRow {
                label: label.clone(),
                cells: vec![None; table.columns.len()],
            });
        }
        TwoDaEditAction::RemoveRow { row_index } => {
            if *row_index >= table.rows.len() {
                return Err(two_da_error(
                    &table.source,
                    "TWO_DA_ROW_OUT_OF_BOUNDS",
                    row_index.to_string(),
                ));
            }
            table.rows.remove(*row_index);
        }
        TwoDaEditAction::SetDefault { value } => table.default_value = value.clone(),
    }
    validate_2da(table)
}

pub fn write_2da(table: &TwoDaTable) -> AppResult<Vec<u8>> {
    validate_2da(table)?;
    let mut output = String::from("2DA V2.0\n");
    if let Some(value) = &table.default_value {
        output.push_str("DEFAULT: ");
        output.push_str(&encode_token(value));
        output.push('\n');
    }
    output.push_str(
        &table
            .columns
            .iter()
            .map(|value| encode_token(value))
            .collect::<Vec<_>>()
            .join(" "),
    );
    output.push('\n');
    for row in &table.rows {
        output.push_str(&encode_token(&row.label));
        for cell in &row.cells {
            output.push(' ');
            output.push_str(
                &cell
                    .as_deref()
                    .map(encode_token)
                    .unwrap_or_else(|| "****".to_owned()),
            );
        }
        output.push('\n');
    }
    Ok(output.into_bytes())
}

fn validate_2da(table: &TwoDaTable) -> AppResult<()> {
    if table.columns.is_empty() || table.columns.len() > MAX_COLUMNS {
        return Err(two_da_error(
            &table.source,
            "TWO_DA_COLUMN_LIMIT_EXCEEDED",
            format!("{} columns", table.columns.len()),
        ));
    }
    if table.rows.len() > MAX_ROWS {
        return Err(two_da_error(
            &table.source,
            "TWO_DA_ROW_LIMIT_EXCEEDED",
            format!("{} rows", table.rows.len()),
        ));
    }
    let mut columns = BTreeSet::new();
    for column in &table.columns {
        if column.trim().is_empty() || !columns.insert(column.to_ascii_lowercase()) {
            return Err(two_da_error(
                &table.source,
                "TWO_DA_COLUMN_INVALID",
                format!("invalid or duplicate column {column:?}"),
            ));
        }
    }
    let mut labels = BTreeSet::new();
    for row in &table.rows {
        validate_row_label(table, &row.label)?;
        if !labels.insert(row.label.to_ascii_lowercase()) {
            return Err(two_da_error(
                &table.source,
                "TWO_DA_ROW_DUPLICATE",
                row.label.clone(),
            ));
        }
        if row.cells.len() != table.columns.len() {
            return Err(two_da_error(
                &table.source,
                "TWO_DA_CELL_COUNT_INVALID",
                format!(
                    "row {} has {} cells, expected {}",
                    row.label,
                    row.cells.len(),
                    table.columns.len()
                ),
            ));
        }
    }
    Ok(())
}

fn validate_row_label(table: &TwoDaTable, label: &str) -> AppResult<()> {
    if label.trim().is_empty() || label.chars().any(char::is_whitespace) {
        return Err(two_da_error(
            &table.source,
            "TWO_DA_ROW_LABEL_INVALID",
            format!("row label {label:?} must be one non-empty token"),
        ));
    }
    Ok(())
}

fn encode_token(value: &str) -> String {
    if !value.is_empty()
        && !value.chars().any(char::is_whitespace)
        && !value.contains('"')
        && !value.contains('\\')
        && value != "****"
    {
        return value.to_owned();
    }
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TwoDaDifference {
    pub row_label: String,
    pub column: String,
    pub left: Option<String>,
    pub right: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TwoDaVersion {
    pub source: String,
    pub priority: u32,
    pub table: TwoDaTable,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TwoDaManager {
    tables: BTreeMap<String, Vec<TwoDaVersion>>,
}

impl TwoDaManager {
    pub fn insert(&mut self, name: impl Into<String>, version: TwoDaVersion) {
        let versions = self
            .tables
            .entry(name.into().to_ascii_lowercase())
            .or_default();
        versions.push(version);
        versions.sort_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.source.cmp(&right.source))
        });
    }

    pub fn versions(&self, name: &str) -> &[TwoDaVersion] {
        self.tables
            .get(&name.to_ascii_lowercase())
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub fn selected(&self, name: &str) -> Option<&TwoDaVersion> {
        self.versions(name).first()
    }

    pub fn compare_versions(
        &self,
        name: &str,
        left: usize,
        right: usize,
    ) -> Option<Vec<TwoDaDifference>> {
        let versions = self.versions(name);
        Some(compare_2da(
            &versions.get(left)?.table,
            &versions.get(right)?.table,
        ))
    }
}

pub fn compare_2da(left: &TwoDaTable, right: &TwoDaTable) -> Vec<TwoDaDifference> {
    let columns = left
        .columns
        .iter()
        .chain(&right.columns)
        .map(|value| value.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let left_rows = row_map(left);
    let right_rows = row_map(right);
    let labels = left_rows
        .keys()
        .chain(right_rows.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut differences = Vec::new();
    for label in labels {
        for column in &columns {
            let left_value =
                lookup(left, left_rows.get(&label).copied(), column).map(str::to_owned);
            let right_value =
                lookup(right, right_rows.get(&label).copied(), column).map(str::to_owned);
            if left_value != right_value {
                differences.push(TwoDaDifference {
                    row_label: label.clone(),
                    column: column.clone(),
                    left: left_value,
                    right: right_value,
                });
            }
        }
    }
    differences
}

fn row_map(table: &TwoDaTable) -> BTreeMap<String, usize> {
    table
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| (row.label.to_ascii_lowercase(), index))
        .collect()
}
fn lookup<'a>(table: &'a TwoDaTable, row: Option<usize>, column: &str) -> Option<&'a str> {
    row.and_then(|row| table.cell(row, column))
}
fn normalize_cell(value: &str) -> Option<String> {
    (value != "****").then(|| value.to_owned())
}

fn tokenize(line: &str, source: &str) -> AppResult<Vec<String>> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut escaped = false;
    for character in line.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }
        if quoted && character == '\\' {
            escaped = true;
            continue;
        }
        if character == '"' {
            quoted = !quoted;
            continue;
        }
        if character.is_whitespace() && !quoted {
            if !current.is_empty() {
                result.push(std::mem::take(&mut current));
            }
        } else {
            current.push(character);
        }
    }
    if quoted {
        return Err(two_da_error(
            source,
            "TWO_DA_UNTERMINATED_QUOTE",
            format!("Unterminated quote in {line:?}"),
        ));
    }
    if !current.is_empty() {
        result.push(current);
    }
    Ok(result)
}

fn two_da_error(source: &str, code: &str, detail: String) -> Box<AppError> {
    Box::new(
        AppError::new(
            code,
            "La table 2DA est invalide.",
            detail,
            ErrorSeverity::Error,
        )
        .with_source(source)
        .with_import_stage("2da"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_defaults_missing_cells_and_quoted_values() {
        let table = parse_2da(
            b"2DA V2.0\nDEFAULT: fallback\nLabel Value\n0 hello ****\n1 \"two words\" explicit\n",
            "fixture.2da",
        )
        .expect("2DA");
        assert_eq!(table.columns, vec!["Label", "Value"]);
        assert_eq!(table.cell(0, "Label"), Some("hello"));
        assert_eq!(table.cell(0, "Value"), Some("fallback"));
        assert_eq!(table.cell(1, "Label"), Some("two words"));
    }

    #[test]
    fn compares_tables_by_row_label_and_column() {
        let left = parse_2da(b"2DA V2.0\nValue\n0 A\n", "left").expect("left");
        let right = parse_2da(b"2DA V2.0\nValue\n0 B\n", "right").expect("right");
        assert_eq!(compare_2da(&left, &right).len(), 1);
    }

    #[test]
    fn preserves_unquoted_spaces_in_the_last_column_like_nwn_tables() {
        let table = parse_2da(
            b"2DA V2.0\nName\nAction95 Stats Toggle - Cut\n",
            "keymap.2da",
        )
        .expect("2DA");
        assert_eq!(table.cell(0, "Name"), Some("Stats Toggle - Cut"));
    }

    #[test]
    fn manager_selects_highest_priority_and_compares_versions() {
        let low = parse_2da(b"2DA V2.0\nValue\n0 A\n", "base").expect("base");
        let high = parse_2da(b"2DA V2.0\nValue\n0 B\n", "override").expect("override");
        let mut manager = TwoDaManager::default();
        manager.insert(
            "classes",
            TwoDaVersion {
                source: "base".into(),
                priority: 10,
                table: low,
            },
        );
        manager.insert(
            "CLASSES",
            TwoDaVersion {
                source: "override".into(),
                priority: 20,
                table: high,
            },
        );
        assert_eq!(
            manager
                .selected("classes")
                .map(|value| value.source.as_str()),
            Some("override")
        );
        assert_eq!(
            manager
                .compare_versions("classes", 0, 1)
                .expect("versions")
                .len(),
            1
        );
    }

    #[test]
    fn edits_writes_and_reopens_a_table_deterministically() {
        let mut table =
            parse_2da(b"2DA V2.0\nName Value\n0 old ****\n", "table.2da").expect("table");
        apply_2da_edit(
            &mut table,
            &TwoDaEditAction::SetCell {
                row_index: 0,
                column_index: 0,
                value: Some("two words".into()),
            },
        )
        .expect("edit");
        apply_2da_edit(&mut table, &TwoDaEditAction::AddRow { label: "1".into() }).expect("append");
        let first = write_2da(&table).expect("write");
        let reopened = parse_2da(&first, "reopened.2da").expect("reopen");
        let second = write_2da(&reopened).expect("rewrite");
        assert_eq!(first, second);
        assert_eq!(reopened.cell(0, "Name"), Some("two words"));
        assert_eq!(reopened.rows.len(), 2);
    }
}
