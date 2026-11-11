import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { Resvg } from '@resvg/resvg-js';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const brand = path.join(root, 'ui', 'public', 'brand');
const source = await readFile(path.join(brand, 'open-delay-mark.svg'), 'utf8');
const content = source.replace(/^<svg\b[^>]*>/, '').replace(/<\/svg>\s*$/, '');

function render(size) {
  // Rounded plate is specific to app icons; the mark itself stays identical.
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="${size}" height="${size}" color="#ededed"><rect width="256" height="256" rx="56" fill="#111"/><g transform="translate(50 44) scale(6)">${content}</g></svg>`;
  return Buffer.from(new Resvg(svg).render().asPng());
}

function ico(images) {
  let offset = 6 + images.length * 16;
  const header = Buffer.alloc(offset);
  header.writeUInt16LE(1, 2); header.writeUInt16LE(images.length, 4);
  images.forEach(({ size, png }, index) => {
    const pos = 6 + index * 16;
    header[pos] = size === 256 ? 0 : size; header[pos + 1] = size === 256 ? 0 : size;
    header.writeUInt16LE(1, pos + 4); header.writeUInt16LE(32, pos + 6);
    header.writeUInt32LE(png.length, pos + 8); header.writeUInt32LE(offset, pos + 12);
    offset += png.length;
  });
  return Buffer.concat([header, ...images.map(({ png }) => png)]);
}

const images = [16, 24, 32, 48, 64, 128, 256].map(size => ({ size, png: render(size) }));
await writeFile(path.join(root, 'desktop', 'icons', 'icon.png'), images.at(-1).png);
await writeFile(path.join(root, 'desktop', 'icons', 'icon.ico'), ico(images));
await writeFile(path.join(brand, 'marketplace-144.png'), render(144));
await writeFile(path.join(brand, 'marketplace-288.png'), render(288));
