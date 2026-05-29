import { invoke } from "@tauri-apps/api/core";
import type {
  GridSaveResponse,
  KnowledgeDetailResponse,
  KnowledgeInput,
  KnowledgeListResponse,
  KnowledgeType,
} from "./types";

export function listKnowledgeItems(request: {
  itemType?: KnowledgeType | "";
  query?: string;
  dataStatus?: string;
  favoriteOnly?: boolean;
  page?: number;
  pageSize?: number;
}) {
  return invoke<KnowledgeListResponse>("list_knowledge_items", { request });
}

export function getKnowledgeDetail(itemId: number) {
  return invoke<KnowledgeDetailResponse>("get_knowledge_detail", { itemId });
}

export function createKnowledgeItem(input: KnowledgeInput) {
  return invoke<KnowledgeDetailResponse>("create_knowledge_item", { input });
}

export function updateKnowledgeItem(itemId: number, input: KnowledgeInput) {
  return invoke<KnowledgeDetailResponse>("update_knowledge_item", { itemId, input });
}

export function deleteKnowledgeItem(itemId: number) {
  return invoke<void>("delete_knowledge_item", { itemId });
}

export function setKnowledgeFavorite(itemId: number, isFavorite: boolean) {
  return invoke<KnowledgeDetailResponse>("set_knowledge_favorite", {
    request: { itemId, isFavorite },
  });
}

export function saveGridDirtyRows(itemType: KnowledgeType, rows: KnowledgeInput[]) {
  return invoke<GridSaveResponse>("save_grid_dirty_rows", { request: { itemType, rows } });
}
