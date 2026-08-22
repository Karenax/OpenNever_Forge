use aurora_core::ResourceKey;
use aurora_dialogue::adapt_dialogue;
use aurora_gff::{GenericField, GenericGff, GenericStruct, GenericValue};
use serde::Deserialize;
use std::time::Instant;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VolumeDialogue {
    resref: String,
    starting: Vec<u32>,
    expected_cycles: u32,
    expected_unreachable: Vec<String>,
    entries: Vec<EntrySpec>,
    replies: Vec<ReplySpec>,
}

#[derive(Deserialize)]
struct EntrySpec {
    #[serde(default)]
    replies: Vec<u32>,
}

#[derive(Deserialize)]
struct ReplySpec {
    #[serde(default)]
    entries: Vec<u32>,
}

fn dword(label: &str, value: u32) -> GenericField {
    GenericField {
        label: label.into(),
        field_type: 4,
        value: GenericValue::Dword(value),
    }
}

fn list(label: &str, values: Vec<GenericStruct>) -> GenericField {
    GenericField {
        label: label.into(),
        field_type: 15,
        value: GenericValue::List(values),
    }
}

fn index_targets(targets: &[u32]) -> Vec<GenericStruct> {
    targets
        .iter()
        .enumerate()
        .map(|(index, target)| GenericStruct {
            index: index as u32,
            struct_type: 0,
            fields: vec![dword("Index", *target)],
        })
        .collect()
}

fn node(index: usize, outgoing: &str, targets: &[u32]) -> GenericStruct {
    GenericStruct {
        index: index as u32,
        struct_type: 0,
        fields: vec![list(outgoing, index_targets(targets))],
    }
}

#[test]
fn adapts_the_thousand_node_volume_fixture_within_budget() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = format!(
        "{}/../../fixtures/synthetic/volume/dialogue_narrative.json",
        manifest_dir
    );
    let raw_json = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("fixture volume illisible ({path}) : {error}"));
    let fixture: VolumeDialogue =
        serde_json::from_str(&raw_json).expect("fixture volume JSON invalide");

    let root_fields = vec![
        list(
            "EntryList",
            fixture
                .entries
                .iter()
                .enumerate()
                .map(|(index, entry)| node(index, "RepliesList", &entry.replies))
                .collect(),
        ),
        list(
            "ReplyList",
            fixture
                .replies
                .iter()
                .enumerate()
                .map(|(index, reply)| node(index, "EntriesList", &reply.entries))
                .collect(),
        ),
        list("StartingList", index_targets(&fixture.starting)),
    ];

    let raw = GenericGff {
        file_type: "DLG ".into(),
        file_version: "V3.2".into(),
        source: fixture.resref.clone(),
        struct_count: 0,
        field_count: 0,
        root: GenericStruct {
            index: 0,
            struct_type: u32::MAX,
            fields: root_fields,
        },
    };

    let started = Instant::now();
    let graph = adapt_dialogue(
        ResourceKey::new(fixture.resref.clone(), 2029),
        fixture.resref.clone(),
        raw,
    );
    let elapsed = started.elapsed();

    println!(
        "adaptation du dialogue de volume : {} noeuds, {} liens en {:.1} ms",
        graph.nodes.len(),
        graph.links.len(),
        elapsed.as_secs_f64() * 1000.0
    );

    let node_count = fixture.entries.len() + fixture.replies.len();
    assert!(
        node_count >= 1000,
        "le fixture doit contenir au moins 1000 nœuds"
    );
    assert_eq!(
        graph.nodes.len(),
        node_count,
        "tous les nœuds doivent être adaptés"
    );
    assert!(
        graph.cycles.len() as u32 >= fixture.expected_cycles,
        "au moins {} cycles attendus, obtenu {:?}",
        fixture.expected_cycles,
        graph.cycles.len()
    );
    for unreachable in &fixture.expected_unreachable {
        assert!(
            graph
                .unreachable_nodes
                .iter()
                .any(|value| value == unreachable),
            "{unreachable} doit être signalé inaccessible"
        );
    }
    assert!(
        !graph.unreachable_nodes.is_empty(),
        "les nœuds isolés doivent être détectés"
    );
    assert!(
        graph.links.iter().all(|link| !link.broken),
        "aucun lien cassé attendu dans le fixture"
    );
    assert!(
        graph.shared_nodes.contains(&"reply:599".into()),
        "reply:599 est référencé deux fois et doit être partagé"
    );
    assert!(
        !graph.tree.is_empty(),
        "l'arbre borné doit rester constructible"
    );
    assert!(
        elapsed.as_secs_f64() < 5.0,
        "l'adaptation à volume réel doit rester sous 5 s"
    );
}
