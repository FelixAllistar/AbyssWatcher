/**
 * Shared TypeScript types for AbyssWatcher frontend.
 * 
 * These types mirror the Rust structs in src/core/model.rs.
 * Keep in sync when modifying backend data structures.
 */

// ============================================
// Combat Data Types
// ============================================

/** Per-target damage breakdown for a weapon action */
export interface TargetHit {
    target: string;
    value: number;
}

/** A combat action (weapon, repair, neut, cap transfer) with optional target breakdown */
export interface CombatAction {
    name: string;
    action_type: 'Damage' | 'Repair' | 'Capacitor' | 'Neut';
    incoming: boolean;
    value: number;
    targets: TargetHit[];
}

/** DPS update payload from backend event */
export interface DpsUpdate {
    combat_actions_by_character: Record<string, CombatAction[]>;
}

// ============================================
// Character & Settings Types
// ============================================

/** Character tracking state */
export interface CharacterState {
    character: string;
    path: string;
    tracked: boolean;
}

/** Application settings */
export interface Settings {
    gamelog_dir: string;
    dps_window_seconds: number;
}

// ============================================
// Bookmark Types (mirror src/core/inline_bookmarks.rs)
// ============================================

/** Type of bookmark */
export type BookmarkType = 'RunStart' | 'RunEnd' | 'RoomStart' | 'RoomEnd' | 'Highlight';

/** Room marker state - now just boolean */
export type RoomMarkerState = 'Idle' | 'InRoom';

/** A bookmark from the backend (simplified sidecar format) */
export interface Bookmark {
    timestamp_secs: number;
    bookmark_type: string;
    label?: string;
}

/** Room marker toggle response (simplified) */
export interface RoomMarkerResponse {
    room_open: boolean;
}

