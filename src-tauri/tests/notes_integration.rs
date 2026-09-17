//! End-to-end note operations against a real temporary notes folder.
//!
//! These cover the promises that matter most: your bytes come back exactly as
//! you wrote them, deletes are recoverable, and nothing escapes the notes root.

use mushroom_lib::notes::cache;
use mushroom_lib::notes::model::NoteId;
use mushroom_lib::notes::service::NotesService;

fn service() -> (NotesService, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("notes");
    cache::bootstrap(&root).unwrap();
    let svc = NotesService::new();
    svc.set_root(root).unwrap();
    svc.rescan().unwrap();
    (svc, dir)
}

#[test]
fn full_lifecycle_create_edit_rename_move_delete() {
    let (svc, _dir) = service();

    let meta = svc.create("work", "Batch Incident").unwrap();
    assert_eq!(meta.id.as_str(), "work/batch-incident.md");

    let opened = svc.read(&meta.id).unwrap();
    svc.save(
        &meta.id,
        "# Batch Incident\n\nThe orchestrator stops the pool early.\n",
        Some(opened.disk_modified),
    )
    .unwrap();

    let renamed = svc.rename(&meta.id, "GPU Shutdown").unwrap();
    assert_eq!(renamed.id.as_str(), "work/gpu-shutdown.md");
    assert!(svc
        .read(&renamed.id)
        .unwrap()
        .body
        .contains("orchestrator stops the pool early"));

    let moved = svc.move_note(&renamed.id, "projects").unwrap();
    assert_eq!(moved.id.as_str(), "projects/gpu-shutdown.md");

    let entry = svc.delete(&moved.id).unwrap();
    assert!(svc.read(&moved.id).is_err());

    // The trashed copy still holds the content (R7.2).
    let recovered = std::fs::read_to_string(&entry.trashed_path).unwrap();
    assert!(recovered.contains("orchestrator stops the pool early"));
}

/// Task 24: a note's bytes must survive a save untouched, apart from the
/// `updated` timestamp Mushroom owns.
#[test]
fn saving_preserves_awkward_bytes_exactly() {
    let (svc, _dir) = service();
    let meta = svc.create("work", "Fidelity").unwrap();
    let path = svc.root().unwrap().join(meta.id.as_str());

    // CRLF endings, tabs, trailing spaces, unicode, a nested YAML block, a
    // comment, and a key Mushroom knows nothing about.
    let original = "---\r\n\
        # a comment\r\n\
        title: Fidelity\r\n\
        custom_key: keep me\r\n\
        nested:\r\n\
        \x20 a: 1\r\n\
        tags:\r\n\
        \x20 - block\r\n\
        \x20 - style\r\n\
        ---\r\n\
        # Heading\r\n\
        \r\n\
        \tTabbed line with trailing spaces   \r\n\
        Café 日本語 — em dash\r\n";
    std::fs::write(&path, original).unwrap();

    let opened = svc.read(&meta.id).unwrap();
    svc.save(&meta.id, &opened.body, Some(opened.disk_modified))
        .unwrap();

    let after = std::fs::read_to_string(&path).unwrap();

    for fragment in [
        "# a comment",
        "custom_key: keep me",
        "nested:",
        "  a: 1",
        "  - block",
        "  - style",
        "\tTabbed line with trailing spaces   ",
        "Café 日本語 — em dash",
    ] {
        assert!(after.contains(fragment), "lost {fragment:?}\n---\n{after}");
    }

    assert!(after.contains("\r\n"), "CRLF endings must survive");
    assert!(
        !after.contains("\n\n\n"),
        "line endings must not have been mangled"
    );
    // Key order is the user's, not ours.
    assert!(after.find("# a comment").unwrap() < after.find("title:").unwrap());
}

#[test]
fn frontmatter_round_trips_with_unknown_keys() {
    let (svc, _dir) = service();
    let meta = svc.create("ideas", "Keys").unwrap();
    let path = svc.root().unwrap().join(meta.id.as_str());

    std::fs::write(
        &path,
        "---\ntitle: Keys\nauthor: someone else\nweight: 42\n---\nbody\n",
    )
    .unwrap();

    let opened = svc.read(&meta.id).unwrap();
    svc.save(&meta.id, "edited body\n", Some(opened.disk_modified))
        .unwrap();

    let after = std::fs::read_to_string(&path).unwrap();
    assert!(after.contains("author: someone else"));
    assert!(after.contains("weight: 42"));
    assert!(after.contains("edited body"));
}

#[test]
fn concurrent_save_is_refused_and_loses_nothing() {
    let (svc, _dir) = service();
    let meta = svc.create("work", "Contested").unwrap();
    let opened = svc.read(&meta.id).unwrap();
    let path = svc.root().unwrap().join(meta.id.as_str());

    // mtime has one-second resolution, so wait before the competing write.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::write(&path, "---\ntitle: Contested\n---\nthem\n").unwrap();

    let result = svc.save(&meta.id, "us\n", Some(opened.disk_modified));
    assert!(result.is_err(), "must refuse to overwrite blindly");
    assert!(std::fs::read_to_string(&path).unwrap().contains("them"));
}

