'use strict';
const $ = selector => document.querySelector(selector);
const invoke = (command,args={}) => window.__TAURI__ ? window.__TAURI__.core.invoke(command,args) : Promise.reject(new Error('未连接本机程序。请从 DynCursor.exe 打开设置窗口。'));
const types = [{id:32512,source:32651,key:'arrow_file',name:'普通选择',hint:'鼠标箭头'},{id:32513,source:32516,key:'text_file',name:'文本选择',hint:'输入文字时的 I 形光标'},{id:32649,source:32648,key:'link_file',name:'链接选择',hint:'悬停在链接上的手形光标'}];
const sources = [[32512,'普通选择'],[32513,'文本选择'],[32514,'忙'],[32515,'精确选择'],[32516,'候选'],[32642,'对角调整 1'],[32643,'对角调整 2'],[32644,'水平调整'],[32645,'垂直调整'],[32646,'移动'],[32648,'不可用'],[32649,'链接选择'],[32650,'后台运行'],[32651,'帮助选择']];
let draft, saved, busy=false, autostart=false, polling=false, lastRuntimeError=null, toastTimer;
const clone = value => JSON.parse(JSON.stringify(value));
function error(e){$('#error').textContent=String(e.message||e);$('#error').hidden=false;}
function clearError(){$('#error').hidden=true;}
function toast(message){clearTimeout(toastTimer);$('#toast').textContent=message;$('#toast').hidden=false;toastTimer=setTimeout(()=>$('#toast').hidden=true,4500);}
function dirty(){return !!draft&&JSON.stringify(draft)!==JSON.stringify(saved);}
function updateSave(){
  $('#save').disabled=busy||!dirty();$('#discard').disabled=busy||!dirty();$('#save-state').textContent=busy?'正在应用…':!draft?'无法载入设置':dirty()?'有未保存的更改':'设置已保存';
  document.querySelectorAll('[data-setting], [data-cursor], [data-source], [data-pick], [data-clear], #defaults, #reload').forEach(el=>el.disabled=busy||!draft);
}
async function preview(type){
  const path=draft[type.key],box=$(`#preview-${type.id}`);box.replaceChildren();box.classList.remove('preview-error');
  if(!path){box.textContent='系统';return;}box.textContent='…';
  try{const data=await invoke('cursor_preview',{path});if(draft[type.key]!==path||!box.isConnected)return;const img=new Image();img.src=data;img.alt=`${type.name}文件预览`;box.replaceChildren(img);box.title=path;}
  catch(e){if(draft[type.key]===path&&box.isConnected){box.textContent='无预览';box.classList.add('preview-error');box.title=String(e);}}
}
function renderCards(){
  const container=$('#cursor-cards');container.replaceChildren();let mappings;try{mappings=SettingsModel.parseMappings(draft.cursor_mapping);}catch{mappings=new Map();}
  types.forEach(type=>{
    const card=document.createElement('div');card.className='card';card.innerHTML=`<div class="cursor-header"><input id="cursor-${type.id}" type="checkbox" data-cursor="${type.id}"><div><label for="cursor-${type.id}">${type.name}</label><small>${type.hint}</small></div><div class="cursor-preview" id="preview-${type.id}">系统</div></div><label for="file-${type.id}">自定义 .cur 文件（可选）</label><div class="cursor-file"><input id="file-${type.id}" type="text" data-setting="${type.key}" placeholder="留空时使用 Windows 来源指针" spellcheck="false"><button data-pick="${type.id}">选择文件…</button><button data-clear="${type.id}">清除</button></div><div class="source-row"><label for="source-${type.id}">Windows 来源</label><select id="source-${type.id}" data-source="${type.id}"></select></div><p>选了文件时优先使用文件；文件留空时，使用当前 Windows 方案的来源指针。</p>`;
    container.append(card);card.querySelector('[data-cursor]').checked=mappings.has(type.id);const select=card.querySelector('select');sources.forEach(([id,name])=>select.add(new Option(`${name} · ${id}`,id)));select.value=mappings.get(type.id)||type.source;card.querySelector('[data-setting]').value=draft[type.key];preview(type);
  });
}
function render(){document.querySelectorAll('[data-setting]').forEach(el=>{const value=draft[el.dataset.setting];if(el.type==='checkbox')el.checked=value;else el.value=value;});renderCards();updateSave();}
document.querySelectorAll('nav a, .brand').forEach(link=>link.addEventListener('click',e=>{e.preventDefault();const id=link.hash.slice(1);document.querySelectorAll('.page').forEach(page=>page.hidden=page.id!==id);document.querySelectorAll('nav a').forEach(a=>{if(a.hash===link.hash)a.setAttribute('aria-current','page');else a.removeAttribute('aria-current');});$('#page-title').textContent=$(`nav a[href="#${id}"]`).textContent.trim().slice(1).trim();window.scrollTo(0,0);}));
document.addEventListener('input',e=>{const el=e.target;if(!draft||busy||!el.dataset.setting)return;draft[el.dataset.setting]=el.type==='checkbox'?el.checked:el.type==='number'?Number(el.value):el.value;const type=types.find(t=>t.key===el.dataset.setting);if(type)preview(type);updateSave();});
$('#mapping').addEventListener('change',()=>{if(draft){renderCards();updateSave();}});
document.addEventListener('change',e=>{const el=e.target,id=+(el.dataset.cursor||el.dataset.source);if(!draft||busy||!id)return;try{draft.cursor_mapping=SettingsModel.changeMapping(draft.cursor_mapping,id,+$(`#source-${id}`).value,$(`#cursor-${id}`).checked);$('#mapping').value=draft.cursor_mapping;clearError();}catch(e){error(e);renderCards();}updateSave();});
document.addEventListener('click',async e=>{const pick=e.target.closest('[data-pick]'),clear=e.target.closest('[data-clear]');if((!pick&&!clear)||!draft||busy)return;const type=types.find(t=>t.id===+(pick?.dataset.pick||clear.dataset.clear));try{const path=clear?'':await invoke('pick_cursor_file');if(path===null)return;draft[type.key]=path;$(`#file-${type.id}`).value=path;preview(type);updateSave();}catch(e){error(e);}});
async function confirmation(title,text){const dialog=$('#confirmation');dialog.returnValue='cancel';$('#confirm-title').textContent=title;$('#confirm-text').textContent=text;return new Promise(resolve=>{dialog.addEventListener('close',()=>resolve(dialog.returnValue==='confirm'),{once:true});dialog.showModal();});}
async function runAction(action){clearError();try{await invoke('action',{action});if(action==='mouse')toast('已暂停并打开 Windows 指针设置。应用修改后，请重新捕获并恢复运行。');else if(action==='recapture')toast('已重新捕获光标并恢复检测。');else if(action==='restore')toast('已暂停并恢复原始光标。');await poll();}catch(e){error(e);}}
document.querySelectorAll('[data-action]').forEach(el=>el.addEventListener('click',()=>runAction(el.dataset.action)));
$('#reset').addEventListener('click',async()=>{if(await confirmation('重载已保存的 Windows 方案？','这会暂停 DynCursor，并加载 Windows 当前保存的指针方案。它不会自动切回系统出厂方案。之后可按需要恢复动态切换。'))await runAction('reset');});
$('#save').addEventListener('click',async()=>{clearError();try{SettingsModel.validate(draft);}catch(e){error(e);return;}busy=true;updateSave();try{saved=await invoke('save_settings',{settings:clone(draft)});draft=clone(saved);render();toast('设置已保存并应用。暂停状态会保留，可在概览恢复运行。');await poll();}catch(e){error(e);}finally{busy=false;updateSave();}});
$('#discard').addEventListener('click',()=>{draft=clone(saved);clearError();render();});
$('#defaults').addEventListener('click',async()=>{if(!await confirmation('载入默认设置？','窗口内未保存的更改将被替换。点击“保存并应用”后才会写入配置；登录自启动不受影响。'))return;try{draft=await invoke('default_settings');render();toast('默认设置已载入草稿，尚未保存。');}catch(e){error(e);}});
$('#reload').addEventListener('click',async()=>{if(dirty()&&!await confirmation('放弃未保存的更改？','将从磁盘重新读取并应用配置，当前窗口的草稿会丢弃。'))return;busy=true;updateSave();try{await invoke('action',{action:'reload'});const view=await invoke('get_settings');saved=view.settings;draft=clone(saved);render();toast('已重新读取并应用配置。');}catch(e){error(e);}finally{busy=false;updateSave();}});
$('#autostart').addEventListener('change',async e=>{const el=e.target;el.disabled=true;try{autostart=await invoke('set_autostart',{enabled:el.checked});toast(autostart?'已启用登录自启动。':'已关闭登录自启动。');}catch(e){error(e);}finally{el.checked=autostart;el.disabled=false;}});
async function poll(){if(polling)return;polling=true;try{const s=await invoke('get_status'),paused=s.paused||!s.enabled;$('#connection').textContent=s.error?'需要处理':paused?'已暂停':'正在运行';$('#sidebar-status').textContent=paused?'动态切换已暂停':'后台检测中';$('#run-title').textContent=!s.enabled?'自动切换已关闭':s.paused?'已暂停动态切换':s.custom===true?'当前使用自定义光标':'正在跟随输入状态';$('#run-description').textContent=s.error?'当前操作未完成，请查看错误提示并修正。':paused?'原始光标已恢复。启用并保存后，可随时恢复动态切换。':'窗口可以关闭到托盘，后台会继续识别输入法状态。';$('#pause').textContent=paused?'恢复动态切换':'暂停并恢复光标';$('#pause').disabled=!s.enabled;$('#locale').textContent=s.locale||'尚未检测';$('#cursor-state').textContent=s.error?'状态待确认':paused?'原始光标':s.custom===true?'自定义':s.custom===false?'原始光标':'等待检测';$('#version').textContent=`DynCursor ${s.version}`;$('#about-version').textContent=s.version;if(s.error){$('#error').textContent=s.error;$('#error').hidden=false;}else if(lastRuntimeError&&$('#error').textContent===lastRuntimeError){clearError();}lastRuntimeError=s.error;return s;}catch(e){$('#connection').textContent='连接中断';error(e);}finally{polling=false;}}
async function init(){try{const view=await invoke('get_settings');saved=view.settings;draft=clone(saved);autostart=view.autostart;$('#autostart').checked=autostart;$('#autostart').disabled=false;$('#config-path').textContent=view.config_path;render();const status=await poll();if(status?.smoke_test){await invoke('smoke_ready');return;}setInterval(poll,700);}catch(e){error(e);$('#connection').textContent='未连接';updateSave();}}
init();
