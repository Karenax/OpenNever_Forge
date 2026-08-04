use aurora_core::{AppError, AppResult, ErrorSeverity, ResourceKey, decode_nwn_text};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const MAX_SCRIPT_BYTES: usize = 16 * 1024 * 1024;
const MAX_SEARCH_MATCHES: usize = 20;
const COMPILER_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompilerConfig {
    pub executable: PathBuf,
    pub game_install_path: PathBuf,
    pub include_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompileResult {
    pub success: bool,
    pub compiler: String,
    pub stdout: String,
    pub stderr: String,
    pub diagnostics: Vec<ScriptDiagnostic>,
    pub ncs: Option<NcsDocument>,
    #[serde(skip)]
    pub ncs_bytes: Vec<u8>,
}

pub fn compile_nss(
    config: &CompilerConfig,
    resref: &str,
    source: &str,
) -> AppResult<CompileResult> {
    validate_compiler_config(config, resref, source)?;
    let temporary = tempfile::tempdir().map_err(|error| {
        Box::new(AppError::io(
            "create NWScript compiler workspace",
            "system temporary directory",
            &error,
        ))
    })?;
    let input = temporary.path().join(format!("{resref}.nss"));
    fs::write(&input, source.as_bytes()).map_err(|error| {
        Box::new(AppError::io(
            "write temporary NSS source",
            input.display().to_string(),
            &error,
        ))
    })?;
    let args = compiler_arguments(config, temporary.path(), &input);
    let mut child = Command::new(&config.executable)
        .args(&args)
        .current_dir(temporary.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            Box::new(
                AppError::new(
                    "NSS_COMPILER_START_FAILED",
                    "Le compilateur NWScript n’a pas pu démarrer.",
                    format!("{}: {error}", config.executable.display()),
                    ErrorSeverity::Error,
                )
                .with_source(config.executable.display().to_string())
                .with_import_stage("nwscript_compile"),
            )
        })?;
    let started = Instant::now();
    loop {
        if child
            .try_wait()
            .map_err(|error| {
                Box::new(AppError::io(
                    "poll NWScript compiler",
                    config.executable.display().to_string(),
                    &error,
                ))
            })?
            .is_some()
        {
            break;
        }
        if started.elapsed() >= COMPILER_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Box::new(
                AppError::new(
                    "NSS_COMPILER_TIMEOUT",
                    "La compilation NWScript a dépassé 30 secondes.",
                    format!("compiler timed out for {resref}.nss"),
                    ErrorSeverity::Error,
                )
                .with_resource(format!("{resref}.nss"))
                .with_import_stage("nwscript_compile"),
            ));
        }
        thread::sleep(Duration::from_millis(20));
    }
    let output = child.wait_with_output().map_err(|error| {
        Box::new(AppError::io(
            "collect NWScript compiler output",
            config.executable.display().to_string(),
            &error,
        ))
    })?;
    let stdout = bounded_output(&output.stdout);
    let stderr = bounded_output(&output.stderr);
    let diagnostics = compiler_diagnostics(resref, &stdout, &stderr);
    let ncs_path = temporary.path().join(format!("{resref}.ncs"));
    let ncs_bytes = fs::read(&ncs_path).unwrap_or_default();
    let ncs = if ncs_bytes.is_empty() {
        None
    } else {
        Some(inspect_ncs(
            &ncs_bytes,
            &format!("workspace::{resref}.ncs"),
        )?)
    };
    let success = output.status.success() && ncs.as_ref().is_some_and(|value| value.valid_header);
    Ok(CompileResult {
        success,
        compiler: config.executable.display().to_string(),
        stdout,
        stderr,
        diagnostics,
        ncs,
        ncs_bytes,
    })
}

