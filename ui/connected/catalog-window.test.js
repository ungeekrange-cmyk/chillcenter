"use strict";
const {test}=require("node:test"),assert=require("node:assert/strict"),{catalogWindow}=require("./catalog-window.js");
test("3000 media render a bounded viewport, not 3000 DOM cards",()=>{const w=catalogWindow(3000,1000,200,0,800);assert(w.end-w.start<50);assert.equal(w.columns,4);assert(w.height>100000);});
test("scrolling selects later rows without a page control",()=>{const a=catalogWindow(3000,1000,200,0,800),b=catalogWindow(3000,1000,200,15000,800);assert(b.start>a.end);assert(b.top>0);assert(b.end-b.start<50);});
test("list layout and small collections remain bounded",()=>{const w=catalogWindow(2,500,200,0,800,true);assert.equal(w.columns,1);assert.equal(w.start,0);assert.equal(w.end,2);assert.equal(w.rowHeight,132);});
test("empty catalogue has no virtual rows",()=>{const w=catalogWindow(0,1000,200,0,800);assert.equal(w.height,0);assert.equal(w.end,0);});
