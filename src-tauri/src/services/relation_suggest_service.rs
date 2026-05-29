use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::relation::{
    AcceptRelationSuggestionResponse, GenerateRelationSuggestionsRequest,
    GenerateRelationSuggestionsResponse, ListRelationSuggestionsRequest,
    ListRelationSuggestionsResponse,
};
use crate::repositories::relation_repository::{
    self, RelationSource, RelationSuggestionInput, RelationTarget,
};

pub fn generate_relation_suggestions(
    database: &Database,
    request: GenerateRelationSuggestionsRequest,
) -> AppResult<GenerateRelationSuggestionsResponse> {
    let sources = relation_repository::load_sources(
        database,
        request.item_type.as_deref(),
        request.source_item_id,
    )?;
    let herbs = relation_repository::load_targets(database, "herb")?;
    let meridians = relation_repository::load_targets(database, "meridian")?;
    let acupoints = relation_repository::load_targets(database, "acupoint")?;
    let formulas = relation_repository::load_targets(database, "formula")?;
    let syndromes = relation_repository::load_targets(database, "syndrome")?;

    let mut suggestions = Vec::new();
    for source in &sources {
        match source.item_type.as_str() {
            "formula" => suggest_formula_herbs(source, &herbs, &mut suggestions),
            "acupoint" => suggest_acupoint_meridian(source, &meridians, &mut suggestions),
            "disease" => {
                suggest_mentions(
                    source,
                    &acupoints,
                    "mentions_acupoint",
                    0.72,
                    &mut suggestions,
                );
                suggest_mentions(
                    source,
                    &formulas,
                    "mentions_formula",
                    0.72,
                    &mut suggestions,
                );
                suggest_mentions(
                    source,
                    &syndromes,
                    "mentions_syndrome",
                    0.72,
                    &mut suggestions,
                );
            }
            _ => {}
        }
    }
    let suggestions_created = relation_repository::insert_suggestions(database, &suggestions)?;
    Ok(GenerateRelationSuggestionsResponse {
        suggestions_created,
    })
}

pub fn list_relation_suggestions(
    database: &Database,
    request: ListRelationSuggestionsRequest,
) -> AppResult<ListRelationSuggestionsResponse> {
    let page = request.page.unwrap_or(1).max(1);
    let page_size = request.page_size.unwrap_or(50).clamp(1, 200);
    let (total, suggestions) = relation_repository::list_suggestions(
        database,
        request.status.as_deref(),
        page,
        page_size,
    )?;
    Ok(ListRelationSuggestionsResponse {
        total,
        page,
        page_size,
        suggestions,
    })
}

pub fn accept_relation_suggestion(
    database: &Database,
    suggestion_id: i64,
) -> AppResult<AcceptRelationSuggestionResponse> {
    let relation_id = relation_repository::accept_suggestion(database, suggestion_id)?;
    Ok(AcceptRelationSuggestionResponse {
        suggestion_id,
        relation_id,
    })
}

pub fn reject_relation_suggestion(database: &Database, suggestion_id: i64) -> AppResult<()> {
    relation_repository::reject_suggestion(database, suggestion_id)
}

fn suggest_formula_herbs(
    source: &RelationSource,
    herbs: &[RelationTarget],
    suggestions: &mut Vec<RelationSuggestionInput>,
) {
    for herb in herbs {
        if source_mentions_target(&source.content, herb) {
            suggestions.push(RelationSuggestionInput {
                source_item_id: source.item_id,
                target_item_id: herb.item_id,
                relation_type: "contains_herb".to_string(),
                confidence: 0.86,
                reason: format!("方剂组成文本命中中药「{}」", herb.name),
            });
        }
    }
}

