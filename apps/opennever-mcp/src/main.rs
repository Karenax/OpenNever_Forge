use aurora_agent::{
    AgentWorkspaceStore, CapabilityRegistry, PolicyDecision, SecurityLevel, built_in_policy,
    context_allows_capability, evaluate_capability, sanitize_context_value, validate_tool_scope,
};
use aurora_core::{AppError, AppResult, ErrorSeverity, ResourceKey, decode_nwn_text};
use aurora_edit::{
    AiChangeSet, AreaAudioPatch, AreaEnvironmentPatch, AreaStructureAction, EditCommand,
    EditWorkspace, InstancePlacement, MAP_MAX_BLUEPRINTS_PER_RULE, MAP_MAX_DENSITY_RULES,
    MAP_MAX_HEIGHT, MAP_MAX_PLACEMENTS, MAP_MAX_TILES, MAP_MAX_WIDTH, MapCompatibilityReport,
    MapGenerationPlan, MapGenerationSpec, TileState, Transform, add_area_instance,
    ai_change_set_sha256, create_generated_map_resources, edit_area_audio, edit_area_environment,
    edit_area_instance_by_id, edit_area_structure, edit_area_tile_at,
    generate_map_plan_with_compatibility, inspect_area_audio, inspect_area_environment,
    remove_area_instance,
};
use aurora_erf::{ContainerReader, ErfReader};
use aurora_gff::{parse_gff, read_module_info};
use aurora_nwscript::parse_nss;
use aurora_project::{DependencyRoots, ModuleDependencyKind, inspect_module_dependencies};
use aurora_resource::{ResourceCatalog, ResourceManager, ResourceManagerConfig};
use aurora_world::{adapt_area, parse_set_tile_models, render_area_atlas_svg};
use serde::{Deserialize, Serialize};
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
    root: PathBuf,
    workspace: EditWorkspace,
    store: AgentWorkspaceStore,
    registry: CapabilityRegistry,
    call_counts: BTreeMap<String, u32>,
    total_calls: u32,
    started: Instant,
    initialize_seen: bool,
    ready: bool,
    resource_catalog: Option<ResourceCatalog>,
}

