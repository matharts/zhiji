// 从官方快照重建国内地点选项，包含可选的区县层级。
import {
  CHINA_PATH,
  ZONES_PATH,
  json,
  placesRoot,
  readJson,
  writeCatalog,
} from './catalog.ts';
import { runCli } from '../cli.ts';

interface DivisionNode {
  code: string;
  name: string;
  children?: DivisionNode[];
}

interface DivisionTree {
  data: { children: DivisionNode[] };
}

interface PointRecord {
  code: string;
  name: string;
  province: string;
  longitude: number;
}

interface ChinaCatalog {
  source: string;
  retrieved: string;
  source_url: string;
  provinces: {
    id: string;
    name: string;
    cities: {
      id: string;
      name: string;
      longitude: number;
      counties: { id: string; name: string; longitude: number }[];
    }[];
  }[];
}

/** 将省市区编码补齐为统一的六位国内地点 ID。 */
function placeId(code: string): string {
  return `cn:${code.padEnd(6, '0')}`;
}

/** 构建国内树和时区映射，坐标或名称不唯一时停止。 */
export function buildChinaCatalog(
  tree: DivisionNode[],
  points: PointRecord[],
  countyPoints: PointRecord[],
  currentZones: Record<string, string[]>,
): { data: ChinaCatalog; zones: Record<string, string[]> } {
  const zones = structuredClone(currentZones);
  const generated = new Set<string>();
  const provinces = tree.map(province => {
    const code = province.name === '台湾省' ? '71' : province.code;
    const children = ['81', '82'].includes(code) ? [province] : province.children ?? [];
    return {
      id: placeId(code),
      name: province.name,
      cities: children.map(city => {
        const matches = points.filter(point => point.code === city.code && point.name === city.name && point.province === province.name);
        if (matches.length !== 1 || !Number.isFinite(matches[0]?.longitude)) throw Error(`Missing or ambiguous coordinate: ${province.name}/${city.name}`);
        const point = matches[0];
        const cityId = placeId(city.code);
        generated.add(cityId);
        zones[cityId] = code === '65'
          ? ['Asia/Shanghai', 'Asia/Urumqi']
          : [code === '81' ? 'Asia/Hong_Kong' : code === '82' ? 'Asia/Macau' : 'Asia/Shanghai'];
        const counties = (city.children ?? []).map(county => {
          const countyMatches = countyPoints.filter(candidate => candidate.code === county.code && candidate.name === county.name && candidate.province === province.name);
          if (countyMatches.length !== 1 || !Number.isFinite(countyMatches[0]?.longitude)) {
            throw Error(`Missing or ambiguous county coordinate: ${province.name}/${city.name}/${county.name}`);
          }
          const countyId = placeId(county.code);
          generated.add(countyId);
          zones[countyId] = zones[cityId] ?? [];
          return { id: countyId, name: county.name, longitude: countyMatches[0].longitude };
        });
        return { id: cityId, name: city.name, longitude: point.longitude, counties };
      }),
    };
  });
  for (const key of Object.keys(zones)) if (key.startsWith('cn:') && !generated.has(key)) delete zones[key];
  return {
    data: {
      source: '民政部国家地名信息库现行行政区划树',
      retrieved: '2026-11-10',
      source_url: 'https://dmfw.mca.gov.cn/xzqh/getList?code=0&trimCode=true&maxLevel=3',
      provinces,
    },
    zones,
  };
}

function main(): void {
  const root = placesRoot();
  const tree = readJson<DivisionTree>(root, 'tools/birthplaces/sources/mca-division-tree-2026-11-10.json').data.children;
  const points = readJson<{ points: PointRecord[] }>(root, 'tools/birthplaces/sources/mca-birthplace-points-2026-11-10.json').points;
  const countyPoints = readJson<{ points: PointRecord[] }>(root, 'tools/birthplaces/sources/mca-county-points-2026-11-11.json').points;
  const zones = readJson<Record<string, string[]>>(root, ZONES_PATH);
  const result = buildChinaCatalog(tree, points, countyPoints, zones);
  writeCatalog(root, new Map([
    [CHINA_PATH, json(result.data)],
    [ZONES_PATH, json(result.zones)],
  ]));
  const cityCount = result.data.provinces.reduce((count, province) => count + province.cities.length, 0);
  console.log(`Generated ${result.data.provinces.length} provinces and ${cityCount} selectable regions`);
}

runCli(main, import.meta.url);
