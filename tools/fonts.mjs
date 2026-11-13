#!/usr/bin/env node
// 使用 HarfBuzz 和 Node.js 生成项目的 OpenType 衍生字体。
import { createHash } from 'node:crypto';
import { closeSync, existsSync, mkdtempSync, mkdirSync, openSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { coveredCodepoints, renameFont } from './font-sfnt.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const usage = `Usage:
  node tools/fonts.mjs prototype FONT_DIR SOURCE_CN_VF.otf --output DIRECTORY
  node tools/fonts.mjs location SOURCE_CN_VF.otf --output DIRECTORY
  node tools/fonts.mjs supplemental NOTO_SERIF_KR.ttf HANAMIN_B.ttf --output DIRECTORY
Requires hb-subset with variation support and woff2_decompress for prototype fonts. Existing output files are never overwritten.`;
const sourceHashes = {
  cn: '0926369670e4b18b63ae2642ac5b57c2e03343c2f3083c4ec98e8c0117edea4f',
  kr: '11f8d5de6f1b79195efba3828aaa2ec95c1178f5ae976fb23c8d53250a9938f3',
  han: '829543436cdf7b4c6730e2af5806fa406f8ddf0d4b656e0cd49c58d4db1c8d3c',
};
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
/** 枚举参与字体字符收集的 Rust 源文件。 */
function* rustFiles(path) {
  for (const entry of readdirSync(path, { withFileTypes: true })) {
    const p = join(path, entry.name);
    if (entry.isDirectory()) yield* rustFiles(p);
    else if (entry.isFile() && p.endsWith('.rs')) yield p;
  }
}
/** 合并原字体覆盖、ASCII 和当前 Rust 源字符，生成有序子集码点列表。 */
function chars() {
  const points = coveredCodepoints(readFileSync(join(root, 'assets/fonts/ZhijiPrototypeSerif-Regular.otf')));
  for (let i=32;i<127;i++) points.add(i);
  for (const dir of ['src', 'crates/zhiji_core/src', 'crates/zhiji_ui/src']) {
    for (const file of rustFiles(join(root, dir))) for (const c of readFileSync(file, 'utf8')) points.add(c.codePointAt(0));
  }
  return [...points].sort((a,b)=>a-b).map(n=>n.toString(16)).join(',');
}
/** 按模式验证来源、生成临时字体，最后以不覆盖方式写入并在失败时回滚新文件。 */
function main() {
  const args = process.argv.slice(2), pos = args.indexOf('--output');
  if (pos < 0 || pos !== args.length - 2 || !args[pos+1]) throw Error(usage);
  const out = resolve(args[pos+1]), [mode,...inputs]=args.slice(0,pos);
  if (!((mode==='prototype' && inputs.length===2) || (mode==='location' && inputs.length===1) || (mode==='supplemental' && inputs.length===2))) throw Error(usage);
  const targets = mode==='prototype' ? ['ZhijiPrototypeSerif-Regular.otf','ZhijiPrototypeSerif-SemiBold.otf','ZhijiPrototypeSerif-Bold.otf','ZhijiPrototypeSerif-Heavy.otf'] : mode==='location' ? ['ZhijiLocationSerif-VF.otf'] : ['ZhijiKoreanSerif-VF.ttf','ZhijiHanSupplement.ttf'];
  if (targets.some(name=>existsSync(join(out,name)))) throw Error('Output font already exists; choose an empty output directory');
  if (mode==='prototype' || mode==='location') checked(inputs[mode==='prototype'?1:0],sourceHashes.cn);
  if (mode==='supplemental') { checked(inputs[0],sourceHashes.kr); checked(inputs[1],sourceHashes.han); }
  const temp=mkdtempSync(join(tmpdir(),'zhiji-fonts-'));
  try {
    // 临时产物也禁止覆盖，避免同一批生成步骤互相污染。
    const write=(name,data)=>writeFileSync(join(temp,name),data,{flag:'wx'});
    let subsetIndex = 0;
    // 每次子集使用独立路径，并按需固定可变字体实例。
    const subset=(source,unicode,variable=null)=>{
      const file=join(temp,`subset-${subsetIndex}.otf`), list=join(temp,`unicode-${subsetIndex++}.txt`);
      writeFileSync(list,unicode);
      const options=[source,`--output-file=${file}`,`--unicodes-file=${list}`,'--name-IDs=*','--name-languages=*','--name-legacy'];
      if(variable) options.push(`--instance=${variable}`);
      run(process.env.HB_SUBSET || 'hb-subset',options); return readFileSync(file);
    };
    if(mode==='prototype') {
      const points=chars();
      for(const weight of ['Regular','SemiBold','Bold']) {
        const compressed=join(temp,`source-${weight}.woff2`);
        writeFileSync(compressed,readFileSync(join(inputs[0],`SourceHanSerifCN-${weight}.woff2`)));
        run(process.env.WOFF2_DECOMPRESS || 'woff2_decompress',[compressed]);
        const font=subset(join(temp,`source-${weight}.ttf`),points);
        write(`ZhijiPrototypeSerif-${weight}.otf`,renameFont(font,'Zhiji Prototype Serif','ZhijiPrototypeSerif',weight));
      }
      const heavy=subset(inputs[1],'41,42,43,44','wght=900');
      write('ZhijiPrototypeSerif-Heavy.otf',renameFont(heavy,'Zhiji Prototype Serif','ZhijiPrototypeSerif','Heavy'));
    } else if(mode==='location') {
      write(targets[0],renameFont(readFileSync(inputs[0]),'Zhiji Location Serif','ZhijiLocationSerif',null,true,'ZhijiLocationSerif-ExtraLight'));
    } else {
      const korean=subset(inputs[0],'1100-11ff,3130-318f,a960-a97f,ac00-d7af,d7b0-d7ff');
      write(targets[0],renameFont(korean,'Zhiji Korean Serif','ZhijiKoreanSerif',null,true));
      const han=subset(inputs[1],'20bcf,20beb,25464,26b99');
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
