const {test}=require('node:test');
const assert=require('node:assert/strict');
const {easter,windowFor}=require('./calendar.js');
const iso=d=>d.toISOString().slice(0,10);
test('Gregorian computus including earliest/latest and successive years',()=>{
  for(const [year,date] of [[1818,'1818-03-22'],[2038,'2038-04-25'],[2026,'2026-04-05'],[2027,'2027-03-28'],[2028,'2028-04-16']])assert.equal(iso(easter(year)),date);
});
test('Easter window is exactly 30 civil days before Sunday, inclusive until Sunday',()=>{
  const w=windowFor('easter',2027);assert.equal(iso(w.start),'2027-02-26');assert.equal(iso(w.returnToSeason),'2027-03-29');assert.equal((w.end-w.start)/86400000,30);
});
test('Valentine starts January31, ends Feb14 and returns Feb15, leap years included',()=>{
  for(const year of [2026,2027,2028]){const w=windowFor('valentine',year);assert.equal(iso(w.start),`${year}-01-31`);assert.equal(iso(w.end),`${year}-02-14`);assert.equal(iso(w.returnToSeason),`${year}-02-15`);}
});
test('reject invalid dates and event names',()=>{assert.throws(()=>easter(2026.5));assert.throws(()=>easter(1400));assert.throws(()=>windowFor('unknown',2026));});
