import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { catalog } from '../i18n.js';

const ui = join(dirname(fileURLToPath(import.meta.url)), '..');
const rustSrc = join(ui, '..', 'src-tauri', 'src');

// Catalog keys are written as plain string literals everywhere, so a scan finds
// them. A key starts with one of the catalog's own sections.
const SECTIONS = [
  'tabs', 'nav', 'header', 'vault', 'common', 'pending', 'first', 'pair', 'waiting', 'open', 'address',
  'faucet', 'home', 'deposit', 'time', 'send', 'people', 'approvals', 'me', 'error', 'field', 'notice',
  'op', 'payee', 'history', 'resolution',
];
const KEY = new RegExp(`(["'\`])((?:${SECTIONS.join('|')})(?:\\.[a-z0-9_]+)+)\\1`, 'g');

function files(dir, extension) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    const path = join(dir, e.name);
    if (e.isDirectory()) return e.name === 'test' || e.name === 'tools' || e.name === 'locales' ? [] : files(path, extension);
    return extname(e.name) === extension ? [path] : [];
  });
}

function keysIn(paths, { skipMock = false, cutTests = false } = {}) {
  const found = new Set();
  for (const path of paths) {
    if (skipMock && path.endsWith('mock-api.js')) continue;
    let text = readFileSync(path, 'utf8');
    if (cutTests) text = text.split('#[cfg(test)]')[0];
    for (const m of text.matchAll(KEY)) found.add(m[2]);
  }
  return found;
}

const defined = new Set(Object.keys(catalog('en')));
const jsFiles = files(ui, '.js');
const rustFiles = files(rustSrc, '.rs');

test('every key the screens use is in the catalog, in both languages', () => {
  const used = keysIn(jsFiles);
  assert.ok(used.size > 80, `the scan found only ${used.size} keys`);
  for (const key of used) {
    assert.ok(defined.has(key), `used but not in the catalog: ${key}`);
    assert.ok(key in catalog('ru'), `used but not in the Russian catalog: ${key}`);
  }
});

test('every key Rust sends is in the catalog', () => {
  const used = keysIn(rustFiles, { cutTests: true });
  assert.ok(used.size > 50, `the scan found only ${used.size} keys`);
  for (const key of used) assert.ok(defined.has(key), `sent by Rust but not in the catalog: ${key}`);
});

test('no catalog key is left unused', () => {
  const used = new Set([...keysIn(jsFiles, { skipMock: true }), ...keysIn(rustFiles, { cutTests: true })]);
  const unused = [...defined].filter((key) => !used.has(key));
  assert.deepEqual(unused, []);
});
