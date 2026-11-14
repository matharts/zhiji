#!/usr/bin/env node
// 审计独立的 Wikidata/OSM 快照；输出仅供审阅，不作为应用地点目录。
import { writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import {
  COUNTRIES_PATH,
  placesRoot,
  readJson,
} from './catalog.ts';
import { snapshotPath } from './pipeline.ts';
import { runCli } from '../cli.ts';

interface Candidate {
  id: string;
  lat: number;
  lon: number;
  countries: string[];
  types: string[];
  geographicCountries?: string[];
  geographicRegions?: string[];
  timezoneCandidates?: string[];
  capitalFor?: string[];
  name?: string;
  zh?: string;
  population?: number;
  parent?: string;
}

interface CandidateSnapshot {
  countries: Record<string, string>;
  candidates: Candidate[];
  supplements: { qid: string; name: string }[];
}

/** 生成仅用于审阅的候选摘要，不声称已可发布。 */
export function auditCandidates(snapshot: CandidateSnapshot, countryNames: Record<string, string>): {
  summary: { total: number; countryGroups: Record<string, { count: number; chinese: number; unresolved: number }>; conflicts: number };
  candidates: Record<string, unknown>[];
} {
  const supplemental = new Map<string, string[]>();
  for (const item of snapshot.supplements) {
    const names = supplemental.get(item.qid) ?? [];
    names.push(item.name);
    supplemental.set(item.qid, names);
  }
  const seen = new Set<string>();
  const results: Record<string, unknown>[] = [];
  for (const place of snapshot.candidates) {
    if (!/^Q[1-9]\d*$/.test(place.id) || seen.has(place.id)) throw Error(`Invalid or duplicate QID: ${place.id}`);
    seen.add(place.id);
    if (!Number.isFinite(place.lat) || place.lat < -90 || place.lat > 90 || !Number.isFinite(place.lon) || place.lon < -180 || place.lon > 180) {
      throw Error(`Invalid coordinates: ${place.id}`);
    }
    if (!Array.isArray(place.countries) || !Array.isArray(place.types)) throw Error(`Invalid claims: ${place.id}`);
    const ids = [...new Set(place.countries)];
    const country = ids.length === 1 ? snapshot.countries[ids[0]] : null;
    const reasons: string[] = [];
    if (ids.length === 0) reasons.push('no-current-country');
    else if (ids.length > 1) reasons.push('multiple-current-country-claims');
    else if (!country || !countryNames[country]) reasons.push('unmapped-country');
    const geographicCountries = place.geographicCountries ?? [];
    const capitalFor = place.capitalFor ?? [];
    if (geographicCountries.length === 0) reasons.push('no-geographic-country');
    else if (geographicCountries.length > 1) reasons.push('multiple-geographic-countries');
    else if (country && geographicCountries[0] !== country) reasons.push('country-coordinate-conflict');
    const geographicRegions = place.geographicRegions ?? [];
    const timezoneCandidates = place.timezoneCandidates ?? [];
    if (!geographicRegions.length) reasons.push('no-geographic-region');
    if (!timezoneCandidates.length) reasons.push('no-timezone-candidate');
    const names = [...new Set(supplemental.get(place.id) ?? [])].filter(name => /\p{Script=Han}/u.test(name));
    const sourceName = place.zh && /\p{Script=Han}/u.test(place.zh) ? place.zh : null;
    const chinese = sourceName ?? (names.length === 1 ? names[0] : null);
    if (!chinese) reasons.push(names.length > 1 ? 'conflicting-osm-names' : 'missing-chinese-name');
    reasons.push('hierarchy-unverified', 'timezone-unverified');
    results.push({
      id: place.id,
      country: country ?? null,
      geographicCountries,
      geographicRegions,
      timezoneCandidates,
      capitalFor,
      english: place.name,
      chinese,
      longitude: place.lon,
      latitude: place.lat,
      population: place.population,
      parent: place.parent,
      reasons,
    });
  }
  const groups: Record<string, { count: number; chinese: number; unresolved: number }> = {};
  for (const item of results) {
    const code = String(item.country ?? 'unresolved');
    const group = groups[code] ?? { count: 0, chinese: 0, unresolved: 0 };
    group.count++;
    if (item.chinese) group.chinese++;
    if ((item.reasons as string[]).length) group.unresolved++;
    groups[code] = group;
  }
  const summary = {
    total: results.length,
    countryGroups: Object.fromEntries(Object.entries(groups).sort(([a], [b]) => a.localeCompare(b))),
    conflicts: results.filter(item => (item.reasons as string[]).length).length,
  };
  return { summary, candidates: results };
}

/** CLI 读取固定快照，输出路径可以是仓库外文件。 */
function main(): void {
  const root = placesRoot();
  const input = process.argv[2] ?? snapshotPath('overseas-candidates');
  const output = process.argv[3];
  const snapshot = readJson<CandidateSnapshot>(root, input);
  const countries = readJson<{ countries: { code: string; name: string }[] }>(root, COUNTRIES_PATH);
  const countryNames = Object.fromEntries(countries.countries.map(country => [country.code, country.name]));
  const result = auditCandidates(snapshot, countryNames);
  if (output) writeFileSync(resolve(output), `${JSON.stringify(result, null, 2)}\n`);
  console.log(JSON.stringify(result.summary, null, 2));
}

runCli(main, import.meta.url);
