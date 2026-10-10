import{R as at,j as n,b as k}from"./react-C9WEJgbf.js";import{m as D}from"./react-3JIT7HhC.js";import{P as ot,be as d,J as it,lW as rt,lX as st,aX as dt,aY as lt,aZ as ct,aV as ht,aW as pt,C as g,M as ut,F as L,a$ as $,b0 as w,aL as m,q as gt}from"./main-DamfXklH.js";import{P as N,a as E}from"./press-plate-DUu7F7Bk.js";import{u as ft,D as bt}from"./popup-backdrop-CUPirz4x.js";import{A as mt}from"./index-Btjz0C-N.js";const z="#fed404",u="[data-plnty-cursor]",U="[data-hop-lift]",c=`${u} ${U}`,xt='button, a[href], [role="button"], [role="tab"]',_=`${u} :is(${xt})`,Y="rgba(248, 245, 238, 0.05)",I="rgba(10, 10, 10, 0.07)",y=t=>`linear-gradient(${t}, ${t})`,W="--plnty-hop-lift",G="--plnty-hop-ease",j=`translateY(calc(-1 * var(${W}, calc(${d.lift} * var(--tc-u, 1px)))))`,yt=`
/*
 * NO TRANSITION ON THE TINT, DELIBERATELY. background-image is a DISCRETE
 * animation: there is no interpolation between two gradients, so a transition
 * does not fade it in, it waits half the duration and snaps. A tint arriving
 * 90ms after the pointer reads as lag rather than as a fade. Instant is both
 * simpler and what it would have looked like anyway.
 *
 * The lift below is a different matter: transform interpolates properly, and
 * that is what carries the motion on this surface.
 *
 * (No backticks in this string. A backtick ends the template literal that
 * builds this CSS, which has now cost two builds.)
 *
 * THE !important IS NOT OPTIONAL EITHER, AND FOR A REASON THAT IS EASY TO MISS.
 * Nothing else on this surface sets background-image at all, so it looks like a
 * rule with no competition. What beats it is the SHORTHAND: every one of these
 * components sets background inline (background: transparent, background:
 * DF.chip), and the background shorthand resets background-image to none as
 * part of itself. That inline none outranks any stylesheet declaration, so the
 * tint silently did nothing on all 45 controls until this was added. Measured
 * before and after — the same trap the cursor hide hit two commits ago, arriving
 * from a different direction.
 */
${_}:hover,
${_}:focus-visible {
  background-image: ${y(Y)} !important;
}
${u} [data-hop-tint="dark"][data-hop-tint]:hover,
${u} [data-hop-tint="dark"][data-hop-tint]:focus-visible {
  background-image: ${y(I)} !important;
}
/*
 * OPT OUT ENTIRELY, for a control that sets its own hover ground and means it.
 *
 * The veil is a RELATIVE gesture — a tiny bit lighter than whatever you are on
 * — which is right for the 45 controls that have no hover colour of their own.
 * The pop-out menus do: the operator specified #1F1E1D for every row in both
 * dropdowns, and 5 per cent of cream over that composites to about #2A2927. The
 * row was setting the right value and being painted a different one.
 *
 * Doubling the attribute for specificity, exactly as the dark rule above does
 * and for the same arithmetic — (0,4,0) against the light rule's (0,3,1).
 */
${u} [data-hop-tint="none"][data-hop-tint]:hover,
${u} [data-hop-tint="none"][data-hop-tint]:focus-visible {
  background-image: none !important;
}
/*
 * AN ABSOLUTE HOVER, for the two filter strips.
 *
 * The veil above is a relative gesture, which is right for a control that could
 * sit anywhere. It is wrong for two strips that are meant to look like each
 * other while sitting on different grounds — see DF.tabHover for the
 * measurement. background-image is cleared as well as the colour being set,
 * because otherwise the veil would still be painted over the new ground.
 *
 * (No backticks. The header says so twice and this comment still cost a build.)
 *
 * Only INACTIVE chips carry the attribute. An active one already paints its own
 * ground, so the veil composites identically in both strips and there is
 * nothing to correct; forcing this colour on it would make it go DARKER on
 * hover than at rest.
 */
${u} [data-hop-tab][data-hop-tab]:hover,
${u} [data-hop-tab][data-hop-tab]:focus-visible {
  background-image: none !important;
  background-color: ${ot.tabHover} !important;
}
${c} {
  transition:
    background ${d.downDuration} ${d.easeOut},
    color ${d.downDuration} ${d.easeOut},
    transform ${d.downDuration} ${d.easeOut};
}
${c}:hover,
${c}:focus-visible,
${c}:focus-within {
  transform: ${j};
  transition:
    background ${d.duration} ${d.easeOut},
    color ${d.duration} ${d.easeOut},
    transform ${d.duration} var(${G}, ${d.easeBack}) !important;
}
/*
 * THE CARD'S PRESS (Dor, 2026-10-03, approved on the live dashboard): it keeps
 * its lift and shrinks a little while held, the filled buttons' gesture
 * (lib/press-plate) at a card's size. Letting go rides the hover rule's own
 * curve back up, so the release lands with the lift's small bounce.
 *
 * DATA-GRABBED LETS GO AT ONCE. Held long enough, the card is picked up, and
 * useCardGrab marks it grabbed BEFORE it measures it: with no transition the
 * card is at full size in that same instant, so the copy that flies is the
 * card's real size and lands back on it without a jump.
 */
${c}:active:not([data-grabbed]) {
  transform: ${j} scale(${N.cardPressScale}) !important;
  transition: transform ${N.ms}ms ${it} !important;
}
${c}[data-grabbed] {
  transition: none !important;
}
/*
 * Anything inside a lifting element never adds a lift of its own — the card is
 * already carrying one, and both moving would travel twice as far. Cheap
 * insurance today, since a board card holds only its own thumbnail button, and
 * the guard that stops the next nested control from doubling up. It is
 * !important because it has to beat the hover rule's transform above.
 */
${c} [data-hop-lift] {
  transform: none !important;
}
@media (prefers-reduced-motion: reduce) {
  ${c},
  ${c}:hover,
  ${c}:focus-visible,
  ${c}:focus-within,
  ${c}:active:not([data-grabbed]) {
    transform: none !important;
    transition: background ${d.downDuration} linear, color ${d.downDuration} linear;
  }
}
`;function Pt(){return at.useEffect(()=>{const t=document.querySelector(u);if(!t)return;let e=0;const o=()=>{e=0,t.querySelectorAll(U).forEach(b=>{const T=b.getBoundingClientRect().height;T<=0||(b.style.setProperty(W,`${rt(T).toFixed(2)}px`),b.style.setProperty(G,st(T)))})},s=()=>{e||(e=requestAnimationFrame(o))};o();const i=new ResizeObserver(s);i.observe(t);const l=new MutationObserver(s);return l.observe(t,{childList:!0,subtree:!0}),()=>{e&&cancelAnimationFrame(e),i.disconnect(),l.disconnect()}},[]),n.jsx("style",{children:yt})}const h=gt,M=482,V=12,K=12,A=24,O="458 / 316",R=8,q=161,X=119,S=16,f="#F8F8F8",p="#1A1A19",C="#1a1a1a",v="#62625F",P="#E6E3DC",J="#EF0202",Lt="#EBE9E6",Z="#181503",Q="#E8E7E4",vt=Q,Et="#ADADA8",tt="rgba(26, 26, 25, 0.2)",et="rgba(10, 10, 10, 0.5)",H="0px 24px 48px 0px rgba(0, 0, 0, 0.18)",Nt={scrim:et,cardW:M,cardR:V,cardPad:K,cardGap:A,cardShadow:H,heroAspect:O,heroR:R,contentH:q,textH:X,contentPad:S,ground:f,ink:p,btnInk:C,bodyInk:v,warm:P,chipYellow:z,chipInk:Z,dotIdle:tt,btnH:32,btnR:8},_t={chipR:8,chipInk:"#2B2205",bodyInk:"#6C6962",btnInk:p,titleLeading:"28px",displayTracking:"-0.04em",proseTracking:"-0.01em"},wt={backgroundColor:"#F2F2F2",backgroundImage:"repeating-linear-gradient(30deg, #E5E5E5 0px, #E5E5E5 1px, transparent 1px, transparent 11.34px), repeating-linear-gradient(150deg, #E5E5E5 0px, #E5E5E5 1px, transparent 1px, transparent 11.34px)"},r="plnty-anno",kt=`
.${r} [data-anno-hop] {
  transition: transform ${dt}ms ${lt};
}
.${r} [data-anno-hop]:hover,
.${r} [data-anno-hop]:focus-visible {
  transform: translateY(-${ct}px);
  transition: transform ${ht}ms ${pt};
}
/*
 * ⚠ EVERY !important HERE IS LOAD-BEARING, AND NOT FOR SPECIFICITY. These
 * buttons set their ground as an INLINE React style, and background is a
 * SHORTHAND: it resets background-image to none as part of itself, inline,
 * where no stylesheet can reach it. HoverStates shipped without these and
 * measured the tint doing nothing on all 45 of its controls. Same trap, same
 * answer, and it is scoped to the one property each rule owns.
 */
.${r} [data-anno-tint="light"]:hover,
.${r} [data-anno-tint="light"]:focus-visible {
  background-image: ${y(Y)} !important;
}
.${r} [data-anno-tint="dark"]:hover,
.${r} [data-anno-tint="dark"]:focus-visible {
  background-image: ${y(I)} !important;
}
/*
 * A DESTRUCTIVE secondary answers the pointer in red rather than darkening:
 * the leftover card's "No thanks, Lose credits" (Figma 161:3809, third
 * card). The plate is the red at 14% on the card's ground, the ink is the red.
 */
.${r} [data-anno-danger]:hover,
.${r} [data-anno-danger]:focus-visible {
  background: rgba(239, 2, 2, 0.14) !important;
  background-image: none !important;
  color: ${J} !important;
}
/*
 * ⚠ THE IDLE DOTS ONLY. A filled dot is already solid ink, so "a tiny bit
 * darker" has nowhere to go: pushing it would FADE the one dot that means
 * "you are here". The rest sit at 0.2 and darken to answer the pointer.
 */
.${r} [data-anno-dot="idle"]:hover span,
.${r} [data-anno-dot="idle"]:focus-visible span {
  background: rgba(26, 26, 25, 0.45) !important;
}
/*
 * THE DISABLED PLATE IS PAINTED HERE, NOT IN THE STYLE OBJECT, because it has
 * to beat the inline ground the button already carries. Same shorthand trap the
 * tints hit: background is written inline, so only an !important rule reaches
 * it, and background-image: none is what stops a hover veil surviving into the
 * disabled state.
 */
.${r} button:disabled {
  transform: none !important;
  background: ${Q} !important;
  background-image: none !important;
  color: ${Et} !important;
  /* !important because the live button sets its hand cursor INLINE, and an
     inline value beats a stylesheet without it. The house disabled cursor,
     the same one ActionButton reaches for. */
  cursor: ${g["not-allowed"]} !important;
}
/*
 * THE GHOST HAS NO HOVER BUT IT STILL NEEDS A KEYBOARD FOCUS, and those are not
 * the same thing. Dropping data-anno-tint took the pointer state away, as asked,
 * and it took the focus ring with it, leaving the browser's own blue outline as
 * the only signal that a keyboard user had reached the control. This puts the
 * card's own veil back on :focus-visible alone, which a mouse never triggers.
 */
.${r} [data-anno-ghost]:focus-visible {
  background-image: ${y(I)} !important;
}
/*
 * THE FIELDS. Their resting outline and their focus state are both inset
 * shadows, for the reason on CardInput: a border swap moves the text. The
 * placeholder is the body ink at half strength, which is the only colour on
 * this card that is not already named — it is the same ink, weaker.
 */
.${r} [data-card-field] {
  box-shadow: inset 0 0 0 1px rgba(26, 26, 25, 0.12);
  transition: box-shadow 0.15s ease;
}
.${r} [data-card-field]:hover {
  box-shadow: inset 0 0 0 1px rgba(26, 26, 25, 0.22);
}
.${r} [data-card-field]:focus {
  outline: none;
  box-shadow: inset 0 0 0 1.5px ${p};
}
.${r} [data-card-field]::placeholder {
  color: ${v};
  opacity: 0.55;
}
@media (prefers-reduced-motion: reduce) {
  .${r} [data-anno-hop],
  .${r} [data-anno-hop]:hover,
  .${r} [data-anno-hop]:focus-visible {
    transform: none;
    transition: none;
  }
}
`;function jt({children:t,scrim:e,onBackdrop:o,zIndex:s=10001}){const i=ft(),l=e===void 0&&!i;return n.jsxs(D.div,{style:{...a.overlay,background:l?"transparent":e??et,zIndex:s},onClick:o,initial:l?!1:{opacity:0},animate:{opacity:1},transition:{duration:.2,ease:"linear"},"data-plnty-cursor":!0,className:r,children:[n.jsx("style",{children:kt}),l&&n.jsx(bt,{}),t]})}function Ft({children:t,onClick:e,width:o}){return n.jsx(D.div,{style:o?{...a.stack,width:`min(${o}px, calc(100vw - 32px))`}:a.stack,...ut,onClick:e??(s=>s.stopPropagation()),children:t})}const F=E(f,{small:!0});function Bt({onClick:t,label:e="Close"}){return n.jsx("button",{type:"button",style:{...a.close,...F.style},onClick:t,"aria-label":e,...F.attrs,"data-anno-ghost":!0,children:n.jsx(St,{})})}function St({style:t}){return n.jsx("svg",{width:"32",height:"32",viewBox:"0 0 32 32",fill:"none","aria-hidden":!0,style:t,children:n.jsx("path",{d:"M19.3775 10.4966C19.9633 9.91085 20.9128 9.91088 21.4986 10.4966C22.0841 11.0824 22.0843 12.032 21.4986 12.6177L18.828 15.2876C18.4374 15.6781 18.4373 16.3113 18.8279 16.7019L21.5074 19.3814C22.0928 19.9671 22.0929 20.9168 21.5074 21.5025C20.9218 22.0882 19.9722 22.0878 19.3863 21.5025L16.7069 18.8231C16.3163 18.4325 15.683 18.4326 15.2925 18.8233L12.6138 21.5035C12.0282 22.0891 11.0786 22.0889 10.4928 21.5035C9.90696 20.9177 9.90704 19.9682 10.4928 19.3824L13.1716 16.7027C13.562 16.3122 13.562 15.6791 13.1715 15.2886L10.5006 12.6177C9.9148 12.0319 9.91483 11.0824 10.5006 10.4966C11.0863 9.91094 12.0359 9.9109 12.6217 10.4966L15.2926 13.1668C15.6831 13.5572 16.3161 13.5572 16.7066 13.1668L19.3775 10.4966Z",fill:"currentColor"})})}function zt({children:t,style:e,titleId:o,surface:s}){return n.jsx("div",{style:{...a.dialog,...e},role:"dialog","aria-modal":"true","aria-labelledby":o,"data-onboarding-card":s,children:t})}const x={distance:"100%",in:{duration:.34,ease:[.16,1,.3,1]}},Ct={enter:t=>({x:t>0?x.distance:`-${x.distance}`}),center:{x:0},exit:t=>({x:t>0?`-${x.distance}`:x.distance})};function Ut({page:t,hero:e,heroKey:o,children:s}){const i=o??t,l=k.useRef(i),b=i>=l.current?1:-1;return k.useEffect(()=>{l.current=i},[i]),n.jsxs("div",{style:a.column,children:[e!==void 0&&n.jsx("div",{style:a.heroClip,children:n.jsx(mt,{initial:!1,custom:b,children:n.jsx(D.div,{custom:b,style:a.heroLayer,variants:Ct,initial:"enter",animate:"center",exit:"exit",transition:{x:x.in},children:e},i)})}),n.jsx("div",{style:a.column,children:s},t)]})}function Yt({children:t,style:e,field:o=!0}){return n.jsx("div",{style:{...a.hero,...o?wt:Tt,...e},children:t})}const Tt={backgroundColor:"transparent",backgroundImage:"none"};function Wt({children:t,flush:e=!1,standalone:o=!1,compact:s=!1}){const i=e?{...a.content,padding:0}:o?{...a.content,paddingTop:S}:a.content,l=s?{...i,minHeight:0}:i;return n.jsx("div",{style:l,children:t})}function Gt({children:t,floor:e=!0,compact:o=!1}){return n.jsx("div",{style:e&&!o?a.textBlock:{...a.textBlock,minHeight:0},children:t})}function Mt({glyph:t,children:e,style:o}){return n.jsxs("div",{style:{...a.chip,...o},children:[t,n.jsx("span",{children:e})]})}function Vt({id:t,children:e}){return n.jsx("h2",{id:t,style:a.title,children:e})}function Kt({children:t}){return n.jsx("p",{style:a.copy,children:t})}function qt({children:t,style:e}){return n.jsx("p",{style:{...a.meta,...e},children:t})}function Xt({children:t,style:e}){return n.jsx("div",{style:{...a.scroll,...e},children:t})}const Jt=k.forwardRef(function({style:e,large:o,...s},i){return n.jsx("input",{ref:i,...s,style:{...a.input,...o?a.inputLarge:null,...e},"data-card-field":!0})}),Zt=k.forwardRef(function({style:e,...o},s){return n.jsx("textarea",{ref:s,...o,style:{...a.input,...a.textarea,...e},"data-card-field":!0})});function Qt({children:t}){return n.jsx("div",{style:a.footer,children:t})}function te({count:t,page:e,onSelect:o}){return t<2?null:n.jsx("div",{style:a.dots,role:o?"tablist":"presentation",children:Array.from({length:t},(s,i)=>o?n.jsx("button",{type:"button",role:"tab","aria-selected":i===e,"aria-label":`Item ${i+1} of ${t}`,style:a.dotHit,"data-anno-dot":i<=e?"filled":"idle",onClick:()=>o(i),children:n.jsx("span",{style:{...a.dot,...i<=e?a.dotFilled:null}})},i):n.jsx("span",{style:a.dotHit,children:n.jsx("span",{style:{...a.dot,...i<=e?a.dotFilled:null}})},i))})}function ee({children:t}){return n.jsx("div",{style:a.actions,children:t})}function ne({children:t}){return n.jsx("p",{role:"status",style:a.note,children:t})}const B=E(C),$t=E(P),nt=E(null);function ae({children:t,style:e,...o}){return n.jsx("button",{type:"button",...o,style:{...a.primaryBtn,...B.style,...e},...B.attrs,children:t})}function oe({children:t,style:e,tone:o,...s}){const i=o==="danger",l=i?nt:$t;return n.jsx("button",{type:"button",...s,style:{...a.secondaryBtn,...l.style,...e},...l.attrs,"data-anno-danger":i?"":void 0,children:t})}function ie({style:t}){return n.jsx("svg",{width:"9.081",height:"9.081",viewBox:"0 0 9.08105 9.08105",fill:"none",style:t,"aria-hidden":!0,children:n.jsx("path",{d:"M9.08105 8.17871C9.08093 8.67691 8.67692 9.08096 8.17871 9.08105C7.68055 9.08091 7.27649 8.67688 7.27637 8.17871V5.25956C7.27637 4.45563 6.30439 4.05303 5.73593 4.62149L1.54102 8.81641C1.18868 9.16874 0.617058 9.16863 0.264648 8.81641C-0.0876205 8.464 -0.0877047 7.89239 0.264648 7.54004L4.46054 3.34414C5.029 2.77569 4.6264 1.80371 3.82248 1.80371H0.902344C0.404128 1.80361 0.000104874 1.40056 0 0.902344C-1.68533e-07 0.404142 0.404202 0.000259337 0.902344 0H8.17869C8.67705 0 9.08105 0.404002 9.08105 0.902365V8.17871Z",fill:"currentColor"})})}function re({children:t,style:e,...o}){return n.jsx("button",{type:"button",...o,style:{...a.ghostBtn,...e},"data-anno-ghost":!0,children:t})}function se({children:t,titleId:e,style:o}){return n.jsx("div",{role:"dialog","aria-modal":"false","aria-labelledby":e,style:{...a.popover,...o},children:t})}function de({id:t,children:e}){return n.jsx("h3",{id:t,style:a.popTitle,children:e})}function le({children:t}){return n.jsx("p",{style:a.popCopy,children:t})}function ce({children:t}){return n.jsx("div",{style:a.popActions,children:t})}E(f);function he({children:t,style:e,...o}){return n.jsx("button",{type:"button",...o,style:{...a.popBtn,...a.popDanger,...e},...nt.attrs,"data-anno-tint":"light",children:t})}const a={overlay:{position:"fixed",inset:0,display:"flex",alignItems:"center",justifyContent:"center",padding:40,boxSizing:"border-box"},stack:{position:"relative",width:`min(${M}px, calc(100vw - 32px))`},close:{position:"absolute",bottom:"100%",left:"100%",width:32,height:32,padding:0,display:"inline-flex",alignItems:"center",justifyContent:"center",background:f,border:"none",borderRadius:9999,color:p,cursor:g.hand},popover:{position:"absolute",left:105,top:202,width:253,boxSizing:"border-box",padding:6,...h(10),background:p,color:f,zIndex:2,display:"flex",flexDirection:"column",boxShadow:H},popTitle:{...$,fontWeight:500,fontSize:17,lineHeight:"22px",color:f,margin:"10px 14px 0"},popCopy:{...w,fontWeight:500,fontSize:12,lineHeight:"16px",letterSpacing:0,color:"rgba(248, 248, 248, 0.55)",margin:"8px 14px 0"},popActions:{display:"flex",gap:7,margin:"22px 0 0"},popBtn:{outline:"none",height:25,...h(5),padding:"0 10px",border:"none",display:"inline-flex",alignItems:"center",justifyContent:"center",whiteSpace:"nowrap",...m,fontSize:12,lineHeight:"16px",cursor:g.hand},popDanger:{background:"rgba(239, 2, 2, 0.28)",color:J},dialog:{position:"relative",width:"100%",background:f,...h(V),boxShadow:H,display:"flex",flexDirection:"column",gap:A,padding:K,fontFamily:L,maxHeight:"calc(100vh - 120px)",overflow:"hidden"},column:{display:"flex",flexDirection:"column",gap:A},heroClip:{position:"relative",width:"100%",aspectRatio:O,...h(R),overflow:"hidden",flexShrink:0},heroLayer:{position:"absolute",inset:0},hero:{position:"relative",width:"100%",aspectRatio:O,...h(R),overflow:"hidden",flexShrink:0,display:"flex",alignItems:"center",justifyContent:"center"},content:{padding:`0 ${S}px`,minHeight:q,boxSizing:"border-box",display:"flex",flexDirection:"column",gap:16,alignItems:"flex-start"},textBlock:{display:"flex",flexDirection:"column",gap:8,alignItems:"flex-start",width:"100%",minHeight:X,boxSizing:"border-box"},chip:{display:"inline-flex",alignItems:"center",gap:6,height:26,...h(42),padding:"0 8px",background:z,color:Z,...$,fontWeight:500,fontSize:15,lineHeight:"18px",flexShrink:0},title:{...$,fontWeight:500,fontSize:24,lineHeight:"30.5px",color:p,margin:0},copy:{...w,fontWeight:500,fontSize:12,lineHeight:"20px",letterSpacing:0,color:v,margin:0,whiteSpace:"pre-line",display:"-webkit-box",WebkitBoxOrient:"vertical",WebkitLineClamp:4,overflow:"hidden"},footer:{display:"flex",alignItems:"center",justifyContent:"space-between",gap:4,paddingLeft:5,minHeight:32},dots:{display:"flex",alignItems:"center"},dotHit:{width:12,height:12,padding:0,border:"none",background:"none",display:"inline-flex",alignItems:"center",justifyContent:"center",cursor:g.hand},dot:{width:6,height:6,borderRadius:9999,background:tt,transition:"background 0.15s ease"},dotFilled:{background:p},actions:{display:"flex",alignItems:"center",gap:4,marginLeft:"auto"},note:{...w,fontSize:12,lineHeight:"18px",letterSpacing:0,color:v,margin:"10px 16px 0"},meta:{...w,fontSize:12,lineHeight:"18px",letterSpacing:0,color:v,margin:0},scroll:{flex:"1 1 auto",minHeight:0,overflowY:"auto",overscrollBehavior:"contain",padding:`0 ${S}px`,boxSizing:"border-box",display:"flex",flexDirection:"column",gap:12},input:{width:"100%",boxSizing:"border-box",minHeight:36,...h(8),border:"none",outline:"none",padding:"8px 12px",background:"#FFFFFF",color:p,caretColor:p,...m,fontSize:14,lineHeight:"20px",cursor:g.text},inputLarge:{minHeight:48,...h(12),padding:"12px 16px",background:vt,fontSize:16,lineHeight:"24px",letterSpacing:"-0.01em"},textarea:{minHeight:72,resize:"none",fontFamily:L},primaryBtn:{outline:"none",height:32,...h(8),padding:"0 12px",border:"none",display:"inline-flex",alignItems:"center",gap:8,background:C,color:f,cursor:g.hand,...m,fontSize:16,lineHeight:"24px",whiteSpace:"nowrap",flexShrink:0},secondaryBtn:{outline:"none",height:32,...h(8),padding:"0 12px",border:"none",display:"inline-flex",alignItems:"center",gap:8,background:P,color:C,cursor:g.hand,...m,fontSize:16,lineHeight:"24px",whiteSpace:"nowrap",flexShrink:0},ghostBtn:{outline:"none",flex:"1 0 0",minWidth:0,height:32,...h(8),padding:"0 12px",border:"none",display:"inline-flex",alignItems:"center",gap:8,background:"transparent",color:p,cursor:g.hand,...m,fontSize:16,lineHeight:"24px",whiteSpace:"nowrap",overflow:"hidden"}};export{ne as A,v as B,Bt as C,Xt as D,Et as E,vt as F,f as G,Pt as H,p as I,St as J,M as K,wt as L,_t as M,se as N,Lt as O,jt as P,de as Q,le as R,ce as S,he as T,Q as U,Ft as a,zt as b,Wt as c,Mt as d,Gt as e,Vt as f,Kt as g,qt as h,Qt as i,re as j,ee as k,ae as l,oe as m,te as n,Yt as o,ie as p,P as q,h as r,Zt as s,z as t,Nt as u,J as v,r as w,kt as x,Ut as y,Jt as z};
