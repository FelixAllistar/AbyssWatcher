import { type FC } from 'react';

interface DpsData {
    outgoing_dps: number;
    incoming_dps: number;
    outgoing_hps: number;
    incoming_hps: number;
    outgoing_cap: number;
    incoming_cap: number;
    outgoing_neut: number;
    incoming_neut: number;
}

interface StatusBarProps {
    data: DpsData | null;
}

interface StatPairProps {
    label: string;
    outValue: number;
    inValue: number;
    outClass: string;
    inClass: string;
}

const StatPair: FC<StatPairProps> = ({ label, outValue, inValue, outClass, inClass }) => (
    <div className="stat-pair">
        <span className="stat-label">{label}</span>
        <div className="stat-values">
            <div className={`val-group ${outClass}`}>
                <span className="dir-icon">↗</span>
                <span className="value">{outValue.toFixed(1)}</span>
            </div>
            <div className="stat-divider"></div>
            <div className={`val-group ${inClass}`}>
                <span className="value">{inValue.toFixed(1)}</span>
                <span className="dir-icon">↙</span>
            </div>
        </div>
    </div>
);

const StatusBar: FC<StatusBarProps> = ({ data }) => {
    const d = data || {
        outgoing_dps: 0,
        incoming_dps: 0,
        outgoing_hps: 0,
        incoming_hps: 0,
        outgoing_cap: 0,
        incoming_cap: 0,
        outgoing_neut: 0,
        incoming_neut: 0,
    };

    return (
        <div className="status-bar">
            <StatPair
                label="DPS"
                outValue={d.outgoing_dps}
                inValue={d.incoming_dps}
                outClass="text-dps-out"
                inClass="text-dps-in"
            />
            <StatPair
                label="REP"
                outValue={d.outgoing_hps}
                inValue={d.incoming_hps}
                outClass="text-rep-out"
                inClass="text-rep-in"
            />
            <StatPair
                label="CAP"
                outValue={d.outgoing_cap}
                inValue={d.incoming_cap}
                outClass="text-cap-out"
                inClass="text-cap-in"
            />
            <StatPair
                label="NEUT"
                outValue={d.outgoing_neut}
                inValue={d.incoming_neut}
                outClass="text-neut-out"
                inClass="text-neut-in"
            />
        </div>
    );
};

export default StatusBar;