impl McpServer {
    fn open(root: PathBuf) -> AppResult<Self> {
        let workspace = EditWorkspace::open(&root)?;
        Ok(Self {
            root: root.clone(),
            workspace,
            store: AgentWorkspaceStore::new(&root),
            registry: CapabilityRegistry::standard(),
            call_counts: BTreeMap::new(),
            total_calls: 0,
            started: Instant::now(),
            initialize_seen: false,
            ready: false,
            resource_catalog: None,
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
            "map.context" => {
                let arguments: MapContextArguments = decode_arguments(arguments, name)?;
                self.map_context(arguments)
            }
            "map.inspect" => {
                let arguments: MapAreaArguments = decode_arguments(arguments, name)?;
                self.inspect_map(&arguments.area)
            }
            "map.atlas" => {
                let arguments: MapAreaArguments = decode_arguments(arguments, name)?;
                let inspection = self.inspect_map(&arguments.area)?;
                let area: aurora_world::AreaMap =
                    serde_json::from_value(inspection["area"].clone()).map_err(|error| {
                        mcp_error(
                            "MCP_MAP_INSPECTION_INVALID",
                            "La carte inspectÃ©e ne peut pas Ãªtre convertie en atlas.",
                            error.to_string(),
                        )
                    })?;
                let svg = render_area_atlas_svg(&area);
                Ok(json!({
                    "area":arguments.area,
                    "mimeType":"image/svg+xml",
                    "sha256":hex::encode(Sha256::digest(svg.as_bytes())),
                    "svg":svg,
                }))
            }
            "map.preview" => {
                let arguments: MapPreviewArguments = decode_arguments(arguments, name)?;
                let plan = self.verified_map_plan(&arguments.spec)?;
                serialize_tool_value(plan)
            }
            "map.apply" => {
                let arguments: MapApplyArguments = decode_arguments(arguments, name)?;
                self.apply_map(arguments.spec, Some(&arguments.expected_plan_sha256))
            }
            "map.generate" => {
                let spec: MapGenerationSpec = decode_arguments(arguments, name)?;
                self.apply_map(spec, None)
            }
            "map.environment.edit" => {
                let arguments: MapEnvironmentArguments = decode_arguments(arguments, name)?;
                let patch = arguments.patch;
                self.apply_map_resource_transform(
                    ResourceKey::new(&arguments.area, 2012),
                    &arguments.expected_sha256,
                    serde_json::to_string(&patch).unwrap_or_else(|_| "map_environment".to_owned()),
                    move |bytes, source| edit_area_environment(bytes, source, &patch),
                )
            }
            "map.audio.edit" => {
                let arguments: MapAudioArguments = decode_arguments(arguments, name)?;
                let patch = arguments.patch;
                self.apply_map_resource_transform(
                    ResourceKey::new(&arguments.area, 2023),
                    &arguments.expected_sha256,
                    serde_json::to_string(&patch).unwrap_or_else(|_| "map_audio".to_owned()),
                    move |bytes, source| edit_area_audio(bytes, source, &patch),
                )
            }
            "map.tile.edit" => {
                let arguments: MapTileArguments = decode_arguments(arguments, name)?;
                self.verify_map_tile(&arguments.area, arguments.after.tile_id)?;
                self.apply_map_resource_transform(
                    ResourceKey::new(&arguments.area, 2012),
                    &arguments.expected_sha256,
                    format!("map_tile:{},{}", arguments.x, arguments.y),
                    move |bytes, source| {
                        edit_area_tile_at(
                            bytes,
                            source,
                            arguments.x,
                            arguments.y,
                            arguments.before,
                            arguments.after,
                        )
                    },
                )
            }
            "map.instance.add" => {
                let arguments: MapInstanceAddArguments = decode_arguments(arguments, name)?;
                self.validate_map_placement(&arguments.placement)?;
                let resource = ResourceKey::new(&arguments.area, 2023);
                let source_bytes = self.resolved_catalog_resource_bytes(&resource)?;
                let current = self
                    .workspace
                    .staged_resource_bytes(&resource)?
                    .or_else(|| source_bytes.clone())
                    .ok_or_else(|| {
                        mcp_error(
                            "MCP_MAP_AREA_MISSING",
                            "La ressource GIT de la zone est introuvable.",
                            resource.to_string(),
                        )
                    })?;
                verify_expected_sha256(&current, &arguments.expected_sha256)?;
                let (output, instance_id) = add_area_instance(
                    &current,
                    &format!("mcp::{}", resource.file_name()),
                    &arguments.area,
                    &arguments.placement,
                )?;
                let workspace = self.commit_map_transform(
                    resource,
                    source_bytes.as_deref(),
                    &current,
                    &output,
                    format!("map_instance_add:{instance_id}"),
                )?;
                Ok(json!({"instanceId":instance_id,"workspace":workspace}))
            }
            "map.instance.move" => {
                let arguments: MapInstanceMoveArguments = decode_arguments(arguments, name)?;
                let area = arguments.area;
                let instance_id = arguments.instance_id;
                self.apply_map_resource_transform(
                    ResourceKey::new(&area, 2023),
                    &arguments.expected_sha256,
                    format!("map_instance_move:{instance_id}"),
                    move |bytes, source| {
                        edit_area_instance_by_id(
                            bytes,
                            source,
                            &area,
                            &instance_id,
                            arguments.before,
                            arguments.after,
                        )
                    },
                )
            }
            "map.instance.remove" => {
                let arguments: MapInstanceRemoveArguments = decode_arguments(arguments, name)?;
                let area = arguments.area;
                let instance_id = arguments.instance_id;
                self.apply_map_resource_transform(
                    ResourceKey::new(&area, 2023),
                    &arguments.expected_sha256,
                    format!("map_instance_remove:{instance_id}"),
                    move |bytes, source| remove_area_instance(bytes, source, &area, &instance_id),
                )
            }
            "map.structure.edit" => {
                let arguments: MapStructureArguments = decode_arguments(arguments, name)?;
                let item_template =
                    if let AreaStructureAction::AddInventoryItem { resref, .. } = &arguments.action
                    {
                        let key = ResourceKey::new(resref, 2025);
                        let bytes = self.map_resource_bytes(&key)?.ok_or_else(|| {
                            mcp_error(
                                "MCP_MAP_BLUEPRINT_MISSING",
                                "Le blueprint UTI de l'objet est introuvable.",
                                key.to_string(),
                            )
                        })?;
                        Some(parse_gff(&bytes, &format!("mcp::{}", key.file_name()))?)
                    } else {
                        None
                    };
                let area = arguments.area;
                let action = arguments.action;
                self.apply_map_resource_transform(
                    ResourceKey::new(&area, 2023),
                    &arguments.expected_sha256,
                    serde_json::to_string(&action).unwrap_or_else(|_| "map_structure".to_owned()),
                    move |bytes, source| {
                        edit_area_structure(bytes, source, &area, &action, item_template.as_ref())
                            .map(|(output, _)| output)
                    },
                )
            }
            _ => Err(mcp_error(
                "MCP_TOOL_NOT_IMPLEMENTED",
                "L’outil MCP demandé n’est pas encore exécutable.",
                name.to_owned(),
            )),
        }
    }

    fn load_resource_catalog(&mut self) -> AppResult<ResourceCatalog> {
        if let Some(catalog) = &self.resource_catalog {
            return Ok(catalog.clone());
        }
        let snapshot = self.workspace.snapshot()?;
        let module_key = ResourceKey::new("module", 2014);
        let module_bytes = self.resource_bytes(&module_key)?;
        let module_info = read_module_info(&module_bytes, "mcp::module.ifo")?;
        let policy = self.policy()?;
        let roots = DependencyRoots {
            game_install_path: non_empty_path(&policy.tool_runtime.game_install_path),
            user_data_path: non_empty_path(&policy.tool_runtime.user_data_path),
        };
        let dependencies = inspect_module_dependencies(&module_info, &roots);
        let hak_paths = dependencies
            .dependencies
            .iter()
            .filter(|dependency| dependency.kind == ModuleDependencyKind::Hak)
            .filter_map(|dependency| dependency.selected_path.as_deref().map(PathBuf::from))
            .collect::<Vec<_>>();
        let cache_root = self.root.join("agent");
        std::fs::create_dir_all(&cache_root).map_err(|error| {
            Box::new(AppError::io(
                "create MCP catalog cache",
                cache_root.display().to_string(),
                &error,
            ))
        })?;
        let cancelled = AtomicBool::new(false);
        let build = ResourceManager::build_with_cache(
            &ResourceManagerConfig {
                module_path: PathBuf::from(&snapshot.source.path),
                hak_paths,
                game_install_path: roots.game_install_path,
                user_data_path: roots.user_data_path,
            },
            Some(&cache_root.join("resource-catalog.json")),
            &cancelled,
        )?;
        self.resource_catalog = Some(build.catalog.clone());
        Ok(build.catalog)
    }

    fn map_resource_bytes(&mut self, resource: &ResourceKey) -> AppResult<Option<Vec<u8>>> {
        if let Some(bytes) = self.workspace.staged_resource_bytes(resource)? {
            return Ok(Some(bytes));
        }
        self.resolved_catalog_resource_bytes(resource)
    }

    fn resolved_catalog_resource_bytes(
        &mut self,
        resource: &ResourceKey,
    ) -> AppResult<Option<Vec<u8>>> {
        let catalog = self.load_resource_catalog()?;
        let Some(resolved) = catalog.get(resource) else {
            return Ok(None);
        };
        ResourceManager::read(&resolved.selected, &AtomicBool::new(false)).map(Some)
    }

    fn map_context(&mut self, arguments: MapContextArguments) -> AppResult<Value> {
        let catalog = self.load_resource_catalog()?;
        let snapshot = self.workspace.snapshot()?;
        let staged = snapshot
            .modified_resources
            .iter()
            .map(|resource| resource.resource.clone())
            .collect::<Vec<_>>();
        let mut tilesets = catalog
            .entries
            .iter()
            .map(|entry| &entry.key)
            .chain(staged.iter())
            .filter(|key| key.resource_type == 2013)
            .map(|key| key.resref.clone())
            .collect::<Vec<_>>();
        tilesets.sort();
        tilesets.dedup();
        let selected_resref = arguments.tileset.or_else(|| tilesets.first().cloned());
        let selected_tileset = if let Some(resref) = selected_resref.as_deref() {
            let key = ResourceKey::new(resref, 2013);
            let bytes = self.map_resource_bytes(&key)?.ok_or_else(|| {
                mcp_error(
                    "MCP_MAP_TILESET_MISSING",
                    "Le SET demandÃ© est introuvable.",
                    key.to_string(),
                )
            })?;
            let models = parse_set_tile_models(&bytes);
            if models.is_empty() {
                return Err(mcp_error(
                    "MCP_MAP_TILESET_INVALID",
                    "Le SET demandÃ© ne contient aucune tuile lisible.",
                    key.to_string(),
                ));
            }
            Some(json!({
                "resref":resref,
                "sha256":hex::encode(Sha256::digest(&bytes)),
                "tileCount":models.len(),
                "tileIds":models.keys().copied().collect::<Vec<_>>(),
                "edgeCompatibilityVerified":false,
            }))
        } else {
            None
        };
        let query = arguments.query.trim().to_ascii_lowercase();
        let limit = arguments.limit.clamp(1, 500);
        let mut blueprints = BTreeMap::<String, Vec<String>>::new();
        for category in map_blueprint_categories() {
            let resource_type = map_template_resource_type(category).expect("known category");
            let mut values = catalog
                .entries
                .iter()
                .map(|entry| &entry.key)
                .chain(staged.iter())
                .filter(|key| key.resource_type == resource_type)
                .map(|key| key.resref.clone())
                .filter(|resref| query.is_empty() || resref.contains(&query))
                .collect::<Vec<_>>();
            values.sort();
            values.dedup();
            values.truncate(limit);
            blueprints.insert(category.to_owned(), values);
        }
        let mut areas = catalog
            .entries
            .iter()
            .map(|entry| &entry.key)
            .chain(staged.iter())
            .filter(|key| key.resource_type == 2012)
            .map(|key| key.resref.clone())
            .collect::<Vec<_>>();
        areas.sort();
        areas.dedup();
        Ok(json!({
            "limits":{
                "maxWidth":MAP_MAX_WIDTH,"maxHeight":MAP_MAX_HEIGHT,"maxTiles":MAP_MAX_TILES,
                "maxResrefLength":16,"maxDensityRules":MAP_MAX_DENSITY_RULES,
                "maxBlueprintsPerRule":MAP_MAX_BLUEPRINTS_PER_RULE,"maxPlacements":MAP_MAX_PLACEMENTS,
                "maxPolygonPoints":256,"worldUnitsPerTile":10
            },
            "availableTilesets":tilesets,
            "selectedTileset":selected_tileset,
            "blueprints":blueprints,
            "existingAreas":areas,
            "catalog":catalog.summary(),
            "operations":[
                "map.preview","map.apply","map.generate","map.inspect","map.atlas","map.environment.edit",
                "map.audio.edit","map.tile.edit","map.instance.add","map.instance.move",
                "map.instance.remove","map.structure.edit"
            ],
            "compatibility":{
                "sourceModuleImmutable":snapshot.source_intact,
                "tilesAndBlueprintsResolvedLocally":true,
                "visualTileEdgeCompatibilityVerified":false,
                "recommendedMode":"Use one proven tile id until visual connector validation is available."
            }
        }))
    }

    fn verified_map_plan(&mut self, spec: &MapGenerationSpec) -> AppResult<MapGenerationPlan> {
        let tileset_key = ResourceKey::new(&spec.tileset, 2013);
        let set_bytes = self.map_resource_bytes(&tileset_key)?.ok_or_else(|| {
            mcp_error(
                "MCP_MAP_TILESET_MISSING",
                "Le SET de la carte est introuvable dans le module, les HAK ou NWN.",
                tileset_key.to_string(),
            )
        })?;
        let models = parse_set_tile_models(&set_bytes);
        if models.is_empty() {
            return Err(mcp_error(
                "MCP_MAP_TILESET_INVALID",
                "Le SET de la carte ne contient aucune tuile lisible.",
                tileset_key.to_string(),
            ));
        }
        let mut selected_tile_ids = vec![spec.base_tile_id];
        selected_tile_ids.extend(spec.variant_tile_ids.iter().copied());
        selected_tile_ids.sort_unstable();
        selected_tile_ids.dedup();
        let missing = selected_tile_ids
            .iter()
            .filter(|tile_id| !models.contains_key(tile_id))
            .copied()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(mcp_error(
                "MCP_MAP_TILE_MISSING",
                "Une ou plusieurs tuiles n'existent pas dans le SET choisi.",
                format!("{} missing {missing:?}", spec.tileset),
            ));
        }
        for rule in &spec.densities {
            let resource_type = map_template_resource_type(&rule.category).ok_or_else(|| {
                mcp_error(
                    "MCP_MAP_CATEGORY_UNSUPPORTED",
                    "Une catÃ©gorie de placement n'est pas prise en charge.",
                    rule.category.clone(),
                )
            })?;
            for resref in &rule.template_resrefs {
                let key = ResourceKey::new(resref, resource_type);
                if self.map_resource_bytes(&key)?.is_none() {
                    return Err(mcp_error(
                        "MCP_MAP_BLUEPRINT_MISSING",
                        "Un blueprint de la carte est introuvable.",
                        key.to_string(),
                    ));
                }
            }
        }
        generate_map_plan_with_compatibility(
            spec,
            MapCompatibilityReport {
                tileset_resolved: true,
                tileset_sha256: Some(hex::encode(Sha256::digest(&set_bytes))),
                resolved_tile_count: models.len(),
                selected_tile_ids,
                tile_ids_verified: true,
                edge_compatibility_verified: false,
            },
        )
    }

