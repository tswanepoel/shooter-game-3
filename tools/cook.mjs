#!/usr/bin/env node
/**
 * Thin asset cook: gather source kits → hashed packs under assets/cooked.
 * Vite publicDir; sole ship tree is web/dist.
 *
 * V1 packs by demand cadence (not authoring kit layout):
 *   kenney-core — character + blaster kits for lineup / play
 *
 * Pack format (SGPK v1): magic + JSON header + concatenated raw files (glb/png/wav as-is).
 * No custom GPU formats; loaders still decode glTF/PNG / Web Audio decodes wav.
 */

import * as crypto from 'node:crypto';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, '..');
const sourceRoot = path.join(root, 'assets', 'source');
const cookedRoot = path.join(root, 'assets', 'cooked');
const packsDir = path.join(cookedRoot, 'packs');
const stampPath = path.join(cookedRoot, '.cook-stamp');

const MAGIC = Buffer.from('SGPK');
const PACK_VERSION = 1;

/** @typedef {{ id: string; path: string; kind: string }} AssetEntry */
/** @typedef {{ id: string; sources: string[]; assets: () => AssetEntry[] }} PackDef */

/** @type {PackDef[]} */
const PACKS = [
  {
    id: 'kenney-core',
    sources: ['characters', 'blasters'],
    assets() {
      const letters = 'abcdefghijklmnopqr';
      /** @type {AssetEntry[]} */
      const out = [];
      for (const ch of letters) {
        out.push({
          id: `character-${ch}.mesh`,
          path: path.join(sourceRoot, 'characters', 'models', `character-${ch}.glb`),
          kind: 'glb',
        });
        out.push({
          id: `character-${ch}.albedo`,
          path: path.join(sourceRoot, 'characters', 'textures', `texture-${ch}.png`),
          kind: 'png',
        });
      }
      for (const ch of letters) {
        out.push({
          id: `blaster-${ch}.mesh`,
          path: path.join(sourceRoot, 'blasters', 'models', `blaster-${ch}.glb`),
          kind: 'glb',
        });
      }
      out.push({
        id: 'blaster.colormap',
        path: path.join(sourceRoot, 'blasters', 'textures', 'colormap.png'),
        kind: 'png',
      });
      return out;
    },
  },
  {
    id: 'maps-a',
    sources: ['map-a.json'],
    assets() {
      return [
        {
          id: 'map-a.def',
          path: path.join(sourceRoot, 'map-a.json'),
          kind: 'json',
        },
      ];
    },
  },
  {
    id: 'sfx',
    sources: ['sfx'],
    assets() {
      return [
        {
          id: 'bang.wav',
          path: path.join(sourceRoot, 'sfx', 'bang.wav'),
          kind: 'wav',
        },
      ];
    },
  },
];

function sha256File(filePath) {
  const h = crypto.createHash('sha256');
  h.update(fs.readFileSync(filePath));
  return h.digest('hex');
}

function sha256Buf(buf) {
  return crypto.createHash('sha256').update(buf).digest('hex');
}

function sourceFingerprint() {
  /** @type {string[]} */
  const parts = [];
  for (const pack of PACKS) {
    parts.push(`pack:${pack.id}`);
    for (const asset of pack.assets()) {
      if (!fs.existsSync(asset.path)) {
        throw new Error(`missing source asset ${asset.id}: ${asset.path}`);
      }
      const st = fs.statSync(asset.path);
      parts.push(`${asset.id}:${st.size}:${Math.trunc(st.mtimeMs)}:${sha256File(asset.path)}`);
    }
  }
  // Include cook script itself so format changes force a rebuild.
  const cookScript = path.join(root, 'tools', 'cook.mjs');
  parts.push(`cook:${sha256File(cookScript)}`);
  return sha256Buf(Buffer.from(parts.join('\n'), 'utf8'));
}

/**
 * @param {AssetEntry[]} assets
 * @returns {{ buffer: Buffer, header: object }}
 */
function buildPack(assets) {
  /** @type {{ id: string; kind: string; offset: number; size: number }[]} */
  const entries = [];
  /** @type {Buffer[]} */
  const chunks = [];
  let offset = 0;

  for (const asset of assets) {
    const data = fs.readFileSync(asset.path);
    entries.push({
      id: asset.id,
      kind: asset.kind,
      offset,
      size: data.length,
    });
    chunks.push(data);
    offset += data.length;
  }

  const headerObj = {
    version: PACK_VERSION,
    assets: entries,
  };
  const headerJson = Buffer.from(JSON.stringify(headerObj), 'utf8');
  if (headerJson.length > 0xffff_ffff) {
    throw new Error('pack header too large');
  }

  const headerLen = Buffer.alloc(4);
  headerLen.writeUInt32LE(headerJson.length, 0);

  const payload = Buffer.concat(chunks);
  const buffer = Buffer.concat([MAGIC, headerLen, headerJson, payload]);
  return { buffer, header: headerObj };
}

function writePackFile(packId, buffer) {
  const hash = sha256Buf(buffer);
  const short = hash.slice(0, 16);
  const fileName = `${packId}.${short}.sgpk`;
  const abs = path.join(packsDir, fileName);
  fs.writeFileSync(abs, buffer);
  return {
    id: packId,
    url: `/packs/${fileName}`,
    hash,
    size: buffer.length,
  };
}

function cleanPacksExcept(keepNames) {
  if (!fs.existsSync(packsDir)) return;
  for (const name of fs.readdirSync(packsDir)) {
    if (!keepNames.has(name)) {
      fs.unlinkSync(path.join(packsDir, name));
    }
  }
}

function cook() {
  const fp = sourceFingerprint();
  if (fs.existsSync(stampPath) && fs.existsSync(path.join(cookedRoot, 'manifest.json'))) {
    const prev = fs.readFileSync(stampPath, 'utf8').trim();
    if (prev === fp) {
      console.log('[cook] sources unchanged — skip');
      return;
    }
  }

  fs.mkdirSync(packsDir, { recursive: true });

  /** @type {{ id: string; url: string; hash: string; size: number }[]} */
  const packRecords = [];
  /** @type {Set<string>} */
  const keep = new Set();

  for (const pack of PACKS) {
    const assets = pack.assets();
    console.log(`[cook] pack ${pack.id}: ${assets.length} assets`);
    const { buffer } = buildPack(assets);
    const record = writePackFile(pack.id, buffer);
    packRecords.push(record);
    keep.add(path.basename(record.url));
    console.log(
      `[cook]   → ${record.url} (${record.size} bytes, sha256 ${record.hash.slice(0, 12)}…)`,
    );
  }

  cleanPacksExcept(keep);

  const manifest = {
    version: 1,
    packs: packRecords,
  };
  const manifestPath = path.join(cookedRoot, 'manifest.json');
  fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
  fs.writeFileSync(stampPath, `${fp}\n`, 'utf8');
  console.log(`[cook] wrote ${path.relative(root, manifestPath).replace(/\\/g, '/')}`);
}

try {
  cook();
} catch (err) {
  console.error('[cook] failed:', err instanceof Error ? err.message : err);
  process.exit(1);
}