fn validate_compiler_config(config: &CompilerConfig, resref: &str, source: &str) -> AppResult<()> {
    if !config.executable.is_file() {
        return Err(script_error(
            &config.executable.display().to_string(),
            "NSS_COMPILER_NOT_FOUND",
            "compiler path is not a regular file".to_owned(),
        ));
    }
    if !config.game_install_path.is_dir() {
        return Err(script_error(
            &config.game_install_path.display().to_string(),
            "NSS_GAME_PATH_INVALID",
            "game installation path is not a directory".to_owned(),
        ));
    }
    if resref.is_empty()
        || resref.len() > 16
        || !resref
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(script_error(
            resref,
            "NSS_RESREF_INVALID",
            "script ResRef must contain 1 to 16 ASCII letters, digits or underscores".to_owned(),
        ));
    }
    if source.len() > MAX_SCRIPT_BYTES {
        return Err(script_error(
            resref,
            "NSS_SIZE_LIMIT_EXCEEDED",
            format!("{} bytes exceeds {MAX_SCRIPT_BYTES}", source.len()),
        ));
    }
    for include in &config.include_paths {
        if !include.is_dir() {
            return Err(script_error(
                &include.display().to_string(),
                "NSS_INCLUDE_PATH_INVALID",
                "include path is not a directory".to_owned(),
            ));
        }
    }
    Ok(())
}

fn compiler_arguments(config: &CompilerConfig, temporary: &Path, input: &Path) -> Vec<String> {
    let mut includes = vec![temporary.display().to_string()];
    includes.extend(
        config
            .include_paths
            .iter()
            .map(|path| path.display().to_string()),
    );
    vec![
        "-o".to_owned(),
        "-w".to_owned(),
        "-n".to_owned(),
        config.game_install_path.display().to_string(),
        "-i".to_owned(),
        includes.join(";"),
        input.display().to_string(),
    ]
}

fn bounded_output(bytes: &[u8]) -> String {
    decode_nwn_text(&bytes[..bytes.len().min(256 * 1024)])
}

