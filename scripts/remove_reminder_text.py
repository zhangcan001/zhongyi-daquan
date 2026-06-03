#!/usr/bin/env python3
"""Remove fixed reminder copy from seed files and the local database."""

from __future__ import annotations

import datetime as dt
import json
import os
import pathlib
import re
import shutil
import sqlite3
from typing import Any


REMINDER_REPLACEMENTS = [
    ("传统作用学习参考：", "传统作用："),
    ("传统主治范围学习参考：", "传统主治范围："),
    ("学习参考：", ""),
    ("传统针灸学习资料，不作为治疗建议。", ""),
    ("仅供专业学习参考，请勿自行操作。", ""),
    ("需结合专业辨证与定位，勿自行操作。", ""),
    ("不提供可照做的通用下针深度；具体深度须由合格专业人员判断。", ""),
    ("针刺深度、方向和角度没有可脱离个体情况的统一数字；须由合格针灸专业人员依据标准定位、体型胖瘦、局部解剖、禁忌与教材规范判断。本库不提供可照做的下针深度。", ""),
    ("仅供中医针灸知识学习与检索参考，不作为诊断、治疗或自行针刺操作依据；定位、配伍和操作须由合格专业人员执行。", ""),
]

REMINDER_PATTERNS = [
    re.compile(r"仅供[^。]*参考[^。]*。"),
    re.compile(r"不作为[^。]*。"),
    re.compile(r"请勿[^。]*。"),
    re.compile(r"勿自行[^。]*。"),
]


def default_db_path() -> pathlib.Path:
    root = pathlib.Path(os.environ["APPDATA"])
    matches = list(root.glob("com.zhongyi.daquan/*/database/zhongyi.db"))
    if not matches:
        raise FileNotFoundError("未找到本地 zhongyi.db")
    return matches[0]


def clean_text(value: str) -> str:
    output = value
    for old, new in REMINDER_REPLACEMENTS:
        output = output.replace(old, new)
    for pattern in REMINDER_PATTERNS:
        output = pattern.sub("", output)
    output = re.sub(r"[ 　]+", " ", output)
    output = re.sub(r"\s+([。；，、])", r"\1", output)
    output = re.sub(r"([。；]){2,}", r"\1", output)
    return output.strip()


def clean_value(value: Any) -> tuple[Any, int]:
    if isinstance(value, str):
        cleaned = clean_text(value)
        return cleaned, int(cleaned != value)
    if isinstance(value, list):
        changed = 0
        items = []
        for item in value:
            cleaned, item_changed = clean_value(item)
            changed += item_changed
            items.append(cleaned)
        return items, changed
    if isinstance(value, dict):
        changed = 0
        result = {}
        for key, item in value.items():
            cleaned, item_changed = clean_value(item)
            changed += item_changed
            result[key] = cleaned
        return result, changed
    return value, 0


def clean_json_file(path: pathlib.Path) -> int:
    value = json.loads(path.read_text(encoding="utf-8"))
    cleaned, changed = clean_value(value)
    if changed:
        path.write_text(
            json.dumps(cleaned, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
    return changed


def refresh_search(conn: sqlite3.Connection, row: sqlite3.Row) -> None:
    conn.execute("DELETE FROM knowledge_fts WHERE rowid = ?", (row["id"],))
    conn.execute(
        """
        INSERT INTO knowledge_fts(rowid, name, code, alias, pinyin, category, summary, content, tags)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        """,
        (
            row["id"],
            row["name"],
            row["code"],
            row["alias"],
            row["pinyin"],
            row["category"],
            row["summary"],
            row["content"],
            row["tags"],
        ),
    )
    conn.execute(
        """
        UPDATE knowledge_list_view_cache
        SET summary = ?2, tags = ?3, updated_at = datetime('now')
        WHERE item_id = ?1
        """,
        (row["id"], row["summary"], row["tags"]),
    )


def clean_database(db_path: pathlib.Path) -> dict[str, int | str]:
    backup = db_path.with_suffix(f".reminder-backup-{dt.datetime.now().strftime('%Y%m%d%H%M%S')}.db")
    shutil.copy2(db_path, backup)
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    stats: dict[str, int | str] = {
        "knowledge_rows": 0,
        "acupoint_detail_rows": 0,
        "backup": str(backup),
    }
    with conn:
        rows = conn.execute(
            """
            SELECT id, name, code, alias, pinyin, category, summary, content, tags, detail
            FROM knowledge_items
            WHERE type IN ('acupoint', 'acupuncture', 'formula', 'herb', 'meridian')
            """
        ).fetchall()
        for row in rows:
            changed = 0
            summary, changed_summary = clean_value(row["summary"])
            content, changed_content = clean_value(row["content"])
            tags, changed_tags = clean_value(row["tags"])
            detail_obj = json.loads(row["detail"] or "{}")
            detail_obj, changed_detail = clean_value(detail_obj)
            changed += changed_summary + changed_content + changed_tags + changed_detail
            if not changed:
                continue
            conn.execute(
                """
                UPDATE knowledge_items
                SET summary = ?2, content = ?3, tags = ?4, detail = ?5, updated_at = datetime('now')
                WHERE id = ?1
                """,
                (
                    row["id"],
                    summary,
                    content,
                    tags,
                    json.dumps(detail_obj, ensure_ascii=False, separators=(",", ":")),
                ),
            )
            updated = dict(row)
            updated["summary"] = summary
            updated["content"] = content
            updated["tags"] = tags
            refresh_search(conn, updated)
            stats["knowledge_rows"] = int(stats["knowledge_rows"]) + 1

        detail_rows = conn.execute(
            """
            SELECT item_id, functions, indications, needling_summary, moxibustion_summary,
                   massage_summary, contraindications, precautions
            FROM acupoint_details
            """
        ).fetchall()
        for row in detail_rows:
            updates = {}
            for key in row.keys():
                if key == "item_id":
                    continue
                cleaned, changed = clean_value(row[key])
                if changed:
                    updates[key] = cleaned or None
            if not updates:
                continue
            assignments = ", ".join(f"{key} = :{key}" for key in updates)
            updates["item_id"] = row["item_id"]
            conn.execute(f"UPDATE acupoint_details SET {assignments} WHERE item_id = :item_id", updates)
            stats["acupoint_detail_rows"] = int(stats["acupoint_detail_rows"]) + 1
    conn.close()
    return stats


def main() -> None:
    seed_changes = 0
    for path in [
        pathlib.Path("data-seed/acupoints.full.json"),
        pathlib.Path("data-seed/acupoints.sample.json"),
        pathlib.Path("data-seed/meridians.full.json"),
    ]:
        if path.exists():
            seed_changes += clean_json_file(path)
    stats = clean_database(default_db_path())
    stats["seed_changes"] = seed_changes
    print(json.dumps(stats, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
