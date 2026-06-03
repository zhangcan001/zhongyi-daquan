#!/usr/bin/env python3
"""Backfill structured herb properties from existing local knowledge text.

The extraction is intentionally conservative: it only uses text already present
in the app database and leaves uncertain fields empty for later review.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import pathlib
import re
import shutil
import sqlite3
from typing import Any


HERB_COLUMNS = {
    "four_qi": "TEXT",
    "five_flavors": "TEXT",
    "channel_tropism": "TEXT",
    "toxicity": "TEXT",
    "origin": "TEXT",
    "processing": "TEXT",
    "classic_applications": "TEXT",
    "property_notes": "TEXT",
}

FOUR_QI_TERMS = ["大寒", "微寒", "寒", "凉", "平", "微温", "温", "热", "大热"]
FLAVOR_TERMS = ["酸", "苦", "甘", "辛", "咸", "淡", "涩"]
TOXICITY_TERMS = ["无毒", "小毒", "微毒", "有毒", "大毒"]
ORGANS = ["心包", "三焦", "大肠", "小肠", "膀胱", "心", "肝", "脾", "肺", "肾", "胃", "胆"]

LABELS = [
    "性味",
    "本经原文",
    "产地",
    "主治",
    "仲景",
    "用量",
    "禁忌",
    "炮制",
    "别录",
    "甄权",
    "大明",
    "灵胎",
    "括要",
    "元素",
]


def default_db_path() -> pathlib.Path:
    root = pathlib.Path(os.environ["APPDATA"])
    matches = list(root.glob("com.zhongyi.daquan/*/database/zhongyi.db"))
    if not matches:
        raise FileNotFoundError("未找到本地 zhongyi.db")
    return matches[0]


def clean_text(value: Any) -> str:
    if value is None:
        return ""
    if isinstance(value, list):
        return "，".join(clean_text(item) for item in value if clean_text(item))
    if isinstance(value, dict):
        return "；".join(f"{key}：{clean_text(val)}" for key, val in value.items() if clean_text(val))
    return re.sub(r"\s+", " ", str(value)).strip(" ，,;；\n\t")


def unique_join(values: list[str]) -> str:
    seen: set[str] = set()
    result: list[str] = []
    for value in values:
        for part in re.split(r"[，,、;；|/]+", clean_text(value)):
            text = part.strip()
            if text and text not in seen:
                seen.add(text)
                result.append(text)
    return "，".join(result)


def parse_json(text: str | None) -> dict[str, Any]:
    if not text:
        return {}
    try:
        value = json.loads(text)
    except json.JSONDecodeError:
        return {}
    return value if isinstance(value, dict) else {}


def value_from_detail(detail: dict[str, Any], *keys: str) -> str:
    for key in keys:
        value = clean_text(detail.get(key))
        if value:
            return value
    return ""


def labeled_section(text: str, label: str) -> str:
    pattern = rf"【{re.escape(label)}】(.+?)(?=【({'|'.join(map(re.escape, LABELS))})】|$)"
    match = re.search(pattern, text, flags=re.S)
    return clean_text(match.group(1)) if match else ""


def extract_nature_flavor(detail: dict[str, Any], content: str) -> tuple[str, str]:
    section = labeled_section(content, "性味")
    if section:
        return narrow_nature_phrase(section), "content.【性味】"

    bencao = value_from_detail(detail, "bencao_original", "bencaoOriginal")
    candidates = [(bencao, "detail.bencao_original"), (content, "text.rule")]
    for text in candidates:
        source_text, source = text
        if not source_text:
            continue
        match = re.search(r"性味?([^。；;\n]{1,40})", source_text)
        if match:
            return narrow_nature_phrase(match.group(0)), source
        match = re.search(r"味([^。；;\n]{1,32})", source_text)
        if match:
            return narrow_nature_phrase(match.group(0)), source

    existing = value_from_detail(detail, "nature_flavor", "natureFlavor", "性味")
    if existing:
        return narrow_nature_phrase(existing), "detail.nature_flavor"
    return "", ""


def narrow_nature_phrase(text: str) -> str:
    value = clean_text(text)
    value = re.split(r"(?:主治|主|治|功能|功效)", value, maxsplit=1)[0]
    return value.strip(" ，,。；;")


def extract_terms(text: str, terms: list[str]) -> str:
    spans: list[tuple[int, int, str]] = []
    for term in sorted(terms, key=len, reverse=True):
        for match in re.finditer(re.escape(term), text):
            start, end = match.span()
            if any(start >= old_start and end <= old_end for old_start, old_end, _ in spans):
                continue
            spans.append((start, end, term))
    values = [term for _, _, term in sorted(spans)]
    return unique_join(values)


def extract_toxicity(detail: dict[str, Any], nature_flavor: str, content: str) -> str:
    existing = value_from_detail(detail, "toxicity", "毒性")
    if existing:
        return existing
    toxicity = extract_terms(nature_flavor, TOXICITY_TERMS)
    if toxicity:
        return toxicity
    # Scan only nearby toxicity wording to avoid tagging every "解毒" as toxic.
    nearby = " ".join(re.findall(r"[^。；;\n]{0,12}(?:无毒|有毒|小毒|微毒|大毒)[^。；;\n]{0,12}", content))
    return extract_terms(nearby, TOXICITY_TERMS)


def extract_channels(detail: dict[str, Any], content: str) -> tuple[str, str]:
    existing = unique_join(
        [
            value_from_detail(detail, "channel_tropism", "channelTropism", "归经脏腑"),
            value_from_detail(detail, "meridians", "归经"),
        ]
    )
    if existing:
        return normalize_channels(existing), "detail.meridians"

    snippets = []
    patterns = [
        r"归([^。；;\n]{1,24}?)(?:经|脏|腑)",
        r"入([^。；;\n]{1,16}?)(?:经|脏|腑)",
        r"入(心包|三焦|大肠|小肠|膀胱|心|肝|脾|肺|肾|胃|胆)",
    ]
    for pattern in patterns:
        snippets.extend(match.group(1) for match in re.finditer(pattern, content))
    return normalize_channels("，".join(snippets)), "text.rule" if snippets else ""


def normalize_channels(text: str) -> str:
    values: list[str] = []
    for organ in ORGANS:
        if organ in text:
            values.append(f"{organ}经")
    return unique_join(values)


def current_item_content(text: str) -> str:
    value = text.split("【原PDF对应页完整文本校对备份】", 1)[0]
    return clean_text(value)


def split_current_and_backup(text: str) -> tuple[str, str]:
    current, separator, backup = text.partition("【原PDF对应页完整文本校对备份】")
    return clean_text(current), clean_text(f"{separator}{backup}") if separator else ""


def extract_summary(content: str, fallback: str | None) -> str:
    match = re.search(r"——([^。；;\n]+)", content)
    if match:
        return clean_text(match.group(1))
    return clean_text(fallback)


def extract_effects(detail: dict[str, Any], content: str, nature_flavor: str) -> str:
    existing = value_from_detail(detail, "effects", "功效")
    if existing:
        return existing
    section = labeled_section(content, "功效")
    if section:
        return section
    match = re.search(r"功能([^。；;\n]+)", labeled_section(content, "性味") or nature_flavor)
    if match:
        return clean_text(match.group(1))
    return ""


def detail_with_updates(detail: dict[str, Any], updates: dict[str, str]) -> str:
    for key, value in updates.items():
        if value:
            detail[key] = value
    return json.dumps(detail, ensure_ascii=False, separators=(",", ":"))


def ensure_schema(conn: sqlite3.Connection) -> None:
    existing = {row[1] for row in conn.execute("PRAGMA table_info(herb_details)")}
    for column, column_type in HERB_COLUMNS.items():
        if column not in existing:
            conn.execute(f"ALTER TABLE herb_details ADD COLUMN {column} {column_type}")
    conn.execute("CREATE INDEX IF NOT EXISTS idx_herb_details_four_qi ON herb_details(four_qi)")
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_herb_details_channel_tropism ON herb_details(channel_tropism)"
    )
    conn.execute(
        """
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL
        )
        """
    )
    conn.execute(
        """
        INSERT OR IGNORE INTO schema_migrations (version, name, applied_at)
        VALUES (12, 'herb_structured_properties', datetime('now'))
        """
    )
    conn.execute(
        """
        INSERT OR IGNORE INTO schema_migrations (version, name, applied_at)
        VALUES (13, 'herb_classic_sections', datetime('now'))
        """
    )


def append_tags(tags: str | None, four_qi: str, five_flavors: str, channels: str, toxicity: str) -> str:
    additions: list[str] = []
    additions.extend(f"{value}性" for value in four_qi.split("，") if value)
    additions.extend(f"{value}味" for value in five_flavors.split("，") if value)
    additions.extend(f"归{value}" for value in channels.split("，") if value)
    additions.extend(value for value in toxicity.split("，") if value)
    return unique_join([clean_text(tags), *additions])


def refresh_search(
    conn: sqlite3.Connection, item_id: int, row: sqlite3.Row, tags: str, content: str, summary: str
) -> None:
    conn.execute("DELETE FROM knowledge_fts WHERE rowid = ?", (item_id,))
    conn.execute(
        """
        INSERT INTO knowledge_fts(rowid, name, code, alias, pinyin, category, summary, content, tags)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        """,
        (
            item_id,
            row["name"],
            row["code"],
            row["alias"],
            row["pinyin"],
            row["category"],
            summary,
            content,
            tags,
        ),
    )
    conn.execute(
        """
        INSERT INTO knowledge_list_view_cache
          (item_id, type, code, name, pinyin, category, summary, tags, data_status, is_favorite, relation_count, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, COALESCE((SELECT count FROM relation_count_cache WHERE item_id = ?1), 0), datetime('now'))
        ON CONFLICT(item_id) DO UPDATE SET
          summary = excluded.summary,
          tags = excluded.tags,
          data_status = excluded.data_status,
          is_favorite = excluded.is_favorite,
          updated_at = excluded.updated_at
        """,
        (
            item_id,
            row["type"],
            row["code"],
            row["name"],
            row["pinyin"],
            row["category"],
            summary,
            tags,
            row["data_status"],
            row["is_favorite"],
        ),
    )


def load_supplemental_herbs() -> list[dict[str, Any]]:
    path = pathlib.Path("data-seed/herbs.supplemental.json")
    if not path.exists():
        return []
    return json.loads(path.read_text(encoding="utf-8"))


def upsert_supplemental_herbs(conn: sqlite3.Connection) -> int:
    count = 0
    for herb in load_supplemental_herbs():
        name = clean_text(herb.get("name"))
        if not name:
            continue
        detail = herb.get("detail") if isinstance(herb.get("detail"), dict) else {}
        content = clean_text(herb.get("content"))
        summary = clean_text(herb.get("summary"))
        tags = clean_text(herb.get("tags"))
        existing = conn.execute(
            "SELECT id FROM knowledge_items WHERE type = 'herb' AND name = ?1",
            (name,),
        ).fetchone()
        if existing:
            item_id = int(existing[0])
            conn.execute(
                """
                UPDATE knowledge_items
                SET alias = COALESCE(NULLIF(?2, ''), alias),
                    category = COALESCE(NULLIF(?3, ''), category),
                    summary = ?4,
                    content = ?5,
                    source_note = COALESCE(NULLIF(?6, ''), source_note),
                    tags = COALESCE(NULLIF(?7, ''), tags),
                    detail = ?8,
                    data_status = 'reviewed',
                    completeness_status = 'complete',
                    updated_at = datetime('now')
                WHERE id = ?1
                """,
                (
                    item_id,
                    clean_text(herb.get("alias")),
                    clean_text(herb.get("category")),
                    summary,
                    content,
                    clean_text(herb.get("source_note")),
                    tags,
                    json.dumps(detail, ensure_ascii=False, separators=(",", ":")),
                ),
            )
        else:
            cursor = conn.execute(
                """
                INSERT INTO knowledge_items
                  (type, code, name, alias, pinyin, category, summary, content, source_note, tags,
                   data_status, completeness_status, content_version, is_favorite, detail, import_batch_id,
                   source_package, created_at, updated_at)
                VALUES
                  ('herb', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                   'reviewed', 'complete', 1, 0, ?10, NULL,
                   'supplemental-herbs', datetime('now'), datetime('now'))
                """,
                (
                    clean_text(herb.get("code")) or f"HERB-SUP-{name}",
                    name,
                    clean_text(herb.get("alias")),
                    clean_text(herb.get("pinyin")),
                    clean_text(herb.get("category")),
                    summary,
                    content,
                    clean_text(herb.get("source_note")),
                    tags,
                    json.dumps(detail, ensure_ascii=False, separators=(",", ":")),
                ),
            )
            item_id = int(cursor.lastrowid)
        count += 1
    return count


def enrich(db_path: pathlib.Path) -> dict[str, int]:
    backup = db_path.with_suffix(f".herb-backup-{dt.datetime.now().strftime('%Y%m%d%H%M%S')}.db")
    shutil.copy2(db_path, backup)

    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    stats = {
        "herbs": 0,
        "detail_rows": 0,
        "four_qi": 0,
        "five_flavors": 0,
        "channel_tropism": 0,
        "toxicity": 0,
        "origin": 0,
        "classic_applications": 0,
        "content_cleaned": 0,
        "supplemental_upserted": 0,
    }
    with conn:
        ensure_schema(conn)
        stats["supplemental_upserted"] = upsert_supplemental_herbs(conn)
        rows = conn.execute(
            """
            SELECT id, type, code, name, alias, pinyin, category, summary, content, tags,
                   data_status, is_favorite, detail
            FROM knowledge_items
            WHERE type = 'herb'
            ORDER BY id
            """
        ).fetchall()
        for row in rows:
            stats["herbs"] += 1
            item_id = int(row["id"])
            detail = parse_json(row["detail"])
            original_content = clean_text(row["content"])
            content, pdf_backup = split_current_and_backup(original_content)
            if content != original_content:
                stats["content_cleaned"] += 1
                detail.setdefault("pdfBackup", pdf_backup)
            summary = extract_summary(content, row["summary"])

            nature_flavor, nature_source = extract_nature_flavor(detail, content)
            four_qi = value_from_detail(detail, "four_qi", "fourQi") or extract_terms(
                nature_flavor, FOUR_QI_TERMS
            )
            five_flavors = value_from_detail(detail, "five_flavors", "fiveFlavors") or extract_terms(
                nature_flavor, FLAVOR_TERMS
            )
            channel_tropism, channel_source = extract_channels(detail, content)
            meridians = value_from_detail(detail, "meridians", "归经") or channel_tropism
            toxicity = extract_toxicity(detail, nature_flavor, content)
            origin = value_from_detail(detail, "origin", "产地") or labeled_section(content, "产地")
            effects = extract_effects(detail, content, nature_flavor)
            indications = value_from_detail(detail, "indications", "主治") or labeled_section(content, "主治")
            dosage = value_from_detail(detail, "dosage", "用量") or labeled_section(content, "用量")
            contraindications = value_from_detail(detail, "contraindications", "禁忌") or labeled_section(
                content, "禁忌"
            )
            compatibility = value_from_detail(detail, "compatibility", "配伍")
            processing = value_from_detail(detail, "processing", "炮制") or labeled_section(content, "炮制")
            classic_applications = value_from_detail(
                detail, "classic_applications", "classicApplications", "仲景"
            ) or labeled_section(content, "仲景")
            notes = value_from_detail(detail, "notes", "other_notes", "otherNotes", "ni_note", "niNote")

            source_parts = [part for part in [nature_source, channel_source] if part]
            property_notes = value_from_detail(detail, "property_notes", "propertyNotes")
            if not property_notes:
                property_notes = "由既有条目文本规则抽取，需人工校对。"
                if source_parts:
                    property_notes += f" 来源：{unique_join(source_parts)}。"

            detail_json = detail_with_updates(
                detail,
                {
                    "nature_flavor": nature_flavor,
                    "fourQi": four_qi,
                    "fiveFlavors": five_flavors,
                    "meridians": meridians,
                    "channelTropism": channel_tropism,
                    "toxicity": toxicity,
                    "origin": origin,
                    "effects": effects,
                    "indications": indications,
                    "dosage": dosage,
                    "contraindications": contraindications,
                    "compatibility": compatibility,
                    "processing": processing,
                    "classicApplications": classic_applications,
                    "notes": notes,
                    "propertyNotes": property_notes,
                },
            )
            tags = append_tags(row["tags"], four_qi, five_flavors, channel_tropism, toxicity)

            conn.execute(
                """
                INSERT INTO herb_details
                  (item_id, nature_flavor, four_qi, five_flavors, meridians, channel_tropism,
                   toxicity, origin, effects, indications, dosage, contraindications, compatibility,
                   processing, classic_applications, notes, property_notes)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
                ON CONFLICT(item_id) DO UPDATE SET
                  nature_flavor = excluded.nature_flavor,
                  four_qi = excluded.four_qi,
                  five_flavors = excluded.five_flavors,
                  meridians = excluded.meridians,
                  channel_tropism = excluded.channel_tropism,
                  toxicity = excluded.toxicity,
                  origin = excluded.origin,
                  effects = excluded.effects,
                  indications = excluded.indications,
                  dosage = excluded.dosage,
                  contraindications = excluded.contraindications,
                  compatibility = excluded.compatibility,
                  processing = excluded.processing,
                  classic_applications = excluded.classic_applications,
                  notes = excluded.notes,
                  property_notes = excluded.property_notes
                """,
                (
                    item_id,
                    nature_flavor or None,
                    four_qi or None,
                    five_flavors or None,
                    meridians or None,
                    channel_tropism or None,
                    toxicity or None,
                    origin or None,
                    effects or None,
                    indications or None,
                    dosage or None,
                    contraindications or None,
                    compatibility or None,
                    processing or None,
                    classic_applications or None,
                    notes or None,
                    property_notes or None,
                ),
            )
            conn.execute(
                """
                UPDATE knowledge_items
                SET detail = ?2, tags = ?3, content = ?4, summary = ?5, updated_at = datetime('now')
                WHERE id = ?1
                """,
                (item_id, detail_json, tags, content, summary),
            )
            refresh_search(conn, item_id, row, tags, content, summary)

            stats["detail_rows"] += 1
            if four_qi:
                stats["four_qi"] += 1
            if five_flavors:
                stats["five_flavors"] += 1
            if channel_tropism:
                stats["channel_tropism"] += 1
            if toxicity:
                stats["toxicity"] += 1
            if origin:
                stats["origin"] += 1
            if classic_applications:
                stats["classic_applications"] += 1

    conn.close()
    stats["backup_created"] = 1
    print(f"Backup: {backup}")
    print(json.dumps(stats, ensure_ascii=False, indent=2))
    return stats


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--db", type=pathlib.Path, default=None)
    args = parser.parse_args()
    enrich(args.db or default_db_path())


if __name__ == "__main__":
    main()
