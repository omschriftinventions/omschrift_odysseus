// Generate Expo icon/splash assets from the Odysseus app icon (oms_odysseus.png).
import sharp from 'sharp';
import { mkdirSync } from 'fs';

const A = 'assets';
const SRC = 'assets/branding/icon_source.png'; // 1254x1254 finished icon
const NAVY = { r: 0x0b, g: 0x1b, b: 0x34, alpha: 1 }; // splash backdrop
const TRANSPARENT = { r: 0, g: 0, b: 0, alpha: 0 };
mkdirSync(A, { recursive: true });

// iOS / legacy square icon: the source is already a finished rounded icon.
async function icon() {
  await sharp(SRC).resize(1024, 1024, { fit: 'cover' }).png().toFile(`${A}/icon.png`);
  await sharp(SRC).resize(48, 48).png().toFile(`${A}/favicon.png`);
}

// Native static splash icon: logo centered on navy (shown before JS/video loads).
async function splash() {
  const inner = 480;
  const logo = await sharp(SRC).resize(inner, inner, { fit: 'contain', background: TRANSPARENT }).png().toBuffer();
  await sharp({ create: { width: 1242, height: 1242, channels: 4, background: NAVY } })
    .composite([{ input: logo, gravity: 'center' }])
    .png()
    .toFile(`${A}/splash-icon.png`);
}

// Android adaptive icon: logo in safe zone over solid navy.
async function adaptive() {
  const size = 1024;
  const inner = 620; // ~66% safe zone
  const logo = await sharp(SRC).resize(inner, inner, { fit: 'contain', background: TRANSPARENT }).png().toBuffer();
  await sharp({ create: { width: size, height: size, channels: 4, background: TRANSPARENT } })
    .composite([{ input: logo, gravity: 'center' }])
    .png()
    .toFile(`${A}/android-icon-foreground.png`);
  await sharp({ create: { width: size, height: size, channels: 4, background: NAVY } })
    .png()
    .toFile(`${A}/android-icon-background.png`);
  await sharp({ create: { width: size, height: size, channels: 4, background: TRANSPARENT } })
    .composite([{ input: logo, gravity: 'center' }])
    .png()
    .toFile(`${A}/android-icon-monochrome.png`);
}

await icon();
await splash();
await adaptive();
console.log('assets generated from', SRC);