    fn apply_map(
        &mut self,
        spec: MapGenerationSpec,
        expected_plan_sha256: Option<&str>,
    ) -> AppResult<Value> {
        let plan = self.verified_map_plan(&spec)?;
        if expected_plan_sha256
            .is_some_and(|expected| !expected.eq_ignore_ascii_case(&plan.plan_sha256))
        {
            return Err(mcp_error(
                "MCP_MAP_PLAN_CHANGED",
                "Le plan de carte a changÃ© depuis sa prÃ©visualisation.",
                format!("current plan is {}", plan.plan_sha256),
            ));
        }
        let resources = create_generated_map_resources(&plan)?;
        for resource in &resources {
            if self.map_resource_bytes(&resource.key)?.is_some() {
                return Err(mcp_error(
                    "MCP_MAP_AREA_EXISTS",
                    "Une ressource de cette carte existe dÃ©jÃ .",
                    resource.key.to_string(),
                ));
            }
        }
        let workspace = self.workspace.create_resources_atomic(&resources)?;
        Ok(json!({
            "area":plan.spec.resref,
            "planSha256":plan.plan_sha256,
            "metrics":plan.metrics,
            "compatibility":plan.compatibility,
            "warnings":plan.warnings,
            "createdResources":resources.iter().map(|resource| resource.key.to_string()).collect::<Vec<_>>(),
            "workspace":workspace,
        }))
    }

