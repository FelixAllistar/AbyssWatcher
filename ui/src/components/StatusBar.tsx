import { type FC, useMemo } from 'react';

interface CombatAction {
    name: string;
    action_type: 'Damage' | 'Repair' | 'Capacitor' | 'Neut';
    incoming: boolean;
    value: number;
}

interface StatusBarProps {
    combatActions: Record<string, CombatAction[]> | null;
}

interface StatPairProps {
    label: string;
    outValue: number;
    inValue: number;
    outClass: string;
    inClass: string;
}

const StatItem: FC<StatPairProps> = ({ label, outValue, inValue, outClass, inClass }) => (
    <div className="stat-item">
        <span className="stat-label">{label}</span>
        <div className="stat-values">
            <span className={`val-out ${outClass}`}>
                {outValue.toLocaleString(undefined, { minimumFractionDigits: 0, maximumFractionDigits: 1 })}
            </span>
            <span className="val-divider">/</span>
            <span className={`val-in ${inClass}`}>
                {inValue.toLocaleString(undefined, { minimumFractionDigits: 0, maximumFractionDigits: 1 })}
            </span>
        </div>
    </div>
);

/**
 * StatusBar computes its totals by summing all character combat actions.
 * This ensures the top-line totals are always exactly the sum of character breakdowns.
 */
const StatusBar: FC<StatusBarProps> = ({ combatActions }) => {
    const totals = useMemo(() => {
        const result = {
            out: { dps: 0, hps: 0, cap: 0, neut: 0 },
            in: { dps: 0, hps: 0, cap: 0, neut: 0 },
        };

        if (!combatActions) return result;

        // Sum all actions across all characters
        Object.values(combatActions).forEach((actions) => {
            actions.forEach((act) => {
                const dir = act.incoming ? 'in' : 'out';
                if (act.action_type === 'Damage') result[dir].dps += act.value;
                else if (act.action_type === 'Repair') result[dir].hps += act.value;
                else if (act.action_type === 'Capacitor') result[dir].cap += act.value;
                else if (act.action_type === 'Neut') result[dir].neut += act.value;
            });
        });

        return result;
    }, [combatActions]);

    return (
        <div className="status-bar-strip">
            <StatItem
                label="DPS"
                outValue={totals.out.dps}
                inValue={totals.in.dps}
                outClass="text-dps-out"
                inClass="text-dps-in"
            />
            <div className="strip-divider" />
            <StatItem
                label="REP"
                outValue={totals.out.hps}
                inValue={totals.in.hps}
                outClass="text-rep-out"
                inClass="text-rep-in"
            />
            <div className="strip-divider" />
            <StatItem
                label="CAP"
                outValue={totals.out.cap}
                inValue={totals.in.cap}
                outClass="text-cap-out"
                inClass="text-cap-in"
            />
            <div className="strip-divider" />
            <StatItem
                label="NEUT"
                outValue={totals.out.neut}
                inValue={totals.in.neut}
                outClass="text-neut-out"
                inClass="text-neut-in"
            />
        </div>
    );
};

export default StatusBar;
