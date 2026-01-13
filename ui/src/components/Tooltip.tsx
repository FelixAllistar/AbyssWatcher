import React, { useState } from 'react';

interface TooltipProps {
  text: string;
  children: React.ReactNode;
  position?: 'top' | 'bottom';
  align?: 'left' | 'right' | 'center';
}

const Tooltip: React.FC<TooltipProps> = ({ text, children, position = 'bottom', align = 'right' }) => {
  const [isVisible, setIsVisible] = useState(false);
  const tooltipId = React.useId();

  // Clone child to add aria-describedby for accessibility
  const child = React.isValidElement(children)
    ? React.cloneElement(children as React.ReactElement<any>, {
        'aria-describedby': isVisible ? tooltipId : undefined
      })
    : children;

  return (
    <div
      className="tooltip-container"
      onMouseEnter={() => setIsVisible(true)}
      onMouseLeave={() => setIsVisible(false)}
      onFocus={() => setIsVisible(true)}
      onBlur={() => setIsVisible(false)}
    >
      {child}
      {isVisible && (
        <div
          id={tooltipId}
          role="tooltip"
          className={`tooltip-content tooltip-${position} tooltip-align-${align}`}
        >
          {text}
        </div>
      )}
    </div>
  );
};

export default Tooltip;
