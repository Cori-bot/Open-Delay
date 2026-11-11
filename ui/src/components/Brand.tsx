import type { CSSProperties } from 'react';

type BrandMarkProps = { size?: number; className?: string; style?: CSSProperties };

/** The mask is shared with the favicon, desktop icon and Stream Deck assets. */
export function BrandMark({ size = 26, className, style }: BrandMarkProps) {
  return <span aria-hidden="true" className={className} style={{
    display: 'inline-block', flex: 'none', width: size, height: size * 28 / 26,
    backgroundColor: 'currentColor',
    mask: 'url(/brand/open-delay-mark.svg) center / contain no-repeat',
    WebkitMask: 'url(/brand/open-delay-mark.svg) center / contain no-repeat', ...style,
  }} />;
}

export function Wordmark({ className = '', compact = false }: { className?: string; compact?: boolean }) {
  return <span className={`brand ${className}`} aria-label="Open-Delay">
    <BrandMark />
    {!compact && <span aria-hidden="true">open<span className="brand-light">delay</span><span className="brand-dot">.</span></span>}
  </span>;
}
