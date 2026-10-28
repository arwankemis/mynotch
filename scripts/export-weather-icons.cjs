// Offline only. npm install --no-save sharp, or provide its installed path as argv[2].
// Never runs as part of mynotch or its normal Rust build.
const fs = require('fs');
const path = require('path');
const sharp = require(process.argv[2] || 'sharp');
const root = path.resolve(__dirname, '../source/assets/weather');
// Source sheet indices, in the same order as weather_icons::Icon.
const cells = [0, 1, 34, 35, 3, 4, 5, 14, 8, 9, 10, 11, 12, 15, 16, 17, 18, 20, 6, 13, 27];
async function exportStyle(source, output, tile, pitch, top, blackMatte = false) {
    const svg = fs.readFileSync(path.join(root, source), 'utf8');
    // Render at the embedded artwork's original 2x resolution. The gradient
    // sheet has an opaque black matte; the compact sheet already has alpha.
    let sheet = await sharp(Buffer.from(svg), { density: 144 }).png().toBuffer();
    pitch *= 2; top *= 2;
    if (blackMatte) {
        const { data, info } = await sharp(sheet).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
        const original = Buffer.from(data), { width, height } = info;
        for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
            const i = (y * width + x) * 4;
            const m = Math.max(...original.subarray(i, i + 3));
            if (!m) { data[i + 3] = 0; continue; }
            // Unmatte only the narrow antialiased contour; keep interior gradients.
            let touchesBlack = false, peak = m;
            for (let dy = -2; dy <= 2; dy++) for (let dx = -2; dx <= 2; dx++) {
                const xx = x + dx, yy = y + dy;
                if (xx < 0 || yy < 0 || xx >= width || yy >= height) { touchesBlack = true; continue; }
                const j = (yy * width + xx) * 4;
                const v = Math.max(...original.subarray(j, j + 3));
                touchesBlack ||= v === 0; peak = Math.max(peak, v);
            }
            if (touchesBlack) {
                const alpha = m / peak;
                data[i + 3] = Math.round(alpha * 255);
                for (let c = 0; c < 3; c++) data[i + c] = Math.min(255, Math.round(original[i + c] / alpha));
            }
        }
        sheet = await sharp(data, { raw: info }).png().toBuffer();
    }
    const meta = await sharp(sheet).metadata();
    const icons = [];
    for (let index = 0; index < cells.length; index++) {
        const cell = cells[index], left = (cell % 8) * pitch, y = top + Math.floor(cell / 8) * pitch;
        const frame = await sharp(sheet).extract({ left, top: y, width: Math.min(pitch, meta.width - left), height: Math.min(pitch, meta.height - y) }).png().toBuffer();
        const crop = await sharp(frame).trim({ threshold: 1 }).png().toBuffer();
        const inner = Math.floor(tile * 0.88 / 2) * 2;
        const icon = await sharp(crop).resize(inner, inner, { fit: 'contain', background: '#00000000' }).extend({ top: (tile - inner) / 2, bottom: (tile - inner) / 2, left: (tile - inner) / 2, right: (tile - inner) / 2, background: '#00000000' }).png().toBuffer();
        icons.push({ input: icon, left: (index % 7) * tile, top: Math.floor(index / 7) * tile });
    }
    await sharp({ create: { width: tile * 7, height: tile * 3, channels: 4, background: '#00000000' } }).composite(icons).png({ compressionLevel: 9, palette: true, colours: 256, dither: 0 }).toFile(path.join(root, output));
    console.log(output, fs.statSync(path.join(root, output)).size, 'bytes');
}
(async () => {
    await exportStyle('gradient-source.svg', 'gradient.png', 96, 112, 0, true);
    await exportStyle('compact-source.svg', 'compact.png', 40, 100, 153);
})().catch(error => { console.error(error); process.exitCode = 1; });