    fn inspect_map(&mut self, area: &str) -> AppResult<Value> {
        let are_key = ResourceKey::new(area, 2012);
        let git_key = ResourceKey::new(area, 2023);
        let gic_key = ResourceKey::new(area, 2046);
        let are = self.map_resource_bytes(&are_key)?.ok_or_else(|| {
            mcp_error(
                "MCP_MAP_AREA_MISSING",
                "La zone demandÃ©e est introuvable.",
                are_key.to_string(),
            )
        })?;
        let git = self.map_resource_bytes(&git_key)?;
        let gic = self.map_resource_bytes(&gic_key)?;
        let are_document = parse_gff(&are, &format!("mcp::{}", are_key.file_name()))?;
        let git_document = git
            .as_deref()
            .map(|bytes| parse_gff(bytes, &format!("mcp::{}", git_key.file_name())))
            .transpose()?;
        let gic_document = gic
            .as_deref()
            .map(|bytes| parse_gff(bytes, &format!("mcp::{}", gic_key.file_name())))
            .transpose()?;
        let area = adapt_area(
            area,
            &are_document,
            git_document.as_ref(),
            gic_document.as_ref(),
        );
        let environment = inspect_area_environment(&are, &format!("mcp::{}", are_key.file_name()))?;
        let audio = git
            .as_deref()
            .map(|bytes| inspect_area_audio(bytes, &format!("mcp::{}", git_key.file_name())))
            .transpose()?;
        Ok(json!({
            "area":area,
            "environment":environment,
            "audio":audio,
            "resourceSha256":{
                "are":hex::encode(Sha256::digest(&are)),
                "git":git.as_deref().map(|bytes| hex::encode(Sha256::digest(bytes))),
                "gic":gic.as_deref().map(|bytes| hex::encode(Sha256::digest(bytes))),
            }
        }))
    }

