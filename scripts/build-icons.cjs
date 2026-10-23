// npm install --no-save sharp, or provide it through NODE_PATH.
// Every application and website icon is derived from this one SVG source.
const fs = require('node:fs');
const path = require('node:path');
const sharp = require('sharp');
const root = path.resolve(__dirname, '..');

(async () => {
  const source = path.join(root, 'resources/logo.svg');
  const sizes = [16, 20, 24, 32, 40, 48, 64, 128, 256];
  const frames = await Promise.all(sizes.map(size => sharp(source).resize(size, size).png().toBuffer()));
  const header = Buffer.alloc(6 + frames.length * 16);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(frames.length, 4);
  let offset = header.length;
  frames.forEach((frame, i) => {
    const entry = 6 + i * 16;
    header[entry] = sizes[i] === 256 ? 0 : sizes[i];
    header[entry + 1] = header[entry];
    header.writeUInt16LE(1, entry + 4);
    header.writeUInt16LE(32, entry + 6);
    header.writeUInt32LE(frame.length, entry + 8);
    header.writeUInt32LE(offset, entry + 12);
    offset += frame.length;
  });
  fs.writeFileSync(path.join(root, 'resources/logo.ico'), Buffer.concat([header, ...frames]));
  await sharp(source).resize(256, 256).png().toFile(path.join(root, 'resources/logo.png'));
  for (const directory of ['docs/assets', 'ui/assets']) {
    fs.mkdirSync(path.join(root, directory), {recursive:true});
    for (const file of ['logo.svg', 'logo.ico', 'logo.png']) {
      fs.copyFileSync(path.join(root, 'resources', file), path.join(root, directory, file));
    }
  }
  console.log('Generated 9 ICO sizes, PNG and SVG assets for application and website');
})().catch(error => { console.error(error); process.exitCode = 1; });
