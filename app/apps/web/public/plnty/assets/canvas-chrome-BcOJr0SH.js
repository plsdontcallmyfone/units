import{aL as i,C as s,q as d,aM as p}from"./main-DamfXklH.js";import{W as n}from"./weight-surface-iNqwfYjQ.js";const a={surface:"#fcfcfc",surfaceHover:"#f3f3f3",paper:"#ffffff",ink:"#1a1a1a",inverted:"#1a1a1a",invertedInk:"#fcfcfc"},r={height:36,padLeft:14,padRight:16,gap:8,clusterGap:8,label:16,icon:18},l={...i,fontWeight:400},e={h:"var(--cc-h, 36px)",padLeft:`var(--cc-pad-l, ${r.padLeft}px)`,padRight:`var(--cc-pad-r, ${r.padRight}px)`,gap:`var(--cc-gap, ${r.gap}px)`,label:`var(--cc-label, ${r.label}px)`,radius:"var(--cc-radius, 16px)",shrink:"var(--cc-shrink, 0)",ground:"var(--cc-ground, #fcfcfc)",frost:"var(--cc-frost, none)",weight:"var(--cc-weight, 400)"},f={boxSizing:"border-box",display:"inline-flex",alignItems:"center",justifyContent:"center",gap:e.gap,height:e.h,flexShrink:e.shrink,minWidth:0,overflow:"hidden",border:"none",...d(p),borderRadius:e.radius,fontSize:e.label,lineHeight:1,whiteSpace:"nowrap",cursor:s.hand,...l},C={...f,paddingInline:`${e.padLeft} ${e.padRight}`,background:a.surface,color:a.ink,transition:`background ${n.duration} ${n.ease}, color ${n.duration} ${n.ease}`},m={background:a.inverted,color:a.invertedInk},$={boxShadow:`0 0 0 2px ${a.ink}`},t="canvas-chrome-op-pill",h="data-chrome-mask",v=`
.${t}:hover {
  background: ${a.inverted} !important;
  color: ${a.invertedInk} !important;
}
.${t}:hover [${h}] {
  background: ${a.invertedInk} !important;
}
.${t}:hover > *,
.${t}:hover > * * {
  color: ${a.invertedInk} !important;
  fill: ${a.invertedInk} !important;
}
`;let c=!1;function k(){if(c||typeof document>"u")return;const o=document.createElement("style");o.id="canvas-chrome-op-pill-styles",o.textContent=v,document.head.appendChild(o),c=!0}export{e as C,r as a,a as b,$ as c,t as d,l as e,m as f,C as g,h,k as i};
