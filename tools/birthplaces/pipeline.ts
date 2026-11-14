// 海外目录重放与名称审计共享的固定批次顺序，避免新增快照时只改一处。
export const SNAPSHOT_DATE = '2026-11-11';

export const MIGRATION_SNAPSHOTS = [
  'overseas-priority-migration',
  'overseas-global-migration',
  'overseas-osm-migration',
  'overseas-osm-fallback-migration',
  'overseas-wikidata-followup',
] as const;

export const NATURAL_EARTH_SNAPSHOT = 'overseas-natural-earth-names';

export const NAME_SNAPSHOTS = [
  'overseas-cross-source-names',
  'overseas-naer-names',
  'overseas-osm-extra-names',
  'overseas-further-names',
  'overseas-normalized-names',
  'overseas-english-alias-names',
  'overseas-english-alias-wide-names',
  'overseas-additional-types-names',
  'overseas-name-corrections',
  'overseas-cldr-names',
  'overseas-multilingual-names',
  'overseas-multilingual-wide-names',
  'overseas-multilingual-alias-names',
  'overseas-chinese-alias-names',
  'overseas-multilingual-chinese-alias-names',
  'overseas-wikidata-current-names',
  'overseas-chinese-locales-names',
  'overseas-naer-alias-names',
  'overseas-naer-alias-wide-names',
  'overseas-geonames-zh-names',
  'overseas-osm-locality-names',
  'overseas-osm-thailand-names',
  'overseas-osm-vietnam-names',
] as const;

export interface Stage {
  script: string;
  args: string[];
}

/** 固定快照路径，重放不依赖网络或「最新」文件。 */
export function snapshotPath(name: string): string {
  return `tools/birthplaces/sources/${name}-${SNAPSHOT_DATE}.json`;
}

/** 生成完整重放顺序；自然地球补名先于其他补名，洲别与审计最后执行。 */
export function rebuildStages(): Stage[] {
  return [
    ...MIGRATION_SNAPSHOTS.map(name => ({ script: 'migrate-catalog.ts', args: [snapshotPath(name)] })),
    { script: 'apply-natural-earth-names.ts', args: [] },
    ...NAME_SNAPSHOTS.map(name => ({ script: 'apply-names.ts', args: [snapshotPath(name)] })),
    { script: 'generate-continents.ts', args: [] },
    { script: 'audit-names.ts', args: [] },
    { script: 'generate-major-cities.ts', args: [] },
  ];
}

/** 名称审计必须覆盖与重放相同的全部快照，包括 Natural Earth 批次。 */
export function nameAuditSnapshots(): string[] {
  return [NATURAL_EARTH_SNAPSHOT, ...NAME_SNAPSHOTS].map(snapshotPath);
}
