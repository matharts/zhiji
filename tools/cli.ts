import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

/** 判断模块是否由 Node 直接执行；被测试或工具导入时不产生副作用。 */
export function isMain(moduleUrl: string): boolean {
  const entry = process.argv[1];
  return entry !== undefined && resolve(entry) === fileURLToPath(moduleUrl);
}

/** 统一命令入口的错误输出和退出码，避免每个脚本重复同一段样板。 */
export function runCli(main: () => void, moduleUrl: string): void {
  if (!isMain(moduleUrl)) return;
  try {
    main();
  } catch (error) {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  }
}
