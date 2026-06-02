import { invoke } from "@tauri-apps/api/core";
import type {
  GridSaveResponse,
  DashboardStats,
  EnhancedSearchResponse,
  FavoriteItem,
  KnowledgeDetailResponse,
  KnowledgeInput,
  KnowledgeListResponse,
  KnowledgeType,
  RecentView,
  UserNote,
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

export function searchKnowledgeEnhanced(request: {
  query: string;
  filter?: string;
  page?: number;
  pageSize?: number;
}) {
  return invoke<EnhancedSearchResponse>("search_knowledge_enhanced", { request });
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

export function toggleFavorite(itemId: number) {
  return invoke<KnowledgeDetailResponse>("toggle_favorite", { itemId });
}

export function recordRecentView(itemId: number) {
  return invoke<RecentView>("record_recent_view", { itemId });
}

export function listRecentViews(limit = 10) {
  return invoke<RecentView[]>("list_recent_views", { limit });
}

export function listFavorites() {
  return invoke<FavoriteItem[]>("list_favorites");
}

export function saveUserNote(itemId: number, noteText: string) {
  return invoke<UserNote>("save_user_note", { itemId, noteText });
}

export function deleteUserNote(noteId: number) {
  return invoke<void>("delete_user_note", { noteId });
}

export function getDashboardStats() {
  return invoke<DashboardStats>("get_dashboard_stats");
}

export function saveGridDirtyRows(itemType: KnowledgeType, rows: KnowledgeInput[]) {
  return invoke<GridSaveResponse>("save_grid_dirty_rows", { request: { itemType, rows } });
}
