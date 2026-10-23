(function (root) {
  'use strict';
  const ids = [32512,32513,32514,32515,32516,32642,32643,32644,32645,32646,32648,32649,32650,32651];
  function parseMappings(text) {
    const map = new Map();
    for (const item of text.split(',')) {
      const match = item.trim().match(/^(\d+)\s*:\s*(\d+)$/);
      if (!match || !ids.includes(+match[1]) || !ids.includes(+match[2])) throw new Error('映射必须为受支持的“目标 ID:来源 ID”，用逗号分隔。');
      if (map.has(+match[1])) throw new Error('映射中存在重复的目标 ID。');
      map.set(+match[1], +match[2]);
    }
    return map;
  }
  function changeMapping(text, target, source, enabled) {
    const map = parseMappings(text);
    if (enabled) map.set(target, source); else map.delete(target);
    if (!map.size) throw new Error('至少保留一种光标。要全部停用，请关闭“启用自动切换”。');
    return Array.from(map, ([a,b]) => `${a}:${b}`).join(', ');
  }
  function validate(settings) {
    parseMappings(settings.cursor_mapping);
    if (!settings.custom_layouts.split(',').every(code => /^[0-9a-f]{4}$/i.test(code.trim()) && parseInt(code.trim(),16) !== 0)) throw new Error('语言编码必须为四位非零十六进制数，用逗号分隔。');
    for (const [key,min,max] of [['poll_interval_ms',50,2000],['stability_ms',0,2000]]) {
      if (!Number.isInteger(settings[key]) || settings[key]<min || settings[key]>max) throw new Error('检测间隔需为 50–2000 毫秒，稳定时间需为 0–2000 毫秒的整数。');
    }
    return settings;
  }
  const api = {parseMappings,changeMapping,validate};
  if (typeof module !== 'undefined') module.exports = api; else root.SettingsModel = api;
})(globalThis);
