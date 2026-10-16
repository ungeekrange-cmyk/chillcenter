"use strict";
const test=require("node:test"),assert=require("node:assert/strict"),facets=require("./filters.js");
const library=[
 {id:"a",year:2025,genres:["Drame","Mystère"]},
 {id:"b",year:2023,genres:["Aventure"]},
 {id:"c",year:2021,genres:["Drame"]},
 {id:"d",year:0,genres:[]}
];
test("facet choices are deduplicated from the complete local base and years descend",()=>{
 assert.deepEqual(facets.normalizeGenres(library),["Aventure","Drame","Mystère"]);
 assert.deepEqual(facets.availableYears(library),[2025,2023,2021]);
});
test("genre choices combine with OR while the genre/year groups combine with AND",()=>{
 assert.equal(facets.matches(library[0],["Aventure","Mystère"],[2023,2025]),true);
 assert.equal(facets.matches(library[2],["Aventure","Mystère"],[2021,2025]),false);
 assert.equal(facets.matches(library[1],["Aventure"],[2021,2025]),true);
});
test("one year is exact, two endpoints are inclusive, and unknown years never claim a match",()=>{
 assert.equal(facets.yearMatches(2023,[2023]),true);
 assert.equal(facets.yearMatches(2024,[2023]),false);
 assert.equal(facets.yearMatches(2024,[2023,2025]),true);
 assert.equal(facets.yearMatches(0,[2023,2025]),false);
 assert.equal(facets.yearMatches(null,[2023]),false);
});
