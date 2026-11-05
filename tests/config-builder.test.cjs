'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const net = require('node:net');
const root = path.resolve(__dirname, '..');
const html = fs.readFileSync(path.join(root, 'config-builder.html'), 'utf8');
assert.match(html, /<html lang="en-US">/);
assert.doesNotMatch(html, /\p{Script=Han}/u, 'HTML labels and messages must be in English');
const context = vm.createContext({TextEncoder});
vm.runInContext(html.match(/<script>([\s\S]*?)<\/script>/)[1], context);
const {FIELD_SPECS, SCOPES, makeRow, makeFlow, build, ipNumber, validationErrors, matchingFields, fieldGroup, importConfig, duplicateFlow, moveRow, readDraft} = context.NfgenBuilder;
const plain = value => JSON.parse(JSON.stringify(value));
const registry = [...fs.readFileSync(path.join(root, 'src/fields.rs'), 'utf8').matchAll(/"([A-Z0-9_]+)" => \((\d+), (\d+), Kind::(\w+)\)/g)].map(([,name,id,length,kind]) => [name,Number(id),Number(length),kind]);
assert.deepEqual(plain(FIELD_SPECS),registry,'HTML field catalog must match Rust');
assert.equal(FIELD_SPECS.length,53);
const model = flow => ({sourceId:'42', flows:[flow]});
const one = row => model({templateId:'256',type:'normal',scope:[],fields:[row]});
const fixtures=[];
let cases=0;
function validate(input){const result=build(input);fixtures.push({config:result.config,records:result.records,packets:result.packets});cases++;return result;}
function rejects(input,pattern){assert.throws(()=>build(input),pattern);}
{
 for(const preset of ['tcp4','tcp6','icmp4','icmp6','options','mpls','custom']) validate(model(makeFlow(preset,256)));
 for(const [name,,,kind] of FIELD_SPECS){
  const row=makeRow(name);
  if(kind==='Number'&&!['PROTOCOL','TCP_FLAGS','IP_PROTOCOL_VERSION'].includes(name))row.single.value='0';
  validate(one(row));
 }
 for(const [name] of SCOPES){const flow=makeFlow('options',256);flow.scope=[makeRow(name,'1')];validate(model(flow));}
 for(const name of ['SRC_AS','DST_AS']){
  const row=makeRow(name,' AS1 . 10 ');
  assert.equal(validate(one(row)).config.flows[0].fields[0].value,65546);
  row.mode='list';row.items=[{value:'1. 10'},{value:'AS 1 .11'}];
  assert.deepEqual(plain(validate(one(row)).config.flows[0].fields[0].value),[65546,65547]);
  Object.assign(row,{mode:'range',start:'1. 10',end:'AS1 . 12',step:'1'});
  const result=validate(one(row));assert.equal(result.records,3);
  assert.deepEqual(plain(result.config.flows[0].fields[0].value),{start:65546,end:65548,step:1});
 }
 for(const [value,expected] of [['AS0 . 65000 : 100','0:65000:100'],['AS1 . 10:100','2:65546:100'],['2:AS0 . 65000:100','2:65000:100']]){
  assert.equal(validate(one(makeRow('MPLS_VPN_RD',value))).config.flows[0].fields[0].value,expected);
 }
 for(const [name,prefix,expected] of [['IPV4_SRC_ADDR',' 192.0.2.0 / 030 ','192.0.2.0/30'],['IPV6_SRC_ADDR',' 2001:db8:: / 0126 ','2001:db8::/126']]){
  const row=makeRow(name);Object.assign(row,{mode:'prefix',prefix});
  const result=validate(one(row));assert.equal(result.records,4);assert.equal(result.config.flows[0].fields[0].value,expected);
  if(name==='IPV4_SRC_ADDR'){row.hosts=true;const hosts=validate(one(row));assert.equal(hosts.records,2);assert.deepEqual(plain(hosts.config.flows[0].fields[0].value),{prefix:expected,hosts_only:true});}
 }
 for(const [name,kind] of [['IPV4_SRC_ADDR','Ipv4'],['IPV6_SRC_ADDR','Ipv6']]){
  const row=makeRow(name);row.mode='prefix';row.prefix=kind==='Ipv4'?'192.0.2.3/30':'2001:db8::3/126';assert.equal(validate(one(row)).records,4);
  if(kind==='Ipv4'){row.hosts=true;assert.equal(validate(one(row)).records,2);row.prefix='192.0.2.0/31';assert.equal(validate(one(row)).records,2);row.prefix='192.0.2.1/32';assert.equal(validate(one(row)).records,1);}
  else {row.hosts=true;rejects(one(row),/hosts_only/);row.hosts=false;}
  row.prefix=kind==='Ipv4'?'0.0.0.0/0':'::/0';rejects(one(row),/500,000/);
 }
 for(const address of ['::','::1','2001:db8::ffff','::ffff:192.0.2.1','1:2:3:4:5:6:7:8','1:2:3:4:5:6:192.0.2.1']){
  assert.equal(net.isIP(address),6);ipNumber(address,'Ipv6');validate(one(makeRow('IPV6_SRC_ADDR',address)));
 }
 for(const address of ['1::2::3',':::1','1:2:3:4:5:6:7','1:2:3:4:5:6:7:8:9','fe80::1%eth0','192.0.2.1']) rejects(one(makeRow('IPV6_SRC_ADDR',address)),/IPv6/);
 for(const address of ['01.2.3.4','256.1.1.1','1.2.3','1.2.3.4.5','1.2.3.-1','::1']) rejects(one(makeRow('IPV4_SRC_ADDR',address)),/IPv4/);
 for(const [name,start,end,step,count] of [
  ['L4_DST_PORT','10000','10005','2',3],
  ['SRC_AS','AS1.10','1.12','1',3],
  ['IPV6_SRC_ADDR','2001:db8::1','2001:db8::3','1',3],
  ['MPLS_LABEL_1','16000','16002','1',3],
  ['ICMP_TYPE','2048','2050','1',3],
  ['IP_PROTOCOL_VERSION','4','6','2',2],
  ['TCP_FLAGS','2','18','16',2],
  ['TCP_FLAGS','SYN','SYN|ACK','16',2],
 ]){const row=makeRow(name);Object.assign(row,{mode:'range',start,end,step});assert.equal(validate(one(row)).records,count);}
 const row=makeRow('L4_SRC_PORT');row.mode='random';row.min='10000';row.max='20000';assert.deepEqual(plain(validate(one(row)).config.flows[0].fields[0].value),{random_between:[10000,20000]});
 row.min='0';row.max='0';assert.equal(validate(one(row)).records,1);
 row.min='65536';rejects(one(row),/65535/);row.min='2';row.max='1';rejects(one(row),/minimum/);
 for(const name of ['IN_PKTS','IN_BYTES']){
  const counter=makeRow(name);counter.mode='random';counter.min='65536';counter.max='4294967295';
  const result=validate(one(counter));assert.equal(result.records,1);assert.deepEqual(plain(result.config.flows[0].fields[0].value),{random_between:[65536,4294967295]});
  for(const value of ['0','4294967295']){counter.min=value;counter.max=value;validate(one(counter));}
  counter.min='0';counter.max='4294967296';rejects(one(counter),/4294967295/);
  counter.min='2';counter.max='1';rejects(one(counter),/minimum/);
  counter.min='-1';counter.max='2';rejects(one(counter),/integer/);
  counter.min='1.5';rejects(one(counter),/integer/);
  assert.throws(()=>importConfig(JSON.stringify({flows:[{template_id:256,fields:[{name,value:{random:true}}]}]})),/random.*L4 port/);
 }
 const many=makeRow('IN_BYTES');many.mode='range';many.start='0';many.end='499999';many.step='1';assert.equal(validate(one(many)).records,500000);many.end='500000';rejects(one(many),/500,000/);
 many.end='499999';const flow=one(many).flows[0];const two=makeRow('IN_PKTS');two.mode='list';two.items=[{value:'1'},{value:'2'}];flow.fields.push(two);rejects(model(flow),/500,000/);
 const half=makeRow('IN_BYTES');Object.assign(half,{mode:'range',start:'0',end:'249999',step:'1'});
 const total=one(half);duplicateFlow(total,0);assert.equal(build(total).records,500000);
 total.flows[1].fields[0].end='250000';rejects(total,/all flows.*500,000/);
 const prefix=makeRow('IPV4_SRC_ADDR');Object.assign(prefix,{mode:'prefix',prefix:'10.0.0.0/14'});assert.equal(build(one(prefix)).records,262144);
 prefix.prefix='10.0.0.0/13';rejects(one(prefix),/500,000/);
 const a=makeFlow('tcp4',256), b=makeFlow('tcp6',256);rejects({sourceId:'42',flows:[a,b]},/shared template/);
 b.templateId='257';validate({sourceId:'4294967295',flows:[a,b]});
 rejects({sourceId:'4294967296',flows:[a]},/4294967295/);rejects({sourceId:'42',flows:[]},/Add/);
 a.fields.push(makeRow(a.fields[0].name));rejects(model(a),/Duplicate fields are not allowed/);
 const flags=makeRow('TCP_FLAGS');for(const value of ['SA','SYN|ACK','0x12','18','ECE','EC']){flags.single.value=value;validate(one(flags));}flags.single.value='SYNACK';rejects(one(flags),/TCP flags/);
 for(const value of ['65000:100','192.0.2.1:100','AS1.10:100','2:AS0.65000:100','65000: 100','192.0.2.1 :100',' 1: 192.0.2.1 : 100 '])validate(one(makeRow('MPLS_VPN_RD',value)));
 const values=makeRow('ICMP_TYPE');values.mode='list';values.items=[{value:'echo-request',code:'0'},{value:'3',code:'1'}];assert.equal(validate(one(values)).records,2);
 console.log(`Passed: ${cases} generation cases; bounds, invalid input, field catalog and record limits checked.`);
}
{
 for(const fixture of fixtures){
  const result=build(importConfig(JSON.stringify(fixture.config)));
  assert.deepEqual(plain(result.config),plain(fixture.config),'import/export must preserve the generated configuration');
  assert.equal(result.records,fixture.records);assert.equal(result.packets,fixture.packets);
 }
 const config=value=>JSON.stringify({flows:[{template_id:256,fields:[{name:'ICMP_TYPE',value}]}]});
 for(const [input,expected] of [[2049,{type:8,code:1}],['echo-request',{type:'echo-request',code:0}],[{type:3,code:7},{type:3,code:7}]]){
  assert.deepEqual(plain(build(importConfig(config(input))).config.flows[0].fields[0].value),expected);
 }
 const random={flows:[{template_id:256,fields:[{name:'l4_src_port',value:{random:true}}]}]};
 assert.deepEqual(plain(build(importConfig(JSON.stringify(random))).config.flows[0].fields[0].value),{random_between:[1,65535]});
 assert.throws(()=>importConfig(fs.readFileSync(path.join(root,'examples/random-ips.json'),'utf8')),/random_file is only supported in the CLI; nothing was imported/);
 assert.throws(()=>importConfig(JSON.stringify({flows:[{template_id:256,fields:[{name:'PROTOCOL',value:1.5}]}]})),/Enter a nonnegative integer/);
 const invalidImports=[
  ['{',/JSON|property|Expected/],['null',/object/],['{"flows":[]}',/Add/],
  ['{"flows":[],"extra":true}',/unsupported extra/],
  ['{"flows":[{"template_id":256,"mpls_stack":[16]}]}',/unsupported mpls_stack/],
  [config({type:8,code:0,extra:1}),/unsupported extra/],
  [config({start:1,end:2,step:9007199254740993}),/integer|exactly/],
  [config({type:8,code:'0'}),/code must/],
  [config({prefix:'1.1.1.0/32',hosts_only:1}),/hosts_only/],
  [config({random_between:[1,2],extra:3}),/unsupported extra/],
  [config({start:1,end:2,extra:3}),/unsupported extra/],
  [config({start:'echo-request',end:'time-exceeded'}),/ICMP range.*combined integers/],
  [JSON.stringify({source_id:'42',flows:[]}),/source_id/],
 ];
 for(const [text,pattern] of invalidImports)assert.throws(()=>importConfig(text),pattern);
 const invalid=model(makeFlow('custom',256));invalid.sourceId='bad';invalid.flows[0].templateId='1';invalid.flows[0].fields[0].single.value='256';
 assert.deepEqual(plain(validationErrors(invalid)).map(e=>e.path),['sourceId','flows.0.templateId','flows.0.fields.0']);
 invalid.sourceId='42';invalid.flows[0].templateId='256';invalid.flows[0].fields[0].single.value='6';assert.deepEqual(plain(validationErrors(invalid)),[]);
 duplicateFlow(invalid,0);assert.equal(invalid.flows[1].templateId,'257');
 invalid.flows[1].fields[0].single.value='17';assert.equal(invalid.flows[0].fields[0].single.value,'6','duplicated flows must not share values');
 const fields=makeFlow('tcp4',256).fields,names=fields.map(row=>row.name);moveRow(fields,1,-1);assert.equal(fields[0].name,names[1]);moveRow(fields,0,-1);assert.equal(fields[0].name,names[1]);moveRow(fields,0,1);assert.deepEqual(plain(fields.map(row=>row.name)),plain(names));
 invalid.flows[1].templateId='256';assert.deepEqual(plain(validationErrors(invalid)),[]);invalid.flows[1].fields.push(makeRow('IN_BYTES'));assert(validationErrors(invalid).some(e=>e.path==='flows.1.templateId'));
 const huge=makeRow('IN_BYTES');Object.assign(huge,{mode:'range',start:'0',end:'499999',step:'1'});const expanded=one(huge);expanded.flows[0].fields.push(makeRow('IN_PKTS'));expanded.flows[0].fields[1].mode='list';expanded.flows[0].fields[1].items=[{value:'1'},{value:'2'}];assert.equal(validationErrors(expanded)[0].path,'flows.0.fields.1');
 assert.deepEqual(plain(matchingFields(FIELD_SPECS,'interface').map(s=>s[0])),['INPUT_SNMP','OUTPUT_SNMP']);
 assert.deepEqual(plain(matchingFields(FIELD_SPECS,'interface','Interface').map(s=>s[0])),['INPUT_SNMP','OUTPUT_SNMP']);
 assert.equal(matchingFields(FIELD_SPECS,'IPV4','MPLS').length,0);
 assert.equal(fieldGroup(SCOPES[1],true),'Options scope');
 const draft=JSON.stringify({version:1,model:invalid});assert.deepEqual(plain(readDraft(draft)),plain(invalid),'invalid edits must survive draft restoration');
 assert.throws(()=>readDraft('{"version":2,"model":{}}'),/version/);
 const broken=JSON.parse(draft);broken.model.flows[0].fields[0].name='UNKNOWN';assert.throws(()=>readDraft(JSON.stringify(broken)),/unknown field/);
 const brokenValue=JSON.parse(draft);brokenValue.model.flows[0].fields[0].single.value={};assert.throws(()=>readDraft(JSON.stringify(brokenValue)),/Invalid/);
 assert.throws(()=>importConfig(' '.repeat(8*1024*1024+1)),/8 MiB/);
 console.log(`Passed: ${fixtures.length} import/export round trips; import rejection, error paths, search, duplication, ordering and draft restoration checked.`);
}
// Shared IDs keep each flow's values independent and count only unique templates.
{
 const input=model(makeFlow('tcp4',256));duplicateFlow(input,0,true);
 assert.equal(input.flows[1].templateId,'256');
 input.flows[1].fields.find(row=>row.name==='L4_DST_PORT').single.value='80';
 const result=build(input);assert.equal(result.templates,1);assert.equal(result.records,2);assert.equal(result.packets,2);
 assert.equal(result.config.flows[0].fields.find(row=>row.name==='L4_DST_PORT').value,443);
 assert.equal(result.config.flows[1].fields.find(row=>row.name==='L4_DST_PORT').value,80);
 assert.deepEqual(plain(build(importConfig(result.json)).config),plain(result.config));
 input.flows[1].fields.reverse();assert.throws(()=>build(input),/shared template/);
 assert(validationErrors(input).some(issue=>issue.path==='flows.1.templateId'));
 const opts=model(makeFlow('options',300));duplicateFlow(opts,0,true);opts.flows[1].scope[0].single.value='2';assert.equal(build(opts).templates,1);
 opts.flows[1].scope[0]=makeRow('INTERFACE','2');assert.throws(()=>build(opts),/shared template/);
 console.log('Passed: shared Template validation, independent values, Options, unique counts and JSON round trips.');
}
// Calculated counters keep IN and OUT independent and export ordinary CLI values.
{
 const flow=makeFlow('custom',256),input=model(flow);
 for(const direction of ['IN','OUT']){
  const bytes=makeRow(direction+'_BYTES'),packets=makeRow(direction+'_PKTS','10');
  Object.assign(bytes,{mode:'calculated',avgPacketSize:direction==='IN'?'100':'200'});
  flow.fields.push(bytes,packets);
 }
 const value=name=>build(input).config.flows[0].fields.find(row=>row.name===name).value;
 assert.equal(value('IN_BYTES'),1000);assert.equal(value('OUT_BYTES'),2000);
 const packets=flow.fields.find(row=>row.name==='IN_PKTS'),bytes=flow.fields.find(row=>row.name==='IN_BYTES');
 packets.single.value='20';assert.equal(value('IN_BYTES'),2000);assert.equal(value('OUT_BYTES'),2000);
 bytes.avgPacketSize='150';assert.equal(value('IN_BYTES'),3000);
 packets.single.value='0';assert.equal(value('IN_BYTES'),0);
 packets.single.value='1';bytes.avgPacketSize='4294967295';assert.equal(value('IN_BYTES'),4294967295);
 packets.single.value='2';assert.throws(()=>build(input),/BYTES exceeds/);
 for(const average of ['0','-1','1.5','bad']){bytes.avgPacketSize=average;assert.throws(()=>build(input));}
 bytes.avgPacketSize='100';
 for(const mode of ['list','range','random']){packets.mode=mode;assert.throws(()=>build(input),/must use a single value/);}
 packets.mode='single';packets.single.value='10';
 const json=build(input).json;assert.deepEqual(plain(build(importConfig(json)).config),plain(build(input).config));
 assert(!json.includes('avgPacketSize'),'calculation settings stay in the form');
 const draft=JSON.stringify({version:1,model:input});assert.equal(build(readDraft(draft)).json,json);
 const restored=readDraft(draft);assert.equal(restored.flows[0].fields.find(row=>row.name==='OUT_BYTES').mode,'calculated');
 const oldDraft=JSON.parse(draft);for(const row of oldDraft.model.flows[0].fields){if(row.mode==='calculated')row.mode='single';delete row.avgPacketSize;}
 assert.doesNotThrow(()=>readDraft(JSON.stringify(oldDraft)));
 flow.fields.splice(flow.fields.indexOf(packets),1);assert.throws(()=>build(input),/Add the IN_PKTS/);
 assert(validationErrors(input).some(issue=>issue.path==='flows.0.fields.1'));
 console.log('Passed: IN/OUT calculated counters, updates, zero, limits, invalid inputs, PKTS modes, missing PKTS, JSON compatibility and draft restoration.');
}
if(process.argv[2])fs.writeFileSync(process.argv[2],JSON.stringify(fixtures));

require('./config-builder-ui.test.cjs');