    fn verify_map_tile(&mut self, area: &str, tile_id: u32) -> AppResult<()> {
        let are_key = ResourceKey::new(area, 2012);
        let are = self.map_resource_bytes(&are_key)?.ok_or_else(|| {
            mcp_error(
                "MCP_MAP_AREA_MISSING",
                "La zone demandÃ©e est introuvable.",
                are_key.to_string(),
            )
        })?;
        let document = parse_gff(&are, &format!("mcp::{}", are_key.file_name()))?;
        let area_map = adapt_area(area, &document, None, None);
        let tileset = area_map.tileset.ok_or_else(|| {
            mcp_error(
                "MCP_MAP_TILESET_MISSING",
                "La zone ne dÃ©clare aucun tileset.",
                area.to_owned(),
            )
        })?;
        let set_key = ResourceKey::new(tileset, 2013);
        let set = self.map_resource_bytes(&set_key)?.ok_or_else(|| {
            mcp_error(
                "MCP_MAP_TILESET_MISSING",
                "Le SET de la zone est introuvable.",
                set_key.to_string(),
            )
        })?;
        if !parse_set_tile_models(&set).contains_key(&tile_id) {
            return Err(mcp_error(
                "MCP_MAP_TILE_MISSING",
                "La tuile demandÃ©e n'existe pas dans le SET de la zone.",
                format!("{set_key}: tile {tile_id}"),
            ));
        }
        Ok(())
    }

    fn validate_map_placement(&mut self, placement: &InstancePlacement) -> AppResult<()> {
        let resource_type = map_template_resource_type(&placement.category).ok_or_else(|| {
            mcp_error(
                "MCP_MAP_CATEGORY_UNSUPPORTED",
                "La catÃ©gorie de placement n'est pas prise en charge.",
                placement.category.clone(),
            )
        })?;
        if placement.tag.is_empty()
            || placement.tag.len() > 64
            || placement
                .tag
                .chars()
                .any(|character| character.is_control())
            || !placement.x.is_finite()
            || !placement.y.is_finite()
            || !placement.z.is_finite()
            || !placement.bearing.is_finite()
        {
            return Err(mcp_error(
                "MCP_MAP_PLACEMENT_INVALID",
                "Le placement contient un tag ou des coordonnÃ©es invalides.",
                placement.tag.clone(),
            ));
        }
        let key = ResourceKey::new(&placement.template_resref, resource_type);
        if self.map_resource_bytes(&key)?.is_none() {
            return Err(mcp_error(
                "MCP_MAP_BLUEPRINT_MISSING",
                "Le blueprint du placement est introuvable.",
                key.to_string(),
            ));
        }
        Ok(())
    }

