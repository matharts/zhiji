// 按 CLDR 的 M49 式分组补充洲信息，并同步资源清单摘要。
import {
  COUNTRIES_PATH,
  json,
  placesRoot,
  readJson,
  writeCatalog,
} from './catalog.ts';
import { runCli } from '../cli.ts';

interface ContinentSource {
  supplemental: {
    territoryContainment: Record<string, { _contains?: string[] }>;
  };
}

interface CountryCatalog {
  countries: { code: string; continent?: string }[];
  continentSource?: string;
}

/** 递归展开区域包含关系，国家映射出现歧义时拒绝写出。 */
function visit(source: ContinentSource['supplemental']['territoryContainment'], mapping: Map<string, string>, code: string, continent: string): void {
  const item = source[code];
  if (item?._contains) {
    for (const child of item._contains) visit(source, mapping, child, continent);
  } else if (/^[A-Z]{2}$/.test(code)) {
    const prior = mapping.get(code);
    if (prior && prior !== continent) throw Error(`Ambiguous continent for ${code}`);
    mapping.set(code, continent);
  }
}

/** 计算国家到洲的映射，不修改输入对象。 */
export function assignContinents(data: CountryCatalog, source: ContinentSource['supplemental']['territoryContainment']): number {
  const mapping = new Map<string, string>();
  for (const [code, continent] of [['142', 'AS'], ['150', 'EU'], ['002', 'AF'], ['003', 'NA'], ['005', 'SA'], ['009', 'OC']]) {
    visit(source, mapping, code, continent);
  }
  mapping.set('AQ', 'AN');
  for (const country of data.countries) {
    const continent = mapping.get(country.code);
    if (!continent) throw Error(`No CLDR continent for ${country.code}`);
    country.continent = continent;
  }
  data.continentSource = 'Unicode CLDR 48.2 territory containment; AQ placed in Antarctica';
  return data.countries.length;
}

function main(): void {
  const root = placesRoot();
  const source = readJson<ContinentSource>(root, 'tools/birthplaces/sources/cldr-territory-containment-48.2.0.json').supplemental.territoryContainment;
  const data = readJson<CountryCatalog>(root, COUNTRIES_PATH);
  const count = assignContinents(data, source);
  writeCatalog(root, new Map([[COUNTRIES_PATH, json(data)]]));
  console.log(`Grouped ${count} territories by continent.`);
}

runCli(main, import.meta.url);
