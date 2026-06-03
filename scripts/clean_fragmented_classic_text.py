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
PDF_BACKUP_MARKER = "【原PDF对应页完整文本校对备份】"
PDF_LAYOUT_NOISE_PATTERNS = [
    re.compile(r"^倪海厦注(?:《?(?:金匮|伤寒论|神农本草经)》?)?$"),
    re.compile(r"^倪注(?:金匮|伤寒论|神农本草经)?$"),
    re.compile(r"^(?:勤求古訓|勤求古训)\s*(?:博采眾方|博采众方)?$"),
    re.compile(r"^(?:博采眾方|博采众方)$"),
    re.compile(r"^小桂枝[·.．、-]?群龙无首.*(?:校排)?$"),
    re.compile(r"^[\d.。．]+校排$"),
    re.compile(r"^校排$"),
    re.compile(r"^【PDF页码\d+】$"),
]


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
    pdf_noise_lines = sum(1 for line in lines if is_pdf_layout_noise(line))
    duplicate_short_lines = sum(
        1
        for current, nxt in zip(lines, lines[1:])
        if len(current) <= 6 and (current == nxt or compact(nxt).startswith(compact(current)))
    )
    has_pdf_backup = int(PDF_BACKUP_MARKER in text)
    ratio = short / len(lines) if lines else 0
    return {
        "line_count": len(lines),
        "short_line_count": short,
        "short_line_ratio": round(ratio, 3),
        "header_singletons": header_singletons,
        "inline_header": inline_header,
        "pdf_noise_lines": pdf_noise_lines,
        "duplicate_short_lines": duplicate_short_lines,
        "has_pdf_backup": has_pdf_backup,
        "is_suspicious": bool((len(lines) >= 18 and ratio >= 0.28) or header_singletons >= 4 or inline_header),
        "has_layout_noise": bool(pdf_noise_lines or duplicate_short_lines >= 2 or has_pdf_backup),
    }


def normalize_line(line: str) -> str:
    text = line.strip()
    text = text.replace("「协热利」", "协热利")
    text = re.sub(r"[ 　]+", "", text)
    text = re.sub(r"([,，。；;:：]){2,}", r"\1", text)
    return text.strip()


def is_pdf_layout_noise(line: str) -> bool:
    text = re.sub(r"\s+", "", line.strip())
    if not text:
        return False
    if text in {"呚"}:
        return True
    if "勤求古訓博采眾方" in text or "勤求古训博采众方" in text:
        return True
    return any(pattern.match(text) for pattern in PDF_LAYOUT_NOISE_PATTERNS)


def clean_pdf_layout_text(text: str, item_name: str) -> tuple[str, str]:
    current, marker, backup = text.partition(PDF_BACKUP_MARKER)
    pdf_backup = f"{marker}{backup}".strip() if marker else ""
    source_lines = [line.strip() for line in current.splitlines()]
    item_key = compact(item_name)
    output: list[str] = []
    prior_compact_lines: set[str] = set()

    for index, line in enumerate(source_lines):
        if not line:
            if output and output[-1]:
                output.append("")
            continue
        if is_pdf_layout_noise(line):
            continue
        line = re.sub(r"[ 　]{2,}", " ", line).strip()
        line_key = compact(line)
        next_line = next((candidate.strip() for candidate in source_lines[index + 1 :] if candidate.strip()), "")
        next_key = compact(next_line)
        previous_key = compact(output[-1]) if output else ""

        if line_key == previous_key:
            continue
        if item_key and index < 12 and line_key == item_key and any(item_key == compact(part) or compact(part).startswith(item_key) for part in output):
            continue
        if len(line_key) <= 6 and next_key.startswith(line_key) and len(next_key) > len(line_key):
            continue
        if len(line_key) <= 6 and line_key in prior_compact_lines and next_key != line_key:
            continue

        if output and len(output[-1]) == 1 and re.match(r"^[\u4e00-\u9fff]", line) and not re.match(r"^[，,。；;：:、）」』】]", line):
            output[-1] = output[-1] + line
        else:
            output.append(line)
        prior_compact_lines.add(line_key)

    cleaned = "\n".join(output)
    cleaned = re.sub(r"\n{3,}", "\n\n", cleaned)
    cleaned = re.sub(r"([。！？；;])\n([，,。！？；;])", r"\1\2", cleaned)
    return cleaned.strip(), pdf_backup


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
        "layout_noise": 0,
        "pdf_backup_removed": 0,
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
            content = row["content"] or ""
            before = fragmentation_metrics(content)
            detail = parse_json(row["detail"])
            should_clean = is_classic_row(detail) or bool(before["has_pdf_backup"] or before["pdf_noise_lines"])
            if not should_clean:
                continue
            stats["scanned"] += 1
            cleaned, pdf_backup = clean_pdf_layout_text(content, row["name"] or "")
            if before["has_layout_noise"]:
                stats["layout_noise"] += 1
            if pdf_backup:
                stats["pdf_backup_removed"] += 1

            post_layout = fragmentation_metrics(cleaned)
            if not post_layout["is_suspicious"] and cleaned == content:
                continue
            if post_layout["is_suspicious"]:
                stats["suspicious"] += 1
            original_clause = str(detail.get("original_clause") or "").strip()
            if post_layout["is_suspicious"]:
                commentary = clean_commentary_text(cleaned, original_clause)
                cleaned = structured_content(original_clause, commentary, cleaned.strip())
            after = fragmentation_metrics(cleaned)
            if cleaned and cleaned != content:
                detail.setdefault("rawFragmentedBackup", content)
                if pdf_backup:
                    detail.setdefault("pdfBackup", pdf_backup)
                detail["cleanup_method"] = "bulk_classic_fragmented_text_cleanup"
                detail["manual_review_required"] = after["is_suspicious"]
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
