#!/usr/bin/env node
// 使用 HarfBuzz 和 Node.js 生成项目的 OpenType 衍生字体。
import { createHash } from 'node:crypto';
import { closeSync, existsSync, mkdtempSync, mkdirSync, openSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { renameFont } from './font-sfnt.mjs';

const usage = `Usage:
  node tools/fonts.mjs tc SOURCE_TC_VF.otf --output DIRECTORY
  node tools/fonts.mjs supplemental HANAMIN_B.ttf --output DIRECTORY
Requires hb-subset. Existing output files are never overwritten.`;
const sourceHashes = {
  tc: '8b71377f002adbe22387a780b0a09b5e263e9404196df5411c0ff5d682dd2703',
  han: '829543436cdf7b4c6730e2af5806fa406f8ddf0d4b656e0cd49c58d4db1c8d3c',
};
/** 简繁体共用的字符范围；保留布局特性和全部区域变体。 */
const chinese = '20-7e,a0-ff,2000-206f,2e80-2eff,2f00-2fdf,3000-303f,3400-4dbf,4e00-9fff,f900-faff,ff00-ffef';
/** 读取并验证来源字体摘要，防止在未知源文件上生成衍生资源。 */
function checked(path, digest) {
  const data = readFileSync(path); const actual = createHash('sha256').update(data).digest('hex');
  if (actual !== digest) throw Error(`Unexpected font source ${path}: SHA-256 ${actual}`);
  return data;
}
/** 执行字体工具并保留失败输出，不吞掉外部命令错误。 */
function run(command, args) {
  const r = spawnSync(command, args, { encoding: 'utf8', maxBuffer: 8 * 1024 * 1024 });
  if (r.error) throw Error(`Cannot run ${command}: ${r.error.message}`);
  if (r.status !== 0) throw Error(`${command} failed (${r.status}): ${r.stderr || r.stdout}`);
}
/** 按模式验证来源、生成临时字体，最后以不覆盖方式写入并在失败时回滚新文件。 */
function main() {
  const args = process.argv.slice(2), pos = args.indexOf('--output');
  if (pos < 0 || pos !== args.length - 2 || !args[pos+1]) throw Error(usage);
  const out = resolve(args[pos+1]), [mode,...inputs]=args.slice(0,pos);
  if (!((mode==='tc' && inputs.length===1) || (mode==='supplemental' && inputs.length===1))) throw Error(usage);
  const targets = mode==='tc' ? ['ZhijiChineseSerif-VF.otf'] : ['ZhijiHanSupplement.ttf'];
  if (targets.some(name=>existsSync(join(out,name)))) throw Error('Output font already exists; choose an empty output directory');
  if (mode==='tc') checked(inputs[0],sourceHashes.tc);
  if (mode==='supplemental') checked(inputs[0],sourceHashes.han);
  const temp=mkdtempSync(join(tmpdir(),'zhiji-fonts-'));
  try {
    // 临时产物也禁止覆盖，避免同一批生成步骤互相污染。
    const write=(name,data)=>writeFileSync(join(temp,name),data,{flag:'wx'});
    let subsetIndex = 0;
    // 每次子集使用独立路径，避免临时产物互相覆盖。
    const subset=(source,unicode,extra=[])=>{
      const file=join(temp,`subset-${subsetIndex}.otf`), list=join(temp,`unicode-${subsetIndex++}.txt`);
      writeFileSync(list,unicode);
      const options=[source,`--output-file=${file}`,`--unicodes-file=${list}`,'--name-IDs=*','--name-languages=*','--name-legacy'];
      options.push(...extra);
      run(process.env.HB_SUBSET || 'hb-subset',options); return readFileSync(file);
    };
    if(mode==='tc') {
      const font=subset(inputs[0],chinese,['--layout-features=*']);
      write(targets[0],renameFont(font,'Zhiji Chinese Serif','ZhijiChineseSerif',null,true,'ZhijiChineseSerif-ExtraLight'));
    } else {
      const han=subset(inputs[0],'20bcf,20beb,25464,26b99');
      write(targets[1],renameFont(han,'Zhiji Han Supplement','ZhijiHanSupplement'));
    }
    mkdirSync(out,{recursive:true});
    const created=[];
    try {
      for(const name of targets) {
        const destination=join(out,name);
        const content=readFileSync(join(temp,name));
        const fd=openSync(destination,'wx');
        created.push(destination);
        try { writeFileSync(fd,content); } finally { closeSync(fd); }
      }
    } catch(error) {
      for(const file of created) rmSync(file,{force:true});
      throw error;
    }
    for(const name of targets) console.log(join(out,name));
  } finally {rmSync(temp,{recursive:true,force:true});}
}
try { main(); } catch(e) { console.error(e.message); process.exitCode=1; }
