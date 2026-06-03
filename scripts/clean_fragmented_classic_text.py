#!/usr/bin/env python3
"""Clean fragmented classic-import text and report remaining suspicious rows."""

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


CLASSIC_BOOK_IDS = {"renji_shanghanlun", "renji_jingui_yaolue"}
VERTICAL_HEADER_CHARS = list("觀其脈證知犯何逆隨證治之")
VERTICAL_HEADER_SET = set(VERTICAL_HEADER_CHARS)
REPORT_PATH = pathlib.Path("reports/classic_fragmented_text_report.json")


def default_db_path() -> pathlib.Path:
    root = pathlib.Path(os.environ["APPDATA"])
    matches = list(root.glob("com.zhongyi.daquan/*/database/zhongyi.db"))
    if not matches:
        raise FileNotFoundError("未找到本地 zhongyi.db")
    return matches[0]


def parse_json(text: str | None) -> dict[str, Any]:
    if not text:
        return {}
    try:
        value = json.loads(text)
    except json.JSONDecodeError:
        return {}
    return value if isinstance(value, dict) else {}


def compact(text: str | None) -> str:
    return re.sub(r"\s+", "", text or "")


def is_classic_row(detail: dict[str, Any]) -> bool:
    return detail.get("book_id") in CLASSIC_BOOK_IDS or detail.get("classic_name") in {"伤寒论", "金匮要略"}


def fragmentation_metrics(text: str) -> dict[str, Any]:
    lines = [line.strip() for line in text.splitlines() if line.strip()]
    short = sum(1 for line in lines if len(line) <= 3)
    header_singletons = sum(1 for line in lines if line in VERTICAL_HEADER_SET)
    inline_header = int("觀其脈證知犯" in text or "觀\n其\n脈\n證" in text or "观其脉证知犯" in text)
    ratio = short / len(lines) if lines else 0
    return {
        "line_count": len(lines),
        "short_line_count": short,
        "short_line_ratio": round(ratio, 3),
        "header_singletons": header_singletons,
        "inline_header": inline_header,
        "is_suspicious": bool((len(lines) >= 18 and ratio >= 0.28) or header_singletons >= 4 or inline_header),
    }


def normalize_line(line: str) -> str:
    text = line.strip()
    text = text.replace("「协热利」", "协热利")
    text = re.sub(r"[ 　]+", "", text)
    text = re.sub(r"([,，。；;:：]){2,}", r"\1", text)
    return text.strip()


def skip_header_run(lines: list[str], index: int) -> int:
    if lines[index] not in VERTICAL_HEADER_SET:
        return index
    cursor = index
    matched = ""
    while cursor < len(lines) and lines[cursor] in VERTICAL_HEADER_SET:
        matched += lines[cursor]
        cursor += 1
    if matched and "觀其脈證知犯".startswith(matched[: min(len(matched), 6)]):
        return cursor
    return index


def remove_inline_header(text: str) -> str:
    patterns = [
        r"^觀其脈證知犯何逆隨證治之[一二三四五六七八九十百零〇\d]*[:：,，。]*",
        r"^观其脉证知犯何逆随证治之[一二三四五六七八九十百零〇\d]*[:：,，。]*",
        r"^脈證知犯何逆隨證治之[一二三四五六七八九十百零〇\d]*[:：,，。]*",
    ]
    for pattern in patterns:
        text = re.sub(pattern, "", text)
    return text


def clean_commentary_text(text: str, original_clause: str) -> str:
    raw_lines = [normalize_line(line) for line in text.splitlines()]
    raw_lines = [line for line in raw_lines if line]
    lines: list[str] = []
    index = 0
    while index < len(raw_lines):
        skipped = skip_header_run(raw_lines, index)
        if skipped != index:
            index = skipped
            continue
        line = remove_inline_header(raw_lines[index])
        if line and line not in {"觀", "其", "脈", "證", "知", "犯", "何", "逆", "隨", "治", "之"}:
            lines.append(line)
        index += 1

    text = "".join(lines)
    text = remove_inline_header(text)
    text = text.replace(original_clause, "")
    normalized_original = compact(original_clause)
    if normalized_original:
        text = text.replace(normalized_original, "")
    text = re.sub(r"^[一二三四五六七八九十百零〇\d]+[:：]", "", text)
    text = re.sub(r"[,，。；;:：]{2,}", "。", text)
    text = re.sub(r"([。；])", r"\1\n", text)
    paragraphs = [part.strip(" ，,；;") for part in text.splitlines() if part.strip(" ，,；;")]
    result = "\n".join(paragraphs)
    return result.strip()


