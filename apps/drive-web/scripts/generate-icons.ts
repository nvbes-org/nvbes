import { deflateSync } from 'node:zlib';

function generatePng(width: number, height: number): Buffer {
  const header = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);

  const rawPixelData = Buffer.alloc((width * 4 + 1) * height);
  const cx = width / 2;
  const cy = height / 2;
  const barW = width * 0.42;
  const barH = height * 0.07;
  const sq = width * 0.13;

  for (let y = 0; y < height; y++) {
    const rowOffset = y * (width * 4 + 1);
    rawPixelData[rowOffset] = 0; // filter: none
    for (let x = 0; x < width; x++) {
      const px = rowOffset + 1 + x * 4;

      // Rounded rect background
      const inRect = x >= width * 0.1 && x <= width * 0.9 && y >= height * 0.1 && y <= height * 0.9;
      const inCorner =
        (x < width * 0.16 &&
          y < height * 0.16 &&
          Math.sqrt((x - width * 0.16) ** 2 + (y - height * 0.16) ** 2) > width * 0.06) ||
        (x > width * 0.84 &&
          y < height * 0.16 &&
          Math.sqrt((x - width * 0.84) ** 2 + (y - height * 0.16) ** 2) > width * 0.06) ||
        (x < width * 0.16 &&
          y > height * 0.84 &&
          Math.sqrt((x - width * 0.16) ** 2 + (y - height * 0.84) ** 2) > width * 0.06) ||
        (x > width * 0.84 &&
          y > height * 0.84 &&
          Math.sqrt((x - width * 0.84) ** 2 + (y - height * 0.84) ** 2) > width * 0.06);

      const isForeground =
        // top bar
        (Math.abs(y - (cy - barH * 1.5)) < barH && Math.abs(x - cx) < barW) ||
        // bottom bar
        (Math.abs(y - (cy + barH * 1.5)) < barH && Math.abs(x - cx) < barW) ||
        // small square bottom-right
        (x > cx && x < cx + sq && y > cy + barH * 0.5 && y < cy + barH * 0.5 + sq);

      if (inRect && !inCorner && isForeground) {
        rawPixelData[px] = 229; // R
        rawPixelData[px + 1] = 229; // G
        rawPixelData[px + 2] = 229; // B
        rawPixelData[px + 3] = 255; // A
      } else {
        rawPixelData[px] = 10; // R
        rawPixelData[px + 1] = 10; // G
        rawPixelData[px + 2] = 10; // B
        rawPixelData[px + 3] = 255; // A
      }
    }
  }

  const compressed = deflateSync(rawPixelData, { level: 9 });

  const ihdr = Buffer.alloc(25);
  ihdr.writeUInt32BE(13, 0); // length
  ihdr.write('IHDR', 4);
  ihdr.writeUInt32BE(width, 8);
  ihdr.writeUInt32BE(height, 12);
  ihdr[16] = 8; // bit depth
  ihdr[17] = 6; // color type: RGBA
  ihdr[18] = 0; // compression
  ihdr[19] = 0; // filter
  ihdr[20] = 0; // interlace
  const ihdrCrc = crc32(ihdr.subarray(4, 21));
  ihdr.writeInt32BE(ihdrCrc, 21);

  const idatHeader = Buffer.alloc(8);
  idatHeader.writeUInt32BE(compressed.length, 0);
  idatHeader.write('IDAT', 4);
  const idatCrc = crc32(Buffer.concat([idatHeader.subarray(4, 8), compressed]));
  const idatFooter = Buffer.alloc(4);
  idatFooter.writeInt32BE(idatCrc, 0);

  const iendHeader = Buffer.alloc(8);
  iendHeader.writeUInt32BE(0, 0);
  iendHeader.write('IEND', 4);
  const iendCrc = crc32(iendHeader.subarray(4, 8));
  const iendFooter = Buffer.alloc(4);
  iendFooter.writeInt32BE(iendCrc, 0);

  return Buffer.concat([header, ihdr, idatHeader, compressed, idatFooter, iendHeader, iendFooter]);
}

let crcTable: Int32Array | null = null;

function buildCrcTable() {
  crcTable = new Int32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) {
      c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    }
    crcTable[n] = c;
  }
}

function crc32(data: Buffer): number {
  if (!crcTable) buildCrcTable();
  const table = crcTable;
  if (!table) {
    throw new Error('CRC table not initialized');
  }
  let crc = -1;
  for (let i = 0; i < data.length; i++) {
    crc = table[(crc ^ data[i]) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ -1) >>> 0;
}

import { writeFileSync } from 'node:fs';

writeFileSync('public/icon-192.png', generatePng(192, 192));
writeFileSync('public/icon-512.png', generatePng(512, 512));
console.log('Generated icon-192.png and icon-512.png');
