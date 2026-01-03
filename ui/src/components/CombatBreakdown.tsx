import { type FC, useState, useMemo } from 'react';
import type { CombatAction, DpsUpdate, CharacterState } from '../types';

interface CombatBreakdownProps {
    data: DpsUpdate | null;
    characters: CharacterState[];
}

const getMetricStyle = (type: CombatAction['action_type'], incoming: boolean) => {
    const styles: Record<string, { outClass: string; inClass: string; label: string }> = {
        Damage: { outClass: 'text-dps-out', inClass: 'text-dps-in', label: 'DPS' },
        Repair: { outClass: 'text-rep-out', inClass: 'text-rep-in', label: 'HPS' },
        Capacitor: { outClass: 'text-cap-out', inClass: 'text-cap-in', label: 'GJ/s' },
        Neut: { outClass: 'text-neut-out', inClass: 'text-neut-in', label: 'GJ/s' },
    };
    const s = styles[type] || { outClass: 'text-default', inClass: 'text-default', label: '' };
    return { class: incoming ? s.inClass : s.outClass, label: s.label };
};

interface CharacterCardProps {
    name: string;
    actions: CombatAction[];
}

const CharacterCard: FC<CharacterCardProps> = ({ name, actions }) => {
    const [isCollapsed, setIsCollapsed] = useState(false);
    const [collapsedGroups, setCollapsedGroups] = useState<Set<string>>(new Set());

    const stats = useMemo(() => {
        const result = {
            out: { dps: 0, hps: 0, cap: 0, neut: 0 },
            in: { dps: 0, hps: 0, cap: 0, neut: 0 },
        };
        actions.forEach((act) => {
            const dir = act.incoming ? 'in' : 'out';
            if (act.action_type === 'Damage') result[dir].dps += act.value;
            else if (act.action_type === 'Repair') result[dir].hps += act.value;
            else if (act.action_type === 'Capacitor') result[dir].cap += act.value;
            else if (act.action_type === 'Neut') result[dir].neut += act.value;
        });
        return result;
    }, [actions]);

    const groups = useMemo(() => {
        const g: Record<string, CombatAction[]> = {
            Damage: [],
            Repair: [],
            Capacitor: [],
            Neut: [],
        };
        actions.forEach((act) => {
            if (g[act.action_type]) g[act.action_type].push(act);
        });
        // Sort each group: outgoing first, then by value desc
        Object.values(g).forEach((arr) => {
            arr.sort((a, b) => {
                if (a.incoming !== b.incoming) return a.incoming ? 1 : -1;
                return b.value - a.value;
            });
        });
        return g;
    }, [actions]);

    const toggleGroup = (type: string) => {
        setCollapsedGroups((prev) => {
            const next = new Set(prev);
            if (next.has(type)) next.delete(type);
            else next.add(type);
            return next;
        });
    };

    // Track expanded weapons for multi-target display
    const [expandedWeapons, setExpandedWeapons] = useState<Set<string>>(new Set());
    const toggleWeapon = (key: string) => {
        setExpandedWeapons((prev) => {
            const next = new Set(prev);
            if (next.has(key)) next.delete(key);
            else next.add(key);
            return next;
        });
    };

    const renderStatPair = (outVal: number, inVal: number, type: CombatAction['action_type']) => {
        if (outVal <= 0 && inVal <= 0) return null;
        const outStyle = getMetricStyle(type, false);
        const inStyle = getMetricStyle(type, true);
        return (
            <div className="metric-pair" key={type}>
                <span className={`val-out ${outVal > 0 ? outStyle.class : 'text-dim'}`} style={outVal <= 0 ? { opacity: 0.3 } : {}}>
                    {outVal.toFixed(0)}
                </span>
                <span className="val-divider">/</span>
                <span className={`val-in ${inVal > 0 ? inStyle.class : 'text-dim'}`} style={inVal <= 0 ? { opacity: 0.3 } : {}}>
                    {inVal.toFixed(0)}
                </span>
            </div>
        );
    };

    const metricPairs = [
        renderStatPair(stats.out.dps, stats.in.dps, 'Damage'),
        renderStatPair(stats.out.hps, stats.in.hps, 'Repair'),
        renderStatPair(stats.out.cap, stats.in.cap, 'Capacitor'),
        renderStatPair(stats.out.neut, stats.in.neut, 'Neut'),
    ].filter(Boolean);

    return (
        <div className="char-strip-container">
            <div className="char-strip" onClick={() => setIsCollapsed(!isCollapsed)}>
                <div className="char-info">
                    <span className="collapse-indicator">{isCollapsed ? '▶' : '▼'}</span>
                    <span className="char-name">{name}</span>
                </div>
                <div className="metric-container">
                    {metricPairs.length > 0 ? metricPairs : <span className="metric-idle">IDLE</span>}
                </div>
            </div>

            {!isCollapsed && (
                <div className="char-content">
                    {Object.entries(groups).map(([type, items]) => {
                        if (items.length === 0) return null;
                        const isGroupCollapsed = collapsedGroups.has(type);
                        const label = type === 'Damage' ? 'DPS' : type;

                        return (
                            <div className="category-section" key={type}>
                                <div className="category-header" onClick={() => toggleGroup(type)}>
                                    <span>{label}</span>
                                    <span className="collapse-indicator">{isGroupCollapsed ? '▶' : '▼'}</span>
                                </div>
                                {!isGroupCollapsed && (
                                    <div className="category-content">
                                        {items.map((act, idx) => {
                                            const style = getMetricStyle(act.action_type, act.incoming);
                                            const icon = act.incoming ? '↓' : '↑';
                                            const hasMultipleTargets = !act.incoming && act.targets && act.targets.length > 1;
                                            const singleTarget = !act.incoming && act.targets && act.targets.length === 1 ? act.targets[0] : null;
                                            const weaponKey = `${act.name}-${idx}`;
                                            const isWeaponExpanded = expandedWeapons.has(weaponKey);

                                            // Multi-target: collapsible dropdown
                                            if (hasMultipleTargets) {
                                                return (
                                                    <div className="action-row-group" key={weaponKey}>
                                                        <div className="action-row action-row-expandable" onClick={() => toggleWeapon(weaponKey)}>
                                                            <div className={`action-name ${style.class}`}>
                                                                <span className="collapse-indicator-sm">{isWeaponExpanded ? '▼' : '▶'}</span>
                                                                <span className="dir-icon">{icon}</span>
                                                                <span>{act.name}</span>
                                                            </div>
                                                            <div className={`action-value ${style.class}`}>
                                                                {act.value.toLocaleString(undefined, { minimumFractionDigits: 0, maximumFractionDigits: 1 })}
                                                                <span className="action-unit">{style.label}</span>
                                                            </div>
                                                        </div>
                                                        {isWeaponExpanded && (
                                                            <div className="action-targets">
                                                                {act.targets.map((t, ti) => (
                                                                    <div className="action-row-sub" key={ti}>
                                                                        <span className="target-arrow">→</span>
                                                                        <span className="target-name">{t.target}</span>
                                                                        <span className={`target-value ${style.class}`}>
                                                                            {t.value.toLocaleString(undefined, { minimumFractionDigits: 0, maximumFractionDigits: 1 })}
                                                                        </span>
                                                                    </div>
                                                                ))}
                                                            </div>
                                                        )}
                                                    </div>
                                                );
                                            }

                                            // Single target: inline display
                                            return (
                                                <div className="action-row" key={weaponKey}>
                                                    <div className={`action-name ${style.class}`}>
                                                        <span className="dir-icon">{icon}</span>
                                                        <span>{act.name}</span>
                                                        {singleTarget && (
                                                            <span className="inline-target">→ {singleTarget.target}</span>
                                                        )}
                                                    </div>
                                                    <div className={`action-value ${style.class}`}>
                                                        {act.value.toLocaleString(undefined, { minimumFractionDigits: 0, maximumFractionDigits: 1 })}
                                                        <span className="action-unit">{style.label}</span>
                                                    </div>
                                                </div>
                                            );
                                        })}
                                    </div>
                                )}
                            </div>
                        );
                    })}
                </div>
            )}
        </div>
    );
};

const CombatBreakdown: FC<CombatBreakdownProps> = ({ data, characters }) => {
    const activeData = useMemo(() => {
        const map = new Map<string, CombatAction[]>(
            Object.entries(data?.combat_actions_by_character || {})
        );
        // Add tracked but inactive characters
        characters.forEach((char) => {
            if (char.tracked && !map.has(char.character)) {
                map.set(char.character, []);
            }
        });
        return Array.from(map.entries()).sort((a, b) => a[0].localeCompare(b[0]));
    }, [data, characters]);

    return (
        <div id="combat-breakdown">
            {activeData.map(([name, actions]) => (
                <CharacterCard key={name} name={name} actions={actions} />
            ))}
        </div>
    );
};

export default CombatBreakdown;
