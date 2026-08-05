use aurora_agent::{
    AgentWorkspaceStore, CapabilityRegistry, PolicyDecision, SecurityLevel, built_in_policy,
    context_allows_capability, evaluate_capability, sanitize_context_value, validate_tool_scope,
};
use aurora_core::{AppError, AppResult, ErrorSeverity, ResourceKey, decode_nwn_text};
use aurora_edit::{AiChangeSet, EditCommand, EditWorkspace, ai_change_set_sha256};
use aurora_erf::{ContainerReader, ErfReader};
use aurora_gff::parse_gff;
use aurora_nwscript::parse_nss;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::{self, BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

const MAX_MCP_REQUEST_BYTES: usize = 4 * 1024 * 1024;
const MCP_PROTOCOL_CURRENT: &str = "2025-11-25";
const MCP_PROTOCOL_PREVIOUS: &str = "2025-06-18";

fn main() {
    if let Err(error) = run() {
        eprintln!("{}: {}", error.code, error.technical_message);
        std::process::exit(1);
    }
}

fn run() -> AppResult<()> {
    let workspace_root = workspace_argument()?;
    let mut server = McpServer::open(workspace_root)?;
    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    let mut stdout = io::stdout().lock();
    loop {
        let mut line = String::new();
        let read = (&mut stdin)
            .take((MAX_MCP_REQUEST_BYTES + 1) as u64)
            .read_line(&mut line)
            .map_err(|error| Box::new(AppError::io("read MCP stdin", "stdin", &error)))?;
        if read == 0 {
            break;
        }
        if read > MAX_MCP_REQUEST_BYTES {
            return Err(mcp_error(
                "MCP_REQUEST_TOO_LARGE",
                "La requête MCP dépasse la limite autorisée.",
                format!("request exceeds {MAX_MCP_REQUEST_BYTES} bytes"),
            ));
        }
        if line.trim().is_empty() {
            continue;
        }
        let request: Value = serde_json::from_str(&line).map_err(|error| {
            mcp_error(
                "MCP_REQUEST_INVALID",
                "La requête MCP n’est pas un JSON valide.",
                error.to_string(),
            )
        })?;
        if let Some(response) = server.handle(request) {
            serde_json::to_writer(&mut stdout, &response).map_err(|error| {
                mcp_error(
                    "MCP_RESPONSE_SERIALIZE_FAILED",
                    "La réponse MCP n’a pas pu être préparée.",
                    error.to_string(),
                )
            })?;
            stdout
                .write_all(b"\n")
                .map_err(|error| Box::new(AppError::io("write MCP stdout", "stdout", &error)))?;
            stdout
                .flush()
                .map_err(|error| Box::new(AppError::io("flush MCP stdout", "stdout", &error)))?;
        }
    }
    Ok(())
}

fn workspace_argument() -> AppResult<PathBuf> {
    let mut arguments = std::env::args_os().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--workspace" {
            return arguments.next().map(PathBuf::from).ok_or_else(|| {
                mcp_error(
                    "MCP_WORKSPACE_REQUIRED",
                    "Indiquez le workspace OpenNever avec --workspace.",
                    "--workspace has no value",
                )
            });
        }
    }
    Err(mcp_error(
        "MCP_WORKSPACE_REQUIRED",
        "Indiquez le workspace OpenNever avec --workspace.",
        "usage: opennever-mcp --workspace <path>",
    ))
}

struct McpServer {
    workspace: EditWorkspace,
    store: AgentWorkspaceStore,
    registry: CapabilityRegistry,
    call_counts: BTreeMap<String, u32>,
    total_calls: u32,
    started: Instant,
    initialize_seen: bool,
    ready: bool,
}

impl McpServer {
    fn open(root: PathBuf) -> AppResult<Self> {
        let workspace = EditWorkspace::open(&root)?;
        Ok(Self {
            workspace,
            store: AgentWorkspaceStore::new(&root),
            registry: CapabilityRegistry::standard(),
            call_counts: BTreeMap::new(),
            total_calls: 0,
            started: Instant::now(),
            initialize_seen: false,
            ready: false,
        })
    }

