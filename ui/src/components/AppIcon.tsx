import {
  ArrowRight, Check, Clock3, Copy, Grid2X2, LayoutDashboard, MonitorPlay,
  Radio, Settings2, TriangleAlert, CircleHelp, Cable, X,
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';

const icons: Record<string, LucideIcon> = {
  dashboard: LayoutDashboard,
  routes: Radio,
  overlay: MonitorPlay,
  deck: Grid2X2,
  setup: Cable,
  help: CircleHelp,
  settings: Settings2,
  arrow: ArrowRight,
  check: Check,
  copy: Copy,
  clock: Clock3,
  close: X,
  warn: TriangleAlert,
};

export function AppIcon({ name, size = 19, className }: { name: string; size?: number; className?: string }) {
  const Icon = icons[name] ?? Clock3;
  return <Icon size={size} className={className} strokeWidth={1.6} aria-hidden="true" />;
}
