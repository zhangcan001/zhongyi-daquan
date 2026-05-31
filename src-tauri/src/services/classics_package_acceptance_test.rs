use crate::db::connection::Database;
use crate::models::data_pipeline::CreateImportRequest;
use crate::models::search::SearchRequest;
use crate::services::{import_project_service, search_index_service};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const PACKAGE_ROOT: &str = r"C:\Users\ADMIN\Desktop\gpt下载文件\zhongyi_classics_curated_v0_2\zhongyi_classics_curated_v0_2";
const MANIFEST_PACKAGE: &str = r"C:\Users\ADMIN\Documents\zhongyi-daquan\release-assets\zhongyi_classics_curated_v0_3_manifest.zip";

#[test]
fn real_classics_files_are_detected_and_manifest_package_imports() {
    let root = Path::new(PACKAGE_ROOT);
    if !root.exists() || !Path::new(MANIFEST_PACKAGE).exists() {
        eprintln!("skip real classics acceptance test, package files are not present");
        return;
    }

    let knowledge = fs::read_to_string(root.join("json/knowledge_items_import_curated.json"))
        .expect("read knowledge items");
    let classic_passages = fs::read_to_string(root.join("json/classic_passages_curated.json"))
        .expect("read classic passages");
    let search_terms =
        fs::read_to_string(root.join("json/search_terms_curated.json")).expect("read search terms");
    let zip_bytes = fs::read(MANIFEST_PACKAGE).expect("read manifest package");

    let knowledge_preview =
        import_project_service::preview_json(&knowledge).expect("preview knowledge");
    assert_eq!(
        knowledge_preview.detection.detected_type,
        "knowledge_items_v1"
    );
    assert!(knowledge_preview.detection.confidence >= 0.85);
    assert!(knowledge_preview.direct_import_ready);
    assert_eq!(knowledge_preview.detection.record_count, 462);

    let passages_preview =
        import_project_service::preview_json(&classic_passages).expect("preview passages");
    assert_eq!(
        passages_preview.detection.detected_type,
        "classic_passages_v1"
    );
    assert!(passages_preview.detection.confidence >= 0.85);
    assert!(passages_preview.direct_import_ready);

    let search_terms_preview =
        import_project_service::preview_json(&search_terms).expect("preview search terms");
    assert_eq!(
        search_terms_preview.detection.detected_type,
        "search_terms_v1"
    );
    assert!(search_terms_preview.detection.confidence >= 0.85);

    let zip_preview = import_project_service::preview_zip(
        "zhongyi_classics_curated_v0_3_manifest.zip",
        &zip_bytes,
    )
    .expect("preview manifest zip");
    assert_eq!(zip_preview.detection.detected_type, "classics_curated_v1");
    assert!(zip_preview.direct_import_ready);
    assert!(zip_preview
        .warnings
        .iter()
        .any(|item| item.contains("数据包:")));
    assert!(zip_preview
        .warnings
        .iter()
        .any(|item| item.contains("knowledge_items_import_curated.json")));

    let (data_dir, database) = temp_database("real-classics-manifest");
    let summary = import_project_service::import_zip(
        &database,
        CreateImportRequest {
            file_name: "zhongyi_classics_curated_v0_3_manifest.zip".to_string(),
            target_type: "mixed".to_string(),
            content: String::new(),
            mapping: None,
            template_id: None,
        },
        &zip_bytes,
    )
    .expect("import manifest package");
    assert_eq!(summary.error_rows, 0);
    assert!(summary.total_rows >= 462);

    import_project_service::confirm_import(&database, summary.batch.id.unwrap())
        .expect("confirm manifest package");

    database
        .with_connection(|connection| {
            let content: String = connection.query_row(
                "SELECT content FROM knowledge_items WHERE content LIKE '%桂枝汤%' LIMIT 1",
                [],
                |row| row.get(0),
            )?;
            assert!(content.contains("桂枝"));

            let source_note: String = connection.query_row(
                "SELECT source_note FROM knowledge_items WHERE content LIKE '%桂枝汤%' LIMIT 1",
                [],
                |row| row.get(0),
            )?;
            assert!(!source_note.trim().is_empty());

            let tags: String = connection.query_row(
                "SELECT tags FROM knowledge_items WHERE content LIKE '%桂枝汤%' LIMIT 1",
                [],
                |row| row.get(0),
            )?;
            assert!(!tags.trim().is_empty());
            Ok(())
        })
        .expect("inspect imported knowledge item");

    for query in [
        "桂枝汤",
        "太阳病",
        "上古天真论",
        "神农本草经",
        "黄帝内经",
        "金匮要略",
    ] {
        let response = search_index_service::search(
            &database,
            SearchRequest {
                query: query.to_string(),
                item_type: None,
                page: Some(1),
                page_size: Some(10),
            },
        )
        .expect("search imported package");
        assert!(!response.results.is_empty(), "expected hit for {query}");
    }

    let _ = fs::remove_dir_all(data_dir);
}

fn temp_database(test_name: &str) -> (std::path::PathBuf, Database) {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let data_dir = std::env::temp_dir().join(format!("zhongyi-{test_name}-{unique}"));
    let database = Database::initialize(&data_dir).expect("initialize database");
    (data_dir, database)
}
