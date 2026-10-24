use crate::{AllieError, Result};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

// Run after exact-route merging and before generating references or artifacts.
// Explicit manifest IDs are protected. At the map boundary, IDs already assigned
// by discovery are protected too, so adding repository-only surfaces cannot
// change the IDs in the independently promoted replay flow.
pub(super) fn disambiguate_ids<'a>(
    surfaces: impl IntoIterator<Item = (&'a str, &'a mut String, bool)>,
) -> Result<()> {
    disambiguate_with(surfaces, |route| {
        format!("{:x}", Sha256::digest(route.as_bytes()))[..12].to_string()
    })
}

fn disambiguate_with<'a>(
    surfaces: impl IntoIterator<Item = (&'a str, &'a mut String, bool)>,
    route_suffix: impl Fn(&str) -> String,
) -> Result<()> {
    let mut surfaces = surfaces.into_iter().collect::<Vec<_>>();
    surfaces.sort_by_key(|(route, _, _)| *route);
    let mut counts = BTreeMap::new();
    let mut protected_ids = BTreeSet::new();
    for (_, id, protected) in &surfaces {
        *counts.entry(id.to_string()).or_insert(0) += 1;
        if *protected && !protected_ids.insert(id.to_string()) {
            return Err(AllieError::InvalidManifest(format!(
                "distinct discovered routes share explicit state ID {id}; use unique state IDs"
            )));
        }
    }
    // Reserve every existing name, including generated ones, before allocating.
    // A suffix that matches an existing ID or another short hash must not steal
    // that name. Sorted routes and a deterministic counter make this stable even
    // when short hashes collide; uniqueness never relies on hash probability.
    let mut used = counts.keys().cloned().collect::<BTreeSet<_>>();
    for (route, id, protected) in surfaces {
        if protected || counts[id.as_str()] == 1 {
            continue;
        }
        let base = format!("{id}-{}", route_suffix(route));
        let mut candidate = base.clone();
        let mut ordinal = 2;
        while !used.insert(candidate.clone()) {
            candidate = format!("{base}-{ordinal}");
            ordinal += 1;
        }
        *id = candidate;
    }
    Ok(())
}

pub(super) fn route_to_id(route: &str) -> String {
    let mut id = route
        .trim_matches('/')
        .trim_end_matches(".html")
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if id.is_empty() {
        id = "home".to_string();
    }
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allocate(rows: &[(&str, &str, bool)], same_hash: bool) -> Result<BTreeMap<String, String>> {
        let mut rows = rows
            .iter()
            .map(|(route, id, protected)| (route.to_string(), id.to_string(), *protected))
            .collect::<Vec<_>>();
        let entries = rows
            .iter_mut()
            .map(|(route, id, protected)| (route.as_str(), id, *protected));
        if same_hash {
            disambiguate_with(entries, |_| "same".to_string())?;
        } else {
            disambiguate_ids(entries)?;
        }
        Ok(rows.into_iter().map(|(route, id, _)| (route, id)).collect())
    }

    #[test]
    fn unique_ids_and_explicit_collision_names_are_preserved() {
        let rows = [
            ("/", "home", false),
            ("/docs", "docs", true),
            ("/docs/", "docs", false),
            ("/other", "custom", true),
        ];
        let ids = allocate(&rows, false).unwrap();
        assert_eq!(ids["/"], "home");
        assert_eq!(ids["/docs"], "docs");
        assert_eq!(ids["/other"], "custom");
        assert!(ids["/docs/"].starts_with("docs-"));
        assert_ne!(ids["/docs/"], "docs");
    }

    #[test]
    fn generated_collisions_are_route_stable_and_order_independent() {
        let rows = [
            ("/docs", "docs", false),
            ("/docs/", "docs", false),
            ("/docs.html", "docs", false),
            ("/unique", "unique", false),
        ];
        let ids = allocate(&rows, false).unwrap();
        assert_eq!(ids.values().collect::<BTreeSet<_>>().len(), rows.len());
        assert_eq!(ids["/unique"], "unique");
        let reverse = rows.into_iter().rev().collect::<Vec<_>>();
        assert_eq!(ids, allocate(&reverse, false).unwrap());
        // Once discovery assigns an ID, map's repository-only addition may not
        // rename it, even when that new surface's initial slug matches it.
        let preserved = ids["/docs/"].as_str();
        let mapped = allocate(
            &[
                ("/docs/", preserved, true),
                ("/repository-only", preserved, false),
            ],
            false,
        )
        .unwrap();
        assert_eq!(mapped["/docs/"], preserved);
        assert_ne!(mapped["/repository-only"], preserved);
    }

    #[test]
    fn simulated_hash_collisions_cannot_overwrite_existing_or_allocated_ids() {
        let rows = [
            ("/a", "docs", false),
            ("/b", "docs", false),
            ("/c", "docs", false),
            ("/reserved", "docs-same", true),
            ("/also-reserved", "docs-same-2", false),
        ];
        let ids = allocate(&rows, true).unwrap();
        assert_eq!(ids["/reserved"], "docs-same");
        assert_eq!(ids["/also-reserved"], "docs-same-2");
        assert_eq!(ids["/a"], "docs-same-3");
        assert_eq!(ids["/b"], "docs-same-4");
        assert_eq!(ids["/c"], "docs-same-5");
        assert_eq!(ids.values().collect::<BTreeSet<_>>().len(), rows.len());
        assert_eq!(
            ids,
            allocate(&rows.into_iter().rev().collect::<Vec<_>>(), true).unwrap()
        );
    }

    #[test]
    fn ambiguous_explicit_ids_are_rejected_before_any_id_is_changed() {
        let mut ids = [
            "shared".to_string(),
            "shared".to_string(),
            "shared".to_string(),
        ];
        let result = disambiguate_ids(
            ids.iter_mut()
                .enumerate()
                .map(|(index, id)| (["/a", "/b", "/c"][index], id, index > 0)),
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("explicit state ID shared")
        );
        assert_eq!(ids, ["shared", "shared", "shared"]);
    }
}
