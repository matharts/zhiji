#!/usr/bin/env node
// 审计独立的 Wikidata/OSM 快照；输出仅供审阅，不作为应用地点目录。
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve, dirname } from 'node:path';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const input = process.argv[2] ?? 'tools/data-sources/overseas-candidates-2026-11-11.json';
const output = process.argv[3];
const snapshot = JSON.parse(readFileSync(resolve(root, input), 'utf8'));
const countryName = Object.fromEntries(JSON.parse(readFileSync(resolve(root, 'assets/data/birthplace-countries.json'), 'utf8')).countries.map(x => [x.code, x.name]));
const supplemental = new Map();
for (const item of snapshot.supplements) {
  if (!supplemental.has(item.qid)) supplemental.set(item.qid, []);
  supplemental.get(item.qid).push(item);
}
const seen = new Set();
const results = [];
for (const place of snapshot.candidates) {
  if (!/^Q[1-9]\d*$/.test(place.id) || seen.has(place.id)) throw Error(`Invalid or duplicate QID: ${place.id}`);
  seen.add(place.id);
  if (!Number.isFinite(place.lat) || place.lat < -90 || place.lat > 90 || !Number.isFinite(place.lon) || place.lon < -180 || place.lon > 180) throw Error(`Invalid coordinates: ${place.id}`);
  if (!Array.isArray(place.countries) || !Array.isArray(place.types)) throw Error(`Invalid claims: ${place.id}`);
  const ids = [...new Set(place.countries)];
  const country = ids.length === 1 ? snapshot.countries[ids[0]] : null;
  const reasons = [];
  if (ids.length === 0) reasons.push('no-current-country');
  else if (ids.length > 1) reasons.push('multiple-current-country-claims');
  else if (!country || !countryName[country]) reasons.push('unmapped-country');
  const geographicCountries = place.geographicCountries ?? [];
  const capitalFor = place.capitalFor ?? [];
  if (geographicCountries.length === 0) reasons.push('no-geographic-country');
  else if (geographicCountries.length > 1) reasons.push('multiple-geographic-countries');
  else if (country && geographicCountries[0] !== country) reasons.push('country-coordinate-conflict');
  const geographicRegions = place.geographicRegions ?? [];
  const timezoneCandidates = place.timezoneCandidates ?? [];
  if (!geographicRegions.length) reasons.push('no-geographic-region');
  if (!timezoneCandidates.length) reasons.push('no-timezone-candidate');
  const matches = supplemental.get(place.id) ?? [];
  const names = [...new Set(matches.map(x => x.name).filter(x => /\p{Script=Han}/u.test(x)))];
  const sourceName = place.zh && /\p{Script=Han}/u.test(place.zh) ? place.zh : null;
  const zh = sourceName ?? (names.length === 1 ? names[0] : null);
  if (!zh) reasons.push(names.length > 1 ? 'conflicting-osm-names' : 'missing-chinese-name');
  // 上级行政关系和历史时区归属尚未核验。
  reasons.push('hierarchy-unverified', 'timezone-unverified');
  results.push({ id: place.id, country: country ?? null, geographicCountries, geographicRegions, timezoneCandidates, capitalFor, english: place.name, chinese: zh, longitude: place.lon, latitude: place.lat, population: place.population, parent: place.parent, reasons });
}
const groups = new Map();
for (const item of results) {
  const code = item.country ?? 'unresolved';
  const g = groups.get(code) ?? { count: 0, chinese: 0, unresolved: 0 };
  g.count++; if (item.chinese) g.chinese++; if (item.reasons.length) g.unresolved++;
  groups.set(code, g);
}
const summary = { total: results.length, countryGroups: Object.fromEntries([...groups].sort(([a], [b]) => a.localeCompare(b))), conflicts: results.filter(x => x.reasons.length).length };
if (output) writeFileSync(resolve(output), JSON.stringify({ summary, candidates: results }, null, 2) + '\n');
console.log(JSON.stringify(summary, null, 2));
