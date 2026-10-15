#!/usr/bin/env node
// Fails when the interface has text outside the dictionaries (spec 005, D6):
//   1. es.js and en.js must have the same keys and the same {placeholders};
//   2. no JSX text or aria-label/title/placeholder/alt literal with words may
//      be written in a component instead of coming from t().
// Product names, commands and symbols are allowed as they are.
import { readdirSync, readFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ui = join(dirname(fileURLToPath(import.meta.url)), "..");
const src = join(ui, "src");
const problems = [];

const load = async (name) => (await import(pathToFileURL(join(src, "lib", "locales", `${name}.js`)).href)).default;
const [es, en] = await Promise.all([load("es"), load("en")]);
const placeholders = (text) => [...text.matchAll(/\{(\w+)\}/g)].map((match) => match[1]).sort().join(",");
for (const key of new Set([...Object.keys(es), ...Object.keys(en)])) {
  if (!(key in es)) problems.push(`es.js is missing ${key}`);
  else if (!(key in en)) problems.push(`en.js is missing ${key}`);
  else if (placeholders(es[key]) !== placeholders(en[key])) problems.push(`${key} has different placeholders in es and en`);
}

// Words that stay as they are in both languages.
const ALLOWED = new Set(["roudex", "router", "Codex Desktop", "router · auto", "spec"]);
const WORD = /[A-Za-zÁÉÍÓÚÑÜáéíóúñü]{2,}/;
const files = (dir) => readdirSync(dir, { withFileTypes: true }).flatMap((entry) =>
  entry.isDirectory() ? files(join(dir, entry.name)) : entry.name.endsWith(".jsx") ? [join(dir, entry.name)] : []);

for (const file of files(src)) {
  const source = readFileSync(file, "utf8");
  const where = (index) => `${relative(ui, file)}:${source.slice(0, index).split("\n").length}`;
  // Text between a tag's ">" and the next "<", not an arrow function's "=>".
  for (const match of source.matchAll(/(?<![=-])>([^<>{}\n`]+)</g)) {
    const text = match[1].trim();
    if (!WORD.test(text) || ALLOWED.has(text) || /&&|\|\||[=?;()]/.test(text)) continue;
    const tag = source.slice(0, match.index).match(/<([a-zA-Z][\w.]*)[^<>]*$/)?.[1];
    if (["code", "pre", "kbd"].includes(tag)) continue;
    problems.push(`${where(match.index)} untranslated text: ${text}`);
  }
  for (const match of source.matchAll(/\b(aria-label|title|placeholder|alt)="([^"]*)"/g)) {
    if (WORD.test(match[2]) && !ALLOWED.has(match[2])) problems.push(`${where(match.index)} untranslated ${match[1]}: ${match[2]}`);
  }
}

if (problems.length) {
  console.error(`Interface text outside the dictionaries:\n  ${problems.join("\n  ")}`);
  process.exit(1);
}
console.log(`i18n: ${Object.keys(es).length} keys in es and en; no text outside the dictionaries`);
