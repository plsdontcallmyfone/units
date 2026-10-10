import{J as i}from"./main-DamfXklH.js";const t={hoverScale:1.02,pressScale:.96,cardPressScale:.98,smallHoverScale:1.05,smallPressScale:.9,rowPressScale:.985,step:16,ms:180},p="--plnty-press-hover",u=`
[data-press] {
  transition: transform ${t.ms}ms ${i}, background-color ${t.ms}ms ${i} !important;
}
[data-press][data-press]:hover:not(:disabled):not([aria-disabled="true"]) {
  transform: scale(${t.hoverScale}) !important;
}
[data-press][data-press]:active:not(:disabled):not([aria-disabled="true"]) {
  transform: scale(${t.pressScale}) !important;
}
[data-press][data-press][data-press-small]:hover:not(:disabled):not([aria-disabled="true"]) {
  transform: scale(${t.smallHoverScale}) !important;
}
[data-press][data-press][data-press-small]:active:not(:disabled):not([aria-disabled="true"]) {
  transform: scale(${t.smallPressScale}) !important;
}
[data-press="plate"][data-press]:is(:hover, :active):not(:disabled):not([aria-disabled="true"]) {
  background-color: var(${p}) !important;
  background-image: none !important;
}
[data-press-row]:active:not(:disabled):not([aria-disabled="true"]) {
  transform: scale(${t.rowPressScale}) !important;
}
[data-press-only]:active:not(:disabled):not([aria-disabled="true"]) {
  transform: scale(${t.pressScale}) !important;
}
[data-press-only="small"]:active:not(:disabled):not([aria-disabled="true"]) {
  transform: scale(${t.smallPressScale}) !important;
}
/* A flat plate (the rail's search field): the step and the press, no grow
   under the pointer. Wide and set into its ground, it read the 1.02 as a lift
   with a soft edge (Dor, 2026-10-04: "no shadow on hover please"). */
[data-press][data-press][data-press-flat]:hover:not(:disabled):not([aria-disabled="true"]) {
  transform: none !important;
}
[data-press][data-press][data-press-flat]:active:not(:disabled):not([aria-disabled="true"]) {
  transform: scale(${t.pressScale}) !important;
}
/* A pressable inside a pressable (the offer pill's chip): the inner one takes
   the gesture, the outer keeps its plate but does not grow or shrink with it. */
[data-press][data-press][data-press]:is(:hover, :active):has([data-press]:is(:hover, :active)) {
  transform: none !important;
}
@media (prefers-reduced-motion: reduce) {
  [data-press][data-press][data-press]:is(:hover, :active):not(:disabled):not([aria-disabled="true"]),
  [data-press-row][data-press-row]:active:not(:disabled):not([aria-disabled="true"]),
  [data-press-only][data-press-only]:active:not(:disabled):not([aria-disabled="true"]) {
    transform: none !important;
  }
}
`;let d=!1;function l(){if(d||typeof document>"u")return;d=!0;const a=document.createElement("style");a.setAttribute("data-plnty-press",""),a.textContent=u,document.head.appendChild(a)}function f(a){const r=a.trim().toLowerCase(),s=r.match(/^#([0-9a-f]{3}|[0-9a-f]{6})$/);if(s){const e=s[1].length===3?s[1].split("").map(o=>o+o).join(""):s[1];return[0,2,4].map(o=>parseInt(e.slice(o,o+2),16))}const n=r.match(/^rgba?\(([^)]+)\)$/);if(n){const e=n[1].split(/[\s,/]+/).filter(Boolean).map(Number);return e.length<3||e.slice(0,3).some(o=>!Number.isFinite(o))||e.length>3&&e[3]<1?null:[e[0],e[1],e[2]]}return null}function h(a){const r=a?f(a):null;if(!r)return null;const[s,n,e]=r,c=(.2126*s+.7152*n+.0722*e)/255>=.5?-16:t.step;return"#"+[s,n,e].map(m=>Math.max(0,Math.min(255,m+c)).toString(16).padStart(2,"0")).join("")}function v(a,r){l();const s=r?.hover??h(a),n={...r?.small?{"data-press-small":""}:null,...r?.flat?{"data-press-flat":""}:null};return s?{attrs:{"data-press":"plate",...n},style:{[p]:s}}:{attrs:{"data-press":"scale",...n},style:{}}}function S(){return l(),{"data-press-row":""}}function g(a){return l(),{"data-press-only":a?.small?"small":""}}export{t as P,v as a,S as b,g as p};
