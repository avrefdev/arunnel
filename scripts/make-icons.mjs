// Uygulama ve tepsi ikonlarını üretir (harici bağımlılık yok).
// Kullanım: node scripts/make-icons.mjs  →  ardından: npx tauri icon src-tauri/icons/app.png
import { deflateSync } from "node:zlib";
import { writeFileSync } from "node:fs";

const crcTable = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
const crc = (buf) => {
  let c = 0xffffffff;
  for (const b of buf) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
};
const chunk = (type, data) => {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type), data]);
  const c = Buffer.alloc(4);
  c.writeUInt32BE(crc(body));
  return Buffer.concat([len, body, c]);
};
function png(size, pixel) {
  const raw = Buffer.alloc(size * (size * 4 + 1));
  const ss = 4; // kenar yumuşatma için alt örnekleme
  for (let y = 0; y < size; y++) {
    raw[y * (size * 4 + 1)] = 0;
    for (let x = 0; x < size; x++) {
      let r = 0, g = 0, b = 0, a = 0;
      for (let sy = 0; sy < ss; sy++)
        for (let sx = 0; sx < ss; sx++) {
          const p = pixel((x + (sx + 0.5) / ss) / size, (y + (sy + 0.5) / ss) / size);
          r += p[0] * p[3]; g += p[1] * p[3]; b += p[2] * p[3]; a += p[3];
        }
      const o = y * (size * 4 + 1) + 1 + x * 4;
      raw[o] = a ? r / a : 0; raw[o + 1] = a ? g / a : 0; raw[o + 2] = a ? b / a : 0;
      raw[o + 3] = (a / (ss * ss)) * 255;
    }
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0); ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8; ihdr[9] = 6;
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr), chunk("IDAT", deflateSync(raw)), chunk("IEND", Buffer.alloc(0)),
  ]);
}

// Güç simgesi: üstü açık halka + dikey çizgi. (u,v) 0..1, merkez 0.5
function powerGlyph(u, v, scale, stroke) {
  const x = (u - 0.5) / scale, y = (v - 0.5) / scale;
  const r = Math.hypot(x, y);
  const ang = Math.atan2(x, -y); // 0 = yukarı
  const ring = Math.abs(r - 0.3) < stroke / 2 && Math.abs(ang) > 0.62;
  const line = Math.abs(x) < stroke / 2 && y > -0.42 && y < -0.02;
  const cap = Math.hypot(x, y + 0.42) < stroke / 2 || Math.hypot(x, y + 0.02) < stroke / 2;
  return ring || line || cap;
}

function roundedSquare(u, v, radius) {
  const dx = Math.max(Math.abs(u - 0.5) - (0.5 - radius), 0);
  const dy = Math.max(Math.abs(v - 0.5) - (0.5 - radius), 0);
  return Math.hypot(dx, dy) <= radius;
}

const BG = [9, 9, 11], FG = [244, 244, 245];
const app = png(1024, (u, v) => {
  if (!roundedSquare(u, v, 0.22)) return [0, 0, 0, 0];
  return powerGlyph(u, v, 0.62, 0.1) ? [...FG, 1] : [...BG, 1];
});
writeFileSync("src-tauri/icons/app.png", app);

const tray = (color) =>
  png(32, (u, v) => (powerGlyph(u, v, 0.95, 0.14) ? [...color, 1] : [0, 0, 0, 0]));
writeFileSync("src-tauri/icons/tray-off.png", tray([161, 161, 170]));
writeFileSync("src-tauri/icons/tray-on.png", tray([52, 211, 153]));
writeFileSync("src-tauri/icons/tray-error.png", tray([248, 113, 113]));
console.log("ikonlar üretildi");
