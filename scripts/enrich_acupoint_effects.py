import json
import os
import pathlib
import sqlite3
import time

SAFETY = "仅供中医针灸知识学习与检索参考，不作为诊断、治疗或自行针刺操作依据；定位、配伍和操作须由合格专业人员执行。"

SCOPES = {
    "LU": ("宣肺理气、肃降肺气、调理咽喉与胸部气机。", "传统常用于肺系、咽喉、胸部及上肢内侧相关病证的学习归纳。"),
    "LI": ("疏风清热、通调肠腑、调理头面五官。", "传统常用于头面五官、口齿咽喉、肠腑与上肢外侧相关病证的学习归纳。"),
    "ST": ("和胃降逆、调理脾胃、通经活络。", "传统常用于胃肠、头面、胸腹、下肢前外侧及气血调养相关病证的学习归纳。"),
    "SP": ("健脾化湿、调摄气血、和中理滞。", "传统常用于脾胃、消化、水湿、妇科及下肢内侧相关病证的学习归纳。"),
    "HT": ("宁心安神、调理心胸、通利经脉。", "传统常用于心胸、神志、睡眠及上肢内侧相关病证的学习归纳。"),
    "SI": ("通利小肠、疏通肩臂、调理头项耳目。", "传统常用于肩臂、头项、耳目及小肠经循行相关病证的学习归纳。"),
    "BL": ("疏通太阳经气、调理背腰、联系脏腑背俞。", "传统常用于头项、背腰、下肢后侧、泌尿及背俞穴相关脏腑病证的学习归纳。"),
    "KI": ("滋肾纳气、调理下焦、通利腰膝与咽喉。", "传统常用于肾系、泌尿生殖、腰膝、咽喉及下肢内侧相关病证的学习归纳。"),
    "PC": ("宽胸理气、宁心安神、和胃降逆。", "传统常用于心胸、胃脘、神志及上肢内侧相关病证的学习归纳。"),
    "TE": ("通调三焦、疏利耳目咽喉、调畅水道。", "传统常用于耳目咽喉、侧头、胁肋、上肢外侧及水道相关病证的学习归纳。"),
    "GB": ("疏肝利胆、清利头目、舒筋通络。", "传统常用于侧头目耳、胁肋、胆腑、筋脉与下肢外侧相关病证的学习归纳。"),
    "LR": ("疏肝理气、调经和血、平抑肝阳。", "传统常用于肝胆、情志、胁肋、妇科及下肢内侧相关病证的学习归纳。"),
    "GV": ("统摄督脉、振奋阳气、调理头项脊背与神志。", "传统常用于头项、脊柱、背部、阳气与神志相关病证的学习归纳。"),
    "CV": ("统摄任脉、调理阴液与下焦、和中固本。", "传统常用于腹部、泌尿生殖、妇科、脾胃与任脉循行相关病证的学习归纳。"),
}


def app_database() -> pathlib.Path:
    root = pathlib.Path(os.environ["APPDATA"]) / "com.zhongyi.daquan"
    return next(root.rglob("zhongyi.db"))


def point_number(code: str) -> str:
    return "".join(ch for ch in code if ch.isdigit())


def enrich_item(item: dict) -> tuple[str, str]:
    detail = item.setdefault("detail", {})
    meridian_code = detail.get("meridian_code") or "".join(ch for ch in item["code"] if ch.isalpha())
    meridian_name = detail.get("meridian_name") or item.get("category") or ""
    functions, indications = SCOPES.get(
        meridian_code,
        ("按所属经络理解其传统作用。", "传统主治范围需结合所属经络、定位与专业辨证学习。"),
    )
    number = point_number(item["code"])
    item["summary"] = f"{item['name']}，属{meridian_name}，为该经第{number}穴，标准编号 {item['code']}。"
    item["content"] = (
        f"所属经络：{meridian_name}。{item['name']}为{meridian_name}经穴，标准编号 {item['code']}。"
        f"传统作用学习参考：{functions}"
        f"传统主治范围学习参考：{indications}"
        f"{SAFETY}"
    )
    detail["meridian_name"] = meridian_name
    detail["functions"] = functions
    detail["indications"] = indications
    detail["precautions"] = SAFETY
    return functions, indications


