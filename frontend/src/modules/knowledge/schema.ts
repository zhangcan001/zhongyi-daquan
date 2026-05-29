import type { KnowledgeInput, KnowledgeType } from "./types";

export type DetailField = {
  key: string;
  label: string;
  kind?: "text" | "number" | "textarea";
  safetyNote?: string;
};

export const detailFields: Record<KnowledgeType, DetailField[]> = {
  herb: [
    { key: "natureFlavor", label: "性味" },
    { key: "meridians", label: "归经" },
    { key: "effects", label: "功效", kind: "textarea" },
    { key: "indications", label: "主治", kind: "textarea" },
    { key: "dosage", label: "用量" },
    { key: "contraindications", label: "禁忌", kind: "textarea" },
    { key: "compatibility", label: "配伍", kind: "textarea" },
    { key: "notes", label: "备注", kind: "textarea" },
  ],
  formula: [
    { key: "sourceText", label: "出处" },
    { key: "composition", label: "组成", kind: "textarea" },
    { key: "usage", label: "用法", kind: "textarea" },
    { key: "effects", label: "功效", kind: "textarea" },
    { key: "indications", label: "主治", kind: "textarea" },
    { key: "explanation", label: "方解", kind: "textarea" },
    { key: "modifications", label: "加减", kind: "textarea" },
    { key: "contraindications", label: "禁忌", kind: "textarea" },
    { key: "notes", label: "备注", kind: "textarea" },
  ],
  meridian: [
    { key: "meridianCode", label: "经络编号" },
    { key: "category", label: "分类" },
    { key: "yinYang", label: "阴阳" },
    { key: "handFoot", label: "手足" },
    { key: "organRelation", label: "脏腑关联" },
    { key: "pairedMeridian", label: "表里经" },
    { key: "pathwayText", label: "循行", kind: "textarea" },
    { key: "mainIndications", label: "主要关联", kind: "textarea" },
    { key: "notes", label: "备注", kind: "textarea" },
  ],
  acupoint: [
    { key: "acupointCode", label: "穴位编号" },
    { key: "meridianItemId", label: "所属经络 ID", kind: "number" },
    { key: "bodyRegion", label: "部位" },
    { key: "bodySubregion", label: "细分部位" },
    { key: "sideType", label: "单双侧" },
    { key: "standardLocation", label: "标准定位", kind: "textarea", safetyNote: "仅供专业学习参考，请勿自行操作。" },
    { key: "locatingMethod", label: "定位方法", kind: "textarea", safetyNote: "仅供专业学习参考，请勿自行操作。" },
    { key: "boneCun", label: "骨度分寸" },
    { key: "anatomy", label: "解剖", kind: "textarea" },
    { key: "functions", label: "功能", kind: "textarea" },
    { key: "indications", label: "关联症候", kind: "textarea" },
    { key: "needlingSummary", label: "针刺摘要", kind: "textarea", safetyNote: "仅供专业学习参考，请勿自行操作。" },
    { key: "moxibustionSummary", label: "艾灸摘要", kind: "textarea", safetyNote: "仅供专业学习参考，请勿自行操作。" },
    { key: "massageSummary", label: "按揉摘要", kind: "textarea", safetyNote: "仅供专业学习参考，请勿自行操作。" },
    { key: "contraindications", label: "禁忌", kind: "textarea" },
    { key: "precautions", label: "注意事项", kind: "textarea" },
    { key: "riskLevel", label: "风险级别" },
  ],
  syndrome: [
    { key: "symptoms", label: "症状", kind: "textarea" },
    { key: "tongue", label: "舌象" },
    { key: "pulse", label: "脉象" },
    { key: "pathogenesis", label: "病机", kind: "textarea" },
    { key: "treatmentPrinciple", label: "治则", kind: "textarea" },
    { key: "notes", label: "备注", kind: "textarea" },
  ],
  disease: [
    { key: "symptoms", label: "症状", kind: "textarea" },
    { key: "commonSyndromes", label: "常见证型", kind: "textarea" },
    { key: "careAdvice", label: "日常记录", kind: "textarea" },
    { key: "medicalWarning", label: "就医提示", kind: "textarea" },
    { key: "notes", label: "备注", kind: "textarea" },
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
    detail: {},
  };
}