    fn apply_map_resource_transform(
        &mut self,
        resource: ResourceKey,
        expected_sha256: &str,
        operation: String,
        transform: impl FnOnce(&[u8], &str) -> AppResult<Vec<u8>>,
    ) -> AppResult<Value> {
        let source_bytes = self.resolved_catalog_resource_bytes(&resource)?;
        let current = self
            .workspace
            .staged_resource_bytes(&resource)?
            .or_else(|| source_bytes.clone())
            .ok_or_else(|| {
                mcp_error(
                    "MCP_MAP_RESOURCE_MISSING",
                    "La ressource de carte Ã  modifier est introuvable.",
                    resource.to_string(),
                )
            })?;
        verify_expected_sha256(&current, expected_sha256)?;
        let output = transform(&current, &format!("mcp::{}", resource.file_name()))?;
        let workspace = self.commit_map_transform(
            resource,
            source_bytes.as_deref(),
            &current,
            &output,
            operation,
        )?;
        serialize_tool_value(workspace)
    }

    fn commit_map_transform(
        &mut self,
        resource: ResourceKey,
        source_bytes: Option<&[u8]>,
        current: &[u8],
        output: &[u8],
        operation: String,
    ) -> AppResult<aurora_edit::WorkspaceSnapshot> {
        let before_sha256 = hex::encode(Sha256::digest(current));
        let after_sha256 = hex::encode(Sha256::digest(output));
        self.workspace
            .stage_resource(resource.clone(), source_bytes, output)?;
        self.workspace.apply(EditCommand::TransformResource {
            resource,
            operation,
            before_sha256,
            after_sha256,
        })
    }

    fn resource_bytes(&self, resource: &ResourceKey) -> AppResult<Vec<u8>> {
        if let Some(bytes) = self.workspace.staged_resource_bytes(resource)? {
            return Ok(bytes);
        }
        self.source_resource_bytes(resource)
    }

