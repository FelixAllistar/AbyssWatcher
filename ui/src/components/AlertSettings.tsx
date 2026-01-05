/**
 * AlertSettings component for configuring alert rules and character roles.
 * 
 * Allows users to:
 * - Designate characters as "logi" (squishy healer) or "neut-sensitive"
 * - Enable/disable individual alert rules
 */
import { type FC } from 'react';
import type { AlertEngineConfig, AlertRuleId, CharacterState } from '../types';

// Alert rule metadata for display
const ALERT_RULES: { id: AlertRuleId; name: string; description: string }[] = [
    {
        id: 'EnvironmentalDamage',
        name: 'Environmental Damage',
        description: 'Alert when taking damage from Unstable Abyssal Depths',
    },
    {
        id: 'FriendlyFire',
        name: 'Friendly Fire',
        description: 'Alert when tracked characters hit each other (excludes Vorton)',
    },
    {
        id: 'LogiTakingDamage',
        name: 'Logi Taking Damage',
        description: 'Alert when a designated logi character takes damage',
    },
    {
        id: 'NeutSensitiveNeuted',
        name: 'Neut Pressure',
        description: 'Alert when a neut-sensitive character is being neuted',
    },
    {
        id: 'CapacitorFailure',
        name: 'Capacitor Failure',
        description: 'Alert when a module fails to activate due to low cap',
    },
    {
        id: 'LogiNeuted',
        name: 'Logi Neuted',
        description: 'Alert when a designated logi character is neuted',
    },
];

interface AlertSettingsProps {
    config: AlertEngineConfig;
    trackedCharacters: CharacterState[];
    onChange: (config: AlertEngineConfig) => void;
}

const AlertSettings: FC<AlertSettingsProps> = ({ config, trackedCharacters, onChange }) => {
    // Get only tracked character names
    const trackedNames = trackedCharacters
        .filter(c => c.tracked)
        .map(c => c.character);

    const toggleLogiChar = (char: string) => {
        const newLogi = config.roles.logi_characters.includes(char)
            ? config.roles.logi_characters.filter(c => c !== char)
            : [...config.roles.logi_characters, char];

        onChange({
            ...config,
            roles: { ...config.roles, logi_characters: newLogi },
        });
    };

    const toggleNeutSensitiveChar = (char: string) => {
        const newNeut = config.roles.neut_sensitive_characters.includes(char)
            ? config.roles.neut_sensitive_characters.filter(c => c !== char)
            : [...config.roles.neut_sensitive_characters, char];

        onChange({
            ...config,
            roles: { ...config.roles, neut_sensitive_characters: newNeut },
        });
    };

    const toggleRule = (ruleId: AlertRuleId) => {
        const current = config.rules[ruleId];
        onChange({
            ...config,
            rules: {
                ...config.rules,
                [ruleId]: { ...current, enabled: !current.enabled },
            },
        });
    };

    return (
        <div className="alert-settings">
            <h3>Alert Settings</h3>

            {/* Character Roles */}
            {trackedNames.length > 0 && (
                <div className="alert-section">
                    <h4>Character Roles</h4>
                    <p className="help-text">Designate special roles for tracked characters</p>

                    <div className="char-roles-grid">
                        <div className="role-column">
                            <label>Logi (Squishy Healer)</label>
                            {trackedNames.map(char => (
                                <div key={`logi-${char}`} className="role-checkbox">
                                    <input
                                        type="checkbox"
                                        id={`logi-${char}`}
                                        checked={config.roles.logi_characters.includes(char)}
                                        onChange={() => toggleLogiChar(char)}
                                    />
                                    <label htmlFor={`logi-${char}`}>{char}</label>
                                </div>
                            ))}
                        </div>

                        <div className="role-column">
                            <label>Neut-Sensitive</label>
                            {trackedNames.map(char => (
                                <div key={`neut-${char}`} className="role-checkbox">
                                    <input
                                        type="checkbox"
                                        id={`neut-${char}`}
                                        checked={config.roles.neut_sensitive_characters.includes(char)}
                                        onChange={() => toggleNeutSensitiveChar(char)}
                                    />
                                    <label htmlFor={`neut-${char}`}>{char}</label>
                                </div>
                            ))}
                        </div>
                    </div>
                </div>
            )}

            {/* Alert Rules */}
            <div className="alert-section">
                <h4>Alert Rules</h4>

                {ALERT_RULES.map(rule => {
                    const ruleConfig = config.rules[rule.id];
                    const isEnabled = ruleConfig?.enabled ?? true;

                    return (
                        <div key={rule.id} className="alert-rule">
                            <label className="rule-toggle">
                                <input
                                    type="checkbox"
                                    checked={isEnabled}
                                    onChange={() => toggleRule(rule.id)}
                                />
                                <span className="rule-name">{rule.name}</span>
                            </label>
                            <p className="rule-description">{rule.description}</p>
                        </div>
                    );
                })}
            </div>
        </div>
    );
};

export default AlertSettings;
