import React, { useState } from 'react';

interface TooltipProps {
  text: string;
  children: React.ReactNode;
  position?: 'top' | 'bottom';
  align?: 'left' | 'right' | 'center';
}

const Tooltip: React.FC<TooltipProps> = ({ text, children, position = 'bottom', align = 'right' }) => {
  const [isVisible, setIsVisible] = useState(false);

  return (
    <div
      className="tooltip-container"
      onMouseEnter={() => setIsVisible(true)}
      onMouseLeave={() => setIsVisible(false)}
      onFocus={() => setIsVisible(true)}
      onBlur={() => setIsVisible(false)}
    >
      {children}
      {isVisible && (
        <div className={`tooltip-content tooltip-${position} tooltip-align-${align}`}>
          {text}
        </div>
      )}
    </div>
  );
};

export default Tooltip;
