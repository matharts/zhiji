export const MODES = ['resources', 'types', 'rust', 'gpu', 'native', 'updates'] as const;
export type VerifyMode = typeof MODES[number];

export interface VerifyOptions {
  mode: VerifyMode;
  offline: boolean;
  appBundle?: string;
}

export const USAGE = `Usage: node tools/verify.ts [--offline] [--app-bundle PATH] ${MODES.join('|')}`;

/** 解析唯一模式及可选参数，未知参数或重复模式在开始检查前失败。 */
export function parseArguments(args: string[]): VerifyOptions {
  let mode: VerifyMode | undefined;
  let offline = false;
  let appBundle: string | undefined;
  for (let index = 0; index < args.length; index++) {
    const arg = args[index];
    if (arg === '--offline') {
      offline = true;
    } else if (arg === '--app-bundle') {
      const value = args[index + 1];
      if (!value || value.startsWith('--')) throw Error(USAGE);
      appBundle = value;
      index++;
    } else if ((MODES as readonly string[]).includes(arg)) {
      if (mode) throw Error(USAGE);
      mode = arg as VerifyMode;
    } else {
      throw Error(USAGE);
    }
  }
  if (!mode) throw Error(USAGE);
  return { mode, offline, ...(appBundle ? { appBundle } : {}) };
}