#[test]
fn path_escapes_are_rejected() {
    let (svc, _dir) = service();
    for evil in [
        "../escape.md",
        "work/../../escape.md",
        "C:/Windows/System32/config/SAM",
        "CON.md",
    ] {
        assert!(
            svc.read(&NoteId::new(evil)).is_err(),
            "should have refused {evil:?}"
        );
    }
}

#[test]
fn import_never_overwrites_an_existing_note() {
    let (svc, dir) = service();
    let outside = dir.path().join("inbox");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("shared.md"), "from outside\n").unwrap();

    svc.create("work", "shared").unwrap();
    svc.save(&NoteId::new("work/shared.md"), "mine, keep me\n", None)
        .unwrap();

    let report = svc.import(&[outside], "work").unwrap();
    assert_eq!(report.imported, 1);

    let mine = std::fs::read_to_string(svc.root().unwrap().join("work/shared.md")).unwrap();
    assert!(
        mine.contains("mine, keep me"),
        "existing note was clobbered"
    );
    assert!(svc.root().unwrap().join("work/shared-2.md").exists());
}

#[test]
fn deleting_a_folder_recovers_every_note_in_it() {
    let (svc, _dir) = service();
    svc.create("work", "One").unwrap();
    svc.create("work", "Two").unwrap();

    let affected = svc.delete_folder("work").unwrap();
    assert_eq!(affected, 2);

    // Everything is still on disk under .trash, just moved.
    let trash = svc.root().unwrap().join(".trash");
    let recovered: Vec<_> = walkdir::WalkDir::new(&trash)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .collect();
    assert_eq!(recovered.len(), 2, "both notes must be recoverable");
}

#[test]
fn a_rescan_after_external_changes_sees_them() {
    let (svc, _dir) = service();
    let root = svc.root().unwrap();

    std::fs::write(root.join("ideas/dropped-in.md"), "# Dropped In\n").unwrap();
    assert_eq!(svc.list(Some("ideas")).unwrap().len(), 0, "cache is stale");

    svc.rescan().unwrap();
    let listed = svc.list(Some("ideas")).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].title, "Dropped In");
}

/// Task 25: the scan must stay fast enough that startup does not stall.
///
/// Ignored by default because it writes 5,000 files; run with
/// `cargo test --test notes_integration -- --ignored --nocapture`.
#[test]
#[ignore]
fn scales_to_five_thousand_notes() {
    use std::time::Instant;

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("notes");
    cache::bootstrap(&root).unwrap();

    let started = Instant::now();
    for folder in 0..50 {
        let path = root.join(format!("folder-{folder:02}"));
        std::fs::create_dir_all(&path).unwrap();
        for note in 0..100 {
            let body = format!(
                "---\ntitle: Note {folder}-{note}\ncreated: 2026-01-01T00:00:00Z\n---\n\
                 # Note {folder}-{note}\n\n{}\n",
                "Some realistic paragraph text about GPU nodes and orchestration. ".repeat(12)
            );
            std::fs::write(path.join(format!("note-{note:03}.md")), body).unwrap();
        }
    }
    println!("generated 5,000 notes in {:?}", started.elapsed());

    let svc = NotesService::new();
    svc.set_root(root).unwrap();

    let scan_started = Instant::now();
    let count = svc.rescan().unwrap();
    let scan_time = scan_started.elapsed();
    println!("scanned {count} notes in {scan_time:?}");
    assert_eq!(count, 5000);

    let list_started = Instant::now();
    let all = svc.list(None).unwrap();
    println!("listed {} notes in {:?}", all.len(), list_started.elapsed());

    let tree_started = Instant::now();
    let tree = svc.folder_tree().unwrap();
    println!(
        "built a {}-folder tree in {:?}",
        tree.children.len(),
        tree_started.elapsed()
    );

    let open_started = Instant::now();
    let opened = svc.read(&NoteId::new("folder-25/note-050.md")).unwrap();
    println!("opened one note in {:?}", open_started.elapsed());
    assert!(opened.body.contains("GPU nodes"));

    assert!(
        scan_time.as_millis() < 1500,
        "scan took {scan_time:?}, budget is 1.5s"
    );
}

// --- The watcher must not react to our own saves (spec 06, R2.6) ----------

#[test]
fn saving_a_note_claims_its_path_so_the_watcher_ignores_it() {
    // Without this the save's own filesystem event reloads the note being
    // edited: the editor flickers and the cursor jumps mid-sentence.
    use mushroom_lib::notes::{selfwrites, watcher};

    let (service, dir) = service();
    let root = dir.path().join("notes");

    let meta = service.create("", "Watcher Self Write").unwrap();
    let opened = service.read(&meta.id).unwrap();
    service
        .save(&meta.id, "changed by us", Some(opened.disk_modified))
        .unwrap();

    let path = root.join(meta.id.as_str());
    assert!(
        selfwrites::global().is_ours(&path),
        "the save did not claim {}",
        path.display()
    );

    // And the claim is specific: another note is still worth reacting to.
    let other = root.join("someone-else-wrote-this.md");
    assert!(!selfwrites::global().is_ours(&other));

    // The temp file the save went through must never look like a note.
    assert!(!watcher::is_watchable(
        &root,
        &path.with_extension("md.tmp")
    ));
    assert!(watcher::is_watchable(&root, &path));
}