fn suggest_acupoint_meridian(
    source: &RelationSource,
    meridians: &[RelationTarget],
    suggestions: &mut Vec<RelationSuggestionInput>,
) {
    if let Some(meridian_item_id) = source.meridian_item_id {
        if let Some(meridian) = meridians
            .iter()
            .find(|item| item.item_id == meridian_item_id)
        {
            suggestions.push(RelationSuggestionInput {
                source_item_id: source.item_id,
                target_item_id: meridian.item_id,
                relation_type: "belongs_to_meridian".to_string(),
                confidence: 0.96,
                reason: format!("穴位详情已绑定经络「{}」", meridian.name),
            });
            return;
        }
    }

    let haystack = [source.category.as_deref(), Some(source.content.as_str())]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    for meridian in meridians {
        if source_mentions_target(&haystack, meridian)
            || source
                .code
                .as_deref()
                .zip(meridian.code.as_deref())
                .is_some_and(|(source_code, meridian_code)| {
                    source_code
                        .to_uppercase()
                        .starts_with(&meridian_code.to_uppercase())
                })
        {
            suggestions.push(RelationSuggestionInput {
                source_item_id: source.item_id,
                target_item_id: meridian.item_id,
                relation_type: "belongs_to_meridian".to_string(),
                confidence: 0.84,
                reason: format!("穴位分类、内容或编号命中经络「{}」", meridian.name),
            });
        }
    }
}

fn suggest_mentions(
    source: &RelationSource,
    targets: &[RelationTarget],
    relation_type: &str,
    confidence: f64,
    suggestions: &mut Vec<RelationSuggestionInput>,
) {
    for target in targets {
        if source.item_id != target.item_id && source_mentions_target(&source.content, target) {
            suggestions.push(RelationSuggestionInput {
                source_item_id: source.item_id,
                target_item_id: target.item_id,
                relation_type: relation_type.to_string(),
                confidence,
                reason: format!("病症内容文本命中「{}」", target.name),
            });
        }
    }
}

fn source_mentions_target(haystack: &str, target: &RelationTarget) -> bool {
    let normalized_haystack = normalize_text(haystack);
    if normalized_haystack.is_empty() {
        return false;
    }
    let mut needles = vec![target.name.as_str()];
    if let Some(code) = target.code.as_deref() {
        needles.push(code);
    }
    if let Some(alias) = target.alias.as_deref() {
        for part in alias.split([',', '，', ';', '；', '|', '/', '、', ' ']) {
            let normalized = normalize_text(part);
            if !normalized.is_empty() && normalized_haystack.contains(&normalized) {
                return true;
            }
        }
    }
    needles
        .into_iter()
        .map(normalize_text)
        .any(|needle| !needle.is_empty() && normalized_haystack.contains(&needle))
}

