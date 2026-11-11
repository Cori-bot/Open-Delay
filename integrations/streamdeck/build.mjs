import { cp, mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const root = path.dirname(fileURLToPath(import.meta.url));
const plugin = path.join(root, 'dev.opendelay.sdPlugin');
await mkdir(path.join(plugin, 'bin'), { recursive: true });
await cp(path.join(root, 'src'), path.join(plugin, 'bin'), { recursive: true });
await mkdir(path.join(plugin, 'imgs'), { recursive: true });
// The marketplace and category icons share the exact app mark.
const brand = path.resolve(root, '../../ui/public/brand');
await cp(path.join(brand, 'marketplace-144.png'), path.join(plugin, 'imgs', 'marketplace.png'));
await cp(path.join(brand, 'marketplace-288.png'), path.join(plugin, 'imgs', 'marketplace@2x.png'));
await cp(path.join(brand, 'open-delay-mark.svg'), path.join(plugin, 'imgs', 'navigation.svg'));
for (const [name, symbol] of [['delay', 'M12 5a7 7 0 1 1-7 7M5 5v7h7'], ['live', 'm9 6 9 6-9 6Z'], ['toggle', 'M5 8h14m-4-4 4 4-4 4M19 16H5m4-4-4 4 4 4'], ['status', 'M5 17v-5m5 5V7m5 10v-7m5 7V4']]) {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="144" height="144" viewBox="0 0 24 24"><rect width="24" height="24" rx="5" fill="#111"/><path d="${symbol}" fill="none" stroke="#ededed" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
  await writeFile(path.join(plugin, 'imgs', `${name}.svg`), svg);
}
