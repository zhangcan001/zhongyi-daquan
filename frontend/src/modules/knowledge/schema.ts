import type { KnowledgeInput, KnowledgeType } from "./types";

export type DetailField = {
  key: string;
  label: string;
  kind?: "text" | "number" | "textarea";
  safetyNote?: string;
};

export const detailFields: Record<KnowledgeType, DetailField[]> = {
  herb: [
    { key: "bencao_original", label: "本经原文", kind: "textarea" },
    { key: "nature_flavor", label: "性味" },
    { key: "meridians", label: "归经" },
    { key: "effects", label: "功效", kind: "textarea" },
    { key: "dosage", label: "用量" },
    { key: "contraindications", label: "禁忌", kind: "textarea" },
    { key: "processing", label: "炮制", kind: "textarea" },
    { key: "ni_note", label: "倪注", kind: "textarea" },
    { key: "other_notes", label: "其他注解", kind: "textarea" },
    { key: "indications", label: "主治", kind: "textarea" },
  ],
  formula: [
    { key: "formula_name", label: "方名" },
    { key: "classic_name", label: "出处" },
    { key: "chapter_title", label: "篇章" },
    { key: "ingredients", label: "组成", kind: "textarea" },
    { key: "preparation", label: "煎服法", kind: "textarea" },
    { key: "administration", label: "服法", kind: "textarea" },
    { key: "indications", label: "主治", kind: "textarea" },
    { key: "pattern", label: "证型" },
    { key: "symptoms", label: "症状", kind: "textarea" },
    { key: "pulse", label: "脉象" },
    { key: "modifications", label: "加减", kind: "textarea" },
    { key: "contraindications", label: "禁忌", kind: "textarea" },
    { key: "ni_note", label: "倪注", kind: "textarea" },
  ],
  acupuncture: [
    { key: "item_subtype", label: "类型" },
    { key: "meridian", label: "经络" },
    { key: "acupoint_name", label: "穴名" },
    { key: "acupoint_code", label: "穴位编号" },
    { key: "body_region", label: "部位" },
    { key: "location_text", label: "定位", kind: "textarea" },
    { key: "needling_method", label: "针刺", kind: "textarea", safetyNote: "仅供专业学习参考，请勿自行操作。" },
    { key: "moxibustion_method", label: "灸法", kind: "textarea", safetyNote: "仅供专业学习参考，请勿自行操作。" },
    { key: "cupping_method", label: "火罐", kind: "textarea", safetyNote: "仅供专业学习参考，请勿自行操作。" },
    { key: "main_indications", label: "主治", kind: "textarea" },
    { key: "related_symptoms", label: "相关症状", kind: "textarea" },
    { key: "contraindications", label: "禁忌", kind: "textarea" },
    { key: "ni_note", label: "倪注", kind: "textarea" },
  ],
  syndrome: [
    { key: "classic_name", label: "经典" },
    { key: "chapter_title", label: "篇章" },
    { key: "clause_no", label: "条文号" },
    { key: "six_channel", label: "六经" },
    { key: "original_clause", label: "原文", kind: "textarea" },
    { key: "ni_explanation", label: "倪注", kind: "textarea" },
    { key: "symptoms", label: "症状", kind: "textarea" },
    { key: "pulse", label: "脉象" },
    { key: "pattern", label: "证型" },
    { key: "treatment_principle", label: "治法", kind: "textarea" },
    { key: "formula_refs", label: "相关方剂", kind: "textarea" },
  ],
  theory: [
    { key: "classic_name", label: "经典" },
    { key: "chapter_title", label: "篇章" },
    { key: "classic_original", label: "原文", kind: "textarea" },
    { key: "ni_explanation", label: "倪注", kind: "textarea" },
    { key: "main_concepts", label: "核心概念", kind: "textarea" },
    { key: "related_organs", label: "相关脏腑" },
    { key: "yin_yang", label: "阴阳" },
    { key: "five_elements", label: "五行" },
    { key: "season", label: "季节" },
    { key: "pulse_notes", label: "脉诊", kind: "textarea" },
    { key: "diagnosis_notes", label: "诊断说明", kind: "textarea" },
  ],
  note: [
    { key: "item_subtype", label: "笔记类型" },
    { key: "classic_name", label: "经典" },
    { key: "chapter_title", label: "篇章" },
    { key: "clinical_note", label: "临床笔记", kind: "textarea" },
    { key: "ni_note", label: "倪注", kind: "textarea" },
    { key: "other_notes", label: "其他注解", kind: "textarea" },
  ],
  meridian: [
    { key: "meridian_code", label: "经络编号" },
    { key: "pathway_text", label: "循行", kind: "textarea" },
    { key: "main_indications", label: "主要关联", kind: "textarea" },
    { key: "notes", label: "备注", kind: "textarea" },
  ],
  acupoint: [
    { key: "acupoint_code", label: "穴位编号" },
    { key: "standard_location", label: "标准定位", kind: "textarea" },
    { key: "locating_method", label: "定位方法", kind: "textarea" },
    { key: "needling_summary", label: "针刺摘要", kind: "textarea" },
    { key: "moxibustion_summary", label: "艾灸摘要", kind: "textarea" },
    { key: "contraindications", label: "禁忌", kind: "textarea" },
  ],
  disease: [
    { key: "symptoms", label: "症状", kind: "textarea" },
    { key: "common_syndromes", label: "常见证型", kind: "textarea" },
    { key: "care_advice", label: "日常记录", kind: "textarea" },
    { key: "medical_warning", label: "就医提示", kind: "textarea" },
  ],
};

export function emptyKnowledgeInput(itemType: KnowledgeType): KnowledgeInput {
  return {
    itemType,
    code: "",
    name: "",
    alias: "",
    pinyin: "",
    category: "",
    summary: "",
    content: "",
    sourceNote: "",
    tags: "",
    dataStatus: "draft",
    completenessStatus: "partial",
    isFavorite: false,
    importBatchId: "",
    sourcePackage: "",
    detail: {},
  };
}
