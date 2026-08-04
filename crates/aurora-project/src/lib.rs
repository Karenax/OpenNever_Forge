mod analysis;
mod dependencies;
mod dialogues;
mod hashing;
mod model;
mod project;
mod scripts;
mod structured;
mod texture;
mod world;

pub use analysis::{ModuleAnalysis, analyze_module_file, analyze_module_file_with_roots};
pub use aurora_dialogue::{
    DialogueDiagnostic, DialogueGraph, DialogueIndex, DialogueIndexDiagnostic,
    DialogueIndexSummary, DialogueLink, DialogueNode, DialogueNodeKind, DialoguePage,
    DialogueReference, DialogueSearchHit, DialogueTreeNode,
};
pub use aurora_mdl::{GLB_CACHE_SCHEMA_VERSION, GlbArtifact, MdlModel};
pub use aurora_nwscript::{
    InboundScriptReference, NcsDocument, NssDocument, ScriptDiagnostic, ScriptDocument,
    ScriptIndex, ScriptIndexSummary, ScriptPage, ScriptSearchHit, ScriptSymbol, ScriptSymbolKind,
    ScriptTextMatch,
};
pub use aurora_resource::{
    ResolvedResource, ResourceCatalog, ResourceCatalogSummary, ResourceDiagnostic,
    ResourceLocation, ResourceManager, ResourceManagerConfig, ResourcePage, ResourceSourceCount,
    ResourceSourceKind, ResourceTypeCount, ResourceVersion,
};
pub use aurora_world::*;
pub use dependencies::{
    DependencyRoots, ModuleDependency, ModuleDependencyChange, ModuleDependencyKind,
    ModuleDependencyReport, ModuleDependencyState, compare_dependency_reports,
    fingerprint_module_dependencies, inspect_module_dependencies,
};
pub use dialogues::analyze_dialogues;
pub use hashing::{HashProgress, ModuleFingerprint, hash_module_file};
pub use model::{ModelCacheEntry, build_model_preview, cached_model_preview};
pub use project::{PROJECT_FILE_VERSION, ReadonlyProjectDraft, ValidatedProjectPaths};
pub use scripts::analyze_scripts;
pub use structured::{
    AreaDefinition, BlueprintSummary, GffValidationSummary, StructuredResourceSummary,
    TableSummary, TlkTableSummary, analyze_structured_resources,
};
pub use texture::{AssetPreview, build_asset_preview};
pub use world::analyze_world;
