import{b as s,j as d}from"./react-C9WEJgbf.js";import{s as Oe,C as ye}from"./main-DamfXklH.js";import{b as ft,i as ht,p as Ee}from"./ecosystem-model-CaVx5yjR.js";import{u as pt}from"./useAdmin-DQ6ThlJb.js";import{M as ze,r as Fe}from"./index-DrPmQ-od.js";import"./three-CbhGz-nM.js";import"./api-pricing-reference-zLzsMT3F.js";import"./pricing-default-multiplier-CpNirbcJ.js";const Ze=.001,xt=1-Math.pow(Ze,1/300),We=.6;function ae(){return(Math.random()-.5)*1e-6}class mt{bodies=[];links=[];alpha=1;alphaTarget=0;raf=null;distanceMax2=300*300;distanceMin2=100;onTick=null;setGraph(n,o,c){this.bodies=n,this.links=o,c.distanceMax&&(this.distanceMax2=c.distanceMax*c.distanceMax);const a=new Map;for(const l of o)a.set(l.source.id,(a.get(l.source.id)??0)+1),a.set(l.target.id,(a.get(l.target.id)??0)+1);for(const l of o){const x=a.get(l.source.id)??1,m=a.get(l.target.id)??1;l.bias=x/(x+m)}}reheat(n=1){this.alpha=Math.max(this.alpha,n),this.start()}setAlphaTarget(n){this.alphaTarget=n,n>0&&this.start()}start(){if(this.raf!==null)return;const n=()=>{this.raf=requestAnimationFrame(n),this.tick(),this.onTick?.(),this.alpha<Ze&&this.alphaTarget===0&&this.stop()};this.raf=requestAnimationFrame(n)}stop(){this.raf!==null&&cancelAnimationFrame(this.raf),this.raf=null}tick(){this.alpha+=(this.alphaTarget-this.alpha)*xt;const n=this.alpha,o=this.bodies;for(const c of this.links){const a=c.source,l=c.target;let x=l.x+l.vx-a.x-a.vx||ae(),m=l.y+l.vy-a.y-a.vy||ae();const w=Math.sqrt(x*x+m*m),g=(w-c.distance)/w*n*.1;x*=g,m*=g,l.vx-=x*c.bias,l.vy-=m*c.bias,a.vx+=x*(1-c.bias),a.vy+=m*(1-c.bias)}for(let c=0;c<o.length;c++){const a=o[c];for(let l=c+1;l<o.length;l++){const x=o[l];let m=x.x-a.x,w=x.y-a.y,g=m*m+w*w;if(g>=this.distanceMax2)continue;m===0&&(m=ae(),g+=m*m),w===0&&(w=ae(),g+=w*w),g<this.distanceMin2&&(g=Math.sqrt(this.distanceMin2*g));const M=x.charge*n/g,E=a.charge*n/g;a.vx+=m*M,a.vy+=w*M,x.vx-=m*E,x.vy-=w*E}}for(const c of o)c.fx===null?(c.vx*=We,c.x+=c.vx):(c.x=c.fx,c.vx=0),c.fy===null?(c.vy*=We,c.y+=c.vy):(c.y=c.fy,c.vy=0)}bounds(){if(!this.bodies.length)return null;let n=1/0,o=1/0,c=-1/0,a=-1/0;for(const l of this.bodies)l.x<n&&(n=l.x),l.x>c&&(c=l.x),l.y<o&&(o=l.y),l.y>a&&(a=l.y);return{minX:n,minY:o,maxX:c,maxY:a}}}function gt(r,n,o,c){const a=c??10*Math.sqrt(.5+r),l=r*Math.PI*(3-Math.sqrt(5));return{x:n+a*Math.cos(l),y:o+a*Math.sin(l)}}function bt(r){return r.startsWith("tools/")?r.slice(6):r.startsWith("models/")?`model:${r.slice(7)}`:null}function yt(){const{role:r}=pt(),n=r==="superadmin",[o,c]=s.useState(new Map),[a,l]=s.useState(!0),[x,m]=s.useState(0);s.useEffect(()=>{let g=!1;return l(!0),(async()=>{const{data:M,error:E}=await Oe.from("atlas_docs").select("tool_key, mdx, published");if(g)return;const y=new Map;if(!E&&M)for(const R of M)y.set(R.tool_key,{docKey:R.tool_key,mdx:R.mdx,published:R.published});c(y),l(!1)})(),()=>{g=!0}},[x,n]);const w=s.useCallback(async(g,M,E)=>{const{error:y}=await Oe.from("atlas_docs").upsert({tool_key:g,mdx:M,published:E},{onConflict:"tool_key"});return y?{ok:!1,error:y.message}:(m(R=>R+1),{ok:!0})},[]);return{docs:o,loading:a,canEdit:n,saveDoc:w}}function vt({docKey:r,doc:n,canEdit:o,onSave:c,editing:a,onEditingChange:l}){const[x,m]=s.useState(""),[w,g]=s.useState(!0),[M,E]=s.useState(!1),[y,R]=s.useState(null);if(s.useEffect(()=>{R(null)},[r]),!r)return null;const P=l,ee=()=>{m(n?.mdx??""),g(n?.published??!0),R(null),P(!0)},C=async()=>{E(!0),R(null);const S=await c(r,x,w);E(!1),S.ok?P(!1):R(S.error??"Save failed")};return a?d.jsxs("div",{className:"eco-doc-edit",children:[d.jsx("textarea",{value:x,onChange:S=>m(S.target.value),spellCheck:!1,className:"eco-doc-textarea",placeholder:"Write the editorial in markdown…"}),d.jsx("div",{className:"eco-doc-preview-label",children:"Preview"}),d.jsx("div",{className:"eco-doc",children:d.jsx(ze,{rehypePlugins:[Fe],children:x||"_(empty)_"})}),d.jsxs("div",{className:"eco-doc-bar",children:[d.jsxs("label",{className:"eco-doc-publish",children:[d.jsx("input",{type:"checkbox",checked:w,onChange:S=>g(S.target.checked)}),"Published"]}),d.jsx("span",{style:{flex:1}}),y?d.jsx("span",{className:"eco-doc-error",children:y}):null,d.jsx("button",{className:"eco-doc-btn",onClick:()=>P(!1),disabled:M,children:"Cancel"}),d.jsx("button",{className:"eco-doc-btn is-primary",onClick:()=>void C(),disabled:M,children:M?"Saving…":"Save"})]})]}):d.jsxs(d.Fragment,{children:[n?.mdx?d.jsxs("div",{className:"eco-doc",children:[n.published?null:d.jsx("span",{className:"eco-doc-draft",children:"draft"}),d.jsx(ze,{rehypePlugins:[Fe],children:n.mdx})]}):null,o?d.jsx("button",{className:"eco-doc-btn eco-doc-write",onClick:ee,children:n?"Edit editorial":"Write editorial"}):null]})}const wt=s.forwardRef(function({node:n,open:o,pinned:c,editing:a,onEditingChange:l,docKey:x,doc:m,canEdit:w,onSave:g},M){const E=n?.kind==="leaf";return d.jsx("div",{ref:M,className:["eco-card",o?"is-open":"",c?"is-pinned":"",a?"is-editing":""].filter(Boolean).join(" "),onPointerDown:y=>y.stopPropagation(),children:n?d.jsxs(d.Fragment,{children:[!E&&n.subtitle?d.jsx("p",{className:"eco-card-kicker",children:n.subtitle}):null,n.description?d.jsx("p",{className:"eco-card-body",children:n.description}):null,E?d.jsxs("p",{className:"eco-meta",children:[d.jsx("span",{className:"eco-meta-label",children:"Kind"}),d.jsx("span",{children:n.subtitle})]}):null,n.meta?.map(y=>d.jsxs("p",{className:"eco-meta",children:[d.jsx("span",{className:"eco-meta-label",children:y.label}),d.jsx("span",{children:y.value})]},y.label)),d.jsx(vt,{docKey:x,doc:m,canEdit:w,onSave:g,editing:a,onEditingChange:l})]}):null})}),kt="#e6e5e1",$="#1a1a19",le="#f7f7f2",Mt="#d1cfc9",Et="#dfba34",ve="#6c6962",se="#d6d5cf",we="#b8b8b5",de=32,Ct=8,$t=6,Rt=21,Be=32,St=9,ke=50,_t=11,Xe=23,At=50,It=40,Je=360,Lt=560,Nt=12,jt=7,He=16,Ye=22,Tt=12,Q=20,Pt=96,Dt=72,Ue=58,Ot=240,zt=22,Ft=8,Wt=900,Bt=.75,Xt=120,Ht=62,qe=26,Ge=46,Ke=40,Yt={root:-1100,hub:-900,category:-700,leaf:-260},Ut={hub:200,category:150,leaf:110};function qt(r){return Math.min(3,Math.max(1,Math.ceil(r/10)))}function Ve(r,n,o,c){const a=Ut[r]??120;if(r!=="leaf"||n<=8)return Math.min(a,c);const l=qt(n),x=a+Math.sqrt(n/l)*30;return Math.min(x*(1+o%l*.45),c)}function Gt(r,n){const o=Yt[r]??-400;return r!=="leaf"?o:o*Math.min(2.4,Math.max(.8,n/44))}const Kt=.55,Vt=.12,Zt=.72,Me=620,Jt=6;function hn(){const r=s.useMemo(()=>ft(),[]),n=s.useMemo(()=>ht(r),[r]);s.useEffect(()=>{const e=document.title;return document.title="Ecosystem · Plnty",()=>{document.title=e}},[]);const[o,c]=s.useState(()=>en(r)),[a,l]=s.useState(()=>tn(r)),[x,m]=s.useState(null),[w,g]=s.useState(!1),[M,E]=s.useState(0),y=s.useRef(null),R=s.useRef(null),P=s.useRef(new Map),ee=s.useRef(new Map),C=s.useRef(new Map),S=s.useRef(new Map),ie=s.useRef(null);ie.current===null&&(ie.current=new mt);const ue=s.useRef(null),Ce=s.useRef({w:Je,h:220}),$e=s.useRef(40),Re=s.useRef(null),fe=s.useRef(null),_=o[o.length-1]??r.id,q=o.length<=1,te=s.useCallback(e=>{const i=[r],u=[],h=f=>{for(const p of f.children??[])i.push(p),u.push({from:f,to:p})};h(r);for(const f of e){if(f===r.id)continue;const p=n.get(f);p&&h(p)}return{nodes:i,links:u}},[r,n]),{nodes:he,links:Se}=s.useMemo(()=>te(o),[te,o]),[_e,Ae]=s.useState([]),pe=s.useRef(void 0),G=s.useMemo(()=>{const e=new Map;return(function i(u){for(const h of u.children??[])e.set(h.id,u),i(h)})(r),e},[r]);s.useEffect(()=>()=>window.clearTimeout(pe.current),[]);const X=s.useCallback(e=>{const i=te(o).nodes,u=new Set(te(e).nodes.map(f=>f.id)),h=[];for(const f of i){if(u.has(f.id))continue;let p=G.get(f.id);for(;p&&!u.has(p.id);)p=G.get(p.id);p&&h.push({node:f,parent:p})}window.clearTimeout(pe.current),Ae(h),c(e),h.length&&(pe.current=window.setTimeout(()=>Ae([]),Me))},[te,o,G]),{nodes:K,links:ne,leavingIds:V}=s.useMemo(()=>{const e=new Set(he.map(u=>u.id)),i=_e.filter(u=>!e.has(u.node.id));return{nodes:[...he,...i.map(u=>u.node)],links:[...Se,...i.map(u=>({from:u.parent,to:u.node}))],leavingIds:new Set(i.map(u=>u.node.id))}},[he,Se,_e]),xe=s.useMemo(()=>{const e=new Set(o),i=n.get(_);for(const u of i?.children??[])e.add(u.id);return e},[n,_,o]),Ie=a?n.get(a)??null:null,et=x?n.get(x)??null:null,oe=Ie??et,Z=oe?.id??null,Le=s.useRef(null);oe&&(Le.current=oe);const re=oe??Le.current;Z&&(Re.current=Z);const{docs:Ne,canEdit:tt,saveDoc:nt}=yt(),ce=re?re.docKeys?.find(e=>Ne.has(e))??bt(re.id):null;s.useEffect(()=>{g(!1)},[ce]);const J=s.useRef({dx:40,dy:de/2}),je=s.useRef(null),me=s.useRef(null),z=s.useCallback(()=>{const e=ue.current,i=y.current,u=Re.current;if(!e||!i||!u)return;const h=C.current.get(u);if(!h)return;const f=i.clientWidth,p=i.clientHeight,{w:L,h:O}=Ce.current,N=me.current,H=h.x+(S.current.get(u)??40)-Tt,A=N?N.x:Math.max(h.x+J.current.dx,H),I=N?N.y:h.y+J.current.dy;let j=A+He;j+L+Q>f&&(j=A-He-L);let F=I+Ye;F+O+Q>p&&(F=I-Ye-O),j=Math.max(Q,Math.min(j,f-L-Q)),F=Math.max(Q,Math.min(F,p-O-Q)),e.style.transform=`translate(${Math.round(j)}px, ${Math.round(F)}px)`},[]),ot=s.useCallback((e,i)=>{const u=y.current,h=C.current.get(i);if(!u||!h)return;const f=u.getBoundingClientRect();J.current={dx:e.clientX-f.left-h.x,dy:e.clientY-f.top-h.y},je.current=i},[]);s.useEffect(()=>{const e=ue.current;if(!e)return;const i=new ResizeObserver(()=>{Ce.current={w:e.offsetWidth,h:e.offsetHeight},z()});return i.observe(e),()=>i.disconnect()},[z]),s.useEffect(()=>{if(Z){if(je.current!==Z){const e=P.current.get(Z)?.firstElementChild;$e.current=e?e.offsetWidth/2:ke/2,J.current={dx:$e.current,dy:de/2}}z()}},[Z,z]),s.useEffect(()=>{if(!a){me.current=null;return}const e=C.current.get(a);e&&(me.current={x:e.x+J.current.dx,y:e.y+J.current.dy}),z()},[a,z]),s.useEffect(()=>{const e=y.current,i=R.current,u=ie.current;if(!e||!i||!u)return;const h=e.clientWidth,f=e.clientHeight;i.setAttribute("width",String(h)),i.setAttribute("height",String(f));const p=Math.round(h/2),L=Math.round(Math.min(f*.6,f-180)),O=new Map;o.forEach((t,b)=>{O.set(t,b===0?L:L-At-(b-1)*It)});const N=O.get(_)??L,H=Math.max(150,Math.min((h-2*Dt)/2,N-Pt)*.92);for(const t of K){const b=P.current.get(t.id)?.firstElementChild;b?.offsetWidth&&S.current.set(t.id,b.offsetWidth/2)}const A=new Map;for(const t of K){const b=G.get(t.id)?.children?.findIndex(v=>v.id===t.id)??0;A.set(t.id,b<0?0:b)}const I=[];let j=0;K.forEach((t,b)=>{let v=C.current.get(t.id);if(!v){j++;const k=G.get(t.id),W=k?C.current.get(k.id):void 0,Y=k?.children?.length??1,U=Ve(t.kind,Y,A.get(t.id)??0,H);let B;if(W&&k&&k.id===_&&!q){const lt=A.get(t.id)??0,dt=Math.min(4.4,1.7+Y*.07),ut=Y>1?lt/(Y-1)-.5:0,De=-Math.PI/2+ut*dt;B={x:W.x+Math.cos(De)*U,y:W.y+Math.sin(De)*U}}else W?B=gt(b,W.x,W.y,U*Zt):B={x:p,y:L};v={id:t.id,x:B.x,y:B.y,vx:0,vy:0,fx:null,fy:null,charge:-400},C.current.set(t.id,v)}v.charge=Gt(t.kind,S.current.get(t.id)??44)*(V.has(t.id)?.08:1);const T=O.get(t.id);T!==void 0?(v.fx=p,v.fy=T):fe.current!==t.id&&(v.fx=null,v.fy=null),I.push(v)});const F=new Set(K.map(t=>t.id));for(const t of[...C.current.keys()])F.has(t)||C.current.delete(t);for(const t of[...S.current.keys()])F.has(t)||S.current.delete(t);const Te=[];for(const t of ne){const b=C.current.get(t.from.id),v=C.current.get(t.to.id);if(!b||!v)continue;const T=t.from.children?.length??1,k=V.has(t.to.id)?Jt:Ve(t.to.kind,T,A.get(t.to.id)??0,H);Te.push({source:b,target:v,distance:k,bias:.5})}u.setGraph(I,Te,{distanceMax:420});const Pe=q?[]:n.get(_)?.children??[],rt=new Set(Pe.map(t=>t.id)),ct=Math.min(Wt,Ot+Math.max(0,Pe.length-Ft)*zt),at=new Set(q?[]:(r.children??[]).filter(t=>!o.includes(t.id)).map(t=>t.id));return u.onTick=()=>{for(const t of I){if(V.has(t.id)||t.fx!==null)continue;if(rt.has(t.id)){const k=S.current.get(t.id)??40,W=Math.min(ct,Math.abs(t.x-p)*Bt),Y=N-Ue+W;if(t.y>Y&&(t.y=Y,t.vy>0&&(t.vy=0)),t.y>N-Ue){const U=Xt+k,B=t.x-p;Math.abs(B)<U&&(t.x=p+(B>=0?U:-U),t.vx=0)}}else if(at.has(t.id)){const k=L+Ht;t.y<k&&(t.y=k,t.vy<0&&(t.vy=0))}const b=S.current.get(t.id)??40,v=qe+b,T=Math.max(v,h-qe-b);t.x<v?(t.x=v,t.vx<0&&(t.vx=0)):t.x>T&&(t.x=T,t.vx>0&&(t.vx=0)),t.y<Ge?(t.y=Ge,t.vy<0&&(t.vy=0)):t.y>f-Ke&&(t.y=f-Ke,t.vy>0&&(t.vy=0))}for(const t of I){const b=P.current.get(t.id);b&&(b.style.transform=`translate(${t.x}px, ${t.y}px)`)}for(let t=0;t<ne.length;t++){const b=ne[t],v=ee.current.get(`${b.from.id}>${b.to.id}`),T=C.current.get(b.from.id),k=C.current.get(b.to.id);v&&T&&k&&v.setAttribute("d",`M${T.x},${T.y} L${k.x},${k.y}`)}z()},u.reheat(j>0?Kt:Vt),()=>u.stop()},[K,ne,n,r,o,_,q,V,G,z,M]),s.useEffect(()=>{let e=!0;return document.fonts?.ready.then(()=>{e&&E(i=>i+1)}),()=>{e=!1}},[]),s.useEffect(()=>{const e=y.current;if(!e)return;let i;const u=new ResizeObserver(()=>{window.clearTimeout(i),i=window.setTimeout(()=>E(h=>h+1),180)});return u.observe(e),()=>{window.clearTimeout(i),u.disconnect()}},[]);const D=s.useCallback(e=>{const i=e&&e!=="plnty"?`/ecosystem/${e}`:"/ecosystem";window.history.replaceState(null,"",i)},[]),ge=s.useCallback(()=>{if(o.length<=1)return;const e=o.slice(0,-1);X(e),l(null),m(null),D(e[e.length-1]??null)},[o,X,D]),be=s.useCallback(e=>{if(e.id===r.id){X([r.id]),l(null),D(null);return}if(e.kind==="leaf"){const i=a===e.id?null:e.id;l(i),D(i??_);return}if(e.id===_&&o.length>1){ge();return}X(Ee(r,e.id).map(i=>i.id)),l(null),D(e.id)},[r,_,o,a,D,X,ge]),st=s.useCallback(()=>{if(m(null),a){l(null),D(_);return}o.length>1&&(X([r.id]),D(null))},[a,o.length,_,r.id,X,D]),it=s.useCallback((e,i)=>{const u=ie.current,h=C.current.get(i.id),f=y.current;if(!u||!h||!f)return;if(e.preventDefault(),e.stopPropagation(),o.includes(i.id)){be(i);return}const p=e.currentTarget;p.setPointerCapture(e.pointerId);const L=e.clientX,O=e.clientY;let N=!1;fe.current=i.id,u.setAlphaTarget(.3),u.reheat(.3),h.fx=h.x,h.fy=h.y;const H=I=>{Math.hypot(I.clientX-L,I.clientY-O)>3&&(N=!0);const j=f.getBoundingClientRect();h.fx=I.clientX-j.left,h.fy=I.clientY-j.top},A=()=>{p.removeEventListener("pointermove",H),p.removeEventListener("pointerup",A),p.removeEventListener("pointercancel",A),u.setAlphaTarget(0),fe.current=null,h.fx=null,h.fy=null,N||be(i)};p.addEventListener("pointermove",H),p.addEventListener("pointerup",A),p.addEventListener("pointercancel",A)},[be,o]);return d.jsxs("div",{style:nn,ref:y,onPointerDown:st,children:[d.jsx("style",{children:on}),d.jsx("svg",{ref:R,className:"eco-lines",width:960,height:600,children:ne.map(e=>{const i=`${e.from.id}>${e.to.id}`,u=!(xe.has(e.from.id)&&xe.has(e.to.id)),h=V.has(e.to.id);return d.jsx("path",{ref:f=>{f?ee.current.set(i,f):ee.current.delete(i)},className:`eco-link${u?" is-dim":""}${h?" is-leaving":""}`},i)})}),K.map(e=>{const i=e.id===r.id,u=e.id===_&&!q,h=!i&&(e.id===a||e.id===x||o.includes(e.id));return d.jsx("div",{ref:f=>{f?P.current.set(e.id,f):P.current.delete(e.id)},className:["eco-node",i?"is-root":"",xe.has(e.id)?"":"is-dim",h?"is-active":"",V.has(e.id)?"is-leaving":""].filter(Boolean).join(" "),onPointerDown:f=>it(f,e),onPointerEnter:i?void 0:f=>{ot(f,e.id),m(e.id)},onPointerLeave:i?void 0:()=>m(f=>f===e.id?null:f),children:d.jsxs("div",{className:"eco-node-inner",children:[u?d.jsx("button",{type:"button",className:"eco-close","aria-label":`Close ${e.title}`,onPointerDown:f=>{f.preventDefault(),f.stopPropagation(),ge()},children:d.jsx("svg",{width:"10",height:"10",viewBox:"0 0 10 10","aria-hidden":"true",children:d.jsx("path",{d:"M1 1 L9 9 M9 1 L1 9",stroke:"currentColor",strokeWidth:"1.4",strokeLinecap:"round"})})}):null,i?d.jsx("span",{className:"eco-root","aria-label":e.title,role:"img",children:d.jsx(Qt,{})}):d.jsx("span",{className:"eco-pill",children:e.title})]})},e.id)}),d.jsxs("header",{className:`eco-head${q?"":" is-hidden"}`,children:[d.jsx("h1",{children:"Behind the canvas"}),d.jsx("p",{children:r.description})]}),d.jsx(wt,{ref:ue,node:re,open:!!oe,pinned:!!Ie,editing:w,onEditingChange:g,docKey:ce,doc:ce?Ne.get(ce):void 0,canEdit:tt,onSave:nt})]})}function Qt(){return d.jsx("svg",{width:Xe,height:Xe,viewBox:"0 0 394 394",fill:"none","aria-hidden":"true",children:d.jsx("path",{fill:$,d:"M288.044 0C346.11 0.00180656 393.182 47.0721 393.183 105.138C393.183 127.039 386.486 147.37 375.024 164.199C363.065 181.759 363.055 211.407 375.011 228.97C386.474 245.804 393.183 266.14 393.183 288.044C393.18 346.109 346.109 393.181 288.044 393.183C266.14 393.183 245.802 386.475 228.97 375.011C211.408 363.055 181.76 363.054 164.199 375.011C147.371 386.474 127.039 393.183 105.138 393.183C47.0734 393.181 0.0027098 346.109 0 288.044C0 266.142 6.69716 245.805 18.1582 228.97C30.1145 211.409 30.1157 181.759 18.1582 164.199C6.69844 147.369 0 127.039 0 105.138C0.00090327 47.0721 47.0723 0.00180661 105.138 0C127.039 0 147.37 6.69699 164.199 18.1582C181.761 30.1183 211.407 30.1171 228.97 18.1582C245.803 6.69581 266.14 0 288.044 0Z"})})}function Qe(){const r=window.location.pathname.match(/^\/ecosystem\/(.+)$/);return r?decodeURIComponent(r[1]).replace(/\/$/,""):null}function en(r){const n=Qe();if(!n)return[r.id];const o=Ee(r,n);if(!o.length)return[r.id];const c=o[o.length-1],a=o.map(l=>l.id);return c.kind==="leaf"?a.slice(0,-1):a}function tn(r){const n=Qe();if(!n)return null;const o=Ee(r,n);if(!o.length)return null;const c=o[o.length-1];return c.kind==="leaf"?c.id:null}const nn={position:"fixed",inset:0,background:kt,color:$,overflow:"hidden",fontFamily:"'Suisse Intl', -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif"},on=`
.eco-lines { position:absolute; top:0; left:0; pointer-events:none; }
.eco-link {
  fill:none; stroke:${$}; stroke-width:1; stroke-dasharray:3 4;
  opacity:.85; transition:opacity 420ms ease;
}
.eco-link.is-dim { opacity:.2; }
.eco-link.is-leaving { opacity:0; transition:opacity ${Me}ms ease; }

/* A zero-sized point that the tick loop moves. The inner box hangs off it,
 * centred, so the pill sits ON the node's position rather than beside it. */
.eco-node { position:absolute; top:0; left:0; width:0; height:0; cursor:${ye.hand}; }
.eco-node-inner { position:absolute; left:0; top:0; transform:translate(-50%,-50%); }

/* Depth reads as focus: what you are not looking at falls back and goes soft.
 * The blur is what turns a flat diagram into something with a foreground. */
.eco-node, .eco-node-inner { transition:opacity 420ms ease, filter 420ms ease; }
.eco-node.is-dim .eco-node-inner { opacity:.42; filter:blur(2.6px); }
.eco-node.is-leaving { opacity:0; pointer-events:none; transition:opacity ${Me}ms ease; }

.eco-pill {
  display:block; height:${de}px; line-height:${de}px; padding:0 ${Ct}px;
  border-radius:${$t}px; background:${Mt}; color:${$};
  font-size:${Rt}px; font-weight:700; letter-spacing:-.015em; white-space:nowrap;
  transition:background 150ms ease, color 150ms ease;
}
.eco-node.is-active .eco-pill { background:${$}; color:${le}; }

.eco-root {
  display:flex; align-items:center; justify-content:center;
  width:${ke}px; height:${ke}px; border-radius:${_t}px;
  background:${Et}; transition:transform 160ms ease;
}
.eco-node.is-root:hover .eco-root { transform:scale(1.06); }

.eco-close {
  position:absolute; right:100%; top:50%; margin-right:${St}px;
  transform:translateY(-50%); width:${Be}px; height:${Be}px;
  display:flex; align-items:center; justify-content:center;
  padding:0; border:none; border-radius:50%;
  background:${$}; color:${le}; cursor:${ye.hand};
}

/* The cover: shown while nothing is open, gone the moment you step in. */
.eco-head {
  position:fixed; left:50%; top:52px; transform:translateX(-50%);
  width:min(700px, 82vw); text-align:center; z-index:5;
  pointer-events:none; transition:opacity 420ms ease;
}
.eco-head.is-hidden { opacity:0; }
.eco-head h1 {
  margin:0 0 24px; font-size:clamp(36px, 4.6vw, 62px); font-weight:700;
  letter-spacing:-.028em; line-height:1.02; color:${$};
}
/* The measure is set by the copy, not picked: this paragraph needs 631px to
 * break over the two lines it was written for, and below that it drops the word
 * 'mediums.' onto a third line on its own. 660 clears it without inviting a
 * line long enough to be hard to track back. */
.eco-head p { margin:0 auto; max-width:660px; font-size:14px; line-height:21px; color:${ve}; }

/* ── the card ───────────────────────────────────────────────────────────── */
.eco-card {
  position:fixed; left:0; top:0; box-sizing:border-box; z-index:7;
  width:${Je}px; max-height:min(62vh, 560px); padding:${Nt}px;
  background:${le}; border-radius:${jt}px;
  overflow-y:auto; scrollbar-width:none;
  opacity:0; pointer-events:none;
  transition:opacity 180ms ease, width 220ms ease, max-height 220ms ease;
}
.eco-card::-webkit-scrollbar { display:none; }
.eco-card.is-open { opacity:1; }
.eco-card.is-pinned { pointer-events:auto; }
.eco-card.is-editing { width:${Lt}px; max-height:min(80vh, 760px); }

.eco-card-kicker {
  margin:0 0 10px; font-size:11px; letter-spacing:.09em; text-transform:uppercase;
  color:${we};
}
.eco-card-body { margin:0; font-size:15px; line-height:23px; color:${$}; }
.eco-meta { margin:14px 0 0; font-size:14px; line-height:20px; color:${$}; display:flex; gap:12px; }
.eco-card > .eco-meta:first-child { margin-top:0; }
.eco-card-body + .eco-meta { margin-top:20px; }
.eco-meta-label { flex:0 0 74px; color:${we}; }

/* ── Editorial ──────────────────────────────────────────────────────────
 * The written account, under the spec rows. A rule separates it because the
 * two are different registers: above is what the thing IS, below is what it
 * is FOR. Scrolling belongs to the card, not to this block, so a long entry
 * scrolls as one column instead of trapping a second scrollbar inside the
 * first. */
.eco-doc {
  margin-top:20px; padding-top:16px; border-top:1px solid ${se};
  font-size:14px; line-height:21px; color:${$};
}
.eco-doc > :first-child { margin-top:0; }
.eco-doc p { margin:0 0 12px; }
.eco-doc h1, .eco-doc h2, .eco-doc h3 {
  font-size:15px; font-weight:600; line-height:1.3; margin:18px 0 8px;
}
.eco-doc ul, .eco-doc ol { margin:0 0 12px; padding-left:18px; }
.eco-doc li { margin:0 0 4px; }
.eco-doc a { color:${$}; text-underline-offset:2px; }
.eco-doc code {
  font-family:ui-monospace,SFMono-Regular,Menlo,monospace; font-size:12px;
  background:#e9e8e3; border-radius:3px; padding:1px 4px;
}
.eco-doc blockquote {
  margin:0 0 12px; padding-left:12px; border-left:1px solid ${se}; color:${ve};
}
.eco-doc-draft {
  display:inline-block; margin-bottom:8px; font-size:10px; letter-spacing:.08em;
  text-transform:uppercase; color:#8a6d3b; background:#fdf4e0;
  border-radius:3px; padding:1px 6px;
}

/* Authoring chrome. Superadmin-only, so it never renders for a reader. */
.eco-doc-edit { margin-top:20px; padding-top:16px; border-top:1px solid ${se}; }
.eco-doc-textarea {
  width:100%; min-height:240px; box-sizing:border-box; padding:10px;
  font-family:ui-monospace,SFMono-Regular,Menlo,monospace; font-size:12px; line-height:1.55;
  color:${$}; background:#fff; border:1px solid ${se}; border-radius:6px; resize:vertical;
}
.eco-doc-preview-label {
  margin:14px 0 0; font-size:10px; letter-spacing:.14em;
  text-transform:uppercase; color:${we};
}
.eco-doc-bar { display:flex; align-items:center; gap:8px; margin-top:12px; }
.eco-doc-publish { display:flex; align-items:center; gap:5px; font-size:12px; color:${ve}; }
.eco-doc-error { font-size:12px; color:#c0392b; }
.eco-doc-btn {
  font:inherit; font-size:12px; padding:5px 12px; border-radius:999px;
  border:1px solid ${se}; background:#fff; color:${$}; cursor:${ye.hand};
}
.eco-doc-btn:disabled { opacity:.5; cursor:default; }
.eco-doc-btn.is-primary { background:${$}; border-color:${$}; color:${le}; }
.eco-doc-write { margin-top:14px; }
`;export{hn as EcosystemPage};