fn compiler_diagnostics(resref: &str, stdout: &str, stderr: &str) -> Vec<ScriptDiagnostic> {
    stdout
        .lines()
        .chain(stderr.lines())
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            lower.contains("error") || lower.contains("warning")
        })
        .take(200)
        .map(|line| {
            let line_number = line
                .split(|character: char| !character.is_ascii_digit())
                .find_map(|part| part.trim().parse::<usize>().ok());
            ScriptDiagnostic {
                code: if line.to_ascii_lowercase().contains("error") {
                    "NSS_COMPILER_ERROR".to_owned()
                } else {
                    "NSS_COMPILER_WARNING".to_owned()
                },
                message: line.chars().take(500).collect(),
                line: line_number,
                resource: format!("{resref}.nss"),
            }
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScriptSymbolKind {
    Function,
    Constant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptSymbol {
    pub name: String,
    pub kind: ScriptSymbolKind,
    pub line: usize,
    pub declaration: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptInclude {
    pub resref: String,
    pub line: usize,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptCall {
    pub name: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptDiagnostic {
    pub code: String,
    pub message: String,
    pub line: Option<usize>,
    pub resource: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NssDocument {
    pub source: String,
    pub text: String,
    pub line_count: usize,
    pub includes: Vec<ScriptInclude>,
    pub symbols: Vec<ScriptSymbol>,
    pub calls: Vec<ScriptCall>,
    pub diagnostics: Vec<ScriptDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NcsDocument {
    pub source: String,
    pub size: usize,
    pub sha256: String,
    pub header: String,
    pub bytecode_size: usize,
    pub hex_preview: String,
    pub valid_header: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InboundScriptReference {
    pub script: String,
    pub resource: ResourceKey,
    pub field_path: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptDocument {
    pub resref: String,
    pub nss: Option<NssDocument>,
    pub ncs: Option<NcsDocument>,
    pub inbound_references: Vec<InboundScriptReference>,
    pub diagnostics: Vec<ScriptDiagnostic>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptIndexSummary {
    pub scripts: usize,
    pub nss: usize,
    pub ncs: usize,
    pub paired: usize,
    pub missing_source: usize,
    pub includes: usize,
    pub symbols: usize,
    pub calls: usize,
    pub inbound_references: usize,
    pub diagnostics: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptIndex {
    pub documents: Vec<ScriptDocument>,
    pub summary: ScriptIndexSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptTextMatch {
    pub line: usize,
    pub excerpt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptSearchHit {
    pub resref: String,
    pub has_nss: bool,
    pub has_ncs: bool,
    pub symbol_count: usize,
    pub inbound_reference_count: usize,
    pub diagnostic_count: usize,
    pub matches: Vec<ScriptTextMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScriptPage {
    pub items: Vec<ScriptSearchHit>,
    pub offset: usize,
    pub limit: usize,
    pub total: usize,
}

impl ScriptIndex {
    pub fn finalize(&mut self) {
        self.documents.sort_by(|a, b| a.resref.cmp(&b.resref));
        self.summary = ScriptIndexSummary {
            scripts: self.documents.len(),
            nss: self.documents.iter().filter(|v| v.nss.is_some()).count(),
            ncs: self.documents.iter().filter(|v| v.ncs.is_some()).count(),
            paired: self
                .documents
                .iter()
                .filter(|v| v.nss.is_some() && v.ncs.is_some())
                .count(),
            missing_source: self
                .documents
                .iter()
                .filter(|v| v.nss.is_none() && v.ncs.is_some())
                .count(),
            includes: self
                .documents
                .iter()
                .filter_map(|v| v.nss.as_ref())
                .map(|v| v.includes.len())
                .sum(),
            symbols: self
                .documents
                .iter()
                .filter_map(|v| v.nss.as_ref())
                .map(|v| v.symbols.len())
                .sum(),
            calls: self
                .documents
                .iter()
                .filter_map(|v| v.nss.as_ref())
                .map(|v| v.calls.len())
                .sum(),
            inbound_references: self
                .documents
                .iter()
                .map(|v| v.inbound_references.len())
                .sum(),
            diagnostics: self
                .documents
                .iter()
                .map(|v| v.diagnostics.len() + v.nss.as_ref().map_or(0, |n| n.diagnostics.len()))
                .sum(),
        };
    }

    pub fn get(&self, resref: &str) -> Option<&ScriptDocument> {
        self.documents
            .iter()
            .find(|v| v.resref.eq_ignore_ascii_case(resref))
    }

    pub fn search(&self, query: &str, offset: usize, limit: usize) -> ScriptPage {
        let needle = query.trim().to_ascii_lowercase();
        let mut hits = Vec::new();
        for document in &self.documents {
            let mut matches = Vec::new();
            if let Some(nss) = &document.nss {
                for (index, line) in nss.text.lines().enumerate() {
                    if !needle.is_empty()
                        && line.to_ascii_lowercase().contains(&needle)
                        && matches.len() < MAX_SEARCH_MATCHES
                    {
                        matches.push(ScriptTextMatch {
                            line: index + 1,
                            excerpt: line.trim().chars().take(240).collect(),
                        });
                    }
                }
            }
            let metadata_match = needle.is_empty()
                || document.resref.to_ascii_lowercase().contains(&needle)
                || document.nss.as_ref().is_some_and(|n| {
                    n.symbols
                        .iter()
                        .any(|s| s.name.to_ascii_lowercase().contains(&needle))
                })
                || document
                    .inbound_references
                    .iter()
                    .any(|r| r.resource.resref.to_ascii_lowercase().contains(&needle));
            if metadata_match || !matches.is_empty() {
                hits.push(ScriptSearchHit {
                    resref: document.resref.clone(),
                    has_nss: document.nss.is_some(),
                    has_ncs: document.ncs.is_some(),
                    symbol_count: document.nss.as_ref().map_or(0, |n| n.symbols.len()),
                    inbound_reference_count: document.inbound_references.len(),
                    diagnostic_count: document.diagnostics.len()
                        + document.nss.as_ref().map_or(0, |n| n.diagnostics.len()),
                    matches,
                });
            }
        }
        let total = hits.len();
        let limit = limit.clamp(1, 200);
        ScriptPage {
            items: hits.into_iter().skip(offset).take(limit).collect(),
            offset,
            limit,
            total,
        }
    }
}

pub fn parse_nss(bytes: &[u8], source: &str) -> AppResult<NssDocument> {
    if bytes.len() > MAX_SCRIPT_BYTES {
        return Err(script_error(
            source,
            "NSS_SIZE_LIMIT_EXCEEDED",
            format!("{} bytes exceeds {MAX_SCRIPT_BYTES}", bytes.len()),
        ));
    }
    let text = decode_nwn_text(bytes);
    let includes = parse_includes(&text);
    let tokens = tokenize(&text);
    let (symbols, declaration_tokens) = symbols(&tokens, &text);
    let calls = calls(&tokens, &declaration_tokens);
    Ok(NssDocument {
        source: source.into(),
        line_count: text.lines().count(),
        text,
        includes,
        symbols,
        calls,
        diagnostics: Vec::new(),
    })
}

pub fn inspect_ncs(bytes: &[u8], source: &str) -> AppResult<NcsDocument> {
    if bytes.len() > MAX_SCRIPT_BYTES {
        return Err(script_error(
            source,
            "NCS_SIZE_LIMIT_EXCEEDED",
            format!("{} bytes exceeds {MAX_SCRIPT_BYTES}", bytes.len()),
        ));
    }
    let valid_header = bytes.starts_with(b"NCS V1.0");
    let header = String::from_utf8_lossy(&bytes[..bytes.len().min(8)]).into_owned();
    let preview = &bytes[..bytes.len().min(512)];
    Ok(NcsDocument {
        source: source.into(),
        size: bytes.len(),
        sha256: hex::encode(Sha256::digest(bytes)),
        header,
        bytecode_size: bytes.len().saturating_sub(8),
        hex_preview: hex::encode_upper(preview),
        valid_header,
    })
}

#[derive(Clone)]
struct Token {
    text: String,
    line: usize,
}

fn parse_includes(text: &str) -> Vec<ScriptInclude> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let rest = line.trim().strip_prefix("#include")?.trim();
            let value = rest.strip_prefix('"')?.split('"').next()?.trim();
            (!value.is_empty()).then(|| ScriptInclude {
                resref: value.trim_end_matches(".nss").to_ascii_lowercase(),
                line: index + 1,
                resolved: false,
            })
        })
        .collect()
}

fn tokenize(text: &str) -> Vec<Token> {
    let chars = text.char_indices().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut index = 0;
    let mut line = 1;
    while index < chars.len() {
        let (_, ch) = chars[index];
        if ch == '\n' {
            line += 1;
            index += 1;
            continue;
        }
        if ch.is_whitespace() {
            index += 1;
            continue;
        }
        if ch == '/' && chars.get(index + 1).is_some_and(|v| v.1 == '/') {
            while index < chars.len() && chars[index].1 != '\n' {
                index += 1;
            }
            continue;
        }
        if ch == '/' && chars.get(index + 1).is_some_and(|v| v.1 == '*') {
            index += 2;
            while index + 1 < chars.len() && !(chars[index].1 == '*' && chars[index + 1].1 == '/') {
                if chars[index].1 == '\n' {
                    line += 1;
                }
                index += 1;
            }
            index = (index + 2).min(chars.len());
            continue;
        }
        if ch == '"' {
            index += 1;
            while index < chars.len() {
                if chars[index].1 == '\n' {
                    line += 1;
                }
                if chars[index].1 == '\\' {
                    index += 2;
                    continue;
                }
                if chars[index].1 == '"' {
                    index += 1;
                    break;
                }
                index += 1;
            }
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index].1.is_ascii_alphanumeric() || chars[index].1 == '_')
            {
                index += 1;
            }
            tokens.push(Token {
                text: chars[start..index].iter().map(|v| v.1).collect(),
                line,
            });
            continue;
        }
        tokens.push(Token {
            text: ch.to_string(),
            line,
        });
        index += 1;
    }
    tokens
}

fn symbols(tokens: &[Token], text: &str) -> (Vec<ScriptSymbol>, BTreeSet<usize>) {
    let types = [
        "void",
        "int",
        "float",
        "string",
        "object",
        "vector",
        "location",
        "effect",
        "event",
        "itemproperty",
        "talent",
        "sqlquery",
        "json",
    ];
    let mut result = Vec::new();
    let mut declarations = BTreeSet::new();
    for i in 0..tokens.len() {
        if tokens[i].text == "const"
            && i + 2 < tokens.len()
            && types.contains(&tokens[i + 1].text.as_str())
        {
            let name = tokens[i + 2].text.clone();
            declarations.insert(i + 2);
            result.push(ScriptSymbol {
                name,
                kind: ScriptSymbolKind::Constant,
                line: tokens[i].line,
                declaration: line_at(text, tokens[i].line),
            });
        } else if i + 2 < tokens.len()
            && types.contains(&tokens[i].text.as_str())
            && is_identifier(&tokens[i + 1].text)
            && tokens[i + 2].text == "("
        {
            let name = tokens[i + 1].text.clone();
            declarations.insert(i + 1);
            result.push(ScriptSymbol {
                name,
                kind: ScriptSymbolKind::Function,
                line: tokens[i].line,
                declaration: line_at(text, tokens[i].line),
            });
        }
    }
    result.sort_by(|a, b| a.line.cmp(&b.line).then_with(|| a.name.cmp(&b.name)));
    result.dedup_by(|a, b| a.name == b.name && a.line == b.line);
    (result, declarations)
}

fn calls(tokens: &[Token], declarations: &BTreeSet<usize>) -> Vec<ScriptCall> {
    let excluded = ["if", "for", "while", "switch", "return", "sizeof", "struct"];
    let mut result = Vec::new();
    for (index, pair) in tokens.windows(2).enumerate() {
        let name = pair[0].text.to_ascii_lowercase();
        if pair[1].text == "("
            && is_identifier(&pair[0].text)
            && !excluded.contains(&name.as_str())
            && !declarations.contains(&index)
        {
            result.push(ScriptCall {
                name: pair[0].text.clone(),
                line: pair[0].line,
            });
        }
    }
    result
}

fn line_at(text: &str, line: usize) -> String {
    text.lines()
        .nth(line.saturating_sub(1))
        .unwrap_or_default()
        .trim()
        .chars()
        .take(240)
        .collect()
}
fn is_identifier(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|v| v.is_ascii_alphabetic() || v == '_')
        && value.chars().all(|v| v.is_ascii_alphanumeric() || v == '_')
}
fn script_error(source: &str, code: &str, detail: String) -> Box<AppError> {
    Box::new(
        AppError::new(
            code,
            "Le script NWScript est invalide.",
            detail,
            ErrorSeverity::Error,
        )
        .with_source(source)
        .with_import_stage("nwscript"),
    )
}

pub fn group_documents(
    nss: Vec<(String, NssDocument)>,
    ncs: Vec<(String, NcsDocument)>,
) -> ScriptIndex {
    let mut map = BTreeMap::<String, ScriptDocument>::new();
    for (resref, value) in nss {
        map.entry(resref.clone())
            .or_insert_with(|| empty_document(resref))
            .nss = Some(value);
    }
    for (resref, value) in ncs {
        map.entry(resref.clone())
            .or_insert_with(|| empty_document(resref))
            .ncs = Some(value);
    }
    let mut index = ScriptIndex {
        documents: map.into_values().collect(),
        summary: ScriptIndexSummary::default(),
    };
    index.finalize();
    index
}
fn empty_document(resref: String) -> ScriptDocument {
    ScriptDocument {
        resref,
        nss: None,
        ncs: None,
        inbound_references: Vec::new(),
        diagnostics: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn indexes_includes_symbols_calls_and_searches_source() {
        let doc = parse_nss(
            b"#include \"shared\"\nconst int LIMIT = 4;\nvoid main() { SpeakString(\"hello\"); }\n",
            "test.nss",
        )
        .expect("nss");
        assert_eq!(doc.includes[0].resref, "shared");
        assert_eq!(doc.symbols.len(), 2);
        assert_eq!(doc.calls[0].name, "SpeakString");
        let index = group_documents(vec![("test".into(), doc)], Vec::new());
        assert_eq!(index.search("hello", 0, 10).total, 1);
    }
    #[test]
    fn keeps_ncs_as_a_separate_technical_document() {
        let doc = inspect_ncs(b"NCS V1.0\x00\x01", "test.ncs").expect("ncs");
        assert!(doc.valid_header);
        assert_eq!(doc.bytecode_size, 2);
    }

    #[test]
    fn builds_shell_free_compiler_arguments_and_parses_diagnostics() {
        let config = CompilerConfig {
            executable: PathBuf::from("C:/tools/nwnsc.exe"),
            game_install_path: PathBuf::from("C:/NWN"),
            include_paths: vec![PathBuf::from("C:/project/includes")],
        };
        let args = compiler_arguments(
            &config,
            Path::new("C:/temp/work"),
            Path::new("C:/temp/work/test.nss"),
        );
        assert_eq!(args[0..5], ["-o", "-w", "-n", "C:/NWN", "-i"]);
        assert_eq!(args[5], "C:/temp/work;C:/project/includes");
        assert_eq!(args[6], "C:/temp/work/test.nss");
        let diagnostics = compiler_diagnostics(
            "test",
            "test.nss(3): error: unknown identifier\n",
            "warning: unused value",
        );
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].line, Some(3));
        assert_eq!(diagnostics[0].code, "NSS_COMPILER_ERROR");
    }
}
