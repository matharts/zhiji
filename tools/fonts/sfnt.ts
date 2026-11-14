// 字体生成器使用的最小 OpenType 名称表改写及 cmap 检查。
// 其余表逐字节复制，并重新计算 head.checkSumAdjustment。
export type FontTables = Map<string, Buffer>;

interface NameRecord {
  platform: number;
  encoding: number;
  language: number;
  id: number;
  bytes: Buffer;
}

interface NameTable {
  format: number;
  records: NameRecord[];
  tags: Buffer[];
}

type NameOffset = [offset: number, length: number];

/** 校验独立字体表目录并复制表数据，拒绝截断、越界和重复标签。 */
export function tables(data: Buffer): FontTables {
  if (data.length < 12 || !['OTTO', '\u0000\u0001\u0000\u0000', 'true'].includes(data.toString('latin1', 0, 4))) throw Error('Expected a standalone OpenType/TrueType font');
  const count = data.readUInt16BE(4), result: FontTables = new Map();
  for (let i = 0; i < count; i++) {
    const p = 12 + 16 * i;
    if (p + 16 > data.length) throw Error('Truncated table directory');
    const tag = data.toString('latin1', p, p + 4), offset = data.readUInt32BE(p + 8), length = data.readUInt32BE(p + 12);
    if (offset + length > data.length || result.has(tag)) throw Error(`Invalid table ${tag}`);
    result.set(tag, Buffer.from(data.subarray(offset, offset + length)));
  }
  return result;
}
/** 解析名称记录和可选语言标签，逐段检查表内偏移。 */
function names(table: Buffer | undefined): NameTable {
  if (!table || table.length < 6) throw Error('Missing name table');
  const format = table.readUInt16BE(0), count = table.readUInt16BE(2), start = table.readUInt16BE(4);
  if (format > 1 || start > table.length || 6 + count * 12 > table.length) throw Error('Invalid name table');
  const records: NameRecord[] = [];
  for (let i = 0; i < count; i++) {
    const p = 6 + i * 12, offset = start + table.readUInt16BE(p + 10), length = table.readUInt16BE(p + 8);
    if (offset + length > table.length) throw Error('Invalid name record');
    records.push({ platform: table.readUInt16BE(p), encoding: table.readUInt16BE(p + 2), language: table.readUInt16BE(p + 4), id: table.readUInt16BE(p + 6), bytes: table.subarray(offset, offset + length) });
  }
  const tags: Buffer[] = [];
  if (format === 1) {
    const p = 6 + count * 12;
    if (p + 2 > table.length) throw Error('Invalid language tags');
    const n = table.readUInt16BE(p);
    for (let i = 0; i < n; i++) {
      const pos = p + 2 + i * 4;
      if (pos + 4 > table.length) throw Error('Invalid language tag record');
      const len = table.readUInt16BE(pos), off = start + table.readUInt16BE(pos + 2);
      if (off + len > table.length) throw Error('Invalid language tag');
      tags.push(table.subarray(off, off + len));
    }
  }
  return { format, records, tags };
}
/** 按平台编码解码名称，Unicode 平台使用大端 UTF-16。 */
function decode(record: NameRecord): string {
  if (record.platform === 0 || record.platform === 3) {
    const b = Buffer.from(record.bytes);
    if (b.length % 2) throw Error('Invalid UTF-16 name');
    return b.swap16().toString('utf16le');
  }
  return record.bytes.toString('latin1');
}
/** 优先选择 Windows 英文记录，其次 Unicode 记录，缺失则拒绝改名。 */
function preferred(records: NameRecord[], id: number): string {
  const candidates = records.filter(x => x.id === id);
  const record = candidates.find(x => x.platform === 3 && x.language === 0x409) ?? candidates.find(x => x.platform === 0) ?? candidates[0];
  if (!record) throw Error(`Missing name ID ${id}`);
  return decode(record);
}
/** 将 JavaScript 字符串编码为 OpenType 使用的大端 UTF-16。 */
function utf16be(value: string): Buffer { return Buffer.from(value, 'utf16le').swap16(); }
/** 按 OpenType 规则对补齐至四字节的表内容求无符号校验和。 */
function checksum(data: Buffer): number {
  let sum = 0;
  for (let i = 0; i < data.length; i += 4) {
    let n = 0;
    for (let j = 0; j < 4; j++) n = ((n << 8) | (data[i + j] ?? 0)) >>> 0;
    sum = (sum + n) >>> 0;
  }
  return sum;
}
/** 重建名称表和字体目录，保留其余表及可变轴，重算整文件校验和。 */
export function renameFont(
  data: Buffer,
  family: string,
  postscript: string,
  weight: string | null = null,
  variable = false,
  defaultName: string = postscript,
): Buffer {
  const entries = tables(data), name = names(entries.get('name'));
  const version = preferred(name.records, 5);
  const renamed: [number, string][] = weight
    ? [[1,family],[2,weight],[3,`${family} ${weight} 1.0`],[4,`${family} ${weight}`],[6,`${postscript}-${weight}`],[16,family],[17,weight]]
    : [[1,family],[3,`${family} ${version}`],[4,family],[6,defaultName],[16,family],[25,postscript]];
  const updates = new Map<number, string>(renamed);
  if (variable) {
    const fvar = entries.get('fvar');
    if (!fvar || fvar.length < 16) throw Error('Expected a variable font');
    const off = fvar.readUInt16BE(4), axisCount = fvar.readUInt16BE(8), axisSize = fvar.readUInt16BE(10), count = fvar.readUInt16BE(12), size = fvar.readUInt16BE(14);
    if (![...Array(axisCount).keys()].some(i => fvar.toString('latin1', off + i * axisSize, off + i * axisSize + 4) === 'wght')) throw Error('Expected wght axis');
    for (let i = 0; i < count; i++) {
      const p = off + axisCount * axisSize + i * size;
      if (p + size > fvar.length || size < 4 + axisCount * 4) throw Error('Invalid fvar instances');
      if (size >= 6 + axisCount * 4) {
        const id = fvar.readUInt16BE(p + 4 + axisCount * 4);
        if (id !== 0xffff) updates.set(id, `${postscript}-${preferred(name.records, fvar.readUInt16BE(p))}`);
      }
    }
  }
  const headerSize = 6 + name.records.length * 12 + (name.format === 1 ? 2 + name.tags.length * 4 : 0);
  const strings: Buffer[] = [], offsets: NameOffset[] = []; let cursor = 0;
  for (const record of name.records) {
    let bytes = record.bytes;
    const value = updates.get(record.id);
    if (value !== undefined) {
      if (record.platform === 0 || record.platform === 3) bytes = utf16be(value);
      else if (record.platform === 1 && /^[\x20-\x7e]*$/.test(value)) bytes = Buffer.from(value, 'ascii');
      else throw Error(`Unsupported encoding for renamed name record: ${record.platform}`);
    }
    offsets.push([cursor, bytes.length]); strings.push(bytes); cursor += bytes.length;
  }
  const tagOffsets: NameOffset[] = name.tags.map(bytes => { const p = cursor; strings.push(bytes); cursor += bytes.length; return [p, bytes.length]; });
  if (headerSize > 0xffff || cursor > 0xffff) throw Error('Name table exceeds OpenType limits');
  const nameHeader = Buffer.alloc(headerSize); nameHeader.writeUInt16BE(name.format,0); nameHeader.writeUInt16BE(name.records.length,2); nameHeader.writeUInt16BE(headerSize,4);
  name.records.forEach((r,i) => { const p=6+i*12; [r.platform,r.encoding,r.language,r.id,offsets[i][1],offsets[i][0]].forEach((v,j)=>nameHeader.writeUInt16BE(v,p+j*2)); });
  if (name.format === 1) { const p=6+name.records.length*12; nameHeader.writeUInt16BE(name.tags.length,p); tagOffsets.forEach(([off,len],i)=>{nameHeader.writeUInt16BE(len,p+2+i*4);nameHeader.writeUInt16BE(off,p+4+i*4);}); }
  entries.set('name', Buffer.concat([nameHeader,...strings]));
  entries.delete('DSIG'); // 字体内容改变后原数字签名不再有效。
  const head = entries.get('head'); if (!head || head.length < 12) throw Error('Missing head table'); head.writeUInt32BE(0,8);
  const tags = [...entries.keys()].sort(), n=tags.length, maxPower=2**Math.floor(Math.log2(n));
  const directory = Buffer.alloc(12+n*16); data.copy(directory,0,0,4); directory.writeUInt16BE(n,4); directory.writeUInt16BE(maxPower*16,6); directory.writeUInt16BE(Math.log2(maxPower),8); directory.writeUInt16BE(n*16-maxPower*16,10);
  let offset = directory.length, headOffset = 0; const chunks: Buffer[]=[directory];
  tags.forEach((tag,i)=>{const bytes=entries.get(tag); if(!bytes) throw Error(`Missing table ${tag}`); const p=12+i*16; directory.write(tag,p,4,'latin1'); directory.writeUInt32BE(checksum(bytes),p+4); directory.writeUInt32BE(offset,p+8); directory.writeUInt32BE(bytes.length,p+12); if(tag==='head')headOffset=offset; const padded=Buffer.alloc((bytes.length+3)&~3);bytes.copy(padded);chunks.push(padded);offset+=padded.length;});
  const result=Buffer.concat(chunks); result.writeUInt32BE((0xb1b0afba-checksum(result))>>>0,headOffset+8); return result;
}
/** 从受支持的 Unicode cmap 子表提取非空字形覆盖，拒绝非法偏移和范围。 */
export function coveredCodepoints(data: Buffer): Set<number> {
  const cmap=tables(data).get('cmap'); if (!cmap || cmap.length < 4) throw Error('Missing cmap table');
  const result=new Set<number>(), count=cmap.readUInt16BE(2);
  for(let i=0;i<count;i++) {
    const p=4+i*8;if(p+8>cmap.length)throw Error('Invalid cmap');
    const platform=cmap.readUInt16BE(p), encoding=cmap.readUInt16BE(p+2), off=cmap.readUInt32BE(p+4);
    if (!(platform===0 || (platform===3 && (encoding===1 || encoding===10))) || off+2>cmap.length) continue;
    const format=cmap.readUInt16BE(off);
    if(format===12 || format===13) {
      if(off+16>cmap.length)throw Error('Invalid cmap format 12');const len=cmap.readUInt32BE(off+4), groups=cmap.readUInt32BE(off+12);
      if(off+len>cmap.length || 16+groups*12>len)throw Error('Invalid cmap groups');
      for(let g=0;g<groups;g++){const q=off+16+g*12,start=cmap.readUInt32BE(q),end=cmap.readUInt32BE(q+4),glyph=cmap.readUInt32BE(q+8);if(start>end||end>0x10ffff)throw Error('Invalid cmap range');for(let c=start;c<=end;c++)if((format===13?glyph:glyph+c-start)!==0)result.add(c);}
    } else if(format===4) {
      if(off+14>cmap.length)throw Error('Invalid cmap format 4');const len=cmap.readUInt16BE(off+2),segments=cmap.readUInt16BE(off+6)/2;
      if(off+len>cmap.length || !Number.isInteger(segments))throw Error('Invalid cmap segments');
      const end=off+14,start=end+2*segments+2,delta=start+2*segments,range=delta+2*segments;
      if(range+2*segments>off+len)throw Error('Invalid cmap arrays');
      for(let s=0;s<segments;s++){const a=cmap.readUInt16BE(start+s*2),b=cmap.readUInt16BE(end+s*2),d=cmap.readInt16BE(delta+s*2),r=cmap.readUInt16BE(range+s*2);if(a>b)throw Error('Invalid cmap range');for(let c=a;c<=b&&c!==0xffff;c++){let glyph;if(!r)glyph=(c+d)&0xffff;else{const pos=range+s*2+r+2*(c-a);if(pos+2>off+len)throw Error('Invalid cmap glyph');glyph=cmap.readUInt16BE(pos);if(glyph)glyph=(glyph+d)&0xffff;}if(glyph)result.add(c);}}
    }
  }
  if(!result.size)throw Error('No Unicode cmap found');return result;
}
