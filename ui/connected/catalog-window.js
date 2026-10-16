"use strict";
function catalogWindow(count,width,posterSize,scrollTop,viewportHeight,list=false){
 const gap=18,columns=list?1:Math.max(1,Math.floor((width+gap)/(posterSize+gap))),cardWidth=(width-(columns-1)*gap)/columns;
 const rowHeight=list?132:Math.ceil(cardWidth*1.5)+104+gap,rows=Math.ceil(count/columns);
 const first=Math.min(Math.max(0,rows-1),Math.max(0,Math.floor(Math.max(0,scrollTop)/rowHeight)-2)),last=Math.min(rows,Math.ceil((Math.max(0,scrollTop)+viewportHeight)/rowHeight)+3);
 return {columns,rowHeight,gridRowHeight:rowHeight-gap,height:rows*rowHeight,top:first*rowHeight,start:Math.min(count,first*columns),end:Math.min(count,last*columns)};
}
if(typeof module!=="undefined")module.exports={catalogWindow};
