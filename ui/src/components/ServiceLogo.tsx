const services: Record<string, { file: string; color: string }> = {
  youtube: { file: 'youtube', color: '#ff0033' },
  twitch: { file: 'twitch', color: '#9146ff' },
  tiktok: { file: 'tiktok', color: '#ffffff' },
  kick: { file: 'kick', color: '#53fc18' },
  obs: { file: 'obsstudio', color: '#ffffff' },
  obsstudio: { file: 'obsstudio', color: '#ffffff' },
  streamlabs: { file: 'streamlabs', color: '#80f5d2' },
  streamdeck: { file: 'elgato', color: '#ffffff' },
  elgato: { file: 'elgato', color: '#ffffff' },
};

/** Decorative: the adjacent service name provides the accessible label. */
export function ServiceLogo({ service, size = 24, monochrome = false, className }: {
  service: string; size?: number; monochrome?: boolean; className?: string;
}) {
  const icon = services[service.toLowerCase().replace(/[^a-z]/g, '')];
  if (!icon) return <svg aria-hidden="true" className={className} width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3a15 15 0 0 1 0 18M12 3a15 15 0 0 0 0 18"/></svg>;
  const mask = `url(/brand/services/${icon.file}.svg) center / contain no-repeat`;
  return <span aria-hidden="true" className={className} style={{ display: 'inline-block', flex: 'none', width: size, height: size, backgroundColor: monochrome ? 'currentColor' : icon.color, mask, WebkitMask: mask }} />;
}
