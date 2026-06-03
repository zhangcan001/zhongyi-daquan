import type { KnowledgeItem } from "./types";

type NatureTone = "deep-cold" | "cold" | "mild-cold" | "neutral" | "mild-warm" | "warm" | "hot" | "deep-hot";

export type NatureColorInfo = {
  label: string;
  tone: NatureTone;
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

function detailText(value: unknown): string {
  if (typeof value === "string") return value;
  if (typeof value === "number") return String(value);
  if (Array.isArray(value)) return value.map(detailText).join(" ");
  return "";
}
