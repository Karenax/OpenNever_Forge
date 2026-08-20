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
        let map_generation_spec = || {
            json!({
                "type":"object",
                "properties":{
                    "schemaVersion":{"type":"integer","const":1},
                    "brief":{"type":"string","maxLength":65536},
                    "resref":{"type":"string","maxLength":16},
                    "name":{"type":"string","maxLength":1024},
                    "tileset":{"type":"string","maxLength":16},
                    "width":{"type":"integer","minimum":1,"maximum":32},
                    "height":{"type":"integer","minimum":1,"maximum":32},
                    "seed":{"type":"integer","minimum":0,"maximum":4294967295_u64},
                    "baseTileId":{"type":"integer","minimum":0},
                    "variantTileIds":{"type":"array","maxItems":128,"items":{"type":"integer","minimum":0}},
                    "borderMargin":{"type":"integer","minimum":0,"maximum":31},
                    "reservedPercent":{"type":"integer","minimum":0,"maximum":90},
                    "densities":{"type":"array","maxItems":16,"items":{
                        "type":"object",
                        "properties":{
                            "category":{"type":"string","enum":["creature","door","encounter","item","placeable","sound","store","trigger","waypoint"]},
                            "perHundredTiles":{"type":"integer","minimum":0,"maximum":100},
                            "minSpacingTiles":{"type":"integer","minimum":0,"maximum":64},
                            "templateResrefs":{"type":"array","maxItems":128,"items":{"type":"string","maxLength":16}}
                        },
                        "required":["category","perHundredTiles","minSpacingTiles","templateResrefs"],
                        "additionalProperties":false
                    }}
                },
                "required":["schemaVersion","brief","resref","name","tileset","width","height","seed","baseTileId","variantTileIds","borderMargin","reservedPercent","densities"],
                "additionalProperties":false
            })
        };
        let transform = || {
            json!({
                "type":"object",
                "properties":{
                    "x":{"type":"number"},"y":{"type":"number"},"z":{"type":"number"},
                    "bearing":{"type":"number"}
                },
                "required":["x","y","z","bearing"],
                "additionalProperties":false
            })
        };
        let tile_state = || {
            json!({
                "type":"object",
                "properties":{
                    "tileId":{"type":"integer","minimum":0},
                    "orientation":{"type":"integer","minimum":0,"maximum":3},
                    "height":{"type":"integer","minimum":-32,"maximum":32}
                },
                "required":["tileId","orientation","height"],
                "additionalProperties":false
            })
        };
        let instance_placement = || {
            json!({
                "type":"object",
                "properties":{
                    "category":{"type":"string","enum":["creature","door","encounter","item","placeable","sound","store","trigger","waypoint"]},
                    "templateResref":{"type":"string","maxLength":16},
                    "tag":{"type":"string","maxLength":64},
                    "x":{"type":"number"},"y":{"type":"number"},"z":{"type":"number"},
                    "bearing":{"type":"number"},
                    "linkedTo":{"type":["string","null"],"maxLength":64}
                },
                "required":["category","templateResref","tag","x","y","z","bearing","linkedTo"],
                "additionalProperties":false
            })
        };
        let area_structure_action = || {
            json!({
                "oneOf":[
                    {"type":"object","properties":{"kind":{"const":"set_geometry"},"instanceId":{"type":"string"},"points":{"type":"array","maxItems":256,"items":{"type":"object","properties":{"x":{"type":"number"},"y":{"type":"number"},"z":{"type":"number"}},"required":["x","y","z"],"additionalProperties":false}}},"required":["kind","instanceId","points"],"additionalProperties":false},
                    {"type":"object","properties":{"kind":{"const":"set_spawn_points"},"instanceId":{"type":"string"},"points":{"type":"array","maxItems":256,"items":{"type":"object","properties":{"x":{"type":"number"},"y":{"type":"number"},"z":{"type":"number"},"orientation":{"type":"number"}},"required":["x","y","z","orientation"],"additionalProperties":false}}},"required":["kind","instanceId","points"],"additionalProperties":false},
                    {"type":"object","properties":{"kind":{"const":"set_transition"},"instanceId":{"type":"string"},"destination":{"type":"string","maxLength":64},"flags":{"type":"integer","minimum":0,"maximum":255},"loadScreenId":{"type":"integer","minimum":0,"maximum":65535}},"required":["kind","instanceId","destination","flags","loadScreenId"],"additionalProperties":false},
                    {"type":"object","properties":{"kind":{"const":"add_inventory_item"},"instanceId":{"type":"string"},"resref":{"type":"string","maxLength":16},"stackSize":{"type":"integer","minimum":1,"maximum":65535},"x":{"type":"integer","minimum":0,"maximum":65535},"y":{"type":"integer","minimum":0,"maximum":65535},"infinite":{"type":"boolean"},"categoryIndex":{"type":["integer","null"],"minimum":0,"maximum":4}},"required":["kind","instanceId","resref","stackSize","x","y","infinite","categoryIndex"],"additionalProperties":false},
                    {"type":"object","properties":{"kind":{"const":"remove_inventory_item"},"instanceId":{"type":"string"},"itemIndex":{"type":"integer","minimum":0},"categoryIndex":{"type":["integer","null"],"minimum":0,"maximum":4}},"required":["kind","instanceId","itemIndex","categoryIndex"],"additionalProperties":false}
                ]
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
            "map.generate",
            "Générer une carte déterministe",
            "Transforme un brief borné en tuiles et placements reproductibles, puis crée ARE/GIT/GIC dans le workspace.",
            "Construction",
            CapabilityRisk::High,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            map_generation_spec(),
            object_result(),
        );
        add(
            "map.context",
            "DÃ©couvrir le contexte cartographique",
            "RÃ©sout les tilesets, identifiants de tuiles, zones et blueprints disponibles sans transmettre les octets NWN.",
            "Cartes",
            CapabilityRisk::Low,
            CapabilitySideEffect::None,
            true,
            json!({"type":"object","properties":{"tileset":{"type":["string","null"],"maxLength":16},"query":{"type":"string","maxLength":128},"limit":{"type":"integer","minimum":1,"maximum":500}},"required":["tileset","query","limit"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "map.inspect",
            "Inspecter une carte",
            "Retourne la grille, les instances, volumes, transitions, inventaires et empreintes ARE/GIT/GIC d'une zone.",
            "Cartes",
            CapabilityRisk::Low,
            CapabilitySideEffect::None,
            true,
            json!({"type":"object","properties":{"area":{"type":"string","maxLength":16}},"required":["area"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "map.atlas",
            "GÃ©nÃ©rer l'atlas d'une carte",
            "Produit un SVG local dÃ©terministe avec grille, tuiles, hauteurs, orientations et instances, sans texture propriÃ©taire.",
            "Cartes",
            CapabilityRisk::Low,
            CapabilitySideEffect::None,
            true,
            json!({"type":"object","properties":{"area":{"type":"string","maxLength":16}},"required":["area"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "map.preview",
            "PrÃ©visualiser une carte dÃ©terministe",
            "Valide le SET, les blueprints et le plan d'une carte sans modifier le workspace.",
            "Cartes",
            CapabilityRisk::Low,
            CapabilitySideEffect::None,
            true,
            json!({"type":"object","properties":{"spec":map_generation_spec()},"required":["spec"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "map.apply",
            "Appliquer un plan de carte",
            "Recalcule un plan prÃ©visualisÃ© et crÃ©e atomiquement ARE/GIT/GIC si son empreinte est inchangÃ©e.",
            "Cartes",
            CapabilityRisk::High,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"spec":map_generation_spec(),"expectedPlanSha256":{"type":"string","minLength":64,"maxLength":64}},"required":["spec","expectedPlanSha256"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "map.environment.edit",
            "Modifier l'environnement d'une carte",
            "Modifie les scripts de zone, mÃ©tÃ©o, Ã©clairage, brouillard, repos, JcJ et Ã©cran de chargement avec empreinte ARE prÃ©alable.",
            "Cartes",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"area":{"type":"string","maxLength":16},"expectedSha256":{"type":"string","minLength":64,"maxLength":64},"patch":{"type":"object","properties":{"tag":{"type":["string","null"],"maxLength":64},"comments":{"type":["string","null"],"maxLength":16384},"dayNightCycle":{"type":["boolean","null"]},"isNight":{"type":["boolean","null"]},"noRest":{"type":["boolean","null"]},"playerVsPlayer":{"type":["integer","null"],"minimum":0,"maximum":2},"chanceRain":{"type":["integer","null"],"minimum":0,"maximum":100},"chanceSnow":{"type":["integer","null"],"minimum":0,"maximum":100},"chanceLightning":{"type":["integer","null"],"minimum":0,"maximum":100},"windPower":{"type":["integer","null"],"minimum":0,"maximum":2},"fogClipDistance":{"type":["number","null"],"minimum":1,"maximum":1000},"skyBox":{"type":["integer","null"],"minimum":0,"maximum":255},"loadScreenId":{"type":["integer","null"],"minimum":0,"maximum":65535},"lightingScheme":{"type":["integer","null"],"minimum":0,"maximum":255},"shadowOpacity":{"type":["integer","null"],"minimum":0,"maximum":100},"sunAmbientColor":{"type":["integer","null"],"minimum":0,"maximum":4294967295_u64},"sunDiffuseColor":{"type":["integer","null"],"minimum":0,"maximum":4294967295_u64},"sunFogColor":{"type":["integer","null"],"minimum":0,"maximum":4294967295_u64},"sunFogAmount":{"type":["integer","null"],"minimum":0,"maximum":255},"sunShadows":{"type":["boolean","null"]},"moonAmbientColor":{"type":["integer","null"],"minimum":0,"maximum":4294967295_u64},"moonDiffuseColor":{"type":["integer","null"],"minimum":0,"maximum":4294967295_u64},"moonFogColor":{"type":["integer","null"],"minimum":0,"maximum":4294967295_u64},"moonFogAmount":{"type":["integer","null"],"minimum":0,"maximum":255},"moonShadows":{"type":["boolean","null"]},"onEnter":{"type":["string","null"],"maxLength":16},"onExit":{"type":["string","null"],"maxLength":16},"onHeartbeat":{"type":["string","null"],"maxLength":16},"onUserDefined":{"type":["string","null"],"maxLength":16}},"additionalProperties":false}},"required":["area","expectedSha256","patch"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "map.audio.edit",
            "Modifier l'ambiance audio d'une carte",
            "Modifie musiques, sons ambiants, volumes et environnement audio avec empreinte GIT prÃ©alable.",
            "Cartes",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"area":{"type":"string","maxLength":16},"expectedSha256":{"type":"string","minLength":64,"maxLength":64},"patch":{"type":"object","properties":{"ambientSoundDay":{"type":["integer","null"],"minimum":0},"ambientSoundNight":{"type":["integer","null"],"minimum":0},"ambientSoundDayVolume":{"type":["integer","null"],"minimum":0,"maximum":127},"ambientSoundNightVolume":{"type":["integer","null"],"minimum":0,"maximum":127},"environmentAudio":{"type":["integer","null"],"minimum":0},"musicBattle":{"type":["integer","null"],"minimum":0},"musicDay":{"type":["integer","null"],"minimum":0},"musicNight":{"type":["integer","null"],"minimum":0},"musicDelay":{"type":["integer","null"],"minimum":0}},"additionalProperties":false}},"required":["area","expectedSha256","patch"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "map.tile.edit",
            "Modifier une tuile de carte",
            "Modifie l'identifiant, l'orientation et la hauteur d'une tuile avec coordonnÃ©es et prÃ©condition exactes.",
            "Cartes",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"area":{"type":"string","maxLength":16},"x":{"type":"integer","minimum":0,"maximum":31},"y":{"type":"integer","minimum":0,"maximum":31},"expectedSha256":{"type":"string","minLength":64,"maxLength":64},"before":tile_state(),"after":tile_state()},"required":["area","x","y","expectedSha256","before","after"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "map.instance.add",
            "Ajouter une instance Ã  une carte",
            "Ajoute une crÃ©ature, porte, rencontre, objet, plaÃ§able, son, marchand, trigger ou waypoint rÃ©solu.",
            "Cartes",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"area":{"type":"string","maxLength":16},"expectedSha256":{"type":"string","minLength":64,"maxLength":64},"placement":instance_placement()},"required":["area","expectedSha256","placement"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "map.instance.move",
            "DÃ©placer une instance de carte",
            "DÃ©place ou rÃ©oriente une instance avec transform prÃ©cÃ©dent exact.",
            "Cartes",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"area":{"type":"string","maxLength":16},"instanceId":{"type":"string","maxLength":256},"expectedSha256":{"type":"string","minLength":64,"maxLength":64},"before":transform(),"after":transform()},"required":["area","instanceId","expectedSha256","before","after"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "map.instance.remove",
            "Supprimer une instance de carte",
            "Supprime une instance dÃ©signÃ©e par son identifiant d'inspection et une empreinte GIT exacte.",
            "Cartes",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"area":{"type":"string","maxLength":16},"instanceId":{"type":"string","maxLength":256},"expectedSha256":{"type":"string","minLength":64,"maxLength":64}},"required":["area","instanceId","expectedSha256"],"additionalProperties":false}),
            object_result(),
        );
        add(
            "map.structure.edit",
            "Modifier la structure d'une instance",
            "Modifie polygones de triggers/rencontres, points d'apparition, transitions et inventaires intÃ©grÃ©s.",
            "Cartes",
            CapabilityRisk::Moderate,
            CapabilitySideEffect::ReversibleWorkspace,
            true,
            json!({"type":"object","properties":{"area":{"type":"string","maxLength":16},"expectedSha256":{"type":"string","minLength":64,"maxLength":64},"action":area_structure_action()},"required":["area","expectedSha256","action"],"additionalProperties":false}),
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
