#!/usr/bin/env python3
"""Curate badly fragmented Shanghanlun syndrome OCR records in the local DB."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import pathlib
import shutil
import sqlite3
from typing import Any


CURATED_CLAUSES: dict[int, dict[str, Any]] = {
    76: {
        "name": "太阳病发汗后，大汗出，胃中干，欲饮水与五苓散证",
        "summary": "太阳病发汗太过后，胃中干而欲饮水者宜少少与饮；若脉浮、小便不利、微热消渴，为五苓散证。",
        "original_clause": "七六：太阳病，发汗后，大汗出，胃中干，烦躁不得眠，欲得饮水者，少少与饮之，令胃气和则愈。若脉浮，小便不利，微热消渴者，五苓散主之。",
        "ni_explanation": (
            "此条辨发汗太过后的两种转归。若只是汗后胃中津液不足，出现胃中干、烦躁不得眠、"
            "想饮水，宜少量频饮，使胃气渐和即可。若见脉浮、小便不利、微热、消渴，"
            "则不只是胃燥口渴，而是水气不化、表里水道不利，水停而津不上承，"
            "故用五苓散化气行水，使小便通利、津液回布。"
        ),
        "symptoms": "大汗出；胃中干；烦躁不得眠；欲饮水；小便不利；微热；消渴",
        "pulse": "浮",
        "pattern": "发汗太过后津伤；或水气不化、膀胱气化不利而成五苓散证",
        "treatment_principle": "胃中干者少少与饮以和胃气；小便不利、微热消渴者化气行水。",
        "formula_refs": ["五苓散"],
    },
    153: {
        "name": "太阳病二三日，不能卧，但欲起，心下必结，脉微弱者，本有久寒也",
        "summary": "太阳病误下后，心下结而脉微弱，多属本有久寒；若误利不止，可成协热利。",
        "original_clause": "一五三：太阳病二三日，不能卧，但欲起，心下必结，脉微弱者，本有久寒也，反下之，若利之，必作结胸；未止者，四日复下之，此作协热利也。",
        "ni_explanation": (
            "此条辨太阳病误下、误利后的转变。太阳病二三日而不能平卧、但欲起坐，"
            "多提示胸膈心下有水饮或寒湿结滞。脉微弱者，说明正气不足且本有久寒，"
            "若反用攻下，寒湿与表邪相结，容易作结胸；若下利未止，又再次攻下，"
            "则邪热与下利相协，可成协热利。"
        ),
        "symptoms": "不能卧，但欲起；心下结；结胸；下利不止；协热利",
        "pulse": "微弱",
        "pattern": "太阳病误下后，久寒、水饮或寒湿内结",
        "treatment_principle": "辨寒湿、水饮与协热利之转归，随证治之。",
        "formula_refs": ["小陷胸汤", "生姜泻心汤"],
    },
    154: {
        "name": "太阳病下之，观其脉证，知犯何逆，随证治之",
        "summary": "太阳病误下后，以脉象辨结胸、咽痛、胁急、心下痛、协热利与下血等转归。",
        "original_clause": "一五四：太阳病，下之，其脉浮，不结胸者，此为欲解也；脉促者，必结胸也；脉细数者，必咽痛；脉弦者，必两胁拘急；脉紧者，头痛未止；脉沉紧者，必心下痛；脉沉滑者，协热利；脉数滑者，必下血。",
        "ni_explanation": (
            "此条总论太阳病误下之后，须观脉证判断病势转归。脉浮而不结胸，"
            "为表邪未陷、病势欲解。脉促，多提示邪陷胸中而成结胸。脉细数，"
            "提示里虚兼热上扰，可见咽痛。脉弦，多与少阳、水饮相关，可见两胁拘急。"
            "脉紧而头痛未止，说明寒束在表，头痛仍在。脉沉紧，病在里且寒，"
            "多见心下痛。脉沉滑，可见协热利。脉数滑，则可见热迫血行而下血。"
        ),
        "symptoms": "结胸；咽痛；两胁拘急；头痛未止；心下痛；协热利；下血",
        "pulse": "浮；促；细数；弦；紧；沉紧；沉滑；数滑",
        "pattern": "太阳病误下后，表邪或陷里，随脉证呈现不同转归",
        "treatment_principle": "观其脉证，知犯何逆，随证治之。",
        "formula_refs": ["小柴胡汤", "麻杏甘石汤", "甘草干姜汤", "葛根汤", "五苓散", "葛芩连汤", "白虎汤", "承气汤"],
    },
}


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


def structured_content(clause: dict[str, Any]) -> str:
    formulas = "；".join(clause["formula_refs"])
    return "\n\n".join(
        [
            f"【原文】\n{clause['original_clause']}",
            f"【倪注整理】\n{clause['ni_explanation']}",
            f"【脉证要点】\n脉象：{clause['pulse']}。\n证候：{clause['symptoms']}。",
            f"【辨证要点】\n{clause['pattern']}",
            f"【治则】\n{clause['treatment_principle']}",
            f"【相关方剂】\n{formulas}。",
        ]
    )


def refresh_search(conn: sqlite3.Connection, row: sqlite3.Row, name: str, summary: str, content: str, tags: str) -> None:
    item_id = int(row["id"])
    conn.execute("DELETE FROM knowledge_fts WHERE rowid = ?", (item_id,))
    conn.execute(
        """
        INSERT INTO knowledge_fts(rowid, name, code, alias, pinyin, category, summary, content, tags)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        """,
        (
            item_id,
            name,
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
          name = excluded.name,
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
            name,
            row["pinyin"],
            row["category"],
            summary,
            tags,
            row["data_status"],
            row["is_favorite"],
        ),
    )


def clause_no(detail: dict[str, Any]) -> int | None:
    value = detail.get("clause_no")
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def curate_database(db_path: pathlib.Path) -> dict[str, Any]:
    backup = db_path.with_suffix(f".classic-cleanup-backup-{dt.datetime.now().strftime('%Y%m%d%H%M%S')}.db")
    shutil.copy2(db_path, backup)
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    stats: dict[str, Any] = {"backup": str(backup), "updated": 0, "ids": []}

    with conn:
        rows = conn.execute(
            """
            SELECT id, type, code, name, alias, pinyin, category, summary, content, tags,
                   data_status, is_favorite, detail
            FROM knowledge_items
            WHERE type = 'syndrome'
            ORDER BY id
            """
        ).fetchall()
        for row in rows:
            detail = parse_json(row["detail"])
            if detail.get("book_id") != "renji_shanghanlun" and detail.get("classic_name") != "伤寒论":
                continue
            number = clause_no(detail)
            if number not in CURATED_CLAUSES:
                continue

            clause = CURATED_CLAUSES[number]
            content = structured_content(clause)
            tags = "，".join(
                dict.fromkeys(
                    part
                    for part in [
                        row["tags"] or "",
                        "伤寒论",
                        "太阳病",
                        "误下",
                        "结胸",
                        "协热利",
                    ]
                    if part
                )
            )
            detail.setdefault("rawOcrBackup", row["content"] or "")
            detail.update(
                {
                    "original_clause": clause["original_clause"],
                    "ni_explanation": clause["ni_explanation"],
                    "symptoms": clause["symptoms"],
                    "pulse": clause["pulse"],
                    "pattern": clause["pattern"],
                    "treatment_principle": clause["treatment_principle"],
                    "formula_refs": clause["formula_refs"],
                    "related_formulas": clause["formula_refs"],
                    "cleanup_method": "manual_shanghan_clause_ocr_cleanup",
                    "manual_review_required": True,
                }
            )
            detail_json = json.dumps(detail, ensure_ascii=False, separators=(",", ":"))
            item_id = int(row["id"])
            conn.execute(
                """
                UPDATE knowledge_items
                SET name = ?2, summary = ?3, content = ?4, tags = ?5, detail = ?6, updated_at = datetime('now')
                WHERE id = ?1
                """,
                (item_id, clause["name"], clause["summary"], content, tags, detail_json),
            )
            conn.execute(
                """
                INSERT INTO syndrome_details
                  (item_id, symptoms, tongue, pulse, pathogenesis, treatment_principle, notes)
                VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6)
                ON CONFLICT(item_id) DO UPDATE SET
                  symptoms = excluded.symptoms,
                  pulse = excluded.pulse,
                  pathogenesis = excluded.pathogenesis,
                  treatment_principle = excluded.treatment_principle,
                  notes = excluded.notes
                """,
                (
                    item_id,
                    clause["symptoms"],
                    clause["pulse"],
                    clause["pattern"],
                    clause["treatment_principle"],
                    "已根据条文与用户反馈清理 OCR 断行碎片，原始文本保存在 detail.rawOcrBackup。",
                ),
            )
            refresh_search(conn, row, clause["name"], clause["summary"], content, tags)
            stats["updated"] += 1
            stats["ids"].append(item_id)

    conn.close()
    return stats


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--db", type=pathlib.Path, default=None)
    args = parser.parse_args()
    stats = curate_database(args.db or default_db_path())
    print(json.dumps(stats, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
