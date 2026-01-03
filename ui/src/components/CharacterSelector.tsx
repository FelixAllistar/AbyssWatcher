import { type FC } from 'react';

interface CharacterState {
    character: string;
    path: string;
    tracked: boolean;
}

interface CharacterSelectorProps {
    characters: CharacterState[];
    onToggle: (char: CharacterState) => void;
}

const CharacterSelector: FC<CharacterSelectorProps> = ({ characters, onToggle }) => {
    return (
        <div id="selection-container">
            {characters.length === 0 ? (
                <div className="text-dim" style={{ padding: '8px' }}>No logs found.</div>
            ) : (
                characters.map((char) => (
                    <div
                        className={`char-row ${!char.tracked ? 'untracked' : ''}`}
                        key={char.path}
                        onClick={() => onToggle(char)}
                    >
                        <input
                            type="checkbox"
                            checked={char.tracked}
                            onChange={(e) => {
                                // Prevent double toggle since outer div has onClick
                                e.stopPropagation();
                                onToggle(char);
                            }}
                        />
                        <span>{char.character}</span>
                    </div>
                ))
            )}
        </div>
    );
};

export default CharacterSelector;