fn normalize_text(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|ch| match ch {
            '\u{3000}' => ' ',
            'Ａ'..='Ｚ' | 'ａ'..='ｚ' | '０'..='９' => {
                char::from_u32(ch as u32 - 0xFEE0).unwrap_or(ch)
            }
            _ => ch,
        })
        .collect::<String>()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::Database;
    use crate::models::relation::{
        ListDuplicateCandidatesRequest, ListRelationSuggestionsRequest,
        MergeDuplicateCandidateRequest, RunDuplicateDetectionRequest,
    };
    use crate::services::dedup_service;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_data_dir(test_name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("zhongyi-thread-e-{test_name}-{unique}"))
    }

    #[test]
    fn duplicate_detection_and_merge_cover_st36_and_huangqi_alias() {
        let data_dir = temp_data_dir("dedup");
        let database = Database::initialize(&data_dir).expect("database initializes");
        seed_duplicate_items(&database);

        let st36_result = dedup_service::run_duplicate_detection(
            &database,
            RunDuplicateDetectionRequest {
                batch_id: None,
                item_type: Some("acupoint".to_string()),
            },
        )
        .expect("detect st36 duplicate");
        assert!(st36_result.fingerprints_upserted >= 2);
        assert!(st36_result.candidates_created >= 1);

        let herb_result = dedup_service::run_duplicate_detection(
            &database,
            RunDuplicateDetectionRequest {
                batch_id: None,
                item_type: Some("herb".to_string()),
            },
        )
        .expect("detect huangqi alias duplicate");
        assert!(herb_result.candidates_created >= 1);

        let candidates = dedup_service::list_duplicate_candidates(
            &database,
            ListDuplicateCandidatesRequest {
                status: Some("pending".to_string()),
                page: Some(1),
                page_size: Some(20),
            },
        )
        .expect("list duplicate candidates");
        assert!(candidates
            .candidates
            .iter()
            .any(|candidate| candidate.match_type == "type_code_exact"));
        assert!(candidates
            .candidates
            .iter()
            .any(|candidate| candidate.match_type == "name_alias_match"));

        let st36_candidate = candidates
            .candidates
            .iter()
            .find(|candidate| candidate.match_type == "type_code_exact")
            .expect("st36 duplicate candidate");
        let merge = dedup_service::merge_duplicate_candidate(
            &database,
            MergeDuplicateCandidateRequest {
                candidate_id: st36_candidate.id,
                strategy: "merge_tags".to_string(),
            },
        )
        .expect("merge duplicate candidate");
        assert_eq!(merge.status, "merged");
        assert!(merge.merge_record_id.is_some());

        database
            .with_connection(|connection| {
                let st36_count: i64 = connection.query_row(
                    "SELECT COUNT(1) FROM knowledge_items WHERE type = 'acupoint' AND upper(code) = 'ST36'",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(st36_count, 1);
                let merge_count: i64 =
                    connection.query_row("SELECT COUNT(1) FROM merge_records", [], |row| {
                        row.get(0)
                    })?;
                assert_eq!(merge_count, 1);
                Ok(())
            })
            .expect("verify merge");

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn relation_suggestions_can_be_accepted_and_rejected() {
        let data_dir = temp_data_dir("relation");
        let database = Database::initialize(&data_dir).expect("database initializes");
        seed_relation_items(&database);

        let response = generate_relation_suggestions(
            &database,
            GenerateRelationSuggestionsRequest {
                item_type: None,
                source_item_id: None,
            },
        )
        .expect("generate relation suggestions");
        assert!(response.suggestions_created >= 5);

        let list = list_relation_suggestions(
            &database,
            ListRelationSuggestionsRequest {
                status: Some("pending".to_string()),
                page: Some(1),
                page_size: Some(50),
            },
        )
        .expect("list relation suggestions");
        assert!(list
            .suggestions
            .iter()
            .any(|suggestion| suggestion.relation_type == "contains_herb"
                && suggestion.source_name.as_deref() == Some("补中益气汤")
                && suggestion.target_name.as_deref() == Some("黄芪")));
        assert!(list
            .suggestions
            .iter()
            .any(
                |suggestion| suggestion.relation_type == "belongs_to_meridian"
                    && suggestion.source_name.as_deref() == Some("足三里")
                    && suggestion.target_name.as_deref() == Some("足阳明胃经")
            ));

        let accept_id = list.suggestions[0].id;
        let accepted =
            accept_relation_suggestion(&database, accept_id).expect("accept relation suggestion");
        assert_eq!(accepted.suggestion_id, accept_id);
        assert!(accepted.relation_id > 0);

        let reject_id = list
            .suggestions
            .iter()
            .find(|suggestion| suggestion.id != accept_id)
            .expect("another suggestion")
            .id;
        reject_relation_suggestion(&database, reject_id).expect("reject relation suggestion");

        database
            .with_connection(|connection| {
                let relation_count: i64 = connection.query_row(
                    "SELECT COUNT(1) FROM knowledge_relations",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(relation_count, 1);
                let rejected_status: String = connection.query_row(
                    "SELECT status FROM relation_suggestions WHERE id = ?1",
                    [reject_id],
                    |row| row.get(0),
                )?;
                assert_eq!(rejected_status, "rejected");
                let cache_count: i64 = connection.query_row(
                    "SELECT COUNT(1) FROM relation_count_cache WHERE count > 0",
                    [],
                    |row| row.get(0),
                )?;
                assert!(cache_count >= 2);
                Ok(())
            })
            .expect("verify relation writes");

        let _ = fs::remove_dir_all(data_dir);
    }

    fn seed_duplicate_items(database: &Database) {
        database
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO knowledge_items
                     (type, code, name, alias, pinyin, category, summary, content, tags,
                      data_status, completeness_status, created_at, updated_at)
                     VALUES
                     ('acupoint', 'ST36', '足三里', NULL, 'zusanli', '足阳明胃经', '', '', '常用穴', 'ready', 'complete', datetime('now'), datetime('now')),
                     ('acupoint', 'st36', '足三里穴', '足三里', 'zu san li', '足阳明胃经', '', '', '胃经', 'ready', 'partial', datetime('now'), datetime('now')),
                     ('herb', 'H0001', '黄芪', NULL, 'huangqi', '补气药', '', '', '补气', 'ready', 'complete', datetime('now'), datetime('now')),
                     ('herb', 'H0002', '黄耆', '黄芪', 'huang qi', '补气药', '', '', '补气药', 'ready', 'partial', datetime('now'), datetime('now'))",
                    [],
                )?;
                Ok(())
            })
            .expect("seed duplicate items");
    }

    fn seed_relation_items(database: &Database) {
        database
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO knowledge_items
                     (type, code, name, alias, pinyin, category, summary, content, tags,
                      data_status, completeness_status, created_at, updated_at)
                     VALUES
                     ('herb', 'H0001', '黄芪', '黄耆', 'huangqi', '补气药', '', '', '补气', 'ready', 'complete', datetime('now'), datetime('now')),
                     ('herb', 'H0003', '人参', NULL, 'renshen', '补气药', '', '', '补气', 'ready', 'complete', datetime('now'), datetime('now')),
                     ('herb', 'H0004', '白术', NULL, 'baizhu', '补气药', '', '', '补气', 'ready', 'complete', datetime('now'), datetime('now')),
                     ('formula', 'F0001', '补中益气汤', NULL, 'buzhongyiqitang', '补益剂', '', '', '方剂', 'ready', 'complete', datetime('now'), datetime('now')),
                     ('meridian', 'ST', '足阳明胃经', '胃经', 'zuyangmingweijing', '十二经脉', '', '', '经络', 'ready', 'complete', datetime('now'), datetime('now')),
                     ('acupoint', 'ST36', '足三里', NULL, 'zusanli', '足阳明胃经', '', '', '常用穴', 'ready', 'complete', datetime('now'), datetime('now')),
                     ('syndrome', 'S0001', '气虚证', NULL, 'qixuzheng', '虚证', '', '', '证型', 'ready', 'complete', datetime('now'), datetime('now')),
                     ('disease', 'D0001', '脾胃虚弱', NULL, 'piweixuruo', '内科', '可见气虚证，常见资料提到足三里与补中益气汤。', '足三里、补中益气汤、气虚证均可作为资料关联线索。', '病症', 'ready', 'complete', datetime('now'), datetime('now'))",
                    [],
                )?;
                let formula_id: i64 = connection.query_row(
                    "SELECT id FROM knowledge_items WHERE name = '补中益气汤'",
                    [],
                    |row| row.get(0),
                )?;
                connection.execute(
                    "INSERT INTO formula_details (item_id, composition)
                     VALUES (?1, '黄芪、人参、白术、甘草、当归、陈皮、升麻、柴胡')",
                    [formula_id],
                )?;
                let meridian_id: i64 = connection.query_row(
                    "SELECT id FROM knowledge_items WHERE name = '足阳明胃经'",
                    [],
                    |row| row.get(0),
                )?;
                let acupoint_id: i64 = connection.query_row(
                    "SELECT id FROM knowledge_items WHERE name = '足三里'",
                    [],
                    |row| row.get(0),
                )?;
                connection.execute(
                    "INSERT INTO acupoint_details (item_id, acupoint_code, meridian_item_id, indications)
                     VALUES (?1, 'ST36', ?2, '胃痛、虚劳')",
                    [acupoint_id, meridian_id],
                )?;
                Ok(())
            })
            .expect("seed relation items");
    }
}
