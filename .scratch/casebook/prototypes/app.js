/* 四种布局共享合成资料与交互，不连接应用或用户数据库。 */
const iconPaths={
  search:'<circle cx="10" cy="10" r="6.5"/><path d="m15 15 5 5"/>',
  plus:'<path d="M12 5v14M5 12h14"/>',
  back:'<path d="m14 5-7 7 7 7"/>',
  close:'<path d="M5 5l14 14M19 5 5 19"/>',
  next:'<path d="m9 5 7 7-7 7"/>',
  moon:'<path d="M20 14A8 8 0 0 1 10 4a8 8 0 1 0 10 10Z"/>',
  folder:'<path d="M3 6h6l2 2h10v12H3Z"/>',
  archive:'<path d="M4 8h16v12H4ZM3 4h18v4H3Zm6 8h6"/>',
  book:'<path d="M5 3h14v18H5a2 2 0 0 1 0-4h14M5 3a2 2 0 0 0-2 2v14M8 7h7"/>',
  trash:'<path d="M4 7h16M9 3h6l1 4M6 7l1 14h10l1-14M10 10v7m4-7v7"/>',
  star:'<path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1.1 6.2-5.6-3-5.6 3 1.1-6.2L3 9.6l6.2-.9Z"/>',
  grid:'<path d="M3 3h7v7H3Zm11 0h7v7h-7ZM3 14h7v7H3Zm11 0h7v7h-7Z"/>',
  list:'<path d="M8 5h13M8 12h13M8 19h13M3 5h.1M3 12h.1M3 19h.1"/>',
  logo:'<path d="M12 2 21 7v10l-9 5-9-5V7Z"/><path d="m3 7 9 5 9-5M12 12v10M7.5 4.5l9 5v10"/>',
  copy:'<rect x="8" y="8" width="12" height="13" rx="2"/><path d="M16 8V3H3v13h5"/>',
  edit:'<path d="m14 4 6 6M3 21l5-1L21 7l-6-6L2 14Z"/>',
  settings:'<path d="M3 6h18M3 12h18M3 18h18"/><circle cx="8" cy="6" r="2" fill="var(--canvas)"/><circle cx="16" cy="12" r="2" fill="var(--canvas)"/><circle cx="10" cy="18" r="2" fill="var(--canvas)"/>',
  sidebar:'<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M15 4v16"/>',
  calendar:'<rect x="3" y="5" width="18" height="16" rx="2"/><path d="M3 10h18M8 3v4m8-4v4M7 14h2m3 0h2"/>',
  more:'<circle cx="5" cy="12" r=".7"/><circle cx="12" cy="12" r=".7"/><circle cx="19" cy="12" r=".7"/>',
  pointer:'<path d="m5 3 14 11-7 1-3 6Z"/>',
  rectangle:'<path d="M3 5h18v14H3Z"/>',
  ellipse:'<ellipse cx="12" cy="12" rx="9" ry="7"/>',
  arrow:'<path d="m4 20 16-16M10 4h10v10"/>',
  line:'<path d="m4 20 16-16"/>',
  pen:'<path d="m5 19 3-8L17 2l5 5-9 9ZM3 21l5-2m0-8 5 5"/>',
  text:'<path d="M4 5h16M12 5v15M8 20h8"/>',
  undo:'<path d="M8 4 3 9l5 5M3 9h11a6 6 0 0 1 0 12"/>',
  download:'<path d="M12 3v12m-5-5 5 5 5-5M4 16v5h16v-5"/>',
};
const icon=name=>`<svg viewBox="0 0 24 24" aria-hidden="true">${iconPaths[name]||iconPaths.book}</svg>`;
const escapeHtml=value=>String(value??'').replace(/[&<>"']/g,char=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[char]));
const chartPath='../../../tests/fixtures/visual/export.png';
const chartZiwei=['辰','酉','子','午','寅','亥','巳','申','卯','戌','丑','未'];
const chartImage=record=>Number.isInteger(record.chartIndex)?`charts/${String(record.chartIndex).padStart(2,'0')}.png`:null;
const chartMarkup=record=>chartImage(record)?`<img src="${chartImage(record)}" alt="${escapeHtml(record.title)}的盘面" loading="lazy">`:'<span class="muted">暂无盘面预览</span>';
const topics=['事业','家庭','学业','迁移','流年','格局'];
const sourceNames=['个人复盘','课堂笔记','书籍摘录'];
const titleSeeds=[
  ['甲子年生人 · 职业转折','事业','迁移','工作调整与异地发展发生在同一阶段，保留原始判断与回访。'],
  ['乙丑年生人 · 家庭与迁居','家庭','迁移','将夫妻、田宅与迁移三宫合看，记录迁居前后的家庭变化。'],
  ['庚午年生人 · 学业复盘','学业','流年','整理升学阶段的取舍，补充当年考试结果与后续反馈。'],
  ['壬申年生人 · 两次创业','事业','格局','同一人的两次创业经历，分别记录资金、合作与退出节点。'],
  ['丙寅年生人 · 外地求学','学业','迁移','离乡求学前后的学习状态与生活变化。'],
  ['丁卯年生人 · 婚后生活','家庭','流年','区分原始资料、当事人反馈与研究者推测。'],
  ['戊辰年生人 · 职业选择','事业','流年','对照两份工作选择与当事人的实际决策。'],
  ['辛未年生人 · 迁居记录','迁移','家庭','整理多次迁居的年份、原因及家庭背景。'],
  ['癸酉年生人 · 合作关系','事业','格局','围绕合作关系变化收集事实，暂不作单一归因。'],
  ['甲戌年生人 · 学习方向','学业','事业','记录专业选择与后来从事工作的联系。'],
  ['乙亥年生人 · 流年回访','流年','家庭','补充上一次复盘尚待核实的时间点。'],
  ['丙子年生人 · 紫微格局','格局','事业','以同类盘面作对照，记录相同结构与不同经历。'],
];
let records=Array.from({length:72},(_,index)=>{
  const seed=titleSeeds[index%titleSeeds.length];
  return {id:index+1,title:seed[0]+(index>=12?`（资料 ${Math.floor(index/12)+1}）`:''),tags:[seed[1],seed[2]],chartIndex:index%12,summary:`${index%2?'女':'男'} · ${['命宫在辰','命宫在午','命宫在酉','命宫在寅'][index%4]} · 紫微在${chartZiwei[index%12]}`,source:sourceNames[index%3],note:seed[3],date:`2026-${index<30?'11':'10'}-${String(15-index%15).padStart(2,'0')}`,star:index%9===0,trash:false};
});
const collections=[
  {name:'事业与选择',topic:'事业',subtitle:'职业转换、合作与创业',cover:'#e8e2ee',ribbon:'#8d76a1',letter:'业'},
  {name:'家庭关系',topic:'家庭',subtitle:'婚姻、家庭与居住变化',cover:'#e6ebe6',ribbon:'#7c947e',letter:'家'},
  {name:'学业与成长',topic:'学业',subtitle:'求学经历与方向选择',cover:'#e6e9f0',ribbon:'#7d8fa8',letter:'学'},
  {name:'迁移与流动',topic:'迁移',subtitle:'迁居、远行与异地生活',cover:'#eee7dd',ribbon:'#ad9775',letter:'迁'},
  {name:'流年验证',topic:'流年',subtitle:'按时间回访与补充反馈',cover:'#eee2e5',ribbon:'#b1818d',letter:'年'},
  {name:'格局研究',topic:'格局',subtitle:'同类盘面的对照与归纳',cover:'#e4e7e8',ribbon:'#809095',letter:'局'},
];
const stage=document.getElementById('stage');
history.scrollRestoration='manual';
const picker=document.querySelector('.proto-picker');
const highlight=picker.querySelector('.proto-picker-highlight');
const items=[...picker.querySelectorAll('.proto-picker-item')];
const initial=Number(new URLSearchParams(location.search).get('v')||document.body.dataset.variant||1)-1;
let current=Math.min(3,Math.max(0,initial));
let state;
let toastTimer;
const unified=document.body.hasAttribute('data-unified');
const resetState=()=>({query:'',topic:'全部',source:'全部来源',sort:'newest',page:1,tab:'library',selected:new Set(),route:'library',recordId:null,installedId:null,collection:null,starOnly:false,previousRoute:'library',more:false,importName:'',libraryView:'list',librarySection:'results',railOpen:true,inputChoice:{gender:'男',stem:'甲',branch:'子',ziwei:'辰',month:'正月',hour:'子',main:'辰',decade:'辰',year:'午'},panel:'chart'});
state=resetState();
const countFor=topic=>records.filter(record=>!record.trash&&record.tags.includes(topic)).length;
function filtered(){
  let result=records.filter(record=>state.tab==='trash'?record.trash:!record.trash);
  if(state.topic!=='全部')result=result.filter(record=>record.tags.includes(state.topic));
  if(state.source!=='全部来源')result=result.filter(record=>record.source===state.source);
  if(state.starOnly)result=result.filter(record=>record.star);
  const query=state.query.trim().toLocaleLowerCase();
  if(query)result=result.filter(record=>[record.title,record.summary,record.source,record.note,...record.tags].join(' ').toLocaleLowerCase().includes(query));
  return result.sort((left,right)=>state.sort==='title'?left.title.localeCompare(right.title,'zh-CN'):state.sort==='oldest'?left.date.localeCompare(right.date)||left.id-right.id:right.date.localeCompare(left.date)||left.id-right.id);
}
const chips=tags=>tags.map(tag=>`<span class="chip">${escapeHtml(tag)}</span>`).join('');
function notify(message){const toast=document.getElementById('toast');toast.textContent=message;toast.classList.add('visible');clearTimeout(toastTimer);toastTimer=setTimeout(()=>toast.classList.remove('visible'),2500)}
function shell(content){
  const modal=unified&&state.route!=='workspace';
  const workspace=state.route==='workspace'||modal;
  const identity=workspace?(state.installedId?records.find(record=>record.id===state.installedId)?.title:'知几'):'知几';
  return `<div class="app-window">
  <header class="app-titlebar" ${modal?'inert':''}>
    <div class="window-controls" aria-hidden="true"><i></i><i></i><i></i></div>
    <div class="app-identity"><img src="../../../assets/icons/app.png" alt="">${escapeHtml(identity||'知几')}</div>
    ${!workspace?`<span class="title-divider"></span><button class="title-back" data-action="workspace" title="返回当前命盘">${icon('back')}命例库</button>`:''}
    <div class="spacer"></div>
    <div class="window-actions">${workspace?`<button class="icon-button" data-action="date-source" aria-label="日期取数" title="日期取数">${icon('calendar')}</button><button class="icon-button" data-action="toggle-columns" aria-label="切换单双列" title="切换单双列">${icon('list')}</button><button class="icon-button" data-action="toggle-sidebar" aria-label="收起或展开右侧面板" title="收起或展开右侧面板">${icon('sidebar')}</button>`:''}<button class="icon-button" data-action="theme" aria-label="切换昼夜主题" title="切换昼夜主题">${icon('moon')}</button><button class="icon-button" data-action="settings" aria-label="设置" title="设置">${icon('settings')}</button></div>
  </header>
  <main class="workspace ${workspace?'chart-workspace':''}" ${modal?'inert':''}>${modal?chartWorkspace():content}</main>
  ${modal?`<div class="library-backdrop"><section class="library-dialog" role="dialog" aria-modal="true" aria-labelledby="library-dialog-title"><header class="library-dialog-header"><span id="library-dialog-title">命例库</span><button class="icon-button library-close" data-action="workspace" aria-label="关闭命例库" title="关闭命例库">${icon('close')}</button></header><div class="library-dialog-content">${content}</div></section></div>`:''}
  <input class="hidden" type="file" id="draft-import" accept=".ziwei" aria-label="导入草稿为命例">
  </div>`;
}
function heading(title,extra='',description=''){
  return `<div class="page-heading"><h1>${escapeHtml(title)}${extra}</h1><div class="spacer"></div><button class="button" data-action="import">导入草稿…</button><button class="button primary" data-action="new">保存当前盘面</button></div>`;
}
function search(){return `<label class="search">${icon('search')}<input id="search" type="search" placeholder="搜索命例、标签或笔记" aria-label="搜索命例" value="${escapeHtml(state.query)}"><kbd>/</kbd></label>`}
function toolbar(withTabs=true){
  return `<div class="toolbar">${withTabs?`<nav class="tabs" aria-label="命例分类"><button data-action="tab" data-value="library" class="${state.tab==='library'?'active':''}">全部命例</button><button data-action="tab" data-value="trash" class="${state.tab==='trash'?'active':''}">回收站</button></nav><div class="spacer"></div>`:''}${search()}<select id="source" aria-label="按来源筛选">${['全部来源',...sourceNames].map(source=>`<option ${source===state.source?'selected':''}>${source}</option>`).join('')}</select><select id="sort" aria-label="命例排序"><option value="newest" ${state.sort==='newest'?'selected':''}>最近更新</option><option value="oldest" ${state.sort==='oldest'?'selected':''}>最早更新</option><option value="title" ${state.sort==='title'?'selected':''}>标题排序</option></select></div>`;
}
function filters(){return `<div class="filterline"><span class="filter-label">标签</span>${['全部',...topics].map(topic=>`<button class="filter-chip ${state.topic===topic?'active':''}" data-action="topic" data-value="${topic}">${topic}</button>`).join('')}</div>`}
function pagination(total){
  const pages=Math.max(1,Math.ceil(total/12));
  state.page=Math.min(state.page,pages);
  return `<div class="pager"><button data-action="page" data-value="${state.page-1}" ${state.page===1?'disabled':''} aria-label="上一页">${icon('back')}</button>${Array.from({length:pages},(_,index)=>`<button data-action="page" data-value="${index+1}" class="mono ${state.page===index+1?'active':''}" aria-label="第 ${index+1} 页" ${state.page===index+1?'aria-current="page"':''}>${index+1}</button>`).join('')}<button data-action="page" data-value="${state.page+1}" ${state.page===pages?'disabled':''} aria-label="下一页">${icon('next')}</button></div>`;
}
function empty(){return `<div class="empty">${icon('search')}<h3>没有匹配的命例</h3><p>试试其他关键词，或清除当前筛选。</p><button class="button outline" data-action="clear">清除筛选</button></div>`}
function table(){
  const result=filtered();const pager=pagination(result.length);const page=result.slice((state.page-1)*12,state.page*12);
  return `<div class="data-panel">${state.selected.size?`<div class="batchbar"><span>已选择 ${state.selected.size} 条</span><button data-action="batch-star">加入收藏</button><button class="danger" data-action="batch-trash">${state.tab==='trash'?'恢复所选':'移入回收站'}</button><div class="spacer"></div><button data-action="unselect">取消选择</button></div>`:''}${result.length?`<div class="table-scroll"><table class="table"><thead><tr><th class="checkcell"><input type="checkbox" id="all" aria-label="选择本页命例" ${page.every(record=>state.selected.has(record.id))?'checked':''}></th><th class="namecol">命例</th><th class="chartcol">盘面摘要</th><th class="tagcol">标签</th><th class="sourcecol">来源</th><th class="datecol">更新时间</th></tr></thead><tbody>${page.map(record=>`<tr class="${state.selected.has(record.id)?'selected':''}"><td class="checkcell"><input type="checkbox" data-select="${record.id}" aria-label="选择 ${escapeHtml(record.title)}" ${state.selected.has(record.id)?'checked':''}></td><td class="casecell" data-open="${record.id}"><button class="record-title" data-open="${record.id}">${escapeHtml(record.title)}</button><small>${escapeHtml(record.note)}</small></td><td class="chartcol"><span class="chart-lines">${escapeHtml(record.summary)}</span></td><td class="tagcol"><div class="tagset">${chips(record.tags)}</div></td><td class="sourcecol muted">${record.source}</td><td class="datecol muted mono" style="font-size:11px">${record.date}</td></tr>`).join('')}</tbody></table></div>`:empty()}<footer class="table-footer"><span>共 <span class="mono">${result.length}</span> 条${result.length?`，显示 ${(state.page-1)*12+1}–${Math.min(state.page*12,result.length)}`:''}</span>${pager}</footer></div>`;
}
function catalog(){return `<div class="page catalog-page">${heading('命例库',`<span class="title-count">${records.filter(record=>!record.trash).length} 条</span>`)}${toolbar()}${filters()}${table()}</div>`}
function caseCard(record){return `<button class="case-card" data-open="${record.id}" aria-label="查看 ${escapeHtml(record.title)}"><div class="card-art">${chartMarkup(record)}</div><div class="card-body"><h3 class="ellipsis">${escapeHtml(record.title)}</h3><div class="mini-data">${escapeHtml(record.summary)}</div><p>${escapeHtml(record.note)}</p><div class="card-foot">${chips(record.tags)}<time>${record.date.slice(5).replace('-','月')}日</time></div></div></button>`}
function gallery(){const result=filtered();const pager=pagination(result.length);return `<div class="page gallery-page">${heading('命例库',`<span class="title-count">${records.filter(record=>!record.trash).length} 条</span>`)}${toolbar()}${filters()}${result.length?`<div class="gallery-grid">${result.slice((state.page-1)*12,state.page*12).map(caseCard).join('')}</div>`:empty()}<div class="gallery-footer"><span>共 <span class="mono">${result.length}</span> 条命例</span>${pager}</div></div>`}
function rail(){return `<aside class="group-rail"><nav aria-label="命例导航"><button class="rail-item ${state.topic==='全部'&&state.tab!=='trash'?'active':''}" data-action="rail-all">${icon('archive')}全部命例<span class="count">${records.filter(record=>!record.trash).length}</span></button><button class="rail-item ${state.tab==='trash'?'active':''}" data-action="tab" data-value="trash">${icon('trash')}回收站<span class="count">${records.filter(record=>record.trash).length}</span></button></nav><div class="rail-tags"><div class="rail-section">标签</div>${topics.map(topic=>`<button class="rail-item ${state.topic===topic?'active':''}" data-action="topic" data-value="${topic}"><span class="rail-dot"></span>${topic}<span class="count">${countFor(topic)}</span></button>`).join('')}</div><div class="rail-source"><div class="rail-section">来源</div>${sourceNames.map(source=>`<button class="rail-item ${state.source===source?'active':''}" data-action="rail-source" data-value="${source}">${icon('folder')}${source}</button>`).join('')}</div></aside>`}
function grouped(){const title=state.tab==='trash'?'回收站':state.starOnly?'收藏':state.topic==='全部'?'全部命例':state.topic;return `<div class="group-shell">${rail()}<section class="group-main">${heading(title,`<span class="title-count">${filtered().length} 条</span>`)}${toolbar(false)}${table()}</section></div>`}
function shelf(){
  if(state.collection){return `<div class="page shelf-page"><div class="crumb"><button data-action="shelf-home">专题书架</button>${icon('next')}<span>${escapeHtml(state.collection)}</span></div>${heading(state.collection)}${toolbar(false)}${table()}</div>`}
  if(state.query.trim()){const result=filtered();return `<div class="page shelf-page">${heading('书架中的命例')}${toolbar(false)}${table()}</div>`}
  return `<div class="page shelf-page"><div class="shelf-heading">${heading('专题书架','','按研究主题整理命例，每条命例可归入多个专题。')}<div class="toolbar">${search()}<div class="spacer"></div><button class="button outline" data-action="all-shelf">${icon('list')}全部命例</button></div></div><div class="collection-grid">${collections.map(collection=>`<button class="collection" data-action="collection" data-value="${collection.name}" aria-label="打开 ${collection.name}"><div class="book-cover" style="--cover:${collection.cover};--ribbon:${collection.ribbon}"><span class="book-mark"></span><span class="book-meta">知几　命例集</span><span class="book-title">${collection.name}</span><span class="book-letter">${collection.letter}</span></div><div class="collection-info"><h3>${collection.name}</h3><span class="mono">${countFor(collection.topic)} 条</span></div><p class="collection-description">${collection.subtitle}</p></button>`).join('')}</div><div class="section-head"><h2>最近整理</h2><button class="button" data-action="all-shelf">查看全部 ${icon('next')}</button></div><div class="recent-list">${records.filter(record=>!record.trash).slice(0,6).map(record=>`<button class="recent-item" data-open="${record.id}">${icon('book')}<div><b class="ellipsis">${escapeHtml(record.title)}</b><small>${record.tags.join('　')} · ${record.source}</small></div><time>${record.date.slice(5)}</time></button>`).join('')}</div></div>`;
}
/* 命盘页沿用现有桌面工作区：左侧盘面、右侧参数、底部标注工具栏。 */
function inputRow(title,key,values){
  return `<div class="input-row"><span>${title}</span><div class="segmented ${values.length>10?'dense':''}">${values.map(value=>`<button data-action="input-choice" data-key="${key}" data-value="${value}" class="${state.inputChoice[key]===value?'active':''}" aria-pressed="${state.inputChoice[key]===value}">${value}</button>`).join('')}</div></div>`;
}
function settingsPanel(){
  return `<div class="native-settings"><div class="settings-heading"><button class="icon-button" data-action="close-settings" aria-label="返回排盘信息">${icon('back')}</button>设置</div><nav class="tabs"><button class="active">常规</button><button disabled>外观</button><button disabled>排盘</button><button disabled>关于</button></nav><div class="settings-group"><div class="settings-row"><span>图片倍率</span><div class="segmented"><button data-action="export-scale" data-value="1">1×</button><button class="active" data-action="export-scale" data-value="2">2×</button><button data-action="export-scale" data-value="3">3×</button></div></div><div class="settings-row"><span>恢复上次布局</span><label class="native-switch"><input type="checkbox" checked aria-label="恢复上次布局"><span></span></label></div><div class="settings-row"><span>恢复默认设置</span><button class="button" data-action="reset-settings">恢复</button></div></div></div>`;
}
function chartWorkspace(){
  const parameters=inputRow('性别','gender',['男','女'])+
    inputRow('天干','stem',['甲','乙','丙','丁','戊','己','庚','辛','壬','癸'])+
    inputRow('地支','branch',['子','丑','寅','卯','辰','巳','午','未','申','酉','戌','亥'])+
    inputRow('紫微','ziwei',['子','丑','寅','卯','辰','巳','午','未','申','酉','戌','亥'])+
    inputRow('月份','month',['正月','二月','三月','四月','五月','六月','七月','八月','九月','十月','冬月','腊月'])+
    inputRow('时辰','hour',['子','丑','寅','卯','辰','巳','午','未','申','酉','戌','亥'])+
    inputRow('命宫','main',['子','丑','寅','卯','辰','巳','午','未','申','酉','戌','亥'])+
    inputRow('大限','decade',['子','丑','寅','卯','辰','巳','午','未','申','酉','戌','亥'])+
    inputRow('流年','year',['子','丑','寅','卯','辰','巳','午','未','申','酉','戌','亥']);
  const tools=[['pointer','选择宫位'],['rectangle','矩形'],['ellipse','圆形'],['arrow','箭头'],['line','直线'],['pen','画笔'],['pen','高亮笔'],['text','文字'],['undo','撤销标记'],['undo','重做标记'],['trash','清空标记']];
  return `<div class="native-workspace ${state.sidebarCollapsed?'sidebar-collapsed':''}"><section class="chart-pane"><div class="chart-summary">${state.inputChoice.gender==='男'?'阳男':'阴女'} · ${state.inputChoice.stem}${state.inputChoice.branch}年紫微在${state.inputChoice.ziwei} · 命宫在${state.inputChoice.main}</div><div class="chart-board"><img src="${chartPath}" alt="命盘示意图，原型不执行排盘计算"></div><div class="annotation-dock"><div class="annotation-tools" role="toolbar" aria-label="命盘标记工具栏">${tools.map(([name,label],index)=>`<button class="icon-button ${index===0?'active':''}" title="${label}" aria-label="${label}" data-action="mark-tool" ${index>7?'disabled':''}>${icon(name)}</button>`).join('')}<span class="rule"></span><button class="icon-button ${state.more?'active':''}" data-action="more" aria-label="更多操作" title="更多操作">${icon('more')}</button><span class="rule"></span><button class="icon-button" data-action="export" aria-label="导出图片" title="导出图片">${icon('download')}</button></div>${state.more?`<div class="native-menu" role="menu" aria-label="更多操作">${[['新建草稿','demo-file'],['打开草稿','demo-file'],['打开最近…','demo-file'],['保存草稿…','demo-file'],['另存为…','demo-file'],['保存为命例…','new'],['导入为命例…','import'],['命例库…','open-library']].map(([title,action])=>`<button role="menuitem" data-action="${action}">${title}</button>`).join('')}<hr><button role="menuitem" data-action="copy-image">复制图片</button><button role="menuitem" data-action="hide-marks">隐藏标注</button></div>`:''}</div></section><aside class="parameter-pane">${state.panel==='settings'?settingsPanel():`<div class="parameter-title">排盘信息<button class="icon-button" data-action="reset-input" aria-label="重置排盘参数" title="重置排盘参数">${icon('undo')}</button></div><div class="input-rows ${state.twoColumns?'two-columns':''}">${parameters}</div><div class="display-heading">显示设置</div><div class="display-toggles">${['来因','宫干','宫支','宫职','流年','位星'].map(title=>`<button class="display-toggle active" aria-pressed="true" data-action="display-toggle"><i></i>${title}</button>`).join('')}</div><div class="display-heading row">生年四化<div class="spacer"></div><button class="button" data-action="all-transform">全部</button></div><div class="transform-row">${['A 禄','B 权','C 科','D 忌'].map(value=>`<button data-action="display-toggle" class="active">${value}</button>`).join('')}</div><div class="display-heading">自化向性</div><div class="display-toggles direction"><button data-action="display-toggle" class="display-toggle active">← 向心</button><button data-action="display-toggle" class="display-toggle active">→ 离心</button></div>`}</aside></div>`;
}
function detail(){
  const record=records.find(record=>record.id===state.recordId)||records[0];
  if(state.route==='chart')return `<section class="detail-page"><button class="button" data-action="detail">${icon('back')}返回命例资料</button><div class="detail-chart-only">${chartMarkup(record)}</div></section>`;
  return `<section class="detail-page"><button class="button detail-back" data-action="home">${icon('back')}返回${state.collection?'专题':'命例库'}</button><div class="detail-heading"><div><h1>${escapeHtml(record.title)}</h1><p>${escapeHtml(record.summary)}</p><div class="tagset">${chips(record.tags)}</div></div><div class="actions"><button class="button" data-action="edit">编辑资料</button><button class="button primary" data-action="open-chart">打开盘面</button></div></div><div class="detail-content"><figure class="full-chart" style="margin:0">${chartMarkup(record)}</figure><article class="detail-notes"><h2>分析笔记</h2><p>${escapeHtml(record.note)}</p><h3>观察记录</h3><p>先保留命例资料和事件经过，再分别记录盘面判断。回访时把实际反馈补充在原始记录之后，避免用后来的结果改写最初判断。</p><h3>待核实事项</h3><p>补充关键事件的具体时间，并明确资料来自当事人陈述、课堂讨论还是书籍记载。</p><dl><dt>资料来源</dt><dd>${record.source}</dd><dt>更新时间</dt><dd class="mono">${record.date}</dd><dt>创建时间</dt><dd class="mono">2026-10-12</dd></dl><div class="row detail-secondary"><button class="button" data-action="copy">复制命例</button><button class="button danger" data-action="trash">${record.trash?'恢复命例':'移入回收站'}</button></div></article></div></section>`;
}
function editor(){
  const record=records.find(record=>record.id===state.recordId);
  return `<section class="detail-page edit-page"><button class="button detail-back" data-action="${record?'detail':'home'}">${icon('back')}返回${record?'命例':'命例库'}</button><div class="edit-heading"><h1>${record?'编辑命例资料':state.importName?'导入为命例':'保存为命例'}</h1><p class="muted">${record?escapeHtml(record.summary):'阳男 · 甲子年紫微在辰 · 命宫在辰'}</p></div><form id="editor"><div class="form-grid"><label class="wide">标题<input name="title" required maxlength="120" value="${escapeHtml(record?.title||state.importName||'甲子年 · 当前盘面')}" placeholder="为这条命例命名"></label><label>标签<input name="tags" value="${escapeHtml(record?.tags.join('，')||'')}" placeholder="用逗号分隔"></label><label>来源<select name="source">${sourceNames.map(source=>`<option ${record?.source===source?'selected':''}>${source}</option>`).join('')}</select></label><label class="wide">分析笔记<textarea name="note" placeholder="记录事件、观察与待核实事项">${escapeHtml(record?.note||'')}</textarea></label></div><div class="form-actions"><button type="button" class="button" data-action="${record?'detail':'home'}">取消</button><button class="button primary" type="submit">保存命例</button></div></form></section>`;
}
const variants=[catalog,gallery,grouped,shelf];
function unifiedRail(){
  const results=state.librarySection==='results';
  return `<aside class="group-rail unified-rail"><nav aria-label="命例导航">
    <button class="rail-item ${results&&state.tab==='library'&&state.topic==='全部'&&!state.collection?'active':''}" data-action="unified-all">${icon('archive')}全部命例<span class="count">${records.filter(record=>!record.trash).length}</span></button>
    <button class="rail-item ${!results?'active':''}" data-action="unified-shelf">${icon('book')}专题书架</button>
    <button class="rail-item ${results&&state.tab==='trash'?'active':''}" data-action="unified-trash">${icon('trash')}回收站<span class="count">${records.filter(record=>record.trash).length}</span></button>
    <div class="rail-section">标签</div>${topics.map(topic=>`<button class="rail-item ${results&&!state.collection&&state.topic===topic?'active':''}" data-action="unified-topic" data-value="${topic}"><span class="rail-dot"></span>${topic}<span class="count">${countFor(topic)}</span></button>`).join('')}
    <div class="rail-section">来源</div>${sourceNames.map(source=>`<button class="rail-item ${results&&state.source===source?'active':''}" data-action="unified-source" data-value="${source}">${icon('folder')}${source}</button>`).join('')}
  </nav></aside>`;
}
function unifiedControls(){
  return `<div class="unified-toolbar">${search()}<select id="source" aria-label="按来源筛选">${['全部来源',...sourceNames].map(source=>`<option ${source===state.source?'selected':''}>${source}</option>`).join('')}</select><select id="sort" aria-label="命例排序"><option value="newest" ${state.sort==='newest'?'selected':''}>最近更新</option><option value="oldest" ${state.sort==='oldest'?'selected':''}>最早更新</option><option value="title" ${state.sort==='title'?'selected':''}>标题排序</option></select><div class="spacer"></div><div class="view-switch" role="group" aria-label="命例显示方式"><button data-action="unified-view" data-value="list" aria-label="列表视图" title="列表视图" aria-pressed="${state.libraryView==='list'}" class="${state.libraryView==='list'?'active':''}">${icon('list')}</button><button data-action="unified-view" data-value="cards" aria-label="卡片视图" title="卡片视图" aria-pressed="${state.libraryView==='cards'}" class="${state.libraryView==='cards'?'active':''}">${icon('grid')}</button></div></div>`;
}
function unifiedCards(){
  const result=filtered();const pager=pagination(result.length);
  return `${result.length?`<div class="gallery-grid">${result.slice((state.page-1)*12,state.page*12).map(caseCard).join('')}</div>`:empty()}<div class="gallery-footer"><span>共 <span class="mono">${result.length}</span> 条命例</span>${pager}</div>`;
}
function unifiedShelf(){
  return `<div class="unified-shelf-scroll"><div class="collection-grid">${collections.map(collection=>`<button class="collection" data-action="unified-collection" data-value="${collection.name}" aria-label="打开 ${collection.name}"><div class="book-cover" style="--cover:${collection.cover};--ribbon:${collection.ribbon}"><span class="book-mark"></span><span class="book-meta">知几　命例集</span><span class="book-title">${collection.name}</span><span class="book-letter">${collection.letter}</span></div><div class="collection-info"><h3>${collection.name}</h3><span class="mono">${countFor(collection.topic)} 条</span></div><p class="collection-description">${collection.subtitle}</p></button>`).join('')}</div><div class="section-head"><h2>最近整理</h2><button class="button" data-action="unified-all">查看全部 ${icon('next')}</button></div><div class="recent-list">${records.filter(record=>!record.trash).slice(0,6).map(record=>`<button class="recent-item" data-open="${record.id}">${icon('book')}<div><b class="ellipsis">${escapeHtml(record.title)}</b><small>${record.tags.join('　')} · ${record.source}</small></div><time>${record.date.slice(5)}</time></button>`).join('')}</div></div>`;
}
function unifiedLibrary(){
  const shelfHome=state.librarySection==='shelf';
  const title=shelfHome?'专题书架':state.tab==='trash'?'回收站':state.collection|| (state.topic!=='全部'?state.topic:'全部命例');
  return `<div class="group-shell unified-shell ${state.railOpen?'':'rail-closed'}">${state.railOpen?unifiedRail():''}<section class="group-main unified-main"><div class="page-heading"><button class="icon-button rail-toggle" data-action="unified-rail" aria-label="${state.railOpen?'收起':'展开'}命例导航" title="${state.railOpen?'收起':'展开'}命例导航">${icon('sidebar')}</button><h1>${escapeHtml(title)}${shelfHome?'':`<span class="title-count">${filtered().length} 条</span>`}</h1><div class="spacer"></div><button class="button" data-action="import">导入草稿…</button><button class="button primary" data-action="new">保存当前盘面</button></div>${shelfHome?unifiedShelf():`${state.collection?`<div class="unified-crumb"><button data-action="unified-shelf">专题书架</button>${icon('next')}<span>${escapeHtml(state.collection)}</span></div>`:''}${unifiedControls()}${state.libraryView==='list'?table():unifiedCards()}`}</section></div>`;
}
function render(){
  stage.innerHTML=shell(state.route==='workspace'?chartWorkspace():state.route==='edit'?editor():state.route==='detail'||state.route==='chart'?detail():unified?unifiedLibrary():variants[current]());
  if(state.route==='workspace'&&state.installedId){
    const record=records.find(item=>item.id===state.installedId);
    const image=stage.querySelector('.chart-board img');
    const summary=stage.querySelector('.chart-summary');
    if(state.chartStale){image.replaceWith(Object.assign(document.createElement('span'),{className:'muted',textContent:'原型不支持重新排盘'}));}
    else if(chartImage(record)){image.src=chartImage(record);image.alt=`${record.title}的盘面`;}
    summary.textContent=state.chartStale?'参数已更改':`${record.title.split(' · ')[0]} · ${record.summary}`;
  }
  bindInputs();
}
function moveHighlight(){const element=items[current];highlight.style.width=element.offsetWidth+'px';highlight.style.transform=`translateX(${element.offsetLeft}px)`}
function mount(index){state=resetState();stage.innerHTML='';requestAnimationFrame(()=>{render();window.scrollTo(0,0)})}
function setActive(index){if(index<0||index>=variants.length)return;current=index;items.forEach((element,itemIndex)=>{element.toggleAttribute('data-active',itemIndex===index);if(itemIndex===index)element.setAttribute('aria-current','true');else element.removeAttribute('aria-current')});moveHighlight();const url=new URL(location);url.searchParams.set('v',index+1);history.replaceState(null,'',url);mount(index)}
function updateList(){state.page=1;state.selected.clear();const input=document.activeElement;const focused=input?.id;const position=input?.selectionStart;render();if(focused){const replacement=document.getElementById(focused);replacement?.focus();if(typeof position==='number')replacement?.setSelectionRange?.(position,position)}}
function bindInputs(){
  document.getElementById('draft-import')?.addEventListener('change',event=>{
    const file=event.target.files?.[0];
    if(!file)return;
    state.importName=file.name.replace(/\.ziwei$/i,'');
    state.recordId=null;
    state.route='edit';
    state.more=false;
    render();
  });
  document.getElementById('search')?.addEventListener('input',event=>{state.query=event.target.value;updateList()});
  document.getElementById('source')?.addEventListener('change',event=>{state.source=event.target.value;updateList()});
  document.getElementById('sort')?.addEventListener('change',event=>{state.sort=event.target.value;updateList()});
  document.querySelectorAll('[data-select]').forEach(input=>input.addEventListener('change',()=>{const id=Number(input.dataset.select);input.checked?state.selected.add(id):state.selected.delete(id);render()}));
  document.getElementById('all')?.addEventListener('change',event=>{filtered().slice((state.page-1)*12,state.page*12).forEach(record=>event.target.checked?state.selected.add(record.id):state.selected.delete(record.id));render()});
  document.getElementById('editor')?.addEventListener('submit',event=>{event.preventDefault();const data=new FormData(event.target);let record=records.find(record=>record.id===state.recordId);if(!record){record={id:Math.max(...records.map(record=>record.id))+1,summary:'阳男 · 命宫在辰',date:'2026-11-15',star:false,trash:false};records.unshift(record)}record.title=String(data.get('title')).trim();record.tags=String(data.get('tags')).split(/[,，]/).map(tag=>tag.trim()).filter(Boolean);record.source=String(data.get('source'));record.note=String(data.get('note'));state.recordId=record.id;state.route='detail';render();notify('已保存至本次原型演示')});
}
stage.addEventListener('click',event=>{
  const open=event.target.closest('[data-open]');if(open){state.recordId=Number(open.dataset.open);state.route='detail';render();window.scrollTo(0,0);return}
  const button=event.target.closest('[data-action]');if(!button)return;
  const action=button.dataset.action,value=button.dataset.value;
  const record=records.find(record=>record.id===state.recordId);
  if(action==='unified-rail'){state.railOpen=!state.railOpen;render();return}
  if(action==='unified-view'){state.libraryView=value;render();return}
  if(action==='unified-shelf'){state.librarySection='shelf';state.collection=null;state.query='';state.tab='library';render();return}
  if(action==='unified-all'){state.librarySection='results';state.collection=null;state.topic='全部';state.source='全部来源';state.tab='library';updateList();return}
  if(action==='unified-trash'){state.librarySection='results';state.collection=null;state.topic='全部';state.source='全部来源';state.tab='trash';updateList();return}
  if(action==='unified-topic'){state.librarySection='results';state.collection=null;state.topic=value;state.tab='library';updateList();return}
  if(action==='unified-source'){state.librarySection='results';state.collection=null;state.source=state.source===value?'全部来源':value;state.tab='library';updateList();return}
  if(action==='unified-collection'){const collection=collections.find(item=>item.name===value);state.librarySection='results';state.collection=value;state.topic=collection.topic;state.source='全部来源';state.tab='library';updateList();return}
  if(action==='workspace'){state.route='workspace';state.more=false;state.panel='chart';render();if(unified)stage.querySelector('[data-action="more"]')?.focus();return}
  if(action==='open-library'){state.route='library';state.more=false;render();if(unified)stage.querySelector('.library-close')?.focus();return}
  if(action==='open-chart'){
    if(!chartImage(record)){notify('该命例暂无盘面预览');return}
    const index=record.chartIndex;
    const year=titleSeeds[index][0].slice(0,2);
    state.inputChoice={gender:index%2?'女':'男',stem:year[0],branch:year[1],ziwei:chartZiwei[index],month:'正月',hour:'子',main:['辰','午','酉','寅'][index%4],decade:'',year:''};
    state.installedId=state.recordId;state.chartStale=false;state.route='workspace';state.more=false;state.panel='chart';render();return
  }
  if(action==='more'){state.more=!state.more;render();return}
  if(action==='settings'){state.route='workspace';state.more=false;state.panel=state.panel==='settings'?'chart':'settings';render();return}
  if(action==='close-settings'){state.panel='chart';render();return}
  if(action==='toggle-sidebar'){state.sidebarCollapsed=!state.sidebarCollapsed;render();return}
  if(action==='toggle-columns'){state.twoColumns=!state.twoColumns;render();return}
  if(action==='input-choice'){state.inputChoice[button.dataset.key]=value;if(state.installedId)state.chartStale=true;render();return}
  if(action==='display-toggle'){button.classList.toggle('active');button.setAttribute('aria-pressed',String(button.classList.contains('active')));return}
  if(action==='all-transform'){document.querySelectorAll('.transform-row button').forEach(element=>element.classList.add('active'));return}
  if(action==='export-scale'){button.parentElement.querySelectorAll('button').forEach(element=>element.classList.toggle('active',element===button));return}
  if(action==='reset-settings'){state.panel='chart';render();notify('已重置演示设置');return}
  if(action==='reset-input'){state.inputChoice=resetState().inputChoice;if(state.installedId)state.chartStale=true;render();return}
  if(action==='mark-tool'){document.querySelectorAll('.annotation-tools .icon-button').forEach(element=>element.classList.toggle('active',element===button));return}
  if(action==='import'){document.getElementById('draft-import').click();return}
  if(action==='date-source'){notify('此原型仅比较命例库，日期取数沿用现有 App');return}
  if(action==='demo-file'||action==='export'||action==='copy-image'||action==='hide-marks'){state.more=false;render();notify('该操作沿用现有 App，本原型不读写草稿与图片');return}
  if(action==='theme'){document.body.classList.toggle('theme-dark');return}
  if(action==='home'){state.route='library';render();window.scrollTo(0,0);return}
  if(action==='new'){state.recordId=null;state.importName='';state.more=false;state.route='edit';render();document.querySelector('[name=title]')?.focus();return}
  if(action==='chart'){state.recordId ||= records[0].id;state.route='chart';render();window.scrollTo(0,0);return}
  if(action==='detail'){state.route='detail';render();return}
  if(action==='edit'){state.route='edit';render();return}
  if(action==='topic'){state.topic=value;state.starOnly=false;state.tab='library';updateList();return}
  if(action==='tab'){state.tab=value;state.topic='全部';state.starOnly=false;updateList();return}
  if(action==='rail-all'){state=resetState();render();return}
  if(action==='star-only'){state.topic='全部';state.starOnly=true;state.tab='library';updateList();return}
  if(action==='rail-source'){state.source=state.source===value?'全部来源':value;updateList();return}
  if(action==='page'){state.page=Number(value);render();return}
  if(action==='clear'){state.query='';state.topic='全部';state.source='全部来源';state.starOnly=false;updateList();return}
  if(action==='reset'){location.reload();return}
  if(action==='unselect'){state.selected.clear();render();return}
  if(action==='batch-star'){records.filter(record=>state.selected.has(record.id)).forEach(record=>record.star=true);state.selected.clear();render();notify('所选命例已收藏');return}
  if(action==='batch-trash'){records.filter(record=>state.selected.has(record.id)).forEach(record=>record.trash=state.tab!=='trash');state.selected.clear();render();notify('已更新所选命例');return}
  if(action==='star'&&record){record.star=!record.star;render();notify(record.star?'已收藏':'已取消收藏');return}
  if(action==='trash'&&record){record.trash=!record.trash;state.route='library';render();notify(record.trash?'已移入回收站，可恢复':'已恢复命例');return}
  if(action==='copy'&&record){const copy={...record,id:Math.max(...records.map(record=>record.id))+1,title:record.title+' 副本',tags:[...record.tags]};records.unshift(copy);state.recordId=copy.id;render();notify('已建立独立副本');return}
  if(action==='collection'){const collection=collections.find(collection=>collection.name===value);state.collection=value;state.topic=collection.topic;state.page=1;render();return}
  if(action==='shelf-home'){state=resetState();render();return}
  if(action==='back-collection'){state.route='library';render();return}
  if(action==='all-shelf'){state.collection='全部命例';state.topic='全部';state.page=1;render();return}
});
items.forEach((element,index)=>element.addEventListener('click',()=>setActive(index)));
window.addEventListener('resize',moveHighlight);
document.addEventListener('keydown',event=>{
  if(unified&&state.route!=='workspace'&&event.key==='Escape'){
    event.preventDefault();
    if(state.route==='detail'||state.route==='edit'||state.route==='chart')state.route='library';
    else state.route='workspace';
    render();return;
  }
  if(unified&&state.route!=='workspace'&&event.key==='Tab'){
    const controls=[...stage.querySelectorAll('.library-dialog button:not([disabled]), .library-dialog input:not([disabled]), .library-dialog select:not([disabled]), .library-dialog textarea:not([disabled])')].filter(element=>element.getClientRects().length);
    if(controls.length){const first=controls[0],last=controls[controls.length-1];if(event.shiftKey&&(document.activeElement===first||!controls.includes(document.activeElement))){event.preventDefault();last.focus()}else if(!event.shiftKey&&(document.activeElement===last||!controls.includes(document.activeElement))){event.preventDefault();first.focus()}}
  }
  if(/^(INPUT|TEXTAREA|SELECT)$/.test(event.target.tagName)||event.target.isContentEditable||event.metaKey||event.ctrlKey||event.altKey)return;
  const number=parseInt(event.key,10);
  if(!unified&&number>=1&&number<=4)setActive(number-1);
  else if(!unified&&event.key==='ArrowRight')setActive((current+1)%4);
  else if(!unified&&event.key==='ArrowLeft')setActive((current+3)%4);
  else if(event.key==='/'){event.preventDefault();document.getElementById('search')?.focus()}
  else if(event.key==='Escape'&&state.route!=='library'){state.route='library';render()}
});
setActive(current);
requestAnimationFrame(()=>requestAnimationFrame(()=>picker.setAttribute('data-ready','')));
