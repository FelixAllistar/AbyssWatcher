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
    if (characters.length === 0) {
        return (
            <div id="selection-container">
                <div className="text-dim" style={{ padding: '4px' }}>No logs found.</div>
            </div>
        );
    }

    return (
        <div id="selection-container">
            {characters.map((char) => (
                <div className="char-row" key={char.path}>
                    <input
                        type="checkbox"
                        checked={char.tracked}
                        onChange={() => onToggle(char)}
                    />
                    {char.character}
                </div>
            ))}
        </div>
    );
};

export default CharacterSelector;