    fn source_resource_bytes(&self, resource: &ResourceKey) -> AppResult<Vec<u8>> {
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
            {"uri":"opennever://agent/capabilities","name":"Capability registry","mimeType":"application/json"},
            {"uri":"opennever://map/authoring-contract","name":"Map authoring contract","mimeType":"application/json"}
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
            "opennever://map/authoring-contract" => Ok(json!({
                "workflow":["map.context","map.preview","map.apply","map.inspect","targeted map edits","map.inspect"],
                "coordinates":{"tileOrigin":"zero-based top-left grid","worldUnitsPerTile":10,"instanceCoordinates":"world units"},
                "editable":["tiles and height","area scripts","weather and lighting","music and ambient audio","creatures","doors","encounters","items","placeables","sounds","stores","triggers","waypoints","trigger and encounter polygons","encounter spawn points","door and trigger transitions","placeable and store inventories"],
                "preconditions":"Every targeted edit uses the current ARE or GIT SHA-256 returned by map.inspect.",
                "compatibility":"SET tile identifiers and blueprint ResRefs are resolved locally. Visual connector compatibility between different tile variants is not yet proven.",
                "sourceSafety":"All writes target the reversible workspace overlay; the source MOD and NWN installation remain immutable."
            })),
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MapContextArguments {
    tileset: Option<String>,
    query: String,
    limit: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MapAreaArguments {
    area: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MapPreviewArguments {
    spec: MapGenerationSpec,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MapApplyArguments {
    spec: MapGenerationSpec,
    expected_plan_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MapEnvironmentArguments {
    area: String,
    expected_sha256: String,
    patch: AreaEnvironmentPatch,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MapAudioArguments {
    area: String,
    expected_sha256: String,
    patch: AreaAudioPatch,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MapTileArguments {
    area: String,
    x: u32,
    y: u32,
    expected_sha256: String,
    before: TileState,
    after: TileState,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MapInstanceAddArguments {
    area: String,
    expected_sha256: String,
    placement: InstancePlacement,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MapInstanceMoveArguments {
    area: String,
    instance_id: String,
    expected_sha256: String,
    before: Transform,
    after: Transform,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MapInstanceRemoveArguments {
    area: String,
    instance_id: String,
    expected_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MapStructureArguments {
    area: String,
    expected_sha256: String,
    action: AreaStructureAction,
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
            | "map.context"
            | "map.inspect"
            | "map.atlas"
            | "map.preview"
            | "map.apply"
            | "map.generate"
            | "map.environment.edit"
            | "map.audio.edit"
            | "map.tile.edit"
            | "map.instance.add"
            | "map.instance.move"
            | "map.instance.remove"
            | "map.structure.edit"
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

fn map_blueprint_categories() -> [&'static str; 9] {
    [
        "creature",
        "door",
        "encounter",
        "item",
        "placeable",
        "sound",
        "store",
        "trigger",
        "waypoint",
    ]
}

fn map_template_resource_type(category: &str) -> Option<u16> {
    match category {
        "creature" => Some(2027),
        "door" => Some(2042),
        "encounter" => Some(2040),
        "item" => Some(2025),
        "placeable" => Some(2044),
        "sound" => Some(2035),
        "store" => Some(2051),
        "trigger" => Some(2032),
        "waypoint" => Some(2058),
        _ => None,
    }
}

fn verify_expected_sha256(bytes: &[u8], expected: &str) -> AppResult<()> {
    let actual = hex::encode(Sha256::digest(bytes));
    if expected.len() != 64 || !expected.eq_ignore_ascii_case(&actual) {
        return Err(mcp_error(
            "MCP_MAP_RESOURCE_CHANGED",
            "La carte a changÃ© depuis son inspection. Inspectez-la de nouveau.",
            format!("expected {expected:?}, current {actual}"),
        ));
    }
    Ok(())
}

fn non_empty_path(value: &str) -> Option<PathBuf> {
    (!value.trim().is_empty()).then(|| PathBuf::from(value))
}

fn serialize_tool_value(value: impl Serialize) -> AppResult<Value> {
    serde_json::to_value(value).map_err(|error| {
        mcp_error(
            "MCP_TOOL_RESULT_INVALID",
            "Le rÃ©sultat MCP n'a pas pu Ãªtre prÃ©parÃ©.",
            error.to_string(),
        )
    })
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

    #[test]
    fn creates_and_edits_a_complete_map_through_mcp_tools() {
        let temporary = tempfile::tempdir().expect("temporary");
        let module = temporary.path().join("source.mod");
        create_empty_module(
            &module,
            &NewModuleDefinition {
                name: "MCP Map Test".to_owned(),
                tag: "MCP_MAP_TEST".to_owned(),
                entry_area: "entry".to_owned(),
                tileset: "tno01".to_owned(),
            },
        )
        .expect("module");
        let module_bytes = fs::read(&module).expect("module bytes");
        let source_digest = hex::encode(Sha256::digest(&module_bytes));
        let workspace_root = temporary.path().join("workspace");
        let mut workspace = EditWorkspace::create(
            &workspace_root,
            &module,
            &source_digest,
            module_bytes.len() as u64,
        )
        .expect("workspace");
        workspace
            .create_resource(
                ResourceKey::new("tno01", 2013),
                b"[TILE0]\nmodel=tno01_a01\n[TILE1]\nmodel=tno01_a02\n",
            )
            .expect("SET");
        workspace
            .create_resource(ResourceKey::new("plc_test", 2044), b"synthetic UTP")
            .expect("UTP");
        workspace
            .create_resource(ResourceKey::new("trg_test", 2032), b"synthetic UTT")
            .expect("UTT");

        let store = AgentWorkspaceStore::new(&workspace_root);
        let mut policy = built_in_policy(SecurityLevel::Operator);
        policy.context.include_module_metadata = true;
        policy.context.include_resource_contents = true;
        for capability in [
            "map.context",
            "map.inspect",
            "map.atlas",
            "map.preview",
            "map.apply",
            "map.environment.edit",
            "map.tile.edit",
            "map.instance.add",
            "map.instance.move",
            "map.instance.remove",
            "map.structure.edit",
        ] {
            policy.capability_overrides.insert(
                capability.to_owned(),
                CapabilityOverride {
                    access: CapabilityAccess::Execute,
                    approval: ApprovalMode::Never,
                    scope: ToolScope::Workspace,
                    max_calls: 20,
                },
            );
        }
        store.save_policy(&policy).expect("policy");
        let mut server = McpServer::open(workspace_root).expect("server");
        server.handle(json!({
            "jsonrpc":"2.0","id":1,"method":"initialize",
            "params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}
        }));
        server.handle(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));

        let context = server
            .handle(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"map.context","arguments":{"tileset":"tno01","query":"","limit":100}}}))
            .expect("context");
        assert_eq!(
            context["result"]["structuredContent"]["selectedTileset"]["tileIds"],
            json!([0, 1])
        );
        assert_eq!(
            context["result"]["structuredContent"]["blueprints"]["placeable"],
            json!(["plc_test"])
        );

        let spec = json!({
            "schemaVersion":1,
            "brief":"Une petite place avec un dÃ©cor reproductible",
            "resref":"mcp_map",
            "name":"Carte MCP",
            "tileset":"tno01",
            "width":4,
            "height":4,
            "seed":42,
            "baseTileId":0,
            "variantTileIds":[],
            "borderMargin":1,
            "reservedPercent":0,
            "densities":[{"category":"placeable","perHundredTiles":25,"minSpacingTiles":1,"templateResrefs":["plc_test"]}]
        });
        let cursor_before = server.workspace.snapshot().expect("snapshot").cursor;
        let preview = server
            .handle(json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"map.preview","arguments":{"spec":spec.clone()}}}))
            .expect("preview");
        assert_eq!(
            server.workspace.snapshot().expect("snapshot").cursor,
            cursor_before
        );
        let plan_sha = preview["result"]["structuredContent"]["planSha256"]
            .as_str()
            .expect("plan sha")
            .to_owned();
        let applied = server
            .handle(json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"map.apply","arguments":{"spec":spec,"expectedPlanSha256":plan_sha}}}))
            .expect("apply");
        assert_eq!(applied["result"]["isError"], false);

        let inspected = server
            .handle(json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"map.inspect","arguments":{"area":"mcp_map"}}}))
            .expect("inspect");
        let atlas = server
            .handle(json!({"jsonrpc":"2.0","id":51,"method":"tools/call","params":{"name":"map.atlas","arguments":{"area":"mcp_map"}}}))
            .expect("atlas");
        assert_eq!(
            atlas["result"]["structuredContent"]["mimeType"],
            "image/svg+xml"
        );
        assert!(
            atlas["result"]["structuredContent"]["svg"]
                .as_str()
                .is_some_and(|svg| svg.starts_with("<svg"))
        );
        let are_sha = inspected["result"]["structuredContent"]["resourceSha256"]["are"]
            .as_str()
            .expect("ARE sha")
            .to_owned();
        let git_sha = inspected["result"]["structuredContent"]["resourceSha256"]["git"]
            .as_str()
            .expect("GIT sha")
            .to_owned();
        let first_tile = inspected["result"]["structuredContent"]["area"]["tiles"][0].clone();

        let environment = server
            .handle(json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"map.environment.edit","arguments":{"area":"mcp_map","expectedSha256":are_sha,"patch":{"comments":"Carte finalisÃ©e via MCP","chanceRain":20,"onEnter":"map_enter"}}}}))
            .expect("environment");
        assert_eq!(environment["result"]["isError"], false);

        let reinspection = server
            .handle(json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"map.inspect","arguments":{"area":"mcp_map"}}}))
            .expect("reinspect");
        let updated_are_sha = reinspection["result"]["structuredContent"]["resourceSha256"]["are"]
            .as_str()
            .expect("updated ARE sha")
            .to_owned();
        let tile = server
            .handle(json!({"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"map.tile.edit","arguments":{"area":"mcp_map","x":0,"y":0,"expectedSha256":updated_are_sha,"before":{"tileId":first_tile["tileId"],"orientation":first_tile["orientation"],"height":first_tile["height"]},"after":{"tileId":1,"orientation":2,"height":first_tile["height"]}}}}))
            .expect("tile");
        assert_eq!(tile["result"]["isError"], false);

        let trigger = server
            .handle(json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"map.instance.add","arguments":{"area":"mcp_map","expectedSha256":git_sha,"placement":{"category":"trigger","templateResref":"trg_test","tag":"trg_exit","x":15.0,"y":15.0,"z":0.0,"bearing":0.0,"linkedTo":"entry"}}}}))
            .expect("trigger");
        let instance_id = trigger["result"]["structuredContent"]["instanceId"]
            .as_str()
            .expect("instance id")
            .to_owned();
        let after_trigger = server.inspect_map("mcp_map").expect("after trigger");
        let trigger_git_sha = after_trigger["resourceSha256"]["git"]
            .as_str()
            .expect("trigger GIT sha")
            .to_owned();
        let geometry = server
            .handle(json!({"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"map.structure.edit","arguments":{"area":"mcp_map","expectedSha256":trigger_git_sha,"action":{"kind":"set_geometry","instanceId":instance_id,"points":[{"x":12.0,"y":12.0,"z":0.0},{"x":18.0,"y":12.0,"z":0.0},{"x":18.0,"y":18.0,"z":0.0}]}}}}))
            .expect("geometry");
        assert_eq!(geometry["result"]["isError"], false);
        let final_map = server.inspect_map("mcp_map").expect("final map");
        assert!(
            final_map["area"]["instances"]
                .as_array()
                .expect("instances")
                .iter()
                .any(|instance| instance["tag"] == "trg_exit"
                    && instance["geometry"]
                        .as_array()
                        .is_some_and(|points| points.len() == 3))
        );
        assert_eq!(
            fs::read(&module).expect("source remains readable"),
            module_bytes
        );
    }
}
