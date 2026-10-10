import{b as c,j}from"./react-C9WEJgbf.js";const s=new Map,d=new Map;function p(e){return(e.startsWith("/assets/")||e.startsWith("/icons/")||e==="/favicon.svg")&&e.endsWith(".svg")&&!e.includes("..")}function G(e){return e.replace(/<script[\s\S]*?<\/script>/gi,"").replace(/<style[\s\S]*?<\/style>/gi,"").replace(/\son\w+\s*=\s*"[^"]*"/gi,"")}function f(e){const n=d.get(e);if(n)return n;const l=fetch(e).then(t=>t.ok?t.text():"").then(t=>/^\s*(<\?xml[^>]*>\s*)?<svg[\s>]/i.test(t)?t:"").then(G).then(t=>(s.set(e,t),t)).catch(()=>(s.set(e,""),""));return d.set(e,l),l}function _(e){for(const n of e)p(n)&&!s.has(n)&&f(n)}function H({src:e,size:n,width:l,height:t,color:i,strokeWidth:S=1.5,preserveFill:y=!1,style:k,className:v,title:r}){const a=p(e),[,m]=c.useState(0),C=a?s.get(e)??"":"";c.useEffect(()=>{if(!a||s.has(e))return;let h=!1;return f(e).then(()=>{h||m(b=>b+1)}),()=>{h=!0}},[e,a]);const w={width:l??n,height:t??n,flexShrink:0,display:"block","--stroke-0":i,"--fill-0":i,color:i,"--shell-glyph-stroke":String(S),...k},x=[o,y?g:"",v??""].filter(Boolean).join(" ");return j.jsx("span",{className:x,style:w,role:r?"img":void 0,"aria-label":r,"aria-hidden":r?void 0:!0,dangerouslySetInnerHTML:{__html:C}})}const o="plnty-shell-glyph",g="plnty-shell-glyph--asis",A=`
.${o} > svg { width: 100%; height: 100%; display: block; overflow: visible; }
.${o} [stroke]:not([stroke="none"]) {
  stroke: currentColor;
  stroke-width: var(--shell-glyph-stroke, 1.5);
  vector-effect: non-scaling-stroke;
  stroke-linecap: round;
  stroke-linejoin: round;
}
.${o}:not(.${g}) [fill]:not([fill="none"]) { fill: currentColor; }
`;let u=!1;function E(){if(u||typeof document>"u")return;u=!0;const e=document.createElement("style");e.setAttribute("data-plnty","shell-glyph"),e.textContent=A,document.head.appendChild(e)}E();export{H as S,_ as p};
