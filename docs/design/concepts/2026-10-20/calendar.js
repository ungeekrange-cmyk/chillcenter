// Proposal only: Western/Gregorian Easter, evaluated for the selected civil year.
// UTC arithmetic avoids local DST turning a calendar day into 23 or 25 hours.
(function(root){
  function easter(year){
    if(!Number.isInteger(year)||year<1583||year>9999)throw new RangeError('Gregorian year required');
    const a=year%19,b=Math.floor(year/100),c=year%100,d=Math.floor(b/4),e=b%4;
    const f=Math.floor((b+8)/25),g=Math.floor((b-f+1)/3);
    const h=(19*a+b-d-g+15)%30,i=Math.floor(c/4),k=c%4;
    const l=(32+2*e+2*i-h-k)%7,m=Math.floor((a+11*h+22*l)/451);
    const month=Math.floor((h+l-7*m+114)/31),day=(h+l-7*m+114)%31+1;
    return new Date(Date.UTC(year,month-1,day));
  }
  const shift=(date,days)=>new Date(date.getTime()+days*86400000);
  function windowFor(event,year){
    const end=event==='easter'?easter(year):new Date(Date.UTC(year,1,14));
    if(!['easter','valentine'].includes(event))throw new RangeError('Unknown event');
    return {start:shift(end,event==='easter'?-30:-14),end,returnToSeason:shift(end,1)};
  }
  const api={easter,windowFor};
  if(typeof module==='object')module.exports=api;else root.eventCalendar=api;
})(globalThis);
