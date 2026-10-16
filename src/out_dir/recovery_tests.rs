use super::*;
use tempfile::tempdir;

#[test]
fn cleanup_keeps_only_the_root_ownership_marker() {
    let temp = tempdir().unwrap();
    let out = temp.path();
    prepare_out_dir(out, "run").unwrap();
    fs::create_dir(out.join("nested")).unwrap();
    fs::write(
        out.join("nested").join(MANIFEST_FILE_NAME),
        "stale nested marker",
    )
    .unwrap();
    fs::write(out.join("stale"), "old output").unwrap();
    finalize_out_dir_manifest(out, "run").unwrap();
    let original = fs::read(out.join(MANIFEST_FILE_NAME)).unwrap();

    // Models interruption after the last deletion but before re-claiming.
    clean_dir_contents(out).unwrap();

    assert_eq!(fs::read(out.join(MANIFEST_FILE_NAME)).unwrap(), original);
    assert!(!out.join("nested").exists());
    assert!(!out.join("stale").exists());
    prepare_out_dir(out, "run").unwrap();
    let manifest: RunManifest = crate::read_json_file(&out.join(MANIFEST_FILE_NAME)).unwrap();
    assert_eq!(manifest.phase, PHASE_IN_PROGRESS);
    assert!(manifest.files.is_empty());
}

#[test]
fn every_cleanup_failure_prefix_preserves_ownership_and_can_be_retried() {
    // Three files and two directories. Inject an I/O failure before each
    // removal in turn, regardless of the filesystem's enumeration order.
    for fail_at in 0..5 {
        let temp = tempdir().unwrap();
        let out = temp.path();
        prepare_out_dir(out, "run").unwrap();
        fs::create_dir_all(out.join("nested/deeper")).unwrap();
        for file in ["stale", "nested/stale", "nested/deeper/stale"] {
            fs::write(out.join(file), "old output").unwrap();
        }
        finalize_out_dir_manifest(out, "run").unwrap();
        let marker = out.join(MANIFEST_FILE_NAME);
        let original = fs::read(&marker).unwrap();
        let mut removed = 0;
        let result = cleanup::clean_dir_except_manifest(out, &marker, &mut |path, is_dir| {
            assert_eq!(fs::read(&marker).unwrap(), original);
            if removed == fail_at {
                return Err(AllieError::Io {
                    context: "injected cleanup failure".to_string(),
                    source: std::io::Error::other("synthetic failure"),
                });
            }
            if is_dir {
                fs::remove_dir(path).unwrap();
            } else {
                fs::remove_file(path).unwrap();
            }
            removed += 1;
            Ok(())
        });
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("synthetic failure")
        );
        assert_eq!(removed, fail_at);
        assert_eq!(fs::read(&marker).unwrap(), original);
        assert!(fs::read_dir(out).unwrap().count() > 1);
        prepare_out_dir(out, "run").unwrap();
        assert_eq!(fs::read_dir(out).unwrap().count(), 1);
        let manifest: RunManifest = crate::read_json_file(&marker).unwrap();
        assert_eq!(manifest.phase, PHASE_IN_PROGRESS);
    }
}

#[cfg(unix)]
#[test]
fn failed_manifest_replacement_keeps_recoverable_ownership() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempdir().unwrap();
    let out = temp.path();
    prepare_out_dir(out, "run").unwrap();
    finalize_out_dir_manifest(out, "run").unwrap();
    let original = fs::read(out.join(MANIFEST_FILE_NAME)).unwrap();
    clean_dir_contents(out).unwrap();
    fs::set_permissions(out, fs::Permissions::from_mode(0o500)).unwrap();
    // Privileged users can bypass mode bits; the portable cleanup-injection
    // test above does not depend on permissions. Do not fail root builds.
    if fs::write(out.join("permission-probe"), "probe").is_ok() {
        fs::set_permissions(out, fs::Permissions::from_mode(0o700)).unwrap();
        eprintln!("skipping permission fault: this user bypasses directory mode bits");
        return;
    }
    let result = write_manifest(out, "run", PHASE_IN_PROGRESS, Vec::new());
    fs::set_permissions(out, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(
        result.is_err(),
        "fixture requires unprivileged filesystem permissions"
    );
    assert_eq!(fs::read(out.join(MANIFEST_FILE_NAME)).unwrap(), original);
    prepare_out_dir(out, "run").unwrap();
}

#[cfg(unix)]
#[test]
fn linked_ownership_marker_is_refused_without_touching_either_directory() {
    use std::os::unix::fs::symlink;
    let temp = tempdir().unwrap();
    let external = temp.path().join("external");
    prepare_out_dir(&external, "run").unwrap();
    let original = fs::read(external.join(MANIFEST_FILE_NAME)).unwrap();
    let out = temp.path().join("foreign");
    fs::create_dir(&out).unwrap();
    fs::write(out.join("keep"), "foreign output").unwrap();
    symlink(
        external.join(MANIFEST_FILE_NAME),
        out.join(MANIFEST_FILE_NAME),
    )
    .unwrap();
    assert!(prepare_out_dir(&out, "run").is_err());
    assert_eq!(fs::read(out.join("keep")).unwrap(), b"foreign output");
    assert!(
        fs::symlink_metadata(out.join(MANIFEST_FILE_NAME))
            .unwrap()
            .is_symlink()
    );
    assert_eq!(
        fs::read(external.join(MANIFEST_FILE_NAME)).unwrap(),
        original
    );
}

#[test]
fn malformed_and_nonregular_markers_fail_closed() {
    for directory_marker in [false, true] {
        let temp = tempdir().unwrap();
        let out = temp.path();
        let marker = out.join(MANIFEST_FILE_NAME);
        fs::write(out.join("keep"), "foreign output").unwrap();
        if directory_marker {
            fs::create_dir(&marker).unwrap();
            fs::write(marker.join("keep"), "nested foreign output").unwrap();
        } else {
            fs::write(&marker, "{truncated").unwrap();
        }
        assert!(prepare_out_dir(out, "run").is_err());
        assert_eq!(fs::read(out.join("keep")).unwrap(), b"foreign output");
        if directory_marker {
            assert_eq!(
                fs::read(marker.join("keep")).unwrap(),
                b"nested foreign output"
            );
        } else {
            assert_eq!(fs::read(&marker).unwrap(), b"{truncated");
        }
    }
}

#[cfg(unix)]
#[test]
fn dangling_marker_is_refused_without_claiming_the_directory() {
    let temp = tempdir().unwrap();
    let out = temp.path();
    let marker = out.join(MANIFEST_FILE_NAME);
    std::os::unix::fs::symlink(out.join("missing"), &marker).unwrap();
    assert!(
        prepare_out_dir(out, "run")
            .unwrap_err()
            .to_string()
            .contains("regular file")
    );
    assert!(fs::symlink_metadata(&marker).unwrap().is_symlink());
    assert!(!out.join("missing").exists());
}

#[test]
fn retry_cleans_stranded_atomic_temp_files_with_a_valid_marker() {
    let temp = tempdir().unwrap();
    let out = temp.path();
    prepare_out_dir(out, "run").unwrap();
    fs::write(
        out.join(".allie-run-manifest.json.tmp-interrupted"),
        "partial json",
    )
    .unwrap();
    prepare_out_dir(out, "run").unwrap();
    assert_eq!(fs::read_dir(out).unwrap().count(), 1);
    let manifest: RunManifest = crate::read_json_file(&out.join(MANIFEST_FILE_NAME)).unwrap();
    assert_eq!(manifest.phase, PHASE_IN_PROGRESS);
}
