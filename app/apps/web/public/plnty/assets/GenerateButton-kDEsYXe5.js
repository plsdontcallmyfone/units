import{b as c,j as e}from"./react-C9WEJgbf.js";import{bC as Y,D as I,F as w,C as M,by as a,bA as C,q as G}from"./main-DamfXklH.js";import{C as E}from"./credit-badge-config-BhXKyJ5c.js";import{d as q}from"./uplift-glyphs-B2_uEbSJ.js";import{M as V}from"./canvas-motion-UVAsDkLO.js";const K=`
.canvas-tool-run-btn-wrap {
  display: flex;
  box-sizing: border-box;
  padding: 1px;
  border-radius: 999px;
  background: linear-gradient(
    100deg,
    rgba(255, 255, 255, 0.14) 0%,
    rgba(255, 255, 255, 0.14) 35%,
    rgba(255, 255, 255, 0.55) 50%,
    rgba(255, 255, 255, 0.14) 65%,
    rgba(255, 255, 255, 0.14) 100%
  );
  background-size: 300% 100%;
  animation: canvas-tool-run-btn-shimmer 4.5s linear infinite;
}
@keyframes canvas-tool-run-btn-shimmer {
  0%   { background-position: 200% 0; }
  100% { background-position: -100% 0; }
}
`;let R=!1;function X(){if(R||typeof document>"u")return;const o=document.createElement("style");o.id="canvas-tool-run-btn-styles",o.textContent=K,document.head.appendChild(o),R=!0}function nt(o=I.generateButton){return{bg:o.bg,text:o.text}}function ot({onClick:o,label:F="Run",credits:u,isGenerating:s=!1,generatingLabel:z="Running...",disabled:x=!1,progressPercent:f,icon:p,trailingIcon:h,rightContent:g,size:B="default",fullWidth:T=!0,title:$,variant:H,arrow:_=!0,anchor:L="run"}){const b=Y(d=>d.overrides),t=c.useMemo(()=>{const d=I.generateButton,S={...d};for(const j of Object.keys(d)){const k=b[`generateButton.${j}`];k!==void 0&&(S[j]=k)}return S},[b]),r=B==="compact"?.85:1,N=Math.round(t.height*r),i=Math.round(t.fontSize*r),O=Math.round(t.paddingH*r),P=Math.round(t.paddingV*r),m=Math.round(t.iconSize*r),y=Math.round(t.creditFontSize*r),[U,v]=c.useState(!1),l=s||x,n=H==="uplift";c.useEffect(()=>{X()},[]);const W=c.useCallback(()=>{l||o()},[l,o]),A=x&&!s?n?1:t.disabledOpacity:s?.6:1,D=l?t.bg:U?t.bgHover:t.bg;return e.jsx("div",{className:n?void 0:"canvas-tool-run-btn-wrap","data-rothko-anchor":L??void 0,style:{width:T?"100%":"auto",opacity:A,transition:"opacity 0.15s ease"},children:e.jsxs("button",{style:{position:"relative",width:"100%",minHeight:n?a.runH:N,border:"none",...n?G(a.runRadius):{borderRadius:999},display:"flex",alignItems:"center",justifyContent:"space-between",gap:n?a.runGap:6,padding:n?`${a.runPadY}px ${a.runPadX}px`:`${P}px ${O}px`,background:D,cursor:l?M["not-allowed"]:M.hand,overflow:"hidden",fontFamily:w,transition:"background 0.2s ease"},onClick:W,onMouseEnter:()=>v(!0),onMouseLeave:()=>v(!1),disabled:l,title:$,children:[f!==void 0&&e.jsx("div",{style:{position:"absolute",left:0,top:0,bottom:0,width:`${f*100}%`,background:"rgba(255, 255, 255, 0.12)",borderRadius:t.borderRadius,transition:"width 1s linear",pointerEvents:"none"}}),e.jsxs("div",{style:{display:"flex",alignItems:n?"center":"baseline",gap:n?a.runLabelGap:6,position:"relative",zIndex:1},children:[p&&e.jsx("span",{style:{display:"flex",flexShrink:0,pointerEvents:"none",alignSelf:"center"},children:p}),e.jsx("span",{style:{fontSize:i,fontWeight:t.fontWeight,color:t.text,letterSpacing:`${i*-.01}px`,lineHeight:`${Math.round(i*1.33)}px`,...n?C.run:null,whiteSpace:"nowrap"},children:e.jsx(V,{text:s?z:F})}),!s&&!p&&_&&(h?e.jsx("span",{style:{display:"inline-flex",alignItems:"center",color:t.text,transform:"translateY(1px)"},children:h}):n?e.jsx(q,{ink:t.text}):e.jsx("span",{style:{fontSize:i,fontWeight:t.fontWeight,color:t.text,lineHeight:`${Math.round(i*1.33)}px`,transform:"translateY(1px)",display:"inline-block"},children:"→"}))]}),g?e.jsx("div",{style:{display:"flex",alignItems:"center",gap:4,position:"relative",zIndex:1},children:g}):u!==void 0&&!s?e.jsxs("div",{style:{display:"flex",alignItems:"center",gap:n?a.runLabelGap:4,position:"relative",zIndex:1},children:[u!=="FREE"&&e.jsx("svg",{width:n?a.creditMark:m,height:n?a.creditMark:m,viewBox:E.starViewBox,fill:t.text,children:e.jsx("path",{d:E.starPath})}),e.jsx("span",{style:{fontSize:y,fontWeight:t.creditFontWeight,color:t.text,fontFamily:w,letterSpacing:`${y*-.01}px`,lineHeight:"1",...n?C.run:null,whiteSpace:"nowrap"},children:u})]}):null]})})}export{ot as G,nt as g};
