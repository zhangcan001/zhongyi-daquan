import type { KnowledgeItem } from "./types";

type NatureTone = "deep-cold" | "cold" | "mild-cold" | "neutral" | "mild-warm" | "warm" | "hot" | "deep-hot";
type ElementTone = "wood" | "fire" | "earth" | "metal" | "water";

export type NatureColorInfo = {
  label: string;
  tone: NatureTone;
};

export type MeridianElementInfo = {
  label: string;
  organ: string;
  tone: ElementTone;
};

export function herbNatureFromItem(item: Pick<KnowledgeItem, "itemType" | "detail">): NatureColorInfo | null {
  if (item.itemType !== "herb") return null;
  return herbNatureFromDetail(item.detail);
}

export function herbNatureFromDetail(detail: Record<string, unknown> | null | undefined): NatureColorInfo | null {
  if (!detail) return null;
  const text = [
    detailText(detail.fourQi),
    detailText(detail.four_qi),
    detailText(detail.natureFlavor),
    detailText(detail.nature_flavor),
    detailText(detail["性味"]),
  ]
    .filter(Boolean)
    .join(" ");
  return parseNatureText(text);
}

export function herbNatureClass(info: NatureColorInfo | null) {
  return info ? `herb-nature-name herb-nature-${info.tone}` : undefined;
}

export function meridianElementFromItem(
  item: Pick<KnowledgeItem, "itemType" | "name" | "code" | "category" | "summary" | "content" | "detail">,
): MeridianElementInfo | null {
  if (item.itemType !== "acupoint" && item.itemType !== "meridian" && item.itemType !== "acupuncture") return null;
  return meridianElementFromText([
    item.name,
    item.code,
    item.category,
    item.summary,
    item.content,
    detailText(item.detail?.meridianName),
    detailText(item.detail?.meridian),
    detailText(item.detail?.organRelation),
    detailText(item.detail?.organ_relation),
    detailText(item.detail?.pairedMeridian),
    detailText(item.detail?.paired_meridian),
    detailText(item.detail?.related_organs),
    detailText(item.detail?.five_elements),
  ]);
}

export function meridianElementFromDetail(
  itemType: string,
  name: string,
  category: string | null | undefined,
  detail: Record<string, unknown> | null | undefined,
): MeridianElementInfo | null {
  if (itemType !== "acupoint" && itemType !== "meridian" && itemType !== "acupuncture") return null;
  return meridianElementFromText([
    name,
    category,
    detailText(detail?.meridianName),
    detailText(detail?.meridian),
    detailText(detail?.organRelation),
    detailText(detail?.organ_relation),
    detailText(detail?.pairedMeridian),
    detailText(detail?.paired_meridian),
    detailText(detail?.related_organs),
    detailText(detail?.five_elements),
  ]);
}

export function meridianElementFromSearch(item: {
  itemType: string;
  name: string;
  code?: string | null;
  category?: string | null;
  summary?: string | null;
  contentSnippet?: string | null;
}): MeridianElementInfo | null {
  if (item.itemType !== "acupoint" && item.itemType !== "meridian" && item.itemType !== "acupuncture") return null;
  return meridianElementFromText([item.name, item.code, item.category, item.summary, item.contentSnippet]);
}

export function meridianElementClass(info: MeridianElementInfo | null) {
  return info ? `meridian-element-name meridian-element-${info.tone}` : undefined;
}

export function parseNatureText(text: string): NatureColorInfo | null {
  const value = text.replace(/\s+/g, "");
  if (!value) return null;
  const rules: Array<[RegExp, NatureColorInfo]> = [
    [/大寒|极寒|至寒/, { label: "大寒", tone: "deep-cold" }],
    [/微寒|小寒/, { label: "微寒", tone: "mild-cold" }],
    [/寒/, { label: "寒", tone: "cold" }],
    [/大热|极热|至热/, { label: "大热", tone: "deep-hot" }],
    [/微热|小热/, { label: "微热", tone: "mild-warm" }],
    [/大温|热/, { label: value.includes("大温") ? "大温" : "热", tone: "hot" }],
    [/微温|小温/, { label: "微温", tone: "mild-warm" }],
    [/温/, { label: "温", tone: "warm" }],
    [/平/, { label: "平", tone: "neutral" }],
  ];
  return rules.find(([pattern]) => pattern.test(value))?.[1] ?? null;
}

function meridianElementFromText(values: Array<string | null | undefined>): MeridianElementInfo | null {
  const text = values.filter(Boolean).join(" ");
  const normalized = text.replace(/\s+/g, "");
  if (!normalized) return null;
  const rules: Array<[RegExp, MeridianElementInfo]> = [
    [/足厥阴|肝经|肝|胆经|胆|LR|GB|木/, { label: "木", organ: normalized.includes("胆") ? "胆" : "肝", tone: "wood" }],
    [
      /手少阴|心经|心|小肠经|小肠|心包|三焦|HT|SI|PC|SJ|TE|火/,
      { label: "火", organ: normalized.includes("小肠") ? "小肠" : normalized.includes("心包") ? "心包" : normalized.includes("三焦") ? "三焦" : "心", tone: "fire" },
    ],
    [/足太阴|脾经|脾|胃经|胃|SP|ST|土/, { label: "土", organ: normalized.includes("胃") ? "胃" : "脾", tone: "earth" }],
    [/手太阴|肺经|肺|大肠经|大肠|LU|LI|金/, { label: "金", organ: normalized.includes("大肠") ? "大肠" : "肺", tone: "metal" }],
    [/足少阴|肾经|肾|膀胱经|膀胱|KI|BL|水/, { label: "水", organ: normalized.includes("膀胱") ? "膀胱" : "肾", tone: "water" }],
  ];
  return rules.find(([pattern]) => pattern.test(normalized))?.[1] ?? null;
}

function detailText(value: unknown): string {
  if (typeof value === "string") return value;
  if (typeof value === "number") return String(value);
  if (Array.isArray(value)) return value.map(detailText).join(" ");
  return "";
}