    fn handle(&mut self, request: Value) -> Option<Value> {
        let id = request.get("id").cloned();
        if request.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return id.map(|id| {
                json!({
                    "jsonrpc":"2.0",
                    "id":id,
                    "error":{"code":-32600,"message":"Invalid Request"}
                })
            });
        }
        let method = request
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if id.is_none() {
            if method == "notifications/initialized" && self.initialize_seen {
                self.ready = true;
            }
            return None;
        }
        id.as_ref()?;
        let result = match method {
            "initialize" => self.initialize(request.get("params")),
            "ping" => Ok(json!({})),
            _ if !self.ready => Err(mcp_error(
                "MCP_NOT_INITIALIZED",
                "La session MCP nâ€™est pas encore initialisÃ©e.",
                "initialize and notifications/initialized are required before operation",
            )),
            "tools/list" => self.list_tools(),
            "tools/call" => self.call_tool(request.get("params").cloned().unwrap_or_default()),
            "resources/list" => Ok(self.list_resources()),
            "resources/read" => {
                self.read_resource(request.get("params").cloned().unwrap_or_default())
            }
            _ => Err(mcp_error(
                "MCP_METHOD_NOT_FOUND",
                "La méthode MCP demandée n’est pas prise en charge.",
                method.to_owned(),
            )),
        };
        Some(match result {
            Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
            Err(error) => json!({
                "jsonrpc":"2.0",
                "id":id,
                "error": {"code":-32000,"message":error.user_message,"data":{"code":error.code,"technicalMessage":error.technical_message}}
            }),
        })
    }

    fn initialize(&mut self, params: Option<&Value>) -> AppResult<Value> {
        let params = params.and_then(Value::as_object).ok_or_else(|| {
            mcp_error(
                "MCP_INITIALIZE_PARAMS_REQUIRED",
                "Les paramÃ¨tres dâ€™initialisation MCP sont manquants.",
                "initialize requires an object params value",
            )
        })?;
        let requested = params
            .get("protocolVersion")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                mcp_error(
                    "MCP_PROTOCOL_VERSION_REQUIRED",
                    "La version de protocole MCP est manquante.",
                    "initialize.params.protocolVersion is required",
                )
            })?;
        if !params.get("capabilities").is_some_and(Value::is_object)
            || !params.get("clientInfo").is_some_and(Value::is_object)
        {
            return Err(mcp_error(
                "MCP_CLIENT_INFO_REQUIRED",
                "Les capacitÃ©s et lâ€™identitÃ© du client MCP sont requises.",
                "initialize requires object capabilities and clientInfo values",
            ));
        }
        let negotiated = if matches!(requested, MCP_PROTOCOL_CURRENT | MCP_PROTOCOL_PREVIOUS) {
            requested
        } else {
            MCP_PROTOCOL_CURRENT
        };
        self.initialize_seen = true;
        self.ready = false;
        Ok(json!({
            "protocolVersion": negotiated,
            "capabilities": {
                "tools": { "listChanged": false },
                "resources": { "subscribe": false, "listChanged": false }
            },
            "serverInfo": {
                "name": "opennever-forge",
                "title": "OpenNever Forge",
                "version": env!("CARGO_PKG_VERSION"),
                "description": "Controlled local NWN module authoring tools"
            },
            "instructions": "Local OpenNever Forge tools. Source NWN modules remain immutable; all writes target the transactional workspace."
        }))
    }

    fn policy(&self) -> AppResult<aurora_agent::AgentPolicy> {
        Ok(self
            .store
            .load_policy()?
            .unwrap_or_else(|| built_in_policy(SecurityLevel::Observer)))
    }

    fn list_tools(&self) -> AppResult<Value> {
        let policy = self.policy()?;
        if self.total_calls >= policy.limits.max_tool_calls
            || self.started.elapsed().as_secs() >= policy.limits.max_duration_seconds
        {
            return Ok(json!({"tools":[]}));
        }
        let tools = self
            .registry
            .capabilities
            .iter()
            .filter(|tool| mcp_tool_is_implemented(&tool.id))
            .filter(|tool| {
                context_allows_capability(&policy, &tool.id)
                    && !matches!(
                        evaluate_capability(
                            &policy,
                            tool,
                            self.call_counts.get(&tool.id).copied().unwrap_or(0),
                        )
                        .1,
                        PolicyDecision::Denied { .. }
                    )
            })
            .map(|tool| {
                json!({
                    "name": tool.id,
                    "title": tool.title,
                    "description": tool.description,
                    "inputSchema": tool.input_schema,
                    "outputSchema": tool.output_schema,
                    "annotations": {
                        "readOnlyHint": tool.side_effect == aurora_agent::CapabilitySideEffect::None,
                        "destructiveHint": false,
                        "idempotentHint": tool.side_effect == aurora_agent::CapabilitySideEffect::None,
                    }
                })
            })
            .collect::<Vec<_>>();
        Ok(json!({"tools":tools}))
    }

    fn call_tool(&mut self, params: Value) -> AppResult<Value> {
        let name = params.get("name").and_then(Value::as_str).ok_or_else(|| {
            mcp_error(
                "MCP_TOOL_NAME_REQUIRED",
                "Le nom de l’outil MCP est manquant.",
                "tools/call requires params.name",
            )
        })?;
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let policy = self.policy()?;
        if self.total_calls >= policy.limits.max_tool_calls {
            return Ok(tool_error(
                "AGENT_TOOL_LIMIT",
                "Le nombre maximal d’appels d’outils de la session est atteint.",
            ));
        }
        if self.started.elapsed().as_secs() >= policy.limits.max_duration_seconds {
            return Ok(tool_error(
                "AGENT_DURATION_LIMIT",
                "La durée maximale de la session MCP est atteinte.",
            ));
        }
        let descriptor = self.registry.get(name).ok_or_else(|| {
            mcp_error(
                "MCP_TOOL_UNKNOWN",
                "L’outil MCP demandé est inconnu.",
                name.to_owned(),
            )
        })?;
        if !context_allows_capability(&policy, &descriptor.id) {
            return Ok(tool_error(
                "AGENT_CONTEXT_DENIED",
                "Capacité interdite par la politique de contexte.",
            ));
        }
        if !mcp_tool_is_implemented(name) {
            return Err(mcp_error(
                "MCP_TOOL_NOT_IMPLEMENTED",
                "L’outil MCP demandé n’est pas encore exécutable.",
                name.to_owned(),
            ));
        }
        let calls_so_far = self.call_counts.get(name).copied().unwrap_or(0);
        let (effective, decision) = evaluate_capability(&policy, descriptor, calls_so_far);
        match &decision {
            PolicyDecision::Denied { reason } => {
                return Ok(tool_error("AGENT_POLICY_DENIED", reason));
            }
            PolicyDecision::ApprovalRequired { .. } | PolicyDecision::Allowed => {}
        }
        if let Err(reason) = validate_tool_scope(&policy, &effective, descriptor, &arguments) {
            return Ok(tool_error("AGENT_SCOPE_DENIED", &reason));
        }
        if let PolicyDecision::ApprovalRequired { reason } = decision {
            return Ok(tool_error("AGENT_APPROVAL_REQUIRED", &reason));
        }
        if name == "resource.read" && calls_so_far >= policy.limits.max_context_resources as u32 {
            return Ok(tool_error(
                "AGENT_CONTEXT_RESOURCE_LIMIT",
                "Le nombre maximal de ressources de contexte est atteint.",
            ));
        }
        if name == "resource.read" {
            let resource: ResourceKey = decode_arguments(arguments.clone(), name)?;
            let bytes = self.resource_bytes(&resource)?;
            if bytes.len() > policy.limits.max_context_resource_bytes {
                return Ok(tool_error(
                    "AGENT_CONTEXT_RESOURCE_TOO_LARGE",
                    "La ressource dépasse la taille maximale de contexte.",
                ));
            }
        }
        let result = sanitize_context_value(
            &self.execute(name, arguments)?,
            policy.context.include_local_paths,
        );
        self.call_counts
            .entry(name.to_owned())
            .and_modify(|count| *count = count.saturating_add(1))
            .or_insert(1);
        self.total_calls = self.total_calls.saturating_add(1);
        Ok(json!({
            "content":[{"type":"text","text":result.to_string()}],
            "structuredContent":result,
            "isError":false
        }))
    }

    fn execute(&mut self, name: &str, arguments: Value) -> AppResult<Value> {
        match name {
            "module.inspect" | "module.validate" | "diagnostics.run" => {
                let snapshot = self.workspace.snapshot()?;
                Ok(json!({
                    "workspace": snapshot,
                    "valid": snapshot.source_intact,
                    "invariant": "source_immutable"
                }))
            }
            "workspace.checkpoint" => {
                let snapshot = self.workspace.snapshot()?;
                Ok(json!({
                    "checkpointId": format!("mcp:{}", snapshot.cursor),
                    "cursor": snapshot.cursor,
                    "sourceSha256": snapshot.source.sha256
                }))
            }
            "resource.read" => {
                let resource: ResourceKey = decode_arguments(arguments, name)?;
                let bytes = self.resource_bytes(&resource)?;
                Ok(json!({
                    "resource":resource,
                    "sha256":hex::encode(Sha256::digest(&bytes)),
                    "content":resource_context(&resource, &bytes)?
                }))
            }
            "resource.set_field" => {
                let arguments: SetFieldArguments = decode_arguments(arguments, name)?;
                let change_set = AiChangeSet {
                    summary: format!("MCP : modifier {}", arguments.resource),
                    commands: vec![EditCommand::SetField {
                        resource: arguments.resource,
                        path: arguments.path,
                        before: arguments.before,
                        after: arguments.after,
                    }],
                };
                self.apply_change_set(change_set)
            }
            "script.replace" => {
                let arguments: ReplaceScriptArguments = decode_arguments(arguments, name)?;
                let change_set = AiChangeSet {
                    summary: format!("MCP : remplacer {}", arguments.resource),
                    commands: vec![EditCommand::ReplaceText {
                        resource: arguments.resource,
                        before: arguments.before,
                        after: arguments.after,
                    }],
                };
                self.apply_change_set(change_set)
            }
            _ => Err(mcp_error(
                "MCP_TOOL_NOT_IMPLEMENTED",
                "L’outil MCP demandé n’est pas encore exécutable.",
                name.to_owned(),
            )),
        }
    }

    fn resource_bytes(&self, resource: &ResourceKey) -> AppResult<Vec<u8>> {
        if let Some(bytes) = self.workspace.staged_resource_bytes(resource)? {
            return Ok(bytes);
        }
        let snapshot = self.workspace.snapshot()?;
        let source = Path::new(&snapshot.source.path);
        let reader = ErfReader::default();
        let cancelled = AtomicBool::new(false);
        let inventory = reader.read_inventory(source, &cancelled)?;
        let entry = inventory
            .resources
            .iter()
            .find(|entry| entry.key == *resource)
            .ok_or_else(|| {
                mcp_error(
                    "MCP_RESOURCE_NOT_FOUND",
                    "La ressource demandée est introuvable.",
                    resource.to_string(),
                )
            })?;
        reader.read_resource(source, entry, &cancelled)
    }

    fn apply_change_set(&mut self, change_set: AiChangeSet) -> AppResult<Value> {
        let digest = ai_change_set_sha256(&change_set)?;
        let mut sources = BTreeMap::new();
        for command in &change_set.commands {
            let resource = match command {
                EditCommand::SetField { resource, .. }
                | EditCommand::ReplaceText { resource, .. } => resource,
                _ => unreachable!("MCP only constructs controlled AI commands"),
            };
            sources.insert(resource.to_string(), self.resource_bytes(resource)?);
        }
        let report =
            self.workspace
                .apply_controlled_ai_change_set(&change_set, &digest, &sources)?;
        serde_json::to_value(report).map_err(|error| {
            mcp_error(
                "MCP_TOOL_RESULT_INVALID",
                "Le résultat MCP n’a pas pu être préparé.",
                error.to_string(),
            )
        })
    }

    fn list_resources(&self) -> Value {
        json!({"resources":[
            {"uri":"opennever://workspace/snapshot","name":"Workspace snapshot","mimeType":"application/json"},
            {"uri":"opennever://agent/policy","name":"Agent policy","mimeType":"application/json"},
            {"uri":"opennever://agent/capabilities","name":"Capability registry","mimeType":"application/json"}
        ]})
    }

    fn read_resource(&self, params: Value) -> AppResult<Value> {
        let uri = params.get("uri").and_then(Value::as_str).ok_or_else(|| {
            mcp_error(
                "MCP_RESOURCE_URI_REQUIRED",
                "L’URI de ressource MCP est manquante.",
                "resources/read requires params.uri",
            )
        })?;
        let policy = self.policy()?;
        let value = match uri {
            "opennever://workspace/snapshot" => serde_json::to_value(self.workspace.snapshot()?),
            "opennever://agent/policy" => serde_json::to_value(&policy),
            "opennever://agent/capabilities" => serde_json::to_value(&self.registry),
            _ => {
                return Err(mcp_error(
                    "MCP_RESOURCE_NOT_FOUND",
                    "La ressource MCP demandée est introuvable.",
                    uri.to_owned(),
                ));
            }
        }
        .map_err(|error| {
            mcp_error(
                "MCP_RESOURCE_SERIALIZE_FAILED",
                "La ressource MCP n’a pas pu être préparée.",
                error.to_string(),
            )
        })?;
        let value = sanitize_context_value(&value, policy.context.include_local_paths);
        Ok(json!({"contents":[{"uri":uri,"mimeType":"application/json","text":value.to_string()}]}))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetFieldArguments {
    resource: ResourceKey,
    path: String,
    before: Value,
    after: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplaceScriptArguments {
    resource: ResourceKey,
    before: String,
    after: String,
}

fn mcp_tool_is_implemented(id: &str) -> bool {
    matches!(
        id,
        "module.inspect"
            | "module.validate"
            | "diagnostics.run"
            | "resource.read"
            | "resource.set_field"
            | "script.replace"
            | "workspace.checkpoint"
    )
}

fn resource_context(resource: &ResourceKey, bytes: &[u8]) -> AppResult<Value> {
    if resource.resource_type == 2009 {
        parse_nss(bytes, &format!("mcp::{resource}"))?;
        return Ok(json!({"kind":"nss","text":decode_nwn_text(bytes)}));
    }
    if is_gff(resource.resource_type) {
        let document = parse_gff(bytes, &format!("mcp::{resource}"))?;
        return serde_json::to_value(document).map_err(|error| {
            mcp_error(
                "MCP_RESOURCE_SERIALIZE_FAILED",
                "La ressource GFF n’a pas pu être préparée.",
                error.to_string(),
            )
        });
    }
    Err(mcp_error(
        "MCP_RESOURCE_TYPE_UNSUPPORTED",
        "Seuls les GFF et NSS sont lisibles par cet outil.",
        resource.to_string(),
    ))
}

fn is_gff(resource_type: u16) -> bool {
    matches!(
        aurora_core::resource_extension(resource_type),
        Some(
            "are"
                | "ifo"
                | "bic"
                | "git"
                | "uti"
                | "utc"
                | "dlg"
                | "itp"
                | "utt"
                | "uts"
                | "gff"
                | "fac"
                | "ute"
                | "utd"
                | "utp"
                | "gic"
                | "gui"
                | "utm"
                | "jrl"
                | "utw"
        )
    )
}

fn decode_arguments<T: serde::de::DeserializeOwned>(value: Value, name: &str) -> AppResult<T> {
    serde_json::from_value(value).map_err(|error| {
        mcp_error(
            "MCP_TOOL_ARGUMENTS_INVALID",
            "Les paramètres de l’outil MCP sont invalides.",
            format!("{name}: {error}"),
        )
    })
}

fn tool_error(code: &str, message: &str) -> Value {
    json!({
        "content":[{"type":"text","text":message}],
        "structuredContent":{"code":code,"message":message},
        "isError":true
    })
}

fn mcp_error(
    code: impl Into<String>,
    user_message: impl Into<String>,
    technical_message: impl Into<String>,
) -> Box<AppError> {
    Box::new(
        AppError::new(code, user_message, technical_message, ErrorSeverity::Error)
            .with_import_stage("mcp_server"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use aurora_agent::{ApprovalMode, CapabilityAccess, CapabilityOverride, ToolScope};
    use aurora_edit::{NewModuleDefinition, create_empty_module};
    use std::fs;

    #[test]
    fn initializes_and_exposes_policy_filtered_tools() {
        let temporary = tempfile::tempdir().expect("temporary");
        let module = temporary.path().join("source.mod");
        create_empty_module(
            &module,
            &NewModuleDefinition {
                name: "MCP Test".to_owned(),
                tag: "MCP_TEST".to_owned(),
                entry_area: "entry".to_owned(),
                tileset: "tno01".to_owned(),
            },
        )
        .expect("module");
        let module_bytes = fs::read(&module).expect("module bytes");
        let digest = hex::encode(Sha256::digest(&module_bytes));
        let workspace_root = temporary.path().join("workspace");
        EditWorkspace::create(&workspace_root, &module, &digest, module_bytes.len() as u64)
            .expect("workspace");
        let store = AgentWorkspaceStore::new(&workspace_root);
        let mut policy = built_in_policy(SecurityLevel::Observer);
        policy.context.include_module_metadata = true;
        policy.capability_overrides.insert(
            "module.inspect".to_owned(),
            CapabilityOverride {
                access: CapabilityAccess::Read,
                approval: ApprovalMode::Never,
                scope: ToolScope::Module,
                max_calls: 1,
            },
        );
        store.save_policy(&policy).expect("policy");
        let mut server = McpServer::open(workspace_root).expect("server");
        let initialized = server
            .handle(json!({
                "jsonrpc":"2.0",
                "id":1,
                "method":"initialize",
                "params":{
                    "protocolVersion":"2025-11-25",
                    "capabilities":{},
                    "clientInfo":{"name":"test","version":"1"}
                }
            }))
            .expect("response");
        assert_eq!(
            initialized["result"]["serverInfo"]["name"],
            "opennever-forge"
        );
        assert_eq!(initialized["result"]["protocolVersion"], "2025-11-25");
        assert!(
            server
                .handle(json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
                .is_none()
        );
        let called = server
            .handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"module.inspect","arguments":{}}}))
            .expect("response");
        assert_eq!(called["result"]["isError"], false);
        assert_eq!(called["result"]["structuredContent"]["valid"], true);
        assert_eq!(
            called["result"]["structuredContent"]["workspace"]["root"],
            "[local path redacted]"
        );
        let listed = server
            .handle(json!({"jsonrpc":"2.0","id":3,"method":"tools/list","params":{}}))
            .expect("tools list");
        let names = listed["result"]["tools"]
            .as_array()
            .expect("tools")
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect::<Vec<_>>();
        assert!(!names.contains(&"module.inspect"));
        assert!(!names.contains(&"resource.set_field"));
        let exhausted = server
            .handle(json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"module.inspect","arguments":{}}}))
            .expect("exhausted call");
        assert_eq!(exhausted["result"]["isError"], true);
        assert_eq!(
            exhausted["result"]["structuredContent"]["code"],
            "AGENT_POLICY_DENIED"
        );
    }

    #[test]
    fn negotiates_the_previous_protocol_and_rejects_operation_before_ready() {
        let temporary = tempfile::tempdir().expect("temporary");
        let module = temporary.path().join("source.mod");
        create_empty_module(
            &module,
            &NewModuleDefinition {
                name: "MCP Test".to_owned(),
                tag: "MCP_TEST".to_owned(),
                entry_area: "entry".to_owned(),
                tileset: "tno01".to_owned(),
            },
        )
        .expect("module");
        let module_bytes = fs::read(&module).expect("module bytes");
        let digest = hex::encode(Sha256::digest(&module_bytes));
        let workspace_root = temporary.path().join("workspace");
        EditWorkspace::create(&workspace_root, &module, &digest, module_bytes.len() as u64)
            .expect("workspace");
        let mut server = McpServer::open(workspace_root).expect("server");
        let initialized = server
            .handle(json!({
                "jsonrpc":"2.0",
                "id":1,
                "method":"initialize",
                "params":{
                    "protocolVersion":"2025-06-18",
                    "capabilities":{},
                    "clientInfo":{"name":"test","version":"1"}
                }
            }))
            .expect("response");
        assert_eq!(initialized["result"]["protocolVersion"], "2025-06-18");
        let early = server
            .handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}))
            .expect("response");
        assert_eq!(early["error"]["data"]["code"], "MCP_NOT_INITIALIZED");
    }
}
