use super::*;
use crate::FlowManifest;
use std::path::Path;

fn command(args: Vec<String>) -> (i32, String) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status = crate::run_cli_with_io(args, &mut stdout, &mut stderr);
    (status, String::from_utf8_lossy(&stderr).into_owned())
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn colliding_route_ids_remain_unique_through_promotion_and_map() {
    let site = DirectorySite::start();
    for explicit_docs in [false, true] {
        let temp = tempdir().unwrap();
        let manifest_path =
            write_live_discovery_manifest(temp.path(), &format!("{}/both", site.base_url));
        let mut manifest = FlowManifest::load(&manifest_path).unwrap();
        manifest.flow.states[0].id = if explicit_docs { "docs" } else { "declared" }.to_string();
        manifest.flow.states[0].path =
            if explicit_docs { "/docs" } else { "/declared" }.to_string();
        std::fs::write(&manifest_path, serde_yaml::to_string(&manifest).unwrap()).unwrap();
        let discovery = temp.path().join("discovery");
        let map_dir = temp.path().join("map");
        let project = temp.path().join("project");
        std::fs::create_dir(&project).unwrap();
        std::fs::write(
            project.join("docs.html"),
            "<title>Repository-only docs</title>",
        )
        .unwrap();
        assert_eq!(
            command(vec![
                "discover".into(),
                "--manifest".into(),
                manifest_path.display().to_string(),
                "--out".into(),
                discovery.display().to_string()
            ]),
            (0, String::new())
        );
        let packet = read_json(&discovery.join("discovery.json"));
        let discovery_ids = packet["surfaces"]
            .as_array()
            .unwrap()
            .iter()
            .map(|surface| {
                (
                    surface["route"].as_str().unwrap().to_string(),
                    surface["id"].as_str().unwrap().to_string(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert_ne!(discovery_ids["/docs"], discovery_ids["/docs/"]);
        assert_eq!(
            discovery_ids.values().collect::<BTreeSet<_>>().len(),
            discovery_ids.len()
        );
        if explicit_docs {
            assert_eq!(discovery_ids["/docs"], "docs");
        }
        let promoted_path = temp.path().join("promoted.yml");
        assert_eq!(
            command(vec![
                "promote-flow".into(),
                "--discovery".into(),
                discovery.join("discovery.json").display().to_string(),
                "--flow-plan".into(),
                discovery.join("flow-plan.json").display().to_string(),
                "--out".into(),
                promoted_path.display().to_string()
            ]),
            (0, String::new())
        );
        let promoted = FlowManifest::load(&promoted_path).unwrap();
        let promoted_ids = promoted
            .flow
            .states
            .iter()
            .map(|state| (state.path.clone(), state.id.clone()))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(promoted_ids, discovery_ids);
        assert_eq!(
            command(vec![
                "map".into(),
                "--manifest".into(),
                manifest_path.display().to_string(),
                "--out".into(),
                map_dir.display().to_string(),
                "--project-root".into(),
                project.display().to_string()
            ]),
            (0, String::new())
        );
        let map = read_json(&map_dir.join("product-map.json"));
        let map_ids = map["surfaces"]
            .as_array()
            .unwrap()
            .iter()
            .map(|surface| {
                (
                    surface["routes"][0].as_str().unwrap().to_string(),
                    surface["id"].as_str().unwrap().to_string(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        for (route, id) in &discovery_ids {
            assert_eq!(&map_ids[route], id);
        }
        assert!(map_ids.contains_key("/docs.html"));
        assert_eq!(
            map_ids.values().collect::<BTreeSet<_>>().len(),
            map_ids.len()
        );
        let referenced = map["workflows"][0]["surface_refs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(referenced, map_ids.values().map(String::as_str).collect());
        if explicit_docs {
            let docs = map["surfaces"]
                .as_array()
                .unwrap()
                .iter()
                .find(|surface| surface["id"] == "docs")
                .unwrap();
            assert_eq!(docs["evidence_refs"], serde_json::json!(["docs"]));
        }
        let mapped = FlowManifest::load(&map_dir.join("generated-flow.yml")).unwrap();
        let mapped_ids = mapped
            .flow
            .states
            .iter()
            .map(|state| (state.path.clone(), state.id.clone()))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(mapped_ids, map_ids);
    }
}

#[test]
fn ambiguous_explicit_ids_emit_no_discovery_or_map_artifacts() {
    let site = DirectorySite::start();
    let temp = tempdir().unwrap();
    let manifest_path = write_live_discovery_manifest(temp.path(), &site.base_url);
    let mut manifest = FlowManifest::load(&manifest_path).unwrap();
    let mut duplicate = manifest.flow.states[0].clone();
    duplicate.path = "/other".to_string();
    manifest.flow.states.push(duplicate);
    std::fs::write(&manifest_path, serde_yaml::to_string(&manifest).unwrap()).unwrap();
    for name in ["discover", "map"] {
        let out = temp.path().join(name);
        let (status, error) = command(vec![
            name.into(),
            "--manifest".into(),
            manifest_path.display().to_string(),
            "--out".into(),
            out.display().to_string(),
        ]);
        assert_ne!(status, 0);
        assert!(error.contains("explicit state ID home"), "{error}");
        assert_eq!(std::fs::read_dir(&out).unwrap().count(), 0);
    }
}
