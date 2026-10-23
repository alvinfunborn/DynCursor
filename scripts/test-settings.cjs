const {test}=require('node:test');
const assert=require('node:assert/strict');
const model=require('../ui/settings-model.js');
test('editing standard rows preserves advanced mappings',()=>{
  const text=model.changeMapping('32512:32651, 32515:32514',32513,32516,true);
  assert.equal(model.parseMappings(text).get(32515),32514);
  assert.equal(model.parseMappings(text).get(32513),32516);
  assert.equal(model.parseMappings(model.changeMapping(text,32512,32651,false)).has(32512),false);
});
test('last row and malformed maps cannot silently lose settings',()=>{
  assert.throws(()=>model.changeMapping('32512:32651',32512,32651,false));
  for(const text of ['', '32512:99999', '32512:32651,32512:32516','32512:32651,'])assert.throws(()=>model.parseMappings(text));
});
test('timing and language validation reject partial and out of range input',()=>{
  const valid={cursor_mapping:'32512:32651',custom_layouts:'0804, 0411',poll_interval_ms:100,stability_ms:0};
  assert.equal(model.validate(valid),valid);
  for(const fields of [{custom_layouts:'804'},{custom_layouts:'0000'},{poll_interval_ms:49},{stability_ms:2001},{stability_ms:1.2}])assert.throws(()=>model.validate({...valid,...fields}));
});
test('legacy and mixed cursor setups are described without changing them',()=>{
  const legacy={arrow_file:'',text_file:'',link_file:'',cursor_mapping:'32512:32514'};
  assert.equal(model.configurationMode(legacy),'windows');
  assert.equal(model.configurationMode({...legacy,arrow_file:'a.cur'}),'mixed');
  assert.equal(model.configurationMode({...legacy,arrow_file:'a.cur',text_file:'b.cur',link_file:'c.cur'}),'files');
  assert.equal(legacy.cursor_mapping,'32512:32514');
});
test('language labels handle presets, casing and multiple custom languages honestly',()=>{
  assert.equal(model.languagePresentation('0804').active,'中文输入');
  assert.equal(model.languagePresentation(' 0C04 ').preset,'0c04');
  assert.equal(model.languagePresentation('0411').active,'日文输入');
  for(const value of ['0804,0411','0409','9999']){
    assert.equal(model.languagePresentation(value).preset,'custom');
    assert.equal(model.languagePresentation(value).active,'选定输入状态');
  }
});
