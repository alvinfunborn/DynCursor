'use strict';
const $ = selector => document.querySelector(selector);
const invoke = (command,args={}) => window.__TAURI__ ? window.__TAURI__.core.invoke(command,args) : Promise.reject(new Error('未连接本机程序。请从 DynCursor.exe 打开设置窗口。'));
const types=[{id:32512,source:32651,key:'arrow_file',name:'箭头',hint:'鼠标移到普通区域时',asset:'arrow',glyph:'↖'},{id:32513,source:32516,key:'text_file',name:'文字里的 I 形',hint:'鼠标移到可以输入文字的地方时',asset:'beam',glyph:'Ⅰ'},{id:32649,source:32648,key:'link_file',name:'链接上的小手',hint:'鼠标移到链接上时',asset:'link',glyph:'☝'}];
const sources=[[32512,'正常选择'],[32513,'文本选择'],[32514,'忙'],[32515,'精确选择'],[32516,'候选'],[32642,'沿对角线调整大小 1'],[32643,'沿对角线调整大小 2'],[32644,'水平调整大小'],[32645,'垂直调整大小'],[32646,'移动'],[32648,'不可用'],[32649,'链接选择'],[32650,'后台运行'],[32651,'帮助选择']];
let draft,saved,busy=false,autostart=false,polling=false,lastRuntimeError=null,toastTimer,method='files',status;
const clone=value=>JSON.parse(JSON.stringify(value));
function error(e){$('#error').textContent=String(e.message||e);$('#error').hidden=false;$('#error').scrollIntoView({block:'nearest'});}
function clearError(){$('#error').hidden=true;}
function toast(message){clearTimeout(toastTimer);$('#toast').textContent=message;$('#toast').hidden=false;toastTimer=setTimeout(()=>$('#toast').hidden=true,4500);}
function dirty(){return !!draft&&JSON.stringify(draft)!==JSON.stringify(saved);}
function mappings(){try{return SettingsModel.parseMappings(draft.cursor_mapping);}catch{return new Map();}}
function sourceName(type){const id=mappings().get(type.id)||type.source;return sources.find(([key])=>key===id)?.[1]||String(id);}
function updateSave(){
  $('#save').disabled=busy||!dirty();$('#discard').disabled=busy||!dirty();$('#save-state').textContent=busy?'正在保存…':!draft?'无法载入设置':dirty()?'有更改还没保存':'已保存，可以试试效果';
  document.querySelectorAll('[data-setting], [data-cursor], [data-source], [data-pick], [data-fallback], #language-preset, #builtin, #use-windows, #defaults, #reload').forEach(el=>el.disabled=busy||!draft||(el.dataset.source&&!mappings().has(+el.dataset.source)));
  $('#try-notepad').disabled=busy||!draft;
}
function updateLanguage(keepCustom=false){
  const view=SettingsModel.languagePresentation(draft.custom_layouts);
  $('#language-preset').value=keepCustom?'custom':view.preset;$('#custom-language').hidden=!keepCustom&&view.preset!=='custom';
  $('#language-hint').textContent=view.hint;$('#principle').textContent=view.principle;
  $('#appearance-title').textContent=`${view.active}时，换成什么图案？`;
  const chinese=['0804','0404','0c04'].includes(view.preset);
  $('#try-title').textContent=chinese?'保存后，切换中英文试一试。':'保存后，切换输入状态试一试。';
  $('#try-instructions').textContent=chinese?'点下方“保存并应用”，再打开记事本，切换输入法的“中 / 英”（通常按 Shift）。把鼠标移到普通区域、文字区，看看图案有没有变化。':'点下方“保存并应用”，再打开记事本，在选定语言的输入模式与字母或其他语言之间切换。把鼠标移到普通区域、文字区，看看图案有没有变化。';
  document.querySelectorAll('[data-active-label]').forEach(el=>el.textContent=view.active);
  document.querySelectorAll('[data-original-label]').forEach(el=>el.textContent=view.original);
}
function showMethod(next){method=next;$('#file-method').hidden=next!=='files';$('#windows-method').hidden=next!=='windows';document.querySelectorAll('[data-method]').forEach(el=>el.setAttribute('aria-pressed',String(el.dataset.method===next)));}
function updateMethodSummary(){
  const mode=SettingsModel.configurationMode(draft);
  $('#current-method').textContent=(dirty()?'保存后将使用：':'当前设置：')+(mode==='windows'?'Windows 图案。按 README 配过的不用重做。':mode==='files'?'直接使用光标文件，不需要先改 Windows。':'部分使用文件、部分沿用 Windows 图案，可以分别调整。');
  $('#windows-file-warning').hidden=mode==='windows';
}
async function preview(type){
  const path=draft[type.key],box=$(`#preview-${type.id}`),caption=$(`#file-label-${type.id}`);box.replaceChildren();box.classList.remove('preview-error');
  if(!path){box.textContent='Windows';caption.textContent=`使用“${sourceName(type)}”的图案`;caption.title='图案来自你在 Windows 中设置的方案';return;}
  caption.textContent=path.split(/[\\/]/).pop();caption.title=path;box.textContent='载入图案…';
  try{const data=await invoke('cursor_preview',{path});if(draft[type.key]!==path||!box.isConnected)return;const img=new Image();img.src=data;img.alt=`${type.name}选中的图案`;box.replaceChildren(img);}
  catch(e){if(draft[type.key]===path&&box.isConnected){box.textContent='无法预览';box.classList.add('preview-error');box.title=String(e);}}
}
function renderWindowsGuide(){
  const table=$('#windows-mapping');table.replaceChildren();
  types.forEach(type=>{const row=document.createElement('div');row.className='windows-map-row';const check=document.createElement('input');check.type='checkbox';check.dataset.cursor=type.id;check.checked=mappings().has(type.id);check.setAttribute('aria-label',`切换${type.name}`);const from=document.createElement('strong'),to=document.createElement('span');from.textContent=`Windows 里选“${sourceName(type)}”`;to.textContent=`选择${type.name}的替换图案`;row.append(check,from,document.createTextNode(' → '),to);table.append(row);});
}
function renderSources(){
  const container=$('#source-settings');container.replaceChildren();
  types.forEach(type=>{const row=document.createElement('div');row.className='source-row';const label=document.createElement('label');label.htmlFor=`source-${type.id}`;label.textContent=`${type.name}使用`;const select=document.createElement('select');select.id=`source-${type.id}`;select.dataset.source=type.id;sources.forEach(([id,name])=>select.add(new Option(`${name} · ${id}`,id)));select.value=mappings().get(type.id)||type.source;row.append(label,select);container.append(row);});
}
function renderCards(){
  const container=$('#cursor-cards');container.replaceChildren();
  types.forEach(type=>{
    const card=document.createElement('div');card.className='card cursor-card';card.innerHTML=`<div class="cursor-header"><input id="cursor-${type.id}" type="checkbox" data-cursor="${type.id}"><div><label for="cursor-${type.id}">${type.name}</label><small>${type.hint}</small></div></div><div class="appearance-comparison"><div><span class="comparison-label" data-original-label></span><span class="original-glyph" aria-hidden="true">${type.glyph}</span><small>原来的图案</small></div><span class="comparison-arrow" aria-hidden="true">→</span><div><span class="comparison-label" data-active-label></span><div class="cursor-preview" id="preview-${type.id}"></div><small id="file-label-${type.id}"></small></div></div><div class="button-row"><button data-pick="${type.id}">选择${type.name}图案…</button><button class="text-button" data-fallback="${type.id}">沿用 Windows 图案</button></div><details class="file-details"><summary>查看或手动填写文件路径</summary><label class="sr-only" for="file-${type.id}">${type.name}文件路径</label><input id="file-${type.id}" type="text" data-setting="${type.key}" placeholder="空白表示沿用 Windows 图案" spellcheck="false"></details>`;
    container.append(card);card.querySelector('[data-cursor]').checked=mappings().has(type.id);card.querySelector('[data-setting]').value=draft[type.key];preview(type);
  });
  renderSources();renderWindowsGuide();updateLanguage();updateMethodSummary();updateSave();
}
function render(){document.querySelectorAll('[data-setting]').forEach(el=>{const value=draft[el.dataset.setting];if(el.type==='checkbox')el.checked=value;else el.value=value;});renderCards();showMethod(method);}
function navigate(id){document.querySelectorAll('.page').forEach(page=>page.hidden=page.id!==id);document.querySelectorAll('nav a').forEach(a=>{if(a.hash===`#${id}`)a.setAttribute('aria-current','page');else a.removeAttribute('aria-current');});$('#page-title').textContent=$(`nav a[href="#${id}"] .nav-title`).textContent;window.scrollTo(0,0);}
async function confirmation(title,text,accept='继续'){
  const dialog=$('#confirmation');dialog.returnValue='cancel';$('#confirm-title').textContent=title;$('#confirm-text').textContent=text;$('#confirm-accept').textContent=accept;
  return new Promise(resolve=>{dialog.addEventListener('close',()=>resolve(dialog.returnValue==='confirm'),{once:true});dialog.showModal();});
}
async function save(){
  if(busy||!draft)return false;clearError();try{SettingsModel.validate(draft);}catch(e){error(e);return false;}
  busy=true;updateSave();try{saved=await invoke('save_settings',{settings:clone(draft)});draft=clone(saved);render();toast(status?.paused?'已保存。当前仍暂停，点右上角“继续切换”即可运行。':'已保存，切换输入法试试图案。');await poll();return true;}catch(e){error(e);return false;}finally{busy=false;updateSave();}
}
async function runAction(action){
  clearError();try{await invoke('action',{action});if(action==='mouse')toast('已暂停并打开 Windows 设置。应用图案后，回来点“我已应用，继续切换”。');else if(action==='recapture')toast('已重新读取 Windows 图案，继续切换。');else if(action==='restore')toast('已暂停，恢复原来的图案。');await poll();}catch(e){error(e);}
}
async function poll(){
  if(polling)return;polling=true;
  try{const s=await invoke('get_status');status=s;const paused=s.paused||!s.enabled;
    $('#connection').textContent=s.error?'需要处理':paused?'已暂停':s.custom===true?'正在使用替换图案':'正在使用原来图案';$('#sidebar-status').textContent=paused?'已暂停切换':'正在跟随输入法';
    $('#pause').textContent=paused?'继续切换':'暂停切换';$('#pause').disabled=!s.enabled;$('#pause').title=!s.enabled?'先开启“跟随输入法切换”并保存':'';
    $('#locale').textContent=s.locale||'尚未检测';$('#cursor-state').textContent=s.error?'状态待确认':paused?'原来图案':s.custom===true?'替换图案':s.custom===false?'原来图案':'等待检测';
    $('#version').textContent=`DynCursor ${s.version}`;$('#about-version').textContent=s.version;
    if(s.error){$('#error').textContent=s.error;$('#error').hidden=false;}else if(lastRuntimeError&&$('#error').textContent===lastRuntimeError){clearError();}lastRuntimeError=s.error;return s;
  }catch(e){$('#connection').textContent='连接中断';error(e);}finally{polling=false;}
}
// Navigation and form edits only change the draft. Switching the help tabs never changes files.
document.querySelectorAll('nav a, .brand').forEach(link=>link.addEventListener('click',e=>{e.preventDefault();navigate(link.hash.slice(1));}));
document.querySelectorAll('[data-method]').forEach(button=>button.addEventListener('click',()=>showMethod(button.dataset.method)));
$('#language-preset').addEventListener('change',e=>{if(!draft||busy)return;if(e.target.value==='custom'){$('#custom-language').hidden=false;$('#layouts').focus();return;}draft.custom_layouts=e.target.value;$('#layouts').value=draft.custom_layouts;updateLanguage();updateSave();});
document.addEventListener('input',e=>{const el=e.target;if(!draft||busy||!el.dataset.setting)return;draft[el.dataset.setting]=el.type==='checkbox'?el.checked:el.type==='number'?Number(el.value):el.value;const type=types.find(type=>type.key===el.dataset.setting);if(type){preview(type);updateMethodSummary();}if(el.dataset.setting==='custom_layouts')updateLanguage(true);updateSave();});
$('#mapping').addEventListener('change',()=>{if(draft)renderCards();});
document.addEventListener('change',e=>{const el=e.target,id=+(el.dataset.cursor||el.dataset.source);if(!draft||busy||!id)return;try{const type=types.find(type=>type.id===id);const enabled=el.dataset.cursor?el.checked:mappings().has(id);const source=el.dataset.source?+el.value:mappings().get(id)||type.source;draft.cursor_mapping=SettingsModel.changeMapping(draft.cursor_mapping,id,source,enabled);$('#mapping').value=draft.cursor_mapping;clearError();renderCards();}catch(e){error(e);renderCards();}});
document.addEventListener('click',async e=>{const pick=e.target.closest('[data-pick]'),fallback=e.target.closest('[data-fallback]');if((!pick&&!fallback)||!draft||busy)return;const type=types.find(type=>type.id===+(pick?.dataset.pick||fallback.dataset.fallback));try{const path=fallback?'':await invoke('pick_cursor_file');if(path===null)return;draft[type.key]=path;$(`#file-${type.id}`).value=path;preview(type);updateMethodSummary();updateSave();}catch(e){error(e);}});
$('#builtin').addEventListener('click',()=>{if(!draft||busy)return;types.forEach(type=>draft[type.key]=`resources/cur/custom-${type.asset}.cur`);renderCards();toast('已选内置绿色图案，点“保存并应用”后生效。');});
$('#use-windows').addEventListener('click',async()=>{if(!draft||busy)return;if(!await confirmation('改用 Windows 里配好的图案？','三个直接选择的文件将从设置中清空，文件本身不会删除。保存后，使用下方对应的 Windows 图案。','改用 Windows 图案'))return;types.forEach(type=>draft[type.key]='');render();await save();});
document.querySelectorAll('[data-action]').forEach(el=>el.addEventListener('click',async()=>{if(el.dataset.action==='recapture'&&dirty()){if(!await confirmation('先保存设置，再继续切换？','你刚调整了光标设置。保存后会重新读取 Windows 图案，再继续切换。','保存并继续'))return;if(!await save())return;}await runAction(el.dataset.action);}));
$('#try-notepad').addEventListener('click',async()=>{if(dirty()){if(!await confirmation('先保存刚选的设置？','保存后再打开记事本，就能验证你刚选的图案。','保存并打开记事本'))return;if(!await save())return;}await runAction('notepad');});
$('#save').addEventListener('click',save);
$('#discard').addEventListener('click',()=>{draft=clone(saved);clearError();render();});
$('#reset').addEventListener('click',async()=>{if(await confirmation('加载 Windows 已保存的方案？','会暂停切换，并重新加载 Windows 当前保存的指针方案；不会自动切回出厂图案。','加载已保存方案'))await runAction('reset');});
$('#defaults').addEventListener('click',async()=>{if(!await confirmation('把设置恢复为默认值？','当前未保存的更改会被替换，先前保存的设置暂时不变。检查后点“保存并应用”才生效；登录自启动不受影响。','载入默认值'))return;try{draft=await invoke('default_settings');method='files';render();navigate('cursors');toast('默认值已载入，点“保存并应用”后生效。');}catch(e){error(e);}});
$('#reload').addEventListener('click',async()=>{if(dirty()&&!await confirmation('放弃窗口里的更改，读取文件？','将从磁盘重新读取并应用配置，当前未保存的更改会丢弃。','重新读取配置'))return;busy=true;updateSave();try{await invoke('action',{action:'reload'});const view=await invoke('get_settings');saved=view.settings;draft=clone(saved);method=SettingsModel.configurationMode(draft)==='windows'?'windows':'files';render();toast('已重新读取配置。');}catch(e){error(e);}finally{busy=false;updateSave();}});
$('#autostart').addEventListener('change',async e=>{const el=e.target;el.disabled=true;try{autostart=await invoke('set_autostart',{enabled:el.checked});toast(autostart?'下次登录 Windows 后会自动运行。':'已关闭登录自启动。');}catch(e){error(e);}finally{el.checked=autostart;el.disabled=false;}});
async function init(){try{const view=await invoke('get_settings');saved=view.settings;draft=clone(saved);autostart=view.autostart;$('#autostart').checked=autostart;$('#autostart').disabled=false;$('#config-path').textContent=view.config_path;method=SettingsModel.configurationMode(draft)==='windows'?'windows':'files';render();const s=await poll();if(s?.smoke_test){await invoke('smoke_ready');return;}setInterval(poll,700);}catch(e){error(e);$('#connection').textContent='未连接';updateSave();}}
init();
