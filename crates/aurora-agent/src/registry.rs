use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityRisk {
    Low,
    Moderate,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilitySideEffect {
    None,
    ReversibleWorkspace,
    BuildOutput,
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityDescriptor {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub risk: CapabilityRisk,
    pub side_effect: CapabilitySideEffect,
    pub reversible: bool,
    pub input_schema: Value,
    pub output_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityRegistry {
    pub schema_version: u32,
    pub capabilities: Vec<CapabilityDescriptor>,
}

impl CapabilityRegistry {
    pub fn standard() -> Self {
        let mut capabilities = Vec::new();
        let mut add = |id: &str,
                       title: &str,
                       description: &str,
                       category: &str,
                       risk: CapabilityRisk,
                       side_effect: CapabilitySideEffect,
                       reversible: bool,
                       input_schema: Value,
                       output_schema: Value| {
            capabilities.push(CapabilityDescriptor {
                id: id.to_owned(),
                title: title.to_owned(),
                description: description.to_owned(),
                category: category.to_owned(),
                risk,
                side_effect,
                reversible,
                input_schema,
                output_schema,
            });
        };
        let empty = || json!({"type":"object","additionalProperties":false});
        let resource = || {
            json!({
                "type":"object",
                "properties":{
                    "resref":{"type":"string"},
                    "resourceType":{"type":"integer","minimum":0,"maximum":65535}
                },
                "required":["resref","resourceType"],
                "additionalProperties":false
            })
        };
        let object_result = || json!({"type":"object"});
        let module_blueprint = || {
            json!({
                "type":"object",
                "properties":{
                    "schemaVersion":{"type":"integer"},
                    "name":{"type":"string"},
                    "tag":{"type":"string"},
                    "synopsis":{"type":"string"},
                    "entryArea":{"type":"string"},
                    "defaultTileset":{"type":"string"},
                    "requirements":{"type":"array","items":{"type":"object","properties":{"id":{"type":"string"},"description":{"type":"string"},"acceptanceCriteria":{"type":"array","items":{"type":"string"}}},"required":["id","description","acceptanceCriteria"],"additionalProperties":false}},
                    "areas":{"type":"array","items":{"type":"object","properties":{"resref":{"type":"string"},"name":{"type":"string"},"width":{"type":"integer"},"height":{"type":"integer"},"tileset":{"type":"string"},"purpose":{"type":"string"},"connectsTo":{"type":"array","items":{"type":"string"}}},"required":["resref","name","width","height","tileset","purpose","connectsTo"],"additionalProperties":false}},
                    "scripts":{"type":"array","items":{"type":"object","properties":{"resref":{"type":"string"},"event":{"type":"string"},"purpose":{"type":"string"},"source":{"type":["string","null"]}},"required":["resref","event","purpose","source"],"additionalProperties":false}},
                    "dialogues":{"type":"array","items":{"type":"object","properties":{"resref":{"type":"string"},"ownerTag":{"type":"string"},"purpose":{"type":"string"},"requiredNodes":{"type":"array","items":{"type":"string"}}},"required":["resref","ownerTag","purpose","requiredNodes"],"additionalProperties":false}},
                    "customTlk":{"type":["string","null"]},
                    "hakDependencies":{"type":"array","items":{"type":"string"}}
                },
                "required":["schemaVersion","name","tag","synopsis","entryArea","defaultTileset","requirements","areas","scripts","dialogues","customTlk","hakDependencies"],
                "additionalProperties":false
            })
        };

        add(
            "module.inspect",
            "Inspecter le module",
            "Retourne l’identité, les dépendances et les diagnostics du module.",
            "Lecture",
            CapabilityRisk::Low,
            CapabilitySideEffect::None,
            true,
            empty(),
            object_result(),
        );
        add(
            "resource.search",
            "Rechercher des ressources",
            "Recherche bornée dans le Resource Manager.",
            "Lecture",
            CapabilityRisk::Low,
            CapabilitySideEffect::None,
            true,
            json!({"type":"object","properties":{"query":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":200}},"required":["query","limit"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "resource.read",
            "Lire une ressource",
            "Lit une ressource structurée autorisée du module ou du workspace.",
            "Lecture",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::None,
            true,
            resource(),
            object_result(),
        );
        add(
            "diagnostics.run",
            "Exécuter les diagnostics",
            "Exécute les validateurs locaux sans mutation.",
            "Validation",
            CapabilityRisk::Low,
            CapabilitySideEffect::None,
            true,
            empty(),
            object_result(),
        );
        add(
            "architecture.query",
            "Interroger le graphe",
            "Interroge un sous-graphe d’architecture borné.",
            "Lecture",
            CapabilityRisk::Low,
            CapabilitySideEffect::None,
            true,
            json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "resource.set_field",
            "Modifier un champ GFF",
            "Modifie un champ GFF existant avec précondition exacte.",
            "Édition",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"resource":resource(),"path":{"type":"string"},"before":{},"after":{}},"required":["resource","path","before","after"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "script.replace",
            "Remplacer un NSS",
            "Remplace un script source NSS existant après validation syntaxique.",
            "Scripts",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"resource":resource(),"before":{"type":"string"},"after":{"type":"string"}},"required":["resource","before","after"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "script.create",
            "Créer un NSS",
            "Crée un nouveau script NSS dans le workspace après validation syntaxique.",
            "Scripts",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"resref":{"type":"string"},"event":{"type":"string"},"purpose":{"type":"string"},"source":{"type":"string"}},"required":["resref","event","purpose","source"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "script.compile",
            "Compiler un NSS",
            "Compile un NSS et lie le NCS à l’empreinte exacte de ses entrées.",
            "Scripts",
            CapabilityRisk::High,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"resref":{"type":"string"}},"required":["resref"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "module.create",
            "Créer un module",
            "Crée un nouveau module minimal dans une destination autorisée.",
            "Construction",
            CapabilityRisk::High,
            CapabilitySideEffect::BuildOutput,
            false,
            json!({"type":"object","properties":{"outputPath":{"type":"string"},"name":{"type":"string"},"tag":{"type":"string"},"entryArea":{"type":"string"},"tileset":{"type":"string"}},"required":["outputPath","name","tag","entryArea","tileset"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "area.create",
            "Créer une zone",
            "Crée l’ensemble ARE/GIT/GIC transactionnel d’une zone.",
            "Construction",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"resref":{"type":"string"},"name":{"type":"string"},"width":{"type":"integer","minimum":1,"maximum":32},"height":{"type":"integer","minimum":1,"maximum":32},"tileset":{"type":"string"},"tileId":{"type":"integer","minimum":0}},"required":["resref","name","width","height","tileset","tileId"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "area.instance.add",
            "Ajouter une instance",
            "Ajoute une instance typée dans une zone.",
            "Construction",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"area":{"type":"string"},"placement":{"type":"object"}},"required":["area","placement"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "blueprint.edit",
            "Éditer un blueprint",
            "Applique une opération structurée à un blueprint.",
            "Construction",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"resource":resource(),"action":{"type":"object"}},"required":["resource","action"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "dialogue.edit",
            "Éditer un dialogue",
            "Crée ou modifie un graphe de dialogue avec validation des liens.",
            "Narration",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"resref":{"type":"string"},"action":{"type":"object"}},"required":["resref","action"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "dialogue.create",
            "Créer un dialogue",
            "Crée un dialogue minimal qui pourra être enrichi par opérations structurées.",
            "Narration",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"resref":{"type":"string"},"ownerTag":{"type":"string"},"purpose":{"type":"string"},"requiredNodes":{"type":"array","items":{"type":"string"},"maxItems":10000}},"required":["resref","ownerTag","purpose","requiredNodes"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "journal.edit",
            "Éditer le journal",
            "Crée ou modifie des quêtes et entrées de journal.",
            "Narration",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"resource":resource(),"action":{"type":"object"}},"required":["resource","action"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "faction.edit",
            "Éditer les factions",
            "Modifie la matrice de factions et ses références.",
            "Narration",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"resource":resource(),"action":{"type":"object"}},"required":["resource","action"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "walkmesh.edit",
            "Éditer un walkmesh",
            "Applique une opération topologique validée à un WOK/PWK/DWK.",
            "Géométrie",
            CapabilityRisk::High,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"resref":{"type":"string"},"kind":{"type":"string","enum":["wok","pwk","dwk"]},"operation":{"type":"object"}},"required":["resref","kind","operation"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "tlk.edit",
            "Éditer le TLK",
            "Alloue ou modifie une chaîne dans le TLK de travail.",
            "Contenu personnalisé",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"resource":resource(),"action":{"type":"object"}},"required":["resource","action"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "2da.edit",
            "Éditer une 2DA",
            "Modifie une table 2DA de façon déterministe.",
            "Contenu personnalisé",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"resource":resource(),"action":{"type":"object"}},"required":["resource","action"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "module.dependencies",
            "Configurer les dépendances",
            "Configure les HAK et le TLK du module.",
            "Contenu personnalisé",
            CapabilityRisk::High,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"hakFiles":{"type":"array","items":{"type":"string"}},"customTlk":{"type":["string","null"]}},"required":["hakFiles","customTlk"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "workspace.checkpoint",
            "Créer un checkpoint",
            "Persiste un point de reprise avant une phase d’écriture.",
            "Sécurité",
            CapabilityRisk::Low,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"label":{"type":"string"}},"required":["label"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "blueprint.validate",
            "Valider un plan de module",
            "Valide un ModuleBlueprint sans modifier le workspace.",
            "Planification",
            CapabilityRisk::Low,
            CapabilitySideEffect::None,
            true,
            module_blueprint(),
            object_result(),
        );
        add(
            "blueprint.apply",
            "Appliquer un plan de module",
            "Compile un ModuleBlueprint en graphe de tâches transactionnelles.",
            "Planification",
            CapabilityRisk::High,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            module_blueprint(),
            object_result(),
        );
        add(
            "workspace.undo_batch",
            "Annuler un lot",
            "Restaure le workspace au checkpoint indiqué.",
            "Sécurité",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"checkpointId":{"type":"string"}},"required":["checkpointId"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "module.validate",
            "Valider le module",
            "Exécute les validations structurelles, scripts et dépendances.",
            "Validation",
            CapabilityRisk::Low,
            CapabilitySideEffect::None,
            true,
            empty(),
            object_result(),
        );
        add(
            "module.build",
            "Construire le MOD",
            "Produit un nouveau MOD reproductible sans modifier la source.",
            "Production",
            CapabilityRisk::High,
            CapabilitySideEffect::BuildOutput,
            false,
            json!({"type":"object","properties":{"outputPath":{"type":"string"}},"required":["outputPath"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "development.deploy",
            "Déployer dans development",
            "Déploie les ressources modifiées pour un test live.",
            "Production",
            CapabilityRisk::Critical,
            CapabilitySideEffect::External,
            true,
            json!({"type":"object"}),
            object_result(),
        );
        add(
            "toolset.compare",
            "Comparer avec le Toolset",
            "Compare le workspace OpenNever au dossier temporaire Aurora.",
            "Toolset",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::None,
            true,
            json!({"type":"object"}),
            object_result(),
        );
        add(
            "toolset.sync",
            "Synchroniser le Toolset",
            "Synchronise explicitement des ressources avec sauvegardes et préconditions.",
            "Toolset",
            CapabilityRisk::Critical,
            CapabilitySideEffect::External,
            true,
            json!({"type":"object","properties":{"actions":{"type":"array","items":{"type":"object"}}},"required":["actions"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "nwn.launch",
            "Lancer NWN",
            "Lance un profil NWN autorisé et collecte les diagnostics.",
            "Production",
            CapabilityRisk::Critical,
            CapabilitySideEffect::External,
            false,
            json!({"type":"object"}),
            object_result(),
        );

        Self {
            schema_version: 2,
            capabilities,
        }
    }

    pub fn get(&self, id: &str) -> Option<&CapabilityDescriptor> {
        self.capabilities
            .iter()
            .find(|capability| capability.id == id)
    }

    pub fn by_category(&self) -> BTreeMap<String, Vec<&CapabilityDescriptor>> {
        let mut result = BTreeMap::new();
        for capability in &self.capabilities {
            result
                .entry(capability.category.clone())
                .or_insert_with(Vec::new)
                .push(capability);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_ids_are_unique_and_schemas_are_objects() {
        let registry = CapabilityRegistry::standard();
        let mut ids = std::collections::BTreeSet::new();
        for capability in registry.capabilities {
            assert!(ids.insert(capability.id));
            assert!(capability.input_schema.is_object());
            assert!(capability.output_schema.is_object());
        }
    }
}
