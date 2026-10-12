import test from 'node:test';
import assert from 'node:assert/strict';
import { LOCALES, catalog, currentLocale, detectLocale, hooks, initLocale, setLocale, t, tMsg, tParts } from '../i18n.js';

// A missing key is a failure in tests, never a console warning.
hooks.onMissing = (key) => {
  throw new Error(`missing translation: ${key}`);
};

const placeholders = (text) => [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
const forms = (entry) => (typeof entry === 'string' ? { _: entry } : entry);

test('the saved choice wins, then the system language, then English', () => {
  assert.equal(detectLocale({ stored: 'ru', navigatorLanguage: 'en-US' }), 'ru');
  assert.equal(detectLocale({ stored: 'xx', navigatorLanguage: 'ru-RU' }), 'ru');
  assert.equal(detectLocale({ navigatorLanguage: 'en-GB' }), 'en');
  assert.equal(detectLocale({ navigatorLanguage: 'ru' }), 'ru');
  assert.equal(detectLocale({ navigatorLanguage: 'de-DE' }), 'en');
  assert.equal(detectLocale({}), 'en');
  assert.equal(detectLocale(), 'en');
});

test('setLocale switches the language, remembers it and survives broken storage', () => {
  const saved = {};
  globalThis.localStorage = { setItem: (k, v) => (saved[k] = v), getItem: (k) => saved[k] ?? null };
  setLocale('ru');
  assert.equal(currentLocale(), 'ru');
  assert.equal(saved['trellis.locale'], 'ru');
  assert.equal(initLocale(), 'ru');
  setLocale('en', { persist: false });
  assert.equal(saved['trellis.locale'], 'ru');
  setLocale('de'); // unknown: ignored
  assert.equal(currentLocale(), 'en');
  globalThis.localStorage = {
    setItem: () => {
      throw new Error('blocked');
    },
    getItem: () => {
      throw new Error('blocked');
    },
  };
  setLocale('ru');
  assert.equal(currentLocale(), 'ru');
  assert.equal(initLocale(), 'en'); // no storage, no navigator: English
  delete globalThis.localStorage;
});

test('parameters are filled in by name; an unknown one stays visible', () => {
  setLocale('en', { persist: false });
  assert.equal(t('home.balance_label', { balance: '5 USDC' }), 'Balance: 5 USDC');
  assert.equal(t('home.balance_label', {}), 'Balance: {balance}');
});

test('a parameter can be a message from Rust, translated in turn', () => {
  setLocale('en', { persist: false });
  const what = { key: 'op.send', params: { amount: '5 USDC' } };
  assert.equal(t('pending.banner', { what }).includes('“Sending 5 USDC”'), true);
  setLocale('ru', { persist: false });
  assert.equal(t('error.pending_blocks', { what }).includes('«Отправка 5 USDC»'), true);
  assert.equal(tMsg({ key: 'resolution.none' }), 'Ничего не ожидает проверки.');
});

test('plural forms follow the language', () => {
  const hours = (n) => t('payee.wait_hours', { n });
  setLocale('ru', { persist: false });
  assert.equal(hours(1), 'Можно будет через 1 час');
  assert.equal(hours(2), 'Можно будет через 2 часа');
  assert.equal(hours(5), 'Можно будет через 5 часов');
  assert.equal(hours(11), 'Можно будет через 11 часов');
  assert.equal(hours(21), 'Можно будет через 21 час');
  assert.equal(hours(48), 'Можно будет через 48 часов');
  setLocale('en', { persist: false });
  assert.equal(hours(1), 'Can be used in 1 hour');
  assert.equal(hours(2), 'Can be used in 2 hours');
  assert.equal(hours(48), 'Can be used in 48 hours');
});

test('parts mark which text came from a parameter', () => {
  setLocale('en', { persist: false });
  assert.deepEqual(tParts('home.left_today_of', { left: '70 USDC', limit: '100 USDC' }), [
    { text: 'You can send ' },
    { param: 'left', text: '70 USDC' },
    { text: ' more today out of ' },
    { param: 'limit', text: '100 USDC' },
  ]);
});

test('an unknown key is reported and shown as itself', () => {
  const seen = [];
  const before = hooks.onMissing;
  hooks.onMissing = (key) => seen.push(key);
  assert.equal(t('no.such.key'), 'no.such.key');
  hooks.onMissing = before;
  assert.deepEqual(seen, ['no.such.key']);
});

test('both catalogs have the same keys, none empty', () => {
  const en = Object.keys(catalog('en')).sort();
  const ru = Object.keys(catalog('ru')).sort();
  assert.deepEqual(ru, en);
  for (const name of LOCALES) {
    for (const [key, entry] of Object.entries(catalog(name))) {
      for (const text of Object.values(forms(entry))) assert.ok(text.trim() !== '', `${name}: ${key} is empty`);
    }
  }
});

test('every key has the same parameters in both languages', () => {
  for (const [key, enEntry] of Object.entries(catalog('en'))) {
    const enNames = new Set(Object.values(forms(enEntry)).flatMap(placeholders));
    for (const name of LOCALES) {
      for (const [form, text] of Object.entries(forms(catalog(name)[key]))) {
        assert.deepEqual([...new Set(placeholders(text))].sort(), [...enNames].sort(), `${name}: ${key} (${form})`);
      }
    }
  }
});

test('plural entries have every form their language needs', () => {
  for (const name of LOCALES) {
    const needed = new Intl.PluralRules(name).resolvedOptions().pluralCategories;
    for (const [key, entry] of Object.entries(catalog(name))) {
      if (typeof entry === 'string') continue;
      for (const category of needed) assert.ok(entry[category] ?? entry.other, `${name}: ${key} lacks ${category}`);
      assert.ok(entry.other, `${name}: ${key} lacks "other"`);
    }
  }
});