def main() -> None:
    seed_path = pathlib.Path("data-seed/acupoints.full.json")
    items = json.loads(seed_path.read_text(encoding="utf-8"))
    for item in items:
        enrich_item(item)
    seed_path.write_text(json.dumps(items, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    db = app_database()
    backup = db.parent.parent / "backups" / f"zhongyi-before-acupoint-effects-{time.strftime('%Y%m%d%H%M%S')}.db"
    source = sqlite3.connect(db)
    target = sqlite3.connect(backup)
    source.backup(target)
    target.close()
    source.close()

    conn = sqlite3.connect(db)
    now = time.strftime("%Y-%m-%d %H:%M:%S")
    with conn:
        for item in items:
            detail = item["detail"]
            detail_json = json.dumps(
                {
                    "meridianName": detail["meridian_name"],
                    "meridianCode": detail["meridian_code"],
                    "functions": detail["functions"],
                    "indications": detail["indications"],
                    "precautions": detail["precautions"],
                    "riskLevel": detail.get("risk_level", "learning_only"),
                },
                ensure_ascii=False,
            )
            tags = ",".join(item.get("tags", []))
            row = conn.execute(
                "SELECT id FROM knowledge_items WHERE type = 'acupoint' AND code = ?",
                (item["code"],),
            ).fetchone()
            if not row:
                continue
            item_id = row[0]
            conn.execute(
                """
                UPDATE knowledge_items
                   SET summary = ?, content = ?, detail = ?, tags = ?, updated_at = ?
                 WHERE id = ?
                """,
                (item["summary"], item["content"], detail_json, tags, now, item_id),
            )
            conn.execute(
                """
                UPDATE acupoint_details
                   SET functions = ?, indications = ?, precautions = ?
                 WHERE item_id = ?
                """,
                (detail["functions"], detail["indications"], detail["precautions"], item_id),
            )
            conn.execute("DELETE FROM knowledge_fts WHERE rowid = ?", (item_id,))
            conn.execute(
                """
                INSERT INTO knowledge_fts(rowid, name, code, alias, pinyin, category, summary, content, tags)
                SELECT id, name, code, alias, pinyin, category, summary, content, tags
                  FROM knowledge_items WHERE id = ?
                """,
                (item_id,),
            )
            conn.execute(
                """
                UPDATE knowledge_list_view_cache
                   SET summary = ?, tags = ?, updated_at = ?
                 WHERE item_id = ?
                """,
                (item["summary"], tags, now, item_id),
            )
        conn.execute(
            """
            INSERT INTO audit_logs(action, target_type, after_json, created_at)
            VALUES ('enrich_acupoint_meridian_effects', 'acupoint', ?, ?)
            """,
            (json.dumps({"acupoints": len(items), "mode": "meridian_scope_learning_reference"}, ensure_ascii=False), now),
        )

    result = {
        "backup": str(backup),
        "seedUpdated": str(seed_path),
        "functionsFilled": conn.execute(
            "SELECT COUNT(*) FROM acupoint_details WHERE functions IS NOT NULL AND trim(functions) != ''"
        ).fetchone()[0],
        "indicationsFilled": conn.execute(
            "SELECT COUNT(*) FROM acupoint_details WHERE indications IS NOT NULL AND trim(indications) != ''"
        ).fetchone()[0],
        "ST36": conn.execute(
            """
            SELECT ki.summary, ad.functions, ad.indications
              FROM knowledge_items ki
              JOIN acupoint_details ad ON ad.item_id = ki.id
             WHERE ki.code = 'ST36'
            """
        ).fetchone(),
    }
    conn.close()
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
