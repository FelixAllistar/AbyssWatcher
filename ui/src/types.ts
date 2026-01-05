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
/** Type of bookmark - matches values written to gamelog */
export type BookmarkType = 'RUN_START' | 'RUN_END' | 'ROOM_START' | 'ROOM_END' | 'HIGHLIGHT';

/** Room marker state - now just boolean */
export type RoomMarkerState = 'Idle' | 'InRoom';

/** A bookmark parsed from the gamelog file */
export interface Bookmark {
    timestamp_secs: number;
    bookmark_type: string;
    label?: string;
}

/** Room marker toggle response (simplified) */
export interface RoomMarkerResponse {
    room_open: boolean;
}

// ============================================
// Alert Types (mirror src/core/alerts/model.rs)
// ============================================

/** Unique identifier for hardcoded alert rules */
export type AlertRuleId =
    | 'EnvironmentalDamage'
    | 'FriendlyFire'
    | 'LogiTakingDamage'
    | 'NeutSensitiveNeuted'
    | 'CapacitorFailure'
    | 'LogiNeuted';

/** Sound options for alerts */
export type AlertSound = 'Default' | 'Warning' | 'Critical' | 'None';

/** Per-rule configuration */
export interface AlertRuleConfig {
    enabled: boolean;
    sound: AlertSound;
}

/** Character role designations */
export interface CharacterRoles {
    logi_characters: string[];
    neut_sensitive_characters: string[];
}

/** Alert engine configuration - part of Settings */
export interface AlertEngineConfig {
    rules: Record<AlertRuleId, AlertRuleConfig>;
    roles: CharacterRoles;
}

/** Alert event from backend */
export interface AlertEvent {
    rule_id: AlertRuleId;
    message: string;
    sound: string | null;
}

/** Extended Settings with alert configuration */
export interface SettingsWithAlerts extends Settings {
    alert_settings: AlertEngineConfig;
}