def structured_content(original_clause: str, commentary: str, fallback: str) -> str:
    sections = []
    if original_clause:
        sections.append(f"【原文】\n{original_clause}")
    if commentary:
        sections.append(f"【注解整理】\n{commentary}")
    elif fallback:
        sections.append(f"【正文整理】\n{fallback}")
    return "\n\n".join(sections)


def refresh_search(conn: sqlite3.Connection, row: sqlite3.Row, summary: str, content: str, tags: str) -> None:
    item_id = int(row["id"])
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


def append_tag(tags: str | None, value: str) -> str:
    parts = [part.strip() for part in re.split(r"[，,、;；]+", tags or "") if part.strip()]
    if value not in parts:
        parts.append(value)
    return "，".join(parts)


def scan_and_clean(db_path: pathlib.Path, report_path: pathlib.Path) -> dict[str, Any]:
    backup = db_path.with_suffix(f".fragment-cleanup-backup-{dt.datetime.now().strftime('%Y%m%d%H%M%S')}.db")
    shutil.copy2(db_path, backup)
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    stats: dict[str, Any] = {
        "backup": str(backup),
        "scanned": 0,
        "suspicious": 0,
        "updated": 0,
        "remaining_suspicious": 0,
        "updated_ids": [],
        "remaining": [],
    }

    with conn:
        rows = conn.execute(
            """
            SELECT id, type, code, name, alias, pinyin, category, summary, content, tags,
                   data_status, is_favorite, detail
            FROM knowledge_items
            ORDER BY id
            """
        ).fetchall()
        for row in rows:
            detail = parse_json(row["detail"])
            if not is_classic_row(detail):
                continue
            stats["scanned"] += 1
            content = row["content"] or ""
            before = fragmentation_metrics(content)
            if not before["is_suspicious"]:
                continue
            stats["suspicious"] += 1
            original_clause = str(detail.get("original_clause") or "").strip()
            commentary = clean_commentary_text(content, original_clause)
            cleaned = structured_content(original_clause, commentary, content.strip())
            after = fragmentation_metrics(cleaned)
            if cleaned and cleaned != content:
                detail.setdefault("rawFragmentedBackup", content)
                detail["cleanup_method"] = "bulk_classic_fragmented_text_cleanup"
                detail["manual_review_required"] = True
                detail["fragmentation_before"] = before
                detail["fragmentation_after"] = after
                tags = append_tag(row["tags"], "正文已整理")
                summary = row["summary"] or original_clause[:120] or row["name"]
                conn.execute(
                    """
                    UPDATE knowledge_items
                    SET summary = ?2, content = ?3, tags = ?4, detail = ?5, updated_at = datetime('now')
                    WHERE id = ?1
                    """,
                    (
                        row["id"],
                        summary,
                        cleaned,
                        tags,
                        json.dumps(detail, ensure_ascii=False, separators=(",", ":")),
                    ),
                )
                refresh_search(conn, row, summary, cleaned, tags)
                stats["updated"] += 1
                stats["updated_ids"].append(int(row["id"]))
            remaining = fragmentation_metrics(cleaned)
            if remaining["is_suspicious"]:
                stats["remaining_suspicious"] += 1
                stats["remaining"].append(
                    {
                        "id": int(row["id"]),
                        "type": row["type"],
                        "name": row["name"],
                        "book_id": detail.get("book_id"),
                        "clause_no": detail.get("clause_no"),
                        "metrics": remaining,
                    }
                )

    conn.close()
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(stats, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return stats


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--db", type=pathlib.Path, default=None)
    parser.add_argument("--report", type=pathlib.Path, default=REPORT_PATH)
    args = parser.parse_args()
    stats = scan_and_clean(args.db or default_db_path(), args.report)
    print(json.dumps({k: v for k, v in stats.items() if k != "remaining"}, ensure_ascii=False, indent=2))
    print(f"Report: {args.report}")


if __name__ == "__main__":
    main()
