import{b as i,j as t}from"./react-C9WEJgbf.js";import{h as Me,ad as jt,e as Ue,M as Ce,r as mt,S as Ke,O as ht,Z as Ze,aF as J,U as he,f as ce,W as ca,ab as kt,k as Qe,P as Qa,R as er,j as tr,d as V,D as ar,aU as rr,l as nr,_ as sr,b4 as or,b5 as ir,as as lr,a1 as Fe,ar as cr,aL as dr,aw as ur,X as et,p as Bt,al as qt,m as pr,b6 as hr}from"./three-CbhGz-nM.js";import{a as M,B as Vt,j as $e,c as mr,k as da,e as gr,m as fr,n as vr,o as br,p as xr}from"./brush-textures-BLmKcWao.js";import{p as yr,r as wr,e as Sr,b as Mr}from"./brush-engine-parametric-5IUdZMvq.js";import{g as Cr,P as Rr,a as kr,B as Ie,b as Tr,c as jr}from"./procedural-tips-DZaxAEdM.js";import"./main-DamfXklH.js";import"./MeshBVH-PQsxEzdn.js";const Zt=6;function Br(r){return r==="bilateral"?1:r==="variance-gated"?2:0}const Pr=`
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = vec4(position.xy, 0.0, 1.0);
  }
`,Ar=`
  uniform sampler2D paintTex;
  uniform sampler2D islandMask;
  uniform vec2 texelSize;
  uniform int method;          // 0 gaussian, 1 bilateral, 2 variance-gated
  uniform int kernelRadius;    // 1..MAX_R
  uniform float strength;      // 0..1 blend toward smoothed
  uniform float spatialSigma;  // gaussian sigma in texels
  uniform float colorSigma;    // bilateral colour sigma (rgb units)
  uniform float edgeThreshold; // variance gate level
  uniform float useIslandGate; // >0.5 = island-aware
  varying vec2 vUv;

  const int MAX_R = ${Zt};
  const float ALPHA_EPS = 0.001;

  // Same test as the composite shader: a tap is in the same island only if
  // both texels are claimed (a >= 0.5) and their packed R/G ID bytes match.
  bool sameIsland(vec4 a, vec4 b) {
    if (a.a < 0.5 || b.a < 0.5) return false;
    return abs(a.r - b.r) < (0.5 / 255.0) && abs(a.g - b.g) < (0.5 / 255.0);
  }

  void main() {
    vec4 center = texture2D(paintTex, vUv);
    // Unpainted texels pass straight through — smoothing only blends colour
    // WITHIN painted regions, never erodes the mask or pulls toward base.
    if (center.a < ALPHA_EPS) {
      gl_FragColor = center;
      return;
    }

    vec4 centerId = texture2D(islandMask, vUv);
    float sig2 = 2.0 * spatialSigma * spatialSigma;
    float colSig2 = 2.0 * colorSigma * colorSigma;

    vec3 sumRGB = vec3(0.0);
    float sumW = 0.0;
    // Unweighted stats over painted same-island taps (variance-gated only).
    vec3 meanAcc = vec3(0.0);
    vec3 meanSqAcc = vec3(0.0);
    float cnt = 0.0;

    // Loop bound is the compile-time constant MAX_R (WebGL1 requirement);
    // the runtime kernelRadius gates each iteration via continue.
    for (int dx = -MAX_R; dx <= MAX_R; dx++) {
      if (dx < -kernelRadius || dx > kernelRadius) continue;
      for (int dy = -MAX_R; dy <= MAX_R; dy++) {
        if (dy < -kernelRadius || dy > kernelRadius) continue;
        vec2 sUv = vUv + vec2(float(dx), float(dy)) * texelSize;
        vec4 tap = texture2D(paintTex, sUv);
        if (tap.a < ALPHA_EPS) continue;
        if (useIslandGate > 0.5) {
          vec4 tapId = texture2D(islandMask, sUv);
          if (!sameIsland(centerId, tapId)) continue;
        }
        float d2 = float(dx * dx + dy * dy);
        float spatial = exp(-d2 / max(sig2, 1e-4));
        float w = spatial;
        if (method == 1) {
          vec3 dc = tap.rgb - center.rgb;
          w *= exp(-dot(dc, dc) / max(colSig2, 1e-5));
        }
        sumRGB += w * tap.rgb;
        sumW += w;
        meanAcc += tap.rgb;
        meanSqAcc += tap.rgb * tap.rgb;
        cnt += 1.0;
      }
    }

    if (sumW < 1e-5) {
      gl_FragColor = center;
      return;
    }
    vec3 smoothed = sumRGB / sumW;

    float effStrength = strength;
    if (method == 2) {
      vec3 mean = meanAcc / cnt;
      vec3 varv = meanSqAcc / cnt - mean * mean;
      float variance = varv.r + varv.g + varv.b; // sum of channel variances
      float gate = 1.0 - smoothstep(edgeThreshold * 0.25, edgeThreshold, variance);
      effStrength *= gate;
    }

    vec3 outRGB = mix(center.rgb, smoothed, clamp(effStrength, 0.0, 1.0));
    gl_FragColor = vec4(outRGB, center.a);
  }
`,Ir=`
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = vec4(position.xy, 0.0, 1.0);
  }
`,Ur=`
  uniform sampler2D src;
  varying vec2 vUv;
  void main() {
    gl_FragColor = texture2D(src, vUv);
  }
`;class Fr{smoothMat;copyMat;scene;camera;quad;scratchA=null;scratchB=null;scratchW=0;scratchH=0;scratchColorSpace=null;constructor(){this.smoothMat=new Me({vertexShader:Pr,fragmentShader:Ar,uniforms:{paintTex:{value:null},islandMask:{value:null},texelSize:{value:new Ue(1,1)},method:{value:1},kernelRadius:{value:2},strength:{value:.6},spatialSigma:{value:1},colorSigma:{value:.15},edgeThreshold:{value:.05},useIslandGate:{value:1}},transparent:!1,blending:jt,depthTest:!1,depthWrite:!1}),this.copyMat=new Me({vertexShader:Ir,fragmentShader:Ur,uniforms:{src:{value:null}},transparent:!1,blending:jt,depthTest:!1,depthWrite:!1}),this.quad=new Ce(new mt(2,2),this.smoothMat),this.quad.frustumCulled=!1,this.scene=new Ke,this.scene.add(this.quad),this.camera=new ht(-1,1,1,-1,0,1)}run(s,d,o,v){this.ensureScratch(d);const R=this.scratchA,x=this.scratchB;if(!R||!x)return;const P=Math.max(1,Math.min(Zt,Math.round(v.kernelRadius))),p=Math.max(1,Math.round(v.iterations)),w=this.smoothMat.uniforms;w.texelSize.value.set(1/d.width,1/d.height),w.method.value=Br(v.method),w.kernelRadius.value=P,w.strength.value=v.strength,w.spatialSigma.value=Math.max(.5,P/2),w.colorSigma.value=Math.max(.001,v.colorSigma),w.edgeThreshold.value=Math.max(1e-4,v.edgeThreshold),w.useIslandGate.value=v.useIslandGate&&o?1:0,w.islandMask.value=o;const h=s.getRenderTarget(),C=s.autoClear;s.autoClear=!1,this.quad.material=this.smoothMat;let B=d.texture,L=R,A=R;for(let I=0;I<p;I++)w.paintTex.value=B,s.setRenderTarget(L),s.render(this.scene,this.camera),A=L,B=L.texture,L=L===R?x:R;this.quad.material=this.copyMat,this.copyMat.uniforms.src.value=A.texture,s.setRenderTarget(d),s.render(this.scene,this.camera),s.autoClear=C,s.setRenderTarget(h)}ensureScratch(s){const d=s.texture.colorSpace;if(this.scratchA&&this.scratchB&&this.scratchW===s.width&&this.scratchH===s.height&&this.scratchColorSpace===d)return;this.scratchA?.dispose(),this.scratchB?.dispose();const o=()=>{const v=new Ze(s.width,s.height,{format:ce,type:he,minFilter:J,magFilter:J,generateMipmaps:!1,depthBuffer:!1,stencilBuffer:!1});return v.texture.colorSpace=d,v};this.scratchA=o(),this.scratchB=o(),this.scratchW=s.width,this.scratchH=s.height,this.scratchColorSpace=d}dispose(){this.smoothMat.dispose(),this.copyMat.dispose(),this.quad.geometry.dispose(),this.scratchA?.dispose(),this.scratchB?.dispose(),this.scratchA=null,this.scratchB=null}}const Qt=[{id:M.Radius,label:"Radius",min:.1,max:4,step:.05,category:"Geometry",hint:"Multiplier on pointer-derived size."},{id:M.Hardness,label:"Hardness",min:0,max:1,step:.01,category:"Geometry",hint:"Edge falloff. 0=soft, 1=hard."},{id:M.Roundness,label:"Roundness",min:.05,max:1,step:.01,category:"Geometry",hint:"1=round, lower=ellipse along stroke."},{id:M.Angle,label:"Angle",min:0,max:1,step:.01,category:"Geometry",hint:"0..1 maps to 0..360° rotation."},{id:M.Spacing,label:"Spacing",min:.01,max:.5,step:.01,category:"Emission",hint:"Distance between stamps (fraction of radius)."},{id:M.Count,label:"Count",min:1,max:16,step:1,category:"Emission",hint:"Stamps per emission tick."},{id:M.Scatter,label:"Scatter",min:0,max:2,step:.05,category:"Emission",hint:"Random offset (× radius)."},{id:M.ScatterBothAxes,label:"Scatter Both Axes",min:0,max:1,step:1,category:"Emission",boolean:!0,hint:"Scatter along bitangent too."},{id:M.SizeJitter,label:"Size Jitter",min:0,max:1,step:.01,category:"Jitters"},{id:M.AngleJitter,label:"Angle Jitter",min:0,max:1,step:.01,category:"Jitters"},{id:M.RoundnessJitter,label:"Roundness Jitter",min:0,max:1,step:.01,category:"Jitters"},{id:M.OpacityJitter,label:"Opacity Jitter",min:0,max:1,step:.01,category:"Jitters"},{id:M.FlowJitter,label:"Flow Jitter",min:0,max:1,step:.01,category:"Jitters"},{id:M.Opacity,label:"Opacity",min:0,max:1,step:.01,category:"Color",hint:"Stroke alpha cap."},{id:M.Flow,label:"Flow",min:.01,max:1,step:.01,category:"Color",hint:"Per-stamp deposit. Low=buildup, high=solid."},{id:M.HueJitter,label:"Hue Jitter",min:0,max:.5,step:.01,category:"Color"},{id:M.SatJitter,label:"Sat Jitter",min:0,max:1,step:.01,category:"Color"},{id:M.ValJitter,label:"Val Jitter",min:0,max:1,step:.01,category:"Color"},{id:M.FgBgMix,label:"Fg ⇄ Bg Mix",min:0,max:1,step:.01,category:"Color",hint:"Lerp between fg and bg picker."},{id:M.GrainDepth,label:"Grain Depth",min:0,max:1,step:.01,category:"Texture"},{id:M.GrainScale,label:"Grain Scale",min:.1,max:5,step:.05,category:"Texture"},{id:M.ColorRate,label:"Color Rate",min:0,max:1,step:.01,category:"Wet",hint:"Foreground vs sampled-canvas per stamp."},{id:M.SmudgeLength,label:"Smudge Length",min:0,max:1,step:.01,category:"Wet",hint:"How much canvas color carries forward."},{id:M.SmudgeRadius,label:"Smudge Radius",min:0,max:1,step:.01,category:"Wet"},{id:M.WetMode,label:"Wet Mode",min:0,max:2,step:1,category:"Wet",enumValues:[{value:0,label:"Off"},{value:1,label:"Smear"},{value:2,label:"Dulling"}]},{id:M.TargetChannel,label:"Target Channels",min:1,max:63,step:1,category:"Surface",hint:"Bitmask: BaseColor=1, Normal=2, Roughness=4, Metallic=8, Height=16, Emissive=32."},{id:M.IslandLock,label:"Island Lock",min:0,max:1,step:1,category:"Surface",boolean:!0,hint:"Lock paint to the stroke's starting UV island."}],Dr=["Geometry","Emission","Jitters","Color","Texture","Wet","Surface"];new Map(Qt.map(r=>[r.id,r]));function Lr(){return Dr.map(r=>({category:r,settings:Qt.filter(s=>s.category===r)}))}const fa=[{url:"/brushes/tips/deevad-bristle-thin.png",label:"Bristle Thin",category:"Brushes"},{url:"/brushes/tips/deevad-bristle-mini.png",label:"Bristle Mini",category:"Brushes"},{url:"/brushes/tips/deevad-bristle-circle.png",label:"Bristle Circle",category:"Brushes"},{url:"/brushes/tips/deevad-flat-dirty.png",label:"Flat Dirty",category:"Brushes"},{url:"/brushes/tips/deevad-chisel.png",label:"Chisel Streaks",category:"Brushes"},{url:"/brushes/tips/deevad-chalk.png",label:"Chalk",category:"Chalk / Dry"},{url:"/brushes/tips/deevad-chalk-hard.png",label:"Chalk Hard",category:"Chalk / Dry"},{url:"/brushes/tips/deevad-chalk-sparse.png",label:"Chalk Sparse",category:"Chalk / Dry"},{url:"/brushes/tips/deevad-scribbles.png",label:"Scribbles",category:"Chalk / Dry"},{url:"/brushes/tips/deevad-square-rough.png",label:"Square Rough",category:"Chalk / Dry"},{url:"/brushes/tips/deevad-painterly.png",label:"Painterly",category:"Paint"},{url:"/brushes/tips/nylnook-paint.png",label:"Nylnook Paint",category:"Paint"},{url:"/brushes/tips/nylnook-paint-texture.png",label:"Nylnook Paint Texture",category:"Paint"},{url:"/brushes/tips/deevad-splat-dots.png",label:"Splat Dots",category:"Ink / Splatter"},{url:"/brushes/tips/nylnook-spray.png",label:"Spray",category:"Ink / Splatter"},{url:"/brushes/tips/deevad-snowman.png",label:"Snowman",category:"Texture / Effect"},{url:"/brushes/tips/nylnook-bear-paw.png",label:"Bear Paw",category:"Texture / Effect"},{url:"/brushes/tips/nylnook-iso-triangle.png",label:"Iso Triangle",category:"Texture / Effect"},{url:"/brushes/tips/nylnook-pointy-triangle.png",label:"Pointy Triangle",category:"Texture / Effect"}];new Map(fa.map(r=>[r.url,r]));function Hr(r,s){const d=M[s.id];return r.base?.[d]??0}function Nr(r,s,d){const o=M[s.id];r.base||(r.base={}),r.base[o]=d}function Gr({brushDef:r,revision:s,onMutated:d,onPasteJson:o,onResetToPreset:v,onClose:R}){const[x,P]=i.useState(new Set),[p,w]=i.useState(!1),[h,C]=i.useState(""),B=i.useMemo(()=>Lr(),[]),L=U=>{P(W=>{const Z=new Set(W);return Z.has(U)?Z.delete(U):Z.add(U),Z})},A=i.useCallback((U,W)=>{r&&(Nr(r,U,W),d())},[r,d]),I=i.useCallback(async()=>{if(r)try{await navigator.clipboard.writeText(JSON.stringify(r,null,2))}catch(U){console.warn("[BrushStudio] clipboard write failed",U)}},[r]),O=i.useCallback(()=>{h.trim()&&(o(h),C(""),w(!1))},[o,h]);return r?t.jsxs("div",{style:u.panel,children:[t.jsxs("div",{style:u.headerRow,children:[t.jsx("span",{style:u.heading,children:"Brush Studio"}),t.jsxs("div",{style:u.headerActions,children:[t.jsx("button",{style:u.smallButton,onClick:v,title:"Discard all Studio edits and reload the original preset",children:"Reset"}),t.jsx("button",{style:u.smallButton,onClick:I,title:"Copy this brush definition to clipboard as JSON",children:"Copy JSON"}),t.jsx("button",{style:u.smallButton,onClick:()=>w(U=>!U),title:"Paste a brush definition JSON from clipboard",children:"Paste"}),t.jsx("button",{style:u.closeButton,onClick:R,"aria-label":"Close studio",children:"×"})]})]}),p&&t.jsxs("div",{style:u.pasteRow,children:[t.jsx("textarea",{value:h,onChange:U=>C(U.target.value),placeholder:"Paste brush.json contents here",style:u.pasteArea,rows:4}),t.jsx("button",{style:u.applyButton,onClick:O,children:"Apply"})]}),t.jsxs("div",{style:u.brushIdRow,children:[t.jsx(Wr,{brushDef:r,onPickTip:U=>{U===""?delete r.tipAlphaUrl:U.startsWith("/")?r.tipAlphaUrl=U:r.tipAlphaUrl=`procedural:${U}`,d()}}),t.jsxs("div",{style:u.brushIdMeta,children:[t.jsx("span",{style:u.brushIdLabel,children:r.name}),t.jsxs("span",{style:u.brushIdId,children:["id: ",r.id]}),t.jsx("span",{style:u.brushIdCategory,children:r.category})]})]}),t.jsx(zr,{brushDef:r,onMutated:d}),t.jsx(Or,{brushDef:r,onMutated:d}),t.jsx(Er,{brushDef:r,onMutated:d}),t.jsx("div",{style:u.categoriesRow,children:B.map(({category:U,settings:W})=>{const Z=x.has(U);return t.jsxs("div",{style:u.categoryBlock,children:[t.jsxs("button",{style:u.categoryHeader,onClick:()=>L(U),children:[t.jsx("span",{style:u.categoryCaret,children:Z?"▸":"▾"}),t.jsx("span",{children:U})]}),!Z&&t.jsx("div",{style:u.categoryBody,children:W.map(E=>t.jsx(_r,{meta:E,brushDef:r,onChange:A},E.id))})]},U)})}),t.jsx(Vr,{brushDef:r})]}):t.jsxs("div",{style:u.panel,children:[t.jsxs("div",{style:u.headerRow,children:[t.jsx("span",{style:u.heading,children:"Brush Studio"}),t.jsx("button",{style:u.closeButton,onClick:R,"aria-label":"Close studio",children:"×"})]}),t.jsx("div",{style:u.empty,children:"No brush selected."})]})}function Wr({brushDef:r,onPickTip:s}){const d=i.useRef(null),o=r.tipAlphaUrl,v=o?.startsWith("procedural:")?o.slice(11):null,R=o&&!o.startsWith("procedural:")?o:null,x=R??v??"";i.useEffect(()=>{const p=d.current;if(!p)return;const w=p.getContext("2d");if(w){if(w.fillStyle="#0e0e0e",w.fillRect(0,0,p.width,p.height),!o){w.fillStyle="#3a3a3a",w.font="10px sans-serif",w.textAlign="center",w.textBaseline="middle",w.fillText("(round)",p.width/2,p.height/2);return}if(v){const h=Cr(v);if(!h)return;const C=document.createElement("canvas");C.width=h.size,C.height=h.size;const B=C.getContext("2d");if(!B)return;const L=B.createImageData(h.size,h.size);for(let A=0;A<h.size*h.size;A++){const I=h.buf[A*4];L.data[A*4]=255,L.data[A*4+1]=255,L.data[A*4+2]=255,L.data[A*4+3]=I}B.putImageData(L,0,0),w.imageSmoothingEnabled=!0,w.imageSmoothingQuality="high",w.drawImage(C,0,0,p.width,p.height)}else if(R){const h=new Image;h.crossOrigin="anonymous",h.onload=()=>{const C=d.current;if(!C)return;const B=C.getContext("2d");if(!B)return;B.fillStyle="#0e0e0e",B.fillRect(0,0,C.width,C.height);const L=h.width/h.height;let A=C.width,I=C.height;L>1?I=A/L:A=I*L;const O=(C.width-A)/2,U=(C.height-I)/2;B.imageSmoothingEnabled=!0,B.imageSmoothingQuality="high",B.drawImage(h,O,U,A,I)},h.src=R}}},[o,v,R]);const P=i.useMemo(()=>{const p=new Map;for(const w of fa){const h=p.get(w.category)??[];h.push(w),p.set(w.category,h)}return p},[]);return t.jsxs("div",{style:Yt.wrap,children:[t.jsx("canvas",{ref:d,width:72,height:72,style:Yt.canvas,title:o??"No tip — generated Gaussian"}),t.jsxs("select",{value:x,onChange:p=>s(p.target.value),style:Yt.select,children:[t.jsx("option",{value:"",children:"(no tip — round)"}),t.jsx("optgroup",{label:"Procedural",children:Rr.map(p=>t.jsx("option",{value:p,children:p},p))}),Array.from(P.entries()).map(([p,w])=>t.jsx("optgroup",{label:`Krita CC0 · ${p}`,children:w.map(h=>t.jsx("option",{value:h.url,children:h.label},h.url))},p))]})]})}const Yt={wrap:{display:"flex",flexDirection:"column",alignItems:"center",gap:4},canvas:{width:72,height:72,borderRadius:4,border:"1px solid rgba(255,255,255,0.08)",background:"#0e0e0e",imageRendering:"pixelated"},select:{width:100,background:"#2a2a2a",color:"#eaeaea",border:"1px solid rgba(255,255,255,0.08)",borderRadius:3,padding:"2px 4px",fontSize:9}};function zr({brushDef:r,onMutated:s}){const d=r.grainTextureUrl?.startsWith("procedural:")?r.grainTextureUrl.slice(11):"";return t.jsxs("div",{style:u.stencilBlock,children:[t.jsx("div",{style:u.stencilHeading,children:"Stencil"}),t.jsxs("div",{style:u.stencilGrid,children:[t.jsxs("label",{style:u.stencilRow,title:"Texture sampling filter for the tip image. Nearest = pixel-crisp, Linear = smooth.",children:[t.jsx("span",{style:u.stencilLabel,children:"Tip Filter"}),t.jsxs("select",{value:r.tipFilter??"nearest",onChange:o=>{r.tipFilter=o.target.value,s()},style:u.stencilSelect,children:[t.jsx("option",{value:"nearest",children:"Nearest"}),t.jsx("option",{value:"linear",children:"Linear"})]})]}),t.jsxs("label",{style:u.stencilRow,title:"Grain texture overlaid on each stamp. Static = anchored to surface (paper); follows-stroke = drags with brush (charcoal); follows-rotation = rotates with stamp angle.",children:[t.jsx("span",{style:u.stencilLabel,children:"Grain"}),t.jsxs("select",{value:d,onChange:o=>{o.target.value===""?delete r.grainTextureUrl:r.grainTextureUrl=`procedural:${o.target.value}`,s()},style:u.stencilSelect,children:[t.jsx("option",{value:"",children:"(none)"}),kr.map(o=>t.jsx("option",{value:o,children:o},o))]})]}),t.jsxs("label",{style:u.stencilRow,title:"How the grain pattern anchors. Static = sticks to surface (paper through pencil); follows-stroke = drags with brush (canvas drag); follows-rotation = rotates with stamp angle.",children:[t.jsx("span",{style:u.stencilLabel,children:"Grain Anchor"}),t.jsxs("select",{value:r.grainBehavior??"static",onChange:o=>{r.grainBehavior=o.target.value,s()},style:u.stencilSelect,children:[t.jsx("option",{value:"static",children:"Static (surface)"}),t.jsx("option",{value:"follows-stroke",children:"Follows stroke"}),t.jsx("option",{value:"follows-rotation",children:"Follows rotation"})]})]}),t.jsxs("label",{style:u.stencilRow,title:"How grain modulates the deposit. Multiply = darken under grain dips; Overlay = grain-dependent contrast; Screen = lighten.",children:[t.jsx("span",{style:u.stencilLabel,children:"Grain Blend"}),t.jsxs("select",{value:r.grainBlendMode??"multiply",onChange:o=>{r.grainBlendMode=o.target.value,s()},style:u.stencilSelect,children:[t.jsx("option",{value:"multiply",children:"Multiply"}),t.jsx("option",{value:"overlay",children:"Overlay"}),t.jsx("option",{value:"screen",children:"Screen"})]})]})]})]})}function Or({brushDef:r,onMutated:s}){const d=r.buildupMode,o=d==="continuous"||d==="additive"?"continuous":d==="per-stroke"?"per-stroke":"cap-forever",v=r.base??{},x=1-(v.ColorRate??1),P=v.SmudgeLength??0,p=v.SmudgeRadius??.5,w=v.WetMode??0,h=(C,B)=>{r.base={...r.base??{},[C]:B},s()};return t.jsxs("div",{style:u.stencilBlock,children:[t.jsx("div",{style:u.stencilHeading,children:"Buildup & Color Pickup"}),t.jsxs("div",{style:u.stencilGrid,children:[t.jsxs("label",{style:u.stencilRow,title:"Cap = plateau at brushOpacity forever. Continuous = press to build, alpha grows continuously within a stroke. Per Stroke = caps at brushOpacity within a stroke, but new strokes can layer on top (Procreate's default).",children:[t.jsx("span",{style:u.stencilLabel,children:"Buildup"}),t.jsxs("select",{value:o,onChange:C=>{r.buildupMode=C.target.value,s()},style:u.stencilSelect,children:[t.jsx("option",{value:"cap-forever",children:"Cap forever (plateau)"}),t.jsx("option",{value:"continuous",children:"Continuous (press to build)"}),t.jsx("option",{value:"per-stroke",children:"Per stroke (layer on release)"})]})]}),t.jsxs("label",{style:u.stencilRow,title:"Wet mode: Off = no canvas pickup. Smear = blends carried color with foreground per stamp. Dulling = stamp fills with canvas color, then tints with fg.",children:[t.jsx("span",{style:u.stencilLabel,children:"Pickup Mode"}),t.jsxs("select",{value:w,onChange:C=>h("WetMode",Number(C.target.value)),style:u.stencilSelect,children:[t.jsx("option",{value:0,children:"Off"}),t.jsx("option",{value:1,children:"Smear"}),t.jsx("option",{value:2,children:"Dulling"})]})]})]}),w>0&&t.jsxs("div",{style:u.pickupBlock,children:[t.jsx(Xt,{label:"Pickup",value:x,hint:"How much of the canvas color the brush picks up per stamp. 0 = pure fg. 1 = pure canvas (smudge tool feel).",onChange:C=>h("ColorRate",1-C)}),t.jsx(Xt,{label:"Carry",value:P,hint:"How long the picked-up color carries forward. 0 = sample fresh each stamp. 1 = retain initial sample indefinitely.",onChange:C=>h("SmudgeLength",C)}),t.jsx(Xt,{label:"Radius",value:p,hint:"Sample radius as fraction of brush radius.",onChange:C=>h("SmudgeRadius",C)})]})]})}function Xt({label:r,value:s,hint:d,onChange:o}){return t.jsxs("label",{style:u.settingRow,title:d,children:[t.jsx("span",{style:u.settingLabel,children:r}),t.jsx("input",{type:"range",min:0,max:1,step:.01,value:s,onChange:v=>o(Number(v.target.value)),style:u.settingSlider}),t.jsxs("span",{style:u.settingValue,children:[Math.round(s*100),"%"]})]})}function Er({brushDef:r,onMutated:s}){const d=[{id:M.Radius,label:"Size",min:-1,max:1},{id:M.Opacity,label:"Opacity",min:-1,max:1},{id:M.Flow,label:"Flow",min:-1,max:1},{id:M.Hardness,label:"Hardness",min:-1,max:1},{id:M.Roundness,label:"Roundness",min:-1,max:1},{id:M.Scatter,label:"Scatter",min:-2,max:2}],o=[{id:Vt.Velocity,label:"Velocity",hint:"How fast the brush moves between stamps. 0 = stationary, 1 = ~1.5× brushSize per emission."},{id:Vt.Pressure,label:"Pressure",hint:"Stylus pressure 0..1. Mouse falls back to 0.5 (slot reserved; tablet wiring TODO)."},{id:Vt.StrokeProgress,label:"Stroke Progress",hint:"Position along the stroke 0..1 (saturates at ~1.5s)."}],v=(x,P)=>{const p=r.mappings?.find(w=>w.settingId===x&&w.inputId===P);return!p||p.points.length<2?0:p.points[p.points.length-1].y},R=(x,P,p)=>{r.mappings||(r.mappings=[]);const w=r.mappings.findIndex(h=>h.settingId===x&&h.inputId===P);if(Math.abs(p)<.005)w>=0&&r.mappings.splice(w,1);else{const h={settingId:x,inputId:P,points:[{x:0,y:0},{x:1,y:p}]};w>=0?r.mappings[w]=h:r.mappings.push(h)}s()};return t.jsxs("div",{style:u.dynamicsBlock,children:[t.jsx("div",{style:u.dynamicsHeading,children:"Stamp Dynamics"}),t.jsxs("div",{style:u.dynamicsHint,children:["Each cell maps an input (rows) to a setting (columns). Drag right → setting ",t.jsx("em",{children:"grows"})," as input rises. Drag left → setting ",t.jsx("em",{children:"shrinks"}),". Center → no mapping."]}),t.jsxs("table",{style:u.dynamicsTable,children:[t.jsx("thead",{children:t.jsxs("tr",{children:[t.jsx("th",{style:u.dynamicsCornerCell}),d.map(x=>t.jsx("th",{style:u.dynamicsHeaderCell,children:x.label},x.id))]})}),t.jsx("tbody",{children:o.map(x=>t.jsxs("tr",{children:[t.jsx("th",{style:u.dynamicsRowHeader,title:x.hint,children:x.label}),d.map(P=>{const p=v(P.id,x.id);return t.jsxs("td",{style:u.dynamicsCell,children:[t.jsx("input",{type:"range",min:P.min,max:P.max,step:.05,value:p,onChange:w=>R(P.id,x.id,Number(w.target.value)),style:u.dynamicsSlider,title:`${x.label} → ${P.label}: ${p.toFixed(2)}`}),t.jsx("div",{style:u.dynamicsValue,children:p>0?`+${p.toFixed(2)}`:p.toFixed(2)})]},P.id)})]},x.id))})]})]})}function _r({meta:r,brushDef:s,onChange:d}){const o=Hr(s,r),v=$r(r,o),R=!!r.enumValues;return t.jsxs("label",{style:u.settingRow,title:r.hint,children:[t.jsx("span",{style:u.settingLabel,children:r.label}),R?t.jsx("select",{value:o,onChange:x=>d(r,Number(x.target.value)),style:u.settingSelect,children:r.enumValues.map(x=>t.jsx("option",{value:x.value,children:x.label},x.value))}):r.boolean?t.jsx("input",{type:"checkbox",checked:o>=.5,onChange:x=>d(r,x.target.checked?1:0)}):t.jsx("input",{type:"range",min:r.min,max:r.max,step:r.step,value:o,onChange:x=>d(r,Number(x.target.value)),style:u.settingSlider}),t.jsx("span",{style:u.settingValue,children:v})]})}function Vr({brushDef:r}){const s=r.mappings??[];return s.length===0?null:t.jsxs("div",{style:u.mappingsBlock,children:[t.jsxs("div",{style:u.mappingsHeading,children:["Active Mappings ",t.jsx("span",{style:u.dim,children:"(read-only — curve editor coming in Studio v2)"})]}),s.map((d,o)=>t.jsxs("div",{style:u.mappingRow,children:[t.jsxs("span",{style:u.mappingName,children:[Yr(d.settingId)," ",t.jsx("span",{style:u.dim,children:"←"})," ",Xr(d.inputId)]}),t.jsxs("span",{style:u.mappingPoints,children:[d.points.length," pts"]})]},o))]})}function Yr(r){return Qt.find(s=>s.id===r)?.label??`Setting ${r}`}function Xr(r){return["Pressure","Velocity","Tilt Decl","Tilt Az","Random","Stroke Prog","Direction","Barrel Roll","Custom","Surf Tangent","Normal·View","Curvature","UV Texel","Island Id","Mat Channel"][r]??`Input ${r}`}function $r(r,s){return r.enumValues?r.enumValues.find(d=>d.value===s)?.label??String(s):r.boolean?s>=.5?"On":"Off":r.step>=1?Math.round(s).toString():r.max<=1?`${Math.round(s*100)}%`:s.toFixed(2)}const u={panel:{background:"#1f1f1f",color:"#eaeaea",padding:"8px 14px 12px",fontSize:11,flex:1,minHeight:0,overflowY:"auto",overflowX:"hidden"},headerRow:{display:"flex",alignItems:"center",justifyContent:"space-between",gap:8,marginBottom:8},heading:{fontWeight:600,fontSize:12,letterSpacing:.4,color:"#dadada"},headerActions:{display:"flex",gap:6,alignItems:"center"},smallButton:{padding:"3px 10px",background:"#3a3a3a",color:"#eaeaea",border:"1px solid rgba(255,255,255,0.08)",borderRadius:3,fontSize:10,cursor:"pointer"},closeButton:{width:22,height:22,padding:0,background:"transparent",color:"#979797",border:"1px solid rgba(255,255,255,0.08)",borderRadius:3,fontSize:14,cursor:"pointer"},pasteRow:{display:"flex",gap:6,marginBottom:8},pasteArea:{flex:1,background:"#161616",color:"#dadada",border:"1px solid rgba(255,255,255,0.08)",borderRadius:3,padding:"6px 8px",fontFamily:"ui-monospace, SFMono-Regular, Menlo, monospace",fontSize:10,resize:"vertical"},applyButton:{padding:"6px 12px",background:"#4a6a4a",color:"#eaeaea",border:"1px solid rgba(255,255,255,0.16)",borderRadius:3,fontSize:11,cursor:"pointer",alignSelf:"flex-start"},brushIdRow:{display:"flex",alignItems:"center",gap:14,marginBottom:8,paddingBottom:6,borderBottom:"1px dashed rgba(255,255,255,0.06)"},brushIdMeta:{display:"flex",flexDirection:"column",alignItems:"flex-start",gap:4},brushIdLabel:{fontWeight:600,color:"#eaeaea"},brushIdId:{fontFamily:"ui-monospace, SFMono-Regular, Menlo, monospace",fontSize:10,color:"#979797"},brushIdCategory:{fontSize:10,padding:"1px 6px",background:"rgba(255,255,255,0.06)",borderRadius:2,color:"#cdcdcd"},stencilBlock:{marginBottom:10,padding:"8px 10px",background:"#181818",borderRadius:4,border:"1px solid rgba(255,255,255,0.05)"},stencilHeading:{fontSize:11,fontWeight:600,color:"#dadada",marginBottom:6},stencilGrid:{display:"grid",gridTemplateColumns:"1fr 1fr",gap:"4px 10px"},stencilRow:{display:"grid",gridTemplateColumns:"70px 1fr",alignItems:"center",gap:6},stencilLabel:{fontSize:10,color:"#cdcdcd"},stencilSelect:{background:"#2a2a2a",color:"#eaeaea",border:"1px solid rgba(255,255,255,0.08)",borderRadius:3,padding:"2px 4px",fontSize:10,width:"100%"},pickupBlock:{marginTop:6,paddingTop:6,borderTop:"1px dashed rgba(255,255,255,0.05)",display:"flex",flexDirection:"column",gap:3},dynamicsBlock:{marginBottom:10,padding:"8px 10px",background:"#181818",borderRadius:4,border:"1px solid rgba(255,255,255,0.05)"},dynamicsHeading:{fontSize:11,fontWeight:600,color:"#dadada",marginBottom:4},dynamicsHint:{fontSize:9,color:"#888",marginBottom:8,lineHeight:1.4},dynamicsTable:{width:"100%",borderCollapse:"collapse"},dynamicsCornerCell:{width:80},dynamicsHeaderCell:{fontSize:9,fontWeight:500,color:"#979797",padding:"2px 4px",textAlign:"center"},dynamicsRowHeader:{fontSize:10,color:"#cdcdcd",textAlign:"left",padding:"6px 4px",fontWeight:500,whiteSpace:"nowrap"},dynamicsCell:{padding:"2px 3px",textAlign:"center"},dynamicsSlider:{width:"100%",cursor:"ew-resize"},dynamicsValue:{fontSize:8,color:"#777",fontFamily:"ui-monospace, SFMono-Regular, Menlo, monospace",marginTop:1},categoriesRow:{display:"grid",gridTemplateColumns:"1fr",gap:8},categoryBlock:{background:"#181818",border:"1px solid rgba(255,255,255,0.05)",borderRadius:4,overflow:"hidden"},categoryHeader:{display:"flex",width:"100%",alignItems:"center",gap:6,padding:"5px 8px",background:"#222",color:"#cdcdcd",fontSize:11,fontWeight:600,border:0,cursor:"pointer",textAlign:"left"},categoryCaret:{width:12,color:"#979797",fontSize:10},categoryBody:{padding:"6px 8px",display:"flex",flexDirection:"column",gap:4},settingRow:{display:"grid",gridTemplateColumns:"85px 1fr 38px",alignItems:"center",gap:6},settingLabel:{color:"#cdcdcd",fontSize:10},settingSlider:{width:"100%"},settingValue:{textAlign:"right",fontFamily:"ui-monospace, SFMono-Regular, Menlo, monospace",fontSize:10,color:"#979797"},settingSelect:{background:"#2a2a2a",color:"#eaeaea",border:"1px solid rgba(255,255,255,0.08)",borderRadius:3,padding:"2px 4px",fontSize:10},mappingsBlock:{marginTop:10,padding:"8px 10px",background:"#161616",borderRadius:4,border:"1px solid rgba(255,255,255,0.04)"},mappingsHeading:{fontSize:11,fontWeight:600,color:"#dadada",marginBottom:6},mappingRow:{display:"flex",justifyContent:"space-between",gap:8,padding:"2px 0",fontSize:10},mappingName:{color:"#cdcdcd"},mappingPoints:{color:"#979797",fontFamily:"ui-monospace, SFMono-Regular, Menlo, monospace"},dim:{color:"#777"},empty:{color:"#979797",padding:"10px 0",fontStyle:"italic"}};Ce.prototype.raycast=vr;Bt.prototype.computeBoundsTree=br;Bt.prototype.disposeBoundsTree=xr;const z=2048;function Jr(r=256){const s=new Uint8Array(r*r*4),d=(r-1)*.5,o=-.45,v=.6,R=.65,x=Math.hypot(o,v,R),P=o/x,p=v/x,w=R/x;for(let C=0;C<r;C++)for(let B=0;B<r;B++){const L=(C*r+B)*4,A=(B-d)/d,I=(C-d)/d,O=A*A+I*I;let U;if(O>=1)U=.5;else{const Z=Math.sqrt(1-O),E=Math.max(0,A*P+I*p+Z*w),ge=.35,oe=.5*E,be=Math.pow(E,28)*.55;U=Math.min(1,ge+oe+be)}const W=Math.round(U*255);s[L]=W,s[L+1]=W,s[L+2]=W,s[L+3]=255}const h=new Fe(s,r,r,ce,he);return h.minFilter=et,h.magFilter=et,h.needsUpdate=!0,h}function qr(r=256){const s=new Uint8Array(r*r*4),d=(r-1)*.5;for(let v=0;v<r;v++)for(let R=0;R<r;R++){const x=(v*r+R)*4,P=(R-d)/d,p=(v-d)/d,w=P*P+p*p;let h;if(w>=1)h=0;else{const B=Math.max(0,p),L=Math.max(0,-p);h=.55*B+.05*L+.08;const A=Math.hypot(P-.35,p-.55),I=Math.hypot(P+.45,p-.35);h+=Math.max(0,1-A*5.5)*1.2,h+=Math.max(0,1-I*7)*.9,h+=Math.max(0,1-Math.abs(p)*16)*.18,h=Math.min(1,h)}const C=Math.round(h*255);s[x]=C,s[x+1]=C,s[x+2]=C,s[x+3]=255}const o=new Fe(s,r,r,ce,he);return o.minFilter=et,o.magFilter=et,o.needsUpdate=!0,o}const Kr=`
  varying vec2 vUv;
  varying vec3 vViewNormal;
  void main() {
    vUv = uv;
    vViewNormal = normalize(normalMatrix * normal);
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  }
`,Zr=`
  uniform sampler2D baseColorMap;
  uniform vec3 baseColorTint;
  uniform sampler2D metalnessMap;
  uniform sampler2D roughnessMap;
  uniform sampler2D paintTex;
  uniform sampler2D islandMask;
  uniform vec2 paintTexelSize;
  uniform float useIslandGate;
  uniform int viewMode;
  uniform sampler2D matcapTex;
  uniform float matcapIntensity;
  uniform sampler2D reflectionMatcap;
  uniform float reflectionIntensity;
  uniform float isDimmed;
  varying vec2 vUv;
  varying vec3 vViewNormal;

  bool sameIsland(vec4 a, vec4 b) {
    if (a.a < 0.5 || b.a < 0.5) return false;
    return abs(a.r - b.r) < (0.5 / 255.0) && abs(a.g - b.g) < (0.5 / 255.0);
  }

  vec4 sampleIslandGated(vec2 uv) {
    // Pixel-center coordinate (texel grid offset by 0.5)
    vec2 pix = uv / paintTexelSize - 0.5;
    vec2 base = floor(pix);
    vec2 f = fract(pix);

    vec2 uv00 = (base + vec2(0.5, 0.5)) * paintTexelSize;
    vec2 uv10 = (base + vec2(1.5, 0.5)) * paintTexelSize;
    vec2 uv01 = (base + vec2(0.5, 1.5)) * paintTexelSize;
    vec2 uv11 = (base + vec2(1.5, 1.5)) * paintTexelSize;

    vec4 myId = texture2D(islandMask, uv);

    vec4 t00 = texture2D(paintTex, uv00);
    vec4 t10 = texture2D(paintTex, uv10);
    vec4 t01 = texture2D(paintTex, uv01);
    vec4 t11 = texture2D(paintTex, uv11);

    vec4 m00 = texture2D(islandMask, uv00);
    vec4 m10 = texture2D(islandMask, uv10);
    vec4 m01 = texture2D(islandMask, uv01);
    vec4 m11 = texture2D(islandMask, uv11);

    // A tap contributes only if (a) it's in the same UV island as the
    // fragment AND (b) it has actually been painted. Same-island gutter
    // texels (claimed by Voronoi extension but never written by the brush)
    // would otherwise pull a 0-alpha sample into the bilinear average and
    // wash the painted edge toward base color — that's the residual hairline.
    float w00 = (sameIsland(myId, m00) && t00.a > 0.001) ? 1.0 : 0.0;
    float w10 = (sameIsland(myId, m10) && t10.a > 0.001) ? 1.0 : 0.0;
    float w01 = (sameIsland(myId, m01) && t01.a > 0.001) ? 1.0 : 0.0;
    float w11 = (sameIsland(myId, m11) && t11.a > 0.001) ? 1.0 : 0.0;

    float bw00 = (1.0 - f.x) * (1.0 - f.y) * w00;
    float bw10 = f.x * (1.0 - f.y) * w10;
    float bw01 = (1.0 - f.x) * f.y * w01;
    float bw11 = f.x * f.y * w11;

    float total = bw00 + bw10 + bw01 + bw11;
    if (total < 1e-5) {
      // No same-island taps — fragment sits in a corner of the texel grid
      // where every neighbor is in a different island. Fall back to nearest
      // (no bilinear bleed possible). This is rare in practice.
      return texture2D(paintTex, uv);
    }
    return (t00 * bw00 + t10 * bw10 + t01 * bw01 + t11 * bw11) / total;
  }

  vec3 sampleBaseChannel(vec2 uv) {
    // viewMode: 0=color, 1=metalness, 2=roughness. glTF packs metalness in B
    // and roughness in G of the metallicRoughness texture; we sample those
    // channels directly. M/R maps are linear (NoColorSpace) so the values
    // come through unmodified — colorspace_fragment will encode the output
    // for sRGB display.
    if (viewMode == 1) return vec3(texture2D(metalnessMap, uv).b);
    if (viewMode == 2) return vec3(texture2D(roughnessMap, uv).g);
    return baseColorTint * texture2D(baseColorMap, uv).rgb;
  }

  void main() {
    vec3 base = sampleBaseChannel(vUv);
    vec4 paint;
    if (useIslandGate > 0.5) {
      paint = sampleIslandGated(vUv);
    } else {
      paint = texture2D(paintTex, vUv);
    }
    vec3 final = mix(base, paint.rgb, paint.a);
    // Matcap modulation only on Color view; debug views (metal/rough) stay
    // flat unlit so the raw channel values remain readable. matcap is keyed
    // by the view-space normal — direction-only shading, no metalness or
    // roughness gating that could attenuate paint visibility.
    if (viewMode == 0) {
      vec3 nrm = normalize(vViewNormal);
      vec2 mUv = nrm.xy * 0.5 + 0.5;
      if (matcapIntensity > 0.0) {
        vec3 matcapColor = texture2D(matcapTex, mUv).rgb;
        // matcap mean is ~0.5; *2.0 keeps brightness ~unchanged at intensity=1.
        final *= mix(vec3(1.0), matcapColor * 2.0, matcapIntensity);
      }
      // Fake reflection layer. Sample a stylized "studio" matcap at the
      // same view-normal UV, weighted by a fresnel proxy (1 - |n.z|, peaks
      // at glancing angles where real specular reflection is strongest).
      // Added (not multiplied) so painted color is preserved underneath.
      if (reflectionIntensity > 0.0) {
        vec3 reflColor = texture2D(reflectionMatcap, mUv).rgb;
        float fresnel = pow(1.0 - abs(nrm.z), 3.0);
        final += reflColor * fresnel * reflectionIntensity;
      }
    }
    // Dim post-process for non-active materials. FULLY desaturate to luma
    // then darken hard — recognizably "disabled" state. Mixing partway to
    // grey can BRIGHTEN dark channels (a 0.2 blue mixed toward 0.85 grey
    // ends up brighter than it started), so we go all the way to grey
    // before scaling. 0.3 multiplier picks a value that's clearly darker
    // than any unlit surface but still visible enough to read shape.
    if (isDimmed > 0.5) {
      float luma = dot(final, vec3(0.299, 0.587, 0.114));
      final = vec3(luma) * 0.3;
    }
    gl_FragColor = vec4(final, 1.0);
    #include <colorspace_fragment>
  }
`,Qr=`
  varying vec3 vWorldPos;
  varying vec3 vWorldNormal;
  varying vec2 vBrushUv;
  void main() {
    vBrushUv = uv;
    vWorldPos = (modelMatrix * vec4(position, 1.0)).xyz;
    // Treat normal as a direction (w=0 ignores translation). Assumes
    // roughly uniform scale on the mesh — for skewed transforms a proper
    // normal matrix would be needed.
    vWorldNormal = normalize((modelMatrix * vec4(normal, 0.0)).xyz);
    gl_Position = vec4(uv * 2.0 - 1.0, 0.5, 1.0);
  }
`,en=`
  uniform vec3 brushColor;
  uniform vec3 brushCenter;
  uniform vec3 brushNormal;       // surface normal at hit (world space)
  uniform vec3 brushTangent;      // major-axis direction in tangent plane
  uniform float brushRadius;      // major-axis radius (world units)
  uniform float brushAspect;      // minor / major (1 = circle, <1 = ellipse)
  uniform float edgeSnapThreshold;
  uniform float brushHardness;    // 0 = soft (smoothstep across full radius), 1 = hard cutoff
  uniform float brushOpacity;     // alpha cap — strokes accumulate up to this value
  uniform float brushFlow;        // per-stamp alpha contribution (0..1)
  uniform mat4 viewProjMatrix;    // camera view-projection (for occlusion test)
  uniform sampler2D sceneDepthTex; // depth pre-pass output
  uniform float occlusionEnabled; // 1 = enabled, 0 = disabled
  uniform float cameraNear;       // for linearizing depth before comparison
  uniform float cameraFar;
  uniform sampler2D existingPaint;
  // ── Phase 3: bitmap tip alpha ──
  uniform sampler2D tipAlpha;     // greyscale alpha image (R channel used)
  uniform float useTipAlpha;      // 0 = generated round, 1 = bitmap tip
  // ── Phase 3: grain texture channel ──
  uniform sampler2D grainTex;     // greyscale grain (R channel)
  uniform float useGrain;         // 0 = off, 1 = on
  uniform float grainDepth;       // 0..1 — modulation strength
  uniform float grainScale;       // UV scale multiplier (1 = stamp-sized)
  uniform int grainBlendMode;     // 0=multiply, 1=overlay, 2=screen
  uniform vec2 grainOffset;       // per-stamp UV offset (for follows-stroke / follows-rotation)
  // ── Phase 8.6/8.7: buildup mode ──
  uniform int buildupMode;             // 0 = cap-forever, 1 = continuous, 2 = per-stroke
  uniform sampler2D strokeStartPaint;  // snapshot of paintRT at pointer-down (per-stroke mode only)
  varying vec3 vWorldPos;
  varying vec3 vWorldNormal;
  varying vec2 vBrushUv;
  void main() {
    vec3 toFrag = vWorldPos - brushCenter;
    // Strict 3D sphere test — FIRST line of defense. Fragments outside
    // the brush radius in 3D world space are dropped regardless of how
    // their tangent-plane projection lands. Without this, the projection
    // below collapses the depth axis (the one parallel to brushNormal),
    // so a fragment far from the brush in 3D but aligned with the brush
    // axis — e.g. the back of a head when painting on the nose, or the
    // far side of the model at a silhouette edge — projects to
    // (u, v) ≈ (0, 0) and falsely paints. Production paint-stroke.ts
    // sidesteps this by using pure 3D distance; PaintLab keeps the
    // tangent-plane code below ONLY for elliptical / tip-UV math.
    if (length(toFrag) > brushRadius) discard;
    // Tangent-plane projection. The major axis is brushTangent (world);
    // the minor axis is the binormal (cross of normal and tangent). u,v
    // are signed coords along major/minor in world units.
    vec3 onPlane = toFrag - brushNormal * dot(toFrag, brushNormal);
    vec3 binormal = cross(brushNormal, brushTangent);
    float u = dot(onPlane, brushTangent);
    float v = dot(onPlane, binormal);
    float ru = brushRadius;
    float rv = brushRadius * max(0.05, brushAspect);
    // Normalized elliptical distance from center: 1 = on the boundary.
    float t = sqrt((u * u) / (ru * ru) + (v * v) / (rv * rv));
    if (t > 1.0) discard;
    // Occlusion test: project this fragment to NDC and compare against
    // the scene's depth pre-pass. If something visible is closer to the
    // camera at this screen position, this fragment is hidden — drop it.
    // Comparison is in LINEAR depth (world units along view direction),
    // not perspective-warped NDC depth. Perspective depth has wildly
    // non-uniform precision (most concentrated near the camera), so a
    // fixed NDC epsilon either fails on thin models far away or causes
    // self-shadowing on close geometry. Linear depth gives constant
    // world-unit precision regardless of distance.
    if (occlusionEnabled > 0.5) {
      vec4 clipPos = viewProjMatrix * vec4(vWorldPos, 1.0);
      if (clipPos.w > 0.0) {
        vec3 ndc = clipPos.xyz / clipPos.w;
        if (ndc.x >= -1.0 && ndc.x <= 1.0 && ndc.y >= -1.0 && ndc.y <= 1.0) {
          vec2 sUv = ndc.xy * 0.5 + 0.5;
          // Linearize: ndc.z in [-1, 1] → world distance from camera.
          float zNdc = ndc.z;
          float thisLinear = (2.0 * cameraNear * cameraFar)
            / (cameraFar + cameraNear - zNdc * (cameraFar - cameraNear));
          float sampledZ01 = texture2D(sceneDepthTex, sUv).r;
          float sampledNdc = sampledZ01 * 2.0 - 1.0;
          float sceneLinear = (2.0 * cameraNear * cameraFar)
            / (cameraFar + cameraNear - sampledNdc * (cameraFar - cameraNear));
          // Bias scaled with distance: 0.5% of distance + small floor.
          // Catches anything thicker than ~0.5% of its distance from
          // camera while tolerating FP noise on the front surface.
          float bias = max(0.002, thisLinear * 0.005);
          if (thisLinear > sceneLinear + bias) discard;
        }
      }
    }
    // Edge snap: discard fragments whose surface normal points too far
    // from the hit triangle's normal. A 0 threshold disables the gate.
    if (edgeSnapThreshold > 0.001) {
      float ndot = dot(normalize(vWorldNormal), normalize(brushNormal));
      if (ndot < edgeSnapThreshold) discard;
    }
    // Falloff — the per-fragment alpha contribution before flow/jitter.
    // When a bitmap tip is set, the tip image is authoritative: the
    // brush silhouette comes from it. Hardness then only adds optional
    // extra rolloff at the perimeter (so low-hardness can still soften
    // a hard tip if needed). Without a tip, fall back to the procedural
    // hardness-driven smoothstep Gaussian.
    float falloff;
    if (useTipAlpha > 0.5) {
      vec2 tipUv = vec2(u / ru, v / rv) * 0.5 + 0.5;
      float tipSample = texture2D(tipAlpha, tipUv).r;
      // Extra rolloff only activates at low hardness (< 1.0). At hardness=1.0
      // the tip alpha is used raw. At hardness=0.0 a wide smoothstep multiplies
      // the tip down, producing the "soft tip" feel.
      float edgeRolloff = mix(1.0 - smoothstep(0.6, 1.0, t), 1.0, brushHardness);
      falloff = tipSample * edgeRolloff;
    } else {
      falloff = 1.0 - smoothstep(brushHardness, 1.0, t);
    }
    float contribution = falloff * brushFlow;
    // Phase 3 — grain modulation. When useGrain=1, sample the grain
    // texture and apply per blend-mode to the deposit. grainOffset is
    // the per-stamp UV anchor (set by the caller per brush.grainBehavior).
    if (useGrain > 0.5) {
      vec2 grainUv = grainOffset + vec2(u, v) * grainScale / max(1e-4, brushRadius);
      float g = texture2D(grainTex, grainUv).r;
      float modulated;
      if (grainBlendMode == 1) {
        // Overlay
        modulated = g < 0.5 ? (2.0 * g * 1.0) : (1.0 - 2.0 * (1.0 - g) * 0.0);
      } else if (grainBlendMode == 2) {
        // Screen — 1 - (1 - 1)(1 - g) = g (no change). Use mix instead.
        modulated = g;
      } else {
        // Multiply
        modulated = g;
      }
      contribution *= mix(1.0, modulated, grainDepth);
    }
    vec4 prev = texture2D(existingPaint, vBrushUv);
    // Three alpha buildup modes:
    //  0 = cap-forever: asymptotic toward brushOpacity, never grows past
    //      it even across multiple strokes. Soft media that should plateau.
    //  1 = continuous: each stamp adds contribution × brushOpacity, clamped
    //      at 1.0. Alpha grows continuously within and across strokes.
    //  2 = per-stroke: caps within a single stroke at strokeStart.a +
    //      brushOpacity. New strokes start fresh — each can push pixels up
    //      by AT MOST brushOpacity above where they began. Layered media.
    float depositAlpha;
    float newAlpha;
    if (buildupMode == 1) {
      depositAlpha = contribution * brushOpacity;
      newAlpha = min(1.0, prev.a + depositAlpha);
    } else if (buildupMode == 2) {
      float startA = texture2D(strokeStartPaint, vBrushUv).a;
      float effectiveCap = min(1.0, startA + brushOpacity);
      float headroom = max(0.0, effectiveCap - prev.a);
      depositAlpha = contribution * headroom;
      newAlpha = prev.a + depositAlpha;
    } else {
      float headroom = max(0.0, brushOpacity - prev.a);
      depositAlpha = contribution * headroom;
      newAlpha = prev.a + depositAlpha;
    }
    // Color uses the RAW contribution as a lerp factor decoupled from
    // depositAlpha. This is the fix for the "new strokes go under" bug:
    // the previous Porter-Duff form weighted brushColor by depositAlpha,
    // so when alpha was already at the cap (headroom=0, depositAlpha=0)
    // a new color stroke deposited zero color and the old paint dominated.
    // Now a hard-flow stamp at center (contribution=1) fully replaces
    // prev.rgb regardless of alpha state. Partial-contribution stamps
    // (low flow / soft edges) blend gradually toward brushColor.
    //
    // First-stamp darkening avoidance: when prev.a is essentially 0,
    // prev.rgb is meaningless storage (the paint has no color), so we
    // substitute brushColor in its place. mix(brushColor, brushColor, x)
    // = brushColor for any x, so a soft-edge stamp on empty paint
    // still stores pure brushColor (only alpha controls visibility).
    vec3 prevColor = prev.a > 0.005 ? prev.rgb : brushColor;
    vec3 newColor = mix(prevColor, brushColor, contribution);
    gl_FragColor = vec4(newColor, newAlpha);
  }
`,va=20,tn=`
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = vec4(position.xy, 0.0, 1.0);
  }
`,an=`
  uniform sampler2D src;
  varying vec2 vUv;
  void main() {
    gl_FragColor = texture2D(src, vUv);
  }
`,Kt=`
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = vec4(position.xy, 0.0, 1.0);
  }
`,rn=`
  uniform sampler2D map;
  uniform sampler2D metalnessMap;
  uniform sampler2D roughnessMap;
  uniform vec3 tint;
  uniform int viewMode;
  varying vec2 vUv;
  void main() {
    vec3 c;
    if (viewMode == 1) {
      c = vec3(texture2D(metalnessMap, vUv).b);
    } else if (viewMode == 2) {
      c = vec3(texture2D(roughnessMap, vUv).g);
    } else {
      c = tint * texture2D(map, vUv).rgb;
    }
    gl_FragColor = vec4(c, 1.0);
    #include <colorspace_fragment>
  }
`,nn=`
  uniform sampler2D map;
  varying vec2 vUv;
  void main() {
    vec4 p = texture2D(map, vUv);
    if (p.a < 0.001) discard;
    gl_FragColor = p;
    #include <colorspace_fragment>
  }
`;function Tt(r,s,d){r.undoCopyMat.uniforms.src.value=s;const o=r.renderer.getRenderTarget();r.renderer.setRenderTarget(d),r.renderer.render(r.undoCopyScene,r.undoCopyCam),r.renderer.setRenderTarget(o)}function sn(r,s){const d=r.renderer.getRenderTarget(),o=r.renderer.getClearColor(new Qe),v=r.renderer.getClearAlpha();r.renderer.setRenderTarget(s),r.renderer.setClearColor(0,0),r.renderer.clear(),r.renderer.setClearColor(o,v),r.renderer.setRenderTarget(d)}function $t(r,s){const d=r.renderer.getRenderTarget();r.renderer.setRenderTarget(s.paintRT),r.renderer.copyFramebufferToTexture(s.paintTexCopy),r.renderer.setRenderTarget(d)}function on(r,s){if(s.undoHead<s.undoHistory.length-1){for(let o=s.undoHead+1;o<s.undoHistory.length;o++)s.undoHistory[o].dispose();s.undoHistory.length=s.undoHead+1}s.undoHistory.length>=va&&(s.undoHistory.shift()?.dispose(),s.undoHead>=0&&(s.undoHead-=1));const d=new Ze(z,z,{format:ce,type:he,minFilter:J,magFilter:J,generateMipmaps:!1,depthBuffer:!1,stencilBuffer:!1});Tt(r,s.paintRT.texture,d),s.undoHistory.push(d),s.undoHead=s.undoHistory.length-1}function ln(r,s){const d=new Bt;for(const p of Object.keys(r.attributes))d.setAttribute(p,r.getAttribute(p));const o=r.getIndex();if(!o)return d;const v=r.groups;if(!v||v.length===0)return d.setIndex(o),d;const R=v.find(p=>p.materialIndex===s);if(!R)return d.setIndex(new qt(new Uint32Array(0),1)),d;const x=o.array,P=new Uint32Array(R.count);for(let p=0;p<R.count;p++)P[p]=x[R.start+p];return d.setIndex(new qt(P,1)),d}function ua(){const r=new Fe(new Uint8Array([0,0,0,255]),1,1,ce,he);return r.needsUpdate=!0,r}function pa(r){const s=r?.image;return!!r&&!!s&&(s.width??0)>0&&(s.height??0)>0}function ha(r){r.compositeMat.dispose(),r.paintRT.dispose(),r.paintTexCopy.dispose(),r.strokeStartPaintTex.dispose(),r.whiteFallback?.dispose(),r.metalnessFallback?.dispose(),r.roughnessFallback?.dispose(),r.islandMask?.dispose(),r.islandGutterMaskFallback?.dispose();for(const s of r.undoHistory)s.dispose();r.undoHistory.length=0,r.undoHead=-1;for(const s of r.targets){for(const d of Object.keys(s.subGeometry.attributes))s.subGeometry.deleteAttribute(d);s.subGeometry.setIndex(null),s.subGeometry.dispose()}}function ma(r,s,d){const o=s.targets[0];if(!o)return;const v=o.mesh;r.uvBgMat?.dispose(),r.uvBgMat=null,r.uvPaintMat?.dispose(),r.uvPaintMat=null,r.uvBgMesh&&(r.uvScene.remove(r.uvBgMesh),r.uvBgMesh.geometry.dispose(),r.uvBgMesh=null),r.uvPaintMesh&&(r.uvScene.remove(r.uvPaintMesh),r.uvPaintMesh.geometry.dispose(),r.uvPaintMesh=null),r.uvWireframe&&(r.uvScene.remove(r.uvWireframe),r.uvWireframe.geometry.dispose(),r.uvWireframe.material.dispose(),r.uvWireframe=null);const R=new Me({vertexShader:Kt,fragmentShader:rn,uniforms:{map:{value:s.baseMap},metalnessMap:{value:s.compositeMat.uniforms.metalnessMap.value},roughnessMap:{value:s.compositeMat.uniforms.roughnessMap.value},tint:{value:s.compositeMat.uniforms.baseColorTint.value},viewMode:{value:d}},depthTest:!1,depthWrite:!1}),x=new Ce(new mt(2,2),R);x.renderOrder=0,r.uvScene.add(x);const P=new Me({vertexShader:Kt,fragmentShader:nn,uniforms:{map:{value:s.paintTexCopy}},transparent:!0,depthTest:!1,depthWrite:!1}),p=new Ce(new mt(2,2),P);p.renderOrder=1,r.uvScene.add(p);const w=o.subGeometry.getIndex(),h=v.geometry.getAttribute("uv");if(w&&h){const C=w.array,B=h.array,L=C.length/3,A=new Float32Array(L*3*2*3);let I=0;const O=(E,ge)=>{const oe=B[E*2]*2-1,be=B[E*2+1]*2-1,De=B[ge*2]*2-1,Pt=B[ge*2+1]*2-1;A[I++]=oe,A[I++]=be,A[I++]=0,A[I++]=De,A[I++]=Pt,A[I++]=0};for(let E=0;E<L;E++){const ge=C[E*3],oe=C[E*3+1],be=C[E*3+2];O(ge,oe),O(oe,be),O(be,ge)}const U=new Bt;U.setAttribute("position",new qt(A,3));const W=new pr({color:16777215,transparent:!0,opacity:.35,depthTest:!1,depthWrite:!1}),Z=new hr(U,W);Z.renderOrder=2,r.uvScene.add(Z),r.uvWireframe=Z}r.uvBgMat=R,r.uvPaintMat=P,r.uvBgMesh=x,r.uvPaintMesh=p}const K={Pencil:{hardness:.95,opacity:.5,flow:.25,aspect:1,spacing:.12,streamline:0,velocity:0,taper:0,curvature:0},Marker:{hardness:1,opacity:1,flow:1,aspect:1,spacing:.25,streamline:0,velocity:0,taper:0,curvature:0},Airbrush:{hardness:0,opacity:.6,flow:.12,aspect:1,spacing:.05,streamline:0,velocity:0,taper:0,curvature:0},Ink:{hardness:1,opacity:1,flow:1,aspect:1,spacing:.1,streamline:.65,velocity:0,taper:.3,curvature:0},Charcoal:{hardness:.4,opacity:.7,flow:.45,aspect:1,spacing:.18,streamline:.15,velocity:.2,taper:0,curvature:.2},Calligraphy:{hardness:1,opacity:1,flow:1,aspect:.3,spacing:.08,streamline:.2,velocity:0,taper:.5,curvature:0},Quill:{hardness:1,opacity:1,flow:1,aspect:.45,spacing:.06,streamline:.35,velocity:.55,taper:.7,curvature:0},Stippler:{hardness:.2,opacity:.6,flow:.35,aspect:1,spacing:.65,streamline:0,velocity:0,taper:0,curvature:.4},Crayon:{hardness:.35,opacity:.65,flow:.4,aspect:.85,spacing:.18,streamline:.1,velocity:.4,taper:0,curvature:.25}};function Jt(r){return JSON.parse(JSON.stringify(r))}const cn=new V,dn=new V,un=new V,pn=new V,hn=new V,mn=new V,ga=new V,gn=new V,fn=new V(1,0,0),vn=new V(0,1,0),Je={r:1,g:.4,b:.4},qe={r:0,g:0,b:0},bn=new Qe,pt=new Uint8Array(4);function kn(){const r=i.useRef(null),s=i.useRef(null),d=i.useRef(null),o=i.useRef(null),v=i.useRef(!1),R=i.useRef({active:!1,lastX:0,lastY:0}),[x,P]=i.useState("#FF6B6B"),[p,w]=i.useState("#000000"),[h,C]=i.useState(20),[B,L]=i.useState(!1),[A,I]=i.useState("Upload a GLB to start"),[O,U]=i.useState(!0),[W,Z]=i.useState("color"),[E,ge]=i.useState(.7),[oe,be]=i.useState(.5),[De,Pt]=i.useState(0),[At,ba]=i.useState(!0),[tt,xa]=i.useState(Ie.find(e=>e.id==="custom")?.id??Ie[0]?.id??"default-round"),ae=i.useRef((()=>{const e=Ie.find(a=>a.id==="custom")??Ie[0];return e?Jt(e):null})()),me=i.useRef(ae.current?$e(ae.current):null),gt=i.useRef(null),ya=i.useRef(new Float32Array(mr)),ea=i.useRef(0),ft=i.useRef(0),It=i.useRef(0),Le=i.useRef({x:0,y:0}),[He,at]=i.useState(K.Marker.hardness),[Ne,rt]=i.useState(K.Marker.opacity),[Ge,nt]=i.useState(K.Marker.flow),[We,st]=i.useState(K.Marker.aspect),[ze,ot]=i.useState(K.Marker.spacing),[vt,ta]=i.useState(K.Marker.streamline),[bt,aa]=i.useState(K.Marker.velocity),[xt,ra]=i.useState(K.Marker.taper),[yt,na]=i.useState(K.Marker.curvature),[wa,de]=i.useState("Marker"),Re=i.useRef({spacing:K.Marker.spacing,streamline:K.Marker.streamline,brushSizePx:20,velocity:K.Marker.velocity,taper:K.Marker.taper,curvature:K.Marker.curvature}),Ut=i.useRef(null),[Ft,sa]=i.useState([]),[re,Dt]=i.useState(0),wt=i.useRef(0);i.useEffect(()=>{wt.current=re},[re]);const[xe,St]=i.useState(!1),[Sa,oa]=i.useState(0),it=i.useRef(null),[Oe,Ma]=i.useState("bilateral"),[lt,Ca]=i.useState(.6),[ct,Ra]=i.useState(2),[dt,ka]=i.useState(1),[Mt,Ta]=i.useState(.15),[Ct,ja]=i.useState(.05);i.useEffect(()=>()=>{it.current?.dispose(),it.current=null},[]),i.useEffect(()=>{const e=r.current,a=s.current;if(!e||!a)return;const n=new ca({canvas:e,antialias:!0});n.setPixelRatio(Math.min(window.devicePixelRatio,2)),n.outputColorSpace=kt,n.setClearColor(2236962,1);const f=new ca({canvas:a,antialias:!0});f.setPixelRatio(Math.min(window.devicePixelRatio,2)),f.outputColorSpace=kt,f.setClearColor(1710618,1);const l=new Ke;l.background=new Qe(2236962);const g=new Qa(45,1,.01,1e4),b=new er;b.firstHitOnly=!0;const T=new Me({vertexShader:Qr,fragmentShader:en,transparent:!1,blending:jt,depthTest:!1,depthWrite:!1,side:ar,uniforms:{brushColor:{value:new Qe(x)},brushCenter:{value:new V},brushNormal:{value:new V(0,1,0)},brushTangent:{value:new V(1,0,0)},brushRadius:{value:.05},brushAspect:{value:1},edgeSnapThreshold:{value:0},brushHardness:{value:.8},brushOpacity:{value:1},brushFlow:{value:1},viewProjMatrix:{value:new tr},sceneDepthTex:{value:null},occlusionEnabled:{value:1},cameraNear:{value:.01},cameraFar:{value:1e4},existingPaint:{value:null},tipAlpha:{value:null},useTipAlpha:{value:0},grainTex:{value:null},useGrain:{value:0},grainDepth:{value:0},grainScale:{value:1},grainBlendMode:{value:0},grainOffset:{value:new Ue},buildupMode:{value:0},strokeStartPaint:{value:null}}}),H=new Ke;H.background=new Qe(1710618);const Q=new ht(-1,1,1,-1,0,1),_=new rr(.018,.022,64),N=new nr({color:16777215,transparent:!0,opacity:.9,depthTest:!1,depthWrite:!1}),ee=new Ce(_,N);ee.visible=!1,ee.renderOrder=100,H.add(ee);const m={renderer:n,scene:l,camera:g,raycaster:b,meshRoot:null,meshes:[],brushMat:T,paintScene:new Ke,orthoCam:new ht(-1,1,1,-1,0,1),materials:[],matcapTex:Jr(),reflectionMatcapTex:qr(),depthRT:(()=>{const c=new sr(1024,1024);return c.format=or,c.type=ir,c.minFilter=J,c.magFilter=J,new Ze(1024,1024,{depthTexture:c,depthBuffer:!0,generateMipmaps:!1,minFilter:J,magFilter:J})})(),target:new V(0,0,0),distance:3,theta:Math.PI/4,phi:Math.PI/3,uvRenderer:f,uvScene:H,uvOrthoCam:Q,uvBgMat:null,uvPaintMat:null,uvBgMesh:null,uvPaintMesh:null,uvWireframe:null,uvBrushRing:ee,lastBrushUv:null,brushRadiusUv:.02,strokeState:{targetX:0,targetY:0,smoothedX:0,smoothedY:0,lastStampX:0,lastStampY:0,hasStamped:!1,tailing:!1,stampCount:0,lastStampWorldPos:new V,lastNormal:new V(0,1,0),lastTangent:new V(1,0,0),pendingUndoSnapshot:!1},undoCopyScene:new Ke,undoCopyCam:new ht(-1,1,1,-1,0,1),undoCopyMat:new Me({vertexShader:tn,fragmentShader:an,uniforms:{src:{value:null}},depthTest:!1,depthWrite:!1,transparent:!1,blending:jt}),undoCopyMesh:new Ce(new mt(2,2),void 0)};m.undoCopyMesh.material=m.undoCopyMat,m.undoCopyScene.add(m.undoCopyMesh),o.current=m,T.uniforms.sceneDepthTex.value=m.depthRT.depthTexture,T.uniforms.cameraNear.value=g.near,T.uniforms.cameraFar.value=g.far;const te=()=>{const c=Math.sin(m.phi);g.position.set(m.target.x+m.distance*c*Math.cos(m.theta),m.target.y+m.distance*Math.cos(m.phi),m.target.z+m.distance*c*Math.sin(m.theta)),g.lookAt(m.target)},fe=()=>{const c=e.clientWidth||1,F=e.clientHeight||1;n.setSize(c,F,!1),g.aspect=c/F,g.updateProjectionMatrix();const Y=a.clientWidth||1,ne=a.clientHeight||1;f.setSize(Y,ne,!1)};fe();const k=new ResizeObserver(fe);k.observe(e),k.observe(a);let G=0;const j=()=>{G=requestAnimationFrame(j),te();const c=m.strokeState,F=Re.current;if(v.current||c.tailing){const X=Math.max(.01,1-F.streamline);c.smoothedX+=(c.targetX-c.smoothedX)*X,c.smoothedY+=(c.targetY-c.smoothedY)*X;const ke=performance.now(),ut=Math.max(1,ke-It.current),ye=c.smoothedX-Le.current.x,_e=c.smoothedY-Le.current.y,Te=Math.hypot(ye,_e)/ut,Ve=Math.max(.5,F.brushSizePx/8),je=Math.min(1,Te/Ve);ft.current=ft.current*.6+je*.4,It.current=ke,Le.current.x=c.smoothedX,Le.current.y=c.smoothedY;const Ye=80,Be=c.smoothedX-c.lastStampX,ue=c.smoothedY-c.lastStampY,ie=Math.hypot(Be,ue),y=Math.max(.5,F.brushSizePx*F.spacing);if(!c.hasStamped)Ut.current?.(c.smoothedX,c.smoothedY),c.lastStampX=c.smoothedX,c.lastStampY=c.smoothedY,c.hasStamped=!0;else if(ie>=y){const $=Math.floor(ie/y),se=Math.min($,Ye),pe=se<$?ie/se:y,we=Be/ie,D=ue/ie;for(let le=1;le<=se;le++){const q=c.lastStampX+we*pe*le,Nt=c.lastStampY+D*pe*le;Ut.current?.(q,Nt)}c.lastStampX+=we*pe*se,c.lastStampY+=D*pe*se}if(c.tailing&&Math.hypot(c.targetX-c.smoothedX,c.targetY-c.smoothedY)<.5&&(c.tailing=!1),c.pendingUndoSnapshot&&!c.tailing&&!v.current){c.pendingUndoSnapshot=!1;const $=m.materials[wt.current];if($){$.undoHistory.length>=va&&($.undoHistory.shift()?.dispose(),$.undoHead>=0&&($.undoHead-=1));const se=new Ze(z,z,{format:ce,type:he,minFilter:J,magFilter:J,generateMipmaps:!1,depthBuffer:!1,stencilBuffer:!1});Tt(m,$.paintRT.texture,se),$.undoHistory.push(se),$.undoHead=$.undoHistory.length-1}}}n.render(l,g);const ne=Math.max(.001,m.brushRadiusUv);ee.scale.setScalar(ne/.022),f.render(H,Q)};return G=requestAnimationFrame(j),()=>{cancelAnimationFrame(G),k.disconnect(),T.dispose(),m.materials.forEach(ha),m.materials=[],m.matcapTex.dispose(),m.reflectionMatcapTex.dispose(),m.depthRT.depthTexture?.dispose(),m.depthRT.dispose(),m.uvBgMat?.dispose(),m.uvPaintMat?.dispose(),m.uvBgMesh?.geometry?.dispose(),m.uvPaintMesh?.geometry?.dispose(),m.uvWireframe&&(m.uvWireframe.geometry.dispose(),m.uvWireframe.material.dispose()),_.dispose(),N.dispose(),m.undoCopyMat.dispose(),m.undoCopyMesh.geometry.dispose();for(const c of m.meshes){const F=c.geometry;F.disposeBoundsTree?.(),F.dispose()}m.meshes=[],m.meshRoot&&m.scene.remove(m.meshRoot),n.dispose(),f.dispose()}},[]),i.useEffect(()=>{const e=o.current;if(!e)return;const a=e.brushMat.uniforms.brushColor.value.set(x);Je.r=a.r,Je.g=a.g,Je.b=a.b},[x]),i.useEffect(()=>{const e=bn.set(p);qe.r=e.r,qe.g=e.g,qe.b=e.b},[p]),i.useEffect(()=>{const e=o.current;if(e)for(const a of e.materials)a.compositeMat.uniforms.useIslandGate.value=O?1:0},[O]),i.useEffect(()=>{const e=o.current;if(e)for(const a of e.materials)a.compositeMat.uniforms.matcapIntensity.value=E},[E]),i.useEffect(()=>{const e=o.current;if(e)for(const a of e.materials)a.compositeMat.uniforms.reflectionIntensity.value=oe},[oe]),i.useEffect(()=>{const e=o.current;e&&(e.brushMat.uniforms.edgeSnapThreshold.value=De*.95)},[De]),i.useEffect(()=>{const e=o.current;e&&(e.brushMat.uniforms.occlusionEnabled.value=At?1:0)},[At]),i.useEffect(()=>{const e=o.current;e&&(e.brushMat.uniforms.brushHardness.value=He,e.brushMat.uniforms.brushOpacity.value=Ne,e.brushMat.uniforms.brushFlow.value=Ge,e.brushMat.uniforms.brushAspect.value=We)},[He,Ne,Ge,We]);const Ba=i.useMemo(()=>{const e=new Map;for(const a of Ie){const n=e.get(a.category)??[];n.push(a),e.set(a.category,n)}return Array.from(e.entries()).map(([a,n])=>({label:a,brushes:n}))},[]),Ee=i.useCallback(e=>{const a=o.current;if(!a)return;const n=a.brushMat.uniforms;n.useTipAlpha.value=0,n.useGrain.value=0,n.tipAlpha.value=null,n.grainTex.value=null;const f=e.buildupMode;n.buildupMode.value=f==="continuous"||f==="additive"?1:f==="per-stroke"?2:0;const l=e.tipAlphaUrl;if(l?.startsWith("procedural:")){const b=Tr(l.slice(11));b&&(n.tipAlpha.value=b,n.useTipAlpha.value=1)}else l&&da(l,e.tipFilter??"linear").then(b=>{const T=o.current;if(!T||!b||ae.current?.tipAlphaUrl!==l)return;const H=T.brushMat.uniforms;H.tipAlpha.value=b,H.useTipAlpha.value=1}).catch(()=>{});const g=e.grainTextureUrl;if(g?.startsWith("procedural:")){const b=jr(g.slice(11));if(b){n.grainTex.value=b,n.useGrain.value=1;const T=e.grainBlendMode??"multiply";n.grainBlendMode.value=T==="overlay"?1:T==="screen"?2:0}}else g&&da(g,"linear").then(b=>{const T=o.current;if(!T||!b||ae.current?.grainTextureUrl!==g)return;const H=T.brushMat.uniforms;H.grainTex.value=b,H.useGrain.value=1;const Q=e.grainBlendMode??"multiply";H.grainBlendMode.value=Q==="overlay"?1:Q==="screen"?2:0}).catch(()=>{})},[]);i.useEffect(()=>{const e=Ie.find(n=>n.id===tt);if(!e)return;ae.current=Jt(e),me.current=$e(ae.current);const a=me.current.base;at(a[M.Hardness]),rt(a[M.Opacity]),nt(a[M.Flow]),st(a[M.Roundness]),ot(a[M.Spacing]),de(null),Ee(ae.current)},[tt,Ee]),i.useEffect(()=>{const e=ae.current;e&&(e.base={...e.base??{},Hardness:He,Opacity:Ne,Flow:Ge,Roundness:We,Spacing:ze},me.current=$e(e))},[He,Ne,Ge,We,ze]),i.useEffect(()=>{Re.current.spacing=ze,Re.current.streamline=vt,Re.current.brushSizePx=h,Re.current.velocity=bt,Re.current.taper=xt,Re.current.curvature=yt},[ze,vt,h,bt,xt,yt]),i.useEffect(()=>{const e=o.current;if(!e)return;const a=W==="metalness"?1:W==="roughness"?2:0;for(const n of e.materials)n.compositeMat.uniforms.viewMode.value=a;e.uvBgMat&&(e.uvBgMat.uniforms.viewMode.value=a)},[W]),i.useEffect(()=>{const e=o.current;if(!e||e.meshes.length===0||e.materials.length===0)return;const a=W==="metalness"?1:W==="roughness"?2:0;for(let f=0;f<e.materials.length;f++){const l=f===re;e.materials[f].compositeMat.uniforms.isDimmed.value=l?0:1}const n=e.materials[re];n&&ma(e,n,a)},[re,W]);const Pa=i.useCallback(async e=>{const a=o.current;if(!a){console.warn("[PaintLab] handleFile called but state not ready");return}console.log("[PaintLab] handleFile",e.name,e.size,"bytes"),I(`Loading ${e.name}…`);let n;try{n=await e.arrayBuffer()}catch(l){console.error("[PaintLab] arrayBuffer failed",l),I(`Read failed: ${l instanceof Error?l.message:String(l)}`);return}console.log("[PaintLab] read",n.byteLength,"bytes"),a.meshRoot&&(a.scene.remove(a.meshRoot),a.meshRoot.traverse(l=>{if(l.isMesh){const g=l.geometry;g.disposeBoundsTree?.(),g.dispose()}}),a.meshRoot=null),a.meshes=[],a.materials.forEach(ha),a.materials=[],sa([]),Dt(0),a.uvBgMat?.dispose(),a.uvBgMat=null,a.uvPaintMat?.dispose(),a.uvPaintMat=null,a.uvBgMesh&&(a.uvScene.remove(a.uvBgMesh),a.uvBgMesh.geometry.dispose(),a.uvBgMesh=null),a.uvPaintMesh&&(a.uvScene.remove(a.uvPaintMesh),a.uvPaintMesh.geometry.dispose(),a.uvPaintMesh=null),a.uvWireframe&&(a.uvScene.remove(a.uvWireframe),a.uvWireframe.geometry.dispose(),a.uvWireframe.material.dispose(),a.uvWireframe=null);const f=new lr;try{f.parse(n,"",l=>{console.log("[PaintLab] parsed; scene children:",l.scene.children.length);const g=[];if(l.scene.traverse(k=>{k.isMesh&&g.push(k)}),g.length===0){console.warn("[PaintLab] no meshes in scene"),I("No meshes found in GLB");return}console.log("[PaintLab] meshes:",g.length,g.map(k=>k.name||"(unnamed)"));for(const k of g){if(!k.geometry.getIndex()){const j=k.geometry.getAttribute("position").count,c=[];for(let F=0;F<j;F++)c.push(F);k.geometry.setIndex(c)}k.geometry.computeBoundsTree?.()}const b=gr(l.scene);if(b.length===0){console.warn("[PaintLab] no materials discovered"),I("No materials found in GLB");return}console.log("[PaintLab] materials:",b.length,b.map(k=>k.name));const T=W==="metalness"?1:W==="roughness"?2:0,H=[];for(let k=0;k<b.length;k++){const G=b[k],j=G.material,c=[];for(const D of G.meshes){if(!g.includes(D))continue;const le=Array.isArray(D.material)?D.material:[D.material];for(let q=0;q<le.length;q++)le[q]===G.material&&c.push({mesh:D,slotIndex:q,subGeometry:ln(D.geometry,q)})}if(c.length===0)continue;const F=c[0].slotIndex,Y=j?.map??null,ne=j?.color?j.color.clone():new Qe(1,1,1),X=Y?.image,ke=!!X&&(X.width??0)>1&&(X.height??0)>1;.299*ne.r+.587*ne.g+.114*ne.b<.1&&(ke?ne.setRGB(1,1,1):ne.setRGB(.6,.6,.6));let ye,_e=null;if(Y&&ke)ye=Y;else{const D=new Fe(new Uint8Array([255,255,255,255]),1,1,ce);D.colorSpace=kt,D.needsUpdate=!0,ye=D,_e=D}let Te,Ve=null;if(pa(j?.metalnessMap))Te=j.metalnessMap;else{const D=ua();Ve=D,Te=D}let je,Ye=null;if(pa(j?.roughnessMap))je=j.roughnessMap;else{const D=ua();Ye=D,je=D}const Be=new Ze(z,z,{format:ce,type:he,minFilter:J,magFilter:J,generateMipmaps:!1,depthBuffer:!1,stencilBuffer:!1});a.renderer.setRenderTarget(Be),a.renderer.setClearColor(0,0),a.renderer.clear(),a.renderer.setClearColor(2236962,1),a.renderer.setRenderTarget(null);const ue=new Fe(new Uint8Array(z*z*4),z,z,ce,he);ue.minFilter=J,ue.magFilter=J,ue.needsUpdate=!0;const ie=new Fe(new Uint8Array(z*z*4),z,z,ce,he);ie.minFilter=J,ie.magFilter=J,ie.needsUpdate=!0;let y,$=null,se=null;try{const D=fr(a.renderer,c[0].subGeometry,z,16);$=D.mask,y=D.mask.texture,console.log(`[PaintLab] [${G.name}] island mask: islands =`,D.islandCount,"· targets =",c.length)}catch(D){console.warn(`[PaintLab] [${G.name}] island mask build failed; gating disabled`,D);const le=new Uint8Array([1,0,0,255]),q=new Fe(le,1,1,ce,he);q.minFilter=J,q.magFilter=J,q.needsUpdate=!0,se=q,y=q}const pe=k===0,we=new Me({vertexShader:Kr,fragmentShader:Zr,uniforms:{baseColorMap:{value:ye},baseColorTint:{value:ne},metalnessMap:{value:Te},roughnessMap:{value:je},paintTex:{value:ue},islandMask:{value:y},paintTexelSize:{value:new Ue(1/z,1/z)},useIslandGate:{value:O?1:0},viewMode:{value:T},matcapTex:{value:a.matcapTex},matcapIntensity:{value:E},reflectionMatcap:{value:a.reflectionMatcapTex},reflectionIntensity:{value:oe},isDimmed:{value:pe?0:1}}});H.push({slotIndex:F,name:G.name,swatchColor:G.swatchColor,paintRT:Be,paintTexCopy:ue,strokeStartPaintTex:ie,compositeMat:we,baseMap:ye,whiteFallback:_e,metalnessFallback:Ve,roughnessFallback:Ye,islandMask:$,islandGutterMaskFallback:se,targets:c,undoHistory:[],undoHead:-1})}a.materials=H,a.meshes=g;const Q=H[0].compositeMat;for(const k of g){const G=Array.isArray(k.material)?k.material:[k.material],j=c=>H.find(F=>F.targets.some(Y=>Y.mesh===k&&Y.slotIndex===c))?.compositeMat??Q;if(G.length===1)k.material=j(0);else{const c=[];for(let F=0;F<G.length;F++)c[F]=j(F);k.material=c}}a.scene.add(l.scene),l.scene.updateMatrixWorld(!0);const _=new cr().setFromObject(l.scene),N=_.getCenter(new V),ee=_.getBoundingSphere(new dr);l.scene.position.sub(N),l.scene.updateMatrixWorld(!0),a.meshRoot=l.scene,a.target.set(0,0,0),a.distance=Math.max(.5,ee.radius*2.5),a.theta=Math.PI/4,a.phi=Math.PI/3,sa(H.map(k=>({slotIndex:k.slotIndex,name:k.name,swatchColor:k.swatchColor}))),Dt(0),ma(a,H[0],T),L(!0);let m=0;for(const k of g)m+=(k.geometry.getIndex()?.count??0)/3;const te=H.length===1?"1 mat":`${H.length} mats`,fe=g.length===1?"1 mesh":`${g.length} meshes`;console.log("[PaintLab] meshes added; tris:",m,"sphere radius:",ee.radius,"·",te,"·",fe),I(`Loaded ${e.name} · ${(n.byteLength/1024).toFixed(0)} KB · ${m.toFixed(0)} tris · ${fe} · ${te}`)},l=>{console.error("[PaintLab] GLB parse failed",l);const g=l instanceof Error?l.message:String(l);I(`GLB parse failed: ${g}`)})}catch(l){console.error("[PaintLab] GLB parse threw",l),I(`GLB parse threw: ${l instanceof Error?l.message:String(l)}`)}},[O,W,E,oe]),ia=i.useCallback((e,a)=>{const n=o.current;if(!n||n.meshes.length===0)return;const f=r.current;if(!f)return;const l=n.materials[re];if(!l)return;const g=me.current,b=gt.current;if(!g||!b)return;const T=f.getBoundingClientRect(),H=(e-T.left)/T.width*2-1,Q=-((a-T.top)/T.height)*2+1;n.raycaster.setFromCamera(new Ue(H,Q),n.camera);const _=n.raycaster.intersectObjects(n.meshes,!1);if(_.length===0)return;const N=_[0],ee=N.face?.materialIndex??0;if(!l.targets.some(y=>y.mesh===N.object&&y.slotIndex===ee))return;const te=Math.tan(n.camera.fov*Math.PI/360),k=2*N.distance*te/(f.clientHeight||1),G=h*k,j=n.brushMat.uniforms;j.existingPaint.value=l.paintTexCopy,j.strokeStartPaint.value=l.strokeStartPaintTex;const c=new V(0,1,0);if(N.face){const y=new ur().getNormalMatrix(N.object.matrixWorld);c.copy(N.face.normal).applyMatrix3(y).normalize()}const F=n.strokeState,Y=cn;let ne=!1;F.hasStamped?(Y.subVectors(N.point,F.lastStampWorldPos),Y.lengthSq()<1e-8&&(ne=!0)):ne=!0,ne&&(Y.copy(F.lastTangent),(Math.abs(Y.dot(c))>.99||Y.lengthSq()<1e-8)&&Y.setFromMatrixColumn(n.camera.matrixWorld,0));const X=dn.copy(Y).addScaledVector(c,-c.dot(Y));if(X.lengthSq()<1e-8){const y=Math.abs(c.y)<.99?vn:fn;X.copy(y).addScaledVector(c,-c.dot(y))}X.normalize();const ke=un.crossVectors(c,X).normalize(),ut=ya.current,ye=ft.current,_e=performance.now(),Te=Math.min(1,(_e-ea.current)/1500);ga.copy(N.point).sub(n.camera.position).normalize();const Ve=Math.max(0,-ga.dot(c)),je=gn.setFromMatrixColumn(n.camera.matrixWorld,0),Ye=Math.max(-1,Math.min(1,X.dot(je))),Be=Math.acos(Ye)/Math.PI;yr(ut,{velocity:ye,strokeProgress:Te,direction:0,normalDotView:Ve,surfaceTangent:Be});const ue=wr(g,b,ut,{r:Je.r,g:Je.g,b:Je.b},{r:qe.r,g:qe.g,b:qe.b}),ie=n.renderer.getRenderTarget();n.renderer.setRenderTarget(n.depthRT),n.renderer.clear(),n.renderer.render(n.scene,n.camera),n.renderer.setRenderTarget(ie),j.viewProjMatrix.value.multiplyMatrices(n.camera.projectionMatrix,n.camera.matrixWorldInverse);for(const y of ue){const $=mn.copy(X).multiplyScalar(y.scatterAlongTangent*G).addScaledVector(ke,y.scatterAlongBitangent*G);j.brushCenter.value.copy(N.point).add($);const se=pn.copy(X);if(y.angleRad!==0){const Pe=Math.cos(y.angleRad),Ae=Math.sin(y.angleRad),Se=c.dot(X),Gt=hn.crossVectors(c,X);se.copy(X).multiplyScalar(Pe).addScaledVector(Gt,Ae).addScaledVector(c,Se*(1-Pe))}j.brushNormal.value.copy(c),j.brushTangent.value.copy(se),j.brushRadius.value=Math.max(1e-4,G*y.radius),j.brushAspect.value=Math.max(.05,y.roundness),j.brushHardness.value=y.hardness,j.brushOpacity.value=y.opacity,j.brushFlow.value=y.flow;let pe=y.color.r,we=y.color.g,D=y.color.b;if(y.wetMode>0&&N.uv){const Pe=Math.max(0,Math.min(z-1,Math.floor(N.uv.x*z))),Ae=Math.max(0,Math.min(z-1,Math.floor(N.uv.y*z)));n.renderer.readRenderTargetPixels(l.paintRT,Pe,Ae,1,1,pt);const Se=pt[3]/255,Gt=pt[0]/255,Ka=pt[1]/255,Za=pt[2]/255,Wt=l.compositeMat.uniforms.baseColorTint.value,zt=Wt.r*(1-Se)+Gt*Se,Ot=Wt.g*(1-Se)+Ka*Se,Et=Wt.b*(1-Se)+Za*Se;if(!b.smudgeInitialized)b.smudgeColor=[zt,Ot,Et],b.smudgeInitialized=!0;else if(b.smudgeColor){const ve=b.smudgeColor,_t=1-y.smudgeLength;ve[0]=ve[0]+(zt-ve[0])*_t,ve[1]=ve[1]+(Ot-ve[1])*_t,ve[2]=ve[2]+(Et-ve[2])*_t}const Xe=b.smudgeColor??[zt,Ot,Et];y.wetMode,pe=Xe[0]*(1-y.colorRate)+y.color.r*y.colorRate,we=Xe[1]*(1-y.colorRate)+y.color.g*y.colorRate,D=Xe[2]*(1-y.colorRate)+y.color.b*y.colorRate}j.brushColor.value.setRGB(pe,we,D),j.grainDepth.value=y.grainDepth,j.grainScale.value=y.grainScale;const le=g.grainBehavior,q=j.grainOffset.value;le==="follows-stroke"?q.set(b.stampCount*.05,0):le==="follows-rotation"?q.set(Math.cos(y.angleRad),Math.sin(y.angleRad)):q.set(0,0),n.paintScene.clear();for(const Pe of l.targets){const Ae=new Ce(Pe.subGeometry,n.brushMat);Ae.applyMatrix4(Pe.mesh.matrixWorld),Ae.frustumCulled=!1,n.paintScene.add(Ae)}const Nt=n.renderer.getRenderTarget(),qa=n.renderer.autoClear;n.renderer.autoClear=!1,n.renderer.setRenderTarget(l.paintRT),n.renderer.render(n.paintScene,n.orthoCam),n.renderer.copyFramebufferToTexture(l.paintTexCopy),n.renderer.setRenderTarget(Nt),n.renderer.autoClear=qa,n.paintScene.clear()}N.uv&&(n.lastBrushUv=new Ue(N.uv.x*2-1,N.uv.y*2-1),n.uvBrushRing.position.set(n.lastBrushUv.x,n.lastBrushUv.y,0),n.uvBrushRing.visible=!0,n.brushRadiusUv=h/f.clientHeight*1.5),F.lastStampWorldPos.copy(N.point),F.lastNormal.copy(c),F.lastTangent.copy(X),F.stampCount+=ue.length},[h,re]);i.useEffect(()=>{Ut.current=ia},[ia]);const Rt=i.useCallback(()=>{const e=o.current;if(!e)return;const a=e.materials[wt.current];a&&(v.current||e.strokeState.tailing||e.strokeState.pendingUndoSnapshot||a.undoHead<0||(a.undoHead-=1,a.undoHead<0?sn(e,a.paintRT):Tt(e,a.undoHistory[a.undoHead].texture,a.paintRT),$t(e,a)))},[]),Lt=i.useCallback(()=>{const e=o.current;if(!e)return;const a=e.materials[wt.current];a&&(v.current||e.strokeState.tailing||e.strokeState.pendingUndoSnapshot||a.undoHead+1>=a.undoHistory.length||(a.undoHead+=1,Tt(e,a.undoHistory[a.undoHead].texture,a.paintRT),$t(e,a)))},[]);i.useEffect(()=>{const e=a=>{if(!(a.ctrlKey||a.metaKey))return;const f=a.key.toLowerCase(),l=f==="z"&&!a.shiftKey,g=f==="z"&&a.shiftKey||f==="y";if(!l&&!g)return;const b=a.target;if(b){if(b.tagName==="TEXTAREA"||b.isContentEditable)return;if(b.tagName==="INPUT"){const T=b.type.toLowerCase();if(T==="text"||T==="search"||T==="url"||T==="email"||T==="password"||T==="tel")return}}a.preventDefault(),l?Rt():Lt()};return window.addEventListener("keydown",e),()=>window.removeEventListener("keydown",e)},[Rt,Lt]);const Aa=e=>{if(e.preventDefault(),e.currentTarget.setPointerCapture(e.pointerId),e.button===2)R.current={active:!0,lastX:e.clientX,lastY:e.clientY};else if(e.button===0){const a=o.current;if(a){const l=a.strokeState;l.targetX=e.clientX,l.targetY=e.clientY,l.smoothedX=e.clientX,l.smoothedY=e.clientY,l.lastStampX=e.clientX,l.lastStampY=e.clientY,l.hasStamped=!1,l.tailing=!1,l.stampCount=0}const n=me.current;if(n){const l=(Date.now()&65535^e.pointerId*2654435761)>>>0;gt.current=Mr(n,l)}ft.current=0,It.current=performance.now(),Le.current.x=e.clientX,Le.current.y=e.clientY,ea.current=performance.now();const f=o.current;if(f){const l=f.materials[re];if(l){const g=f.renderer.getRenderTarget();if(f.renderer.setRenderTarget(l.paintRT),f.renderer.copyFramebufferToTexture(l.strokeStartPaintTex),f.renderer.setRenderTarget(g),l.undoHead<l.undoHistory.length-1){for(let b=l.undoHead+1;b<l.undoHistory.length;b++)l.undoHistory[b].dispose();l.undoHistory.length=l.undoHead+1}}f.strokeState.pendingUndoSnapshot=!1}v.current=!0}},Ia=e=>{if(R.current.active){const a=o.current;if(!a)return;const n=e.clientX-R.current.lastX,f=e.clientY-R.current.lastY;R.current.lastX=e.clientX,R.current.lastY=e.clientY,a.theta-=n*.005,a.phi-=f*.005,a.phi=Math.max(.05,Math.min(Math.PI-.05,a.phi))}else if(v.current){const a=o.current;a&&(a.strokeState.targetX=e.clientX,a.strokeState.targetY=e.clientY)}else{const a=o.current,n=r.current;if(a&&a.meshes.length>0&&n){const f=n.getBoundingClientRect(),l=(e.clientX-f.left)/f.width*2-1,g=-((e.clientY-f.top)/f.height)*2+1;a.raycaster.setFromCamera(new Ue(l,g),a.camera);const b=a.raycaster.intersectObjects(a.meshes,!1);if(b.length>0&&b[0].uv){const T=b[0].uv;a.lastBrushUv=new Ue(T.x*2-1,T.y*2-1),a.uvBrushRing.position.set(a.lastBrushUv.x,a.lastBrushUv.y,0),a.uvBrushRing.visible=!0,a.brushRadiusUv=h/n.clientHeight*1.5}else a.uvBrushRing.visible=!1}}},la=e=>{if((R.current.active||v.current)&&e.currentTarget.releasePointerCapture(e.pointerId),R.current.active=!1,v.current){const a=o.current;a&&(a.strokeState.tailing=!0,a.strokeState.pendingUndoSnapshot=!0),gt.current&&Sr(gt.current)}v.current=!1},Ua=()=>{const e=o.current;e&&(e.uvBrushRing.visible=!1)},Fa=e=>{const a=o.current;if(!a)return;e.preventDefault();const n=1+e.deltaY*.001;a.distance=Math.max(.1,Math.min(50,a.distance*n))},Da=i.useCallback(e=>{const a=K[e];a&&(at(a.hardness),rt(a.opacity),nt(a.flow),st(a.aspect),ot(a.spacing),ta(a.streamline),aa(a.velocity),ra(a.taper),na(a.curvature),de(e))},[]),La=i.useCallback(e=>{at(e),de(null)},[]),Ha=i.useCallback(e=>{rt(e),de(null)},[]),Na=i.useCallback(e=>{nt(e),de(null)},[]),Ga=i.useCallback(e=>{st(e),de(null)},[]),Wa=i.useCallback(e=>{ot(e),de(null)},[]),za=i.useCallback(e=>{ta(e),de(null)},[]),Oa=i.useCallback(e=>{aa(e),de(null)},[]),Ea=i.useCallback(e=>{ra(e),de(null)},[]),_a=i.useCallback(e=>{na(e),de(null)},[]),Ht=i.useCallback(()=>{const e=ae.current;if(!e)return;me.current=$e(e);const a=me.current.base;at(a[M.Hardness]),rt(a[M.Opacity]),nt(a[M.Flow]),st(a[M.Roundness]),ot(a[M.Spacing]),Ee(e),oa(n=>n+1)},[Ee]),Va=i.useCallback(()=>{const e=Ie.find(n=>n.id===tt);if(!e)return;ae.current=Jt(e),me.current=$e(ae.current);const a=me.current.base;at(a[M.Hardness]),rt(a[M.Opacity]),nt(a[M.Flow]),st(a[M.Roundness]),ot(a[M.Spacing]),Ee(ae.current),oa(n=>n+1)},[tt,Ee]),Ya=i.useCallback(e=>{try{const a=JSON.parse(e);if(!a||typeof a.id!="string"||typeof a.name!="string"){console.warn("[PaintLab] paste rejected: missing id/name");return}ae.current=a,me.current=$e(a),Ht()}catch(a){console.warn("[PaintLab] paste rejected: not valid JSON",a)}},[Ht]),Xa=()=>{const e=o.current;if(!e)return;const a=e.materials[re];a&&(e.renderer.setRenderTarget(a.paintRT),e.renderer.setClearColor(0,0),e.renderer.clear(),e.renderer.setClearColor(2236962,1),e.renderer.setRenderTarget(null),a.paintTexCopy.image.data.fill(0),a.paintTexCopy.needsUpdate=!0)},$a=i.useCallback(()=>{const e=o.current;if(!e)return;const a=e.materials[re];if(!a||v.current||e.strokeState.tailing||e.strokeState.pendingUndoSnapshot)return;it.current||(it.current=new Fr);const n=a.compositeMat.uniforms.islandMask.value,f=a.paintRT.width,l=a.paintRT.height,g=new Uint8Array(f*l*4);e.renderer.readRenderTargetPixels(a.paintRT,0,0,f,l,g),it.current.run(e.renderer,a.paintRT,n,{method:Oe,strength:lt,kernelRadius:ct,iterations:dt,colorSigma:Mt,edgeThreshold:Ct,useIslandGate:O});const b=new Uint8Array(f*l*4);e.renderer.readRenderTargetPixels(a.paintRT,0,0,f,l,b);let T=0,H=0,Q=0,_=-1;for(let m=0;m<g.length;m+=4){g[m+3]>2&&T++;const te=Math.max(Math.abs(b[m]-g[m]),Math.abs(b[m+1]-g[m+1]),Math.abs(b[m+2]-g[m+2]),Math.abs(b[m+3]-g[m+3]));te>0&&(H++,_<0&&(_=m)),te>Q&&(Q=te)}const N=_>=0?Array.from(g.slice(_,_+4)).join(","):"none",ee=_>=0?Array.from(b.slice(_,_+4)).join(","):"none";$t(e,a),on(e,a),I(`[diag] ${Oe} k${ct} ${dt}x str${Math.round(lt*100)}% gate${O?1:0} island${n?1:0} ${f}x${l} · painted ${T} · changed ${H} · maxΔ ${Q} · ${N} → ${ee}`)},[re,O,Oe,lt,ct,dt,Mt,Ct]),Ja=i.useCallback(()=>{const e=o.current;if(!e)return;const a=e.materials[re];if(!a)return;const n=z,f=new Ze(n,n,{format:ce,type:he,minFilter:et,magFilter:et,generateMipmaps:!1,depthBuffer:!1,stencilBuffer:!1});f.texture.colorSpace=kt;const l=`
      uniform sampler2D baseColorMap;
      uniform vec3 baseColorTint;
      uniform sampler2D paintTex;
      uniform sampler2D islandMask;
      uniform vec2 paintTexelSize;
      uniform float useIslandGate;
      varying vec2 vUv;

      bool sameIsland(vec4 a, vec4 b) {
        if (a.a < 0.5 || b.a < 0.5) return false;
        return abs(a.r - b.r) < (0.5 / 255.0) && abs(a.g - b.g) < (0.5 / 255.0);
      }

      vec4 sampleIslandGated(vec2 uv) {
        vec2 pix = uv / paintTexelSize - 0.5;
        vec2 base = floor(pix);
        vec2 f = fract(pix);
        vec2 uv00 = (base + vec2(0.5, 0.5)) * paintTexelSize;
        vec2 uv10 = (base + vec2(1.5, 0.5)) * paintTexelSize;
        vec2 uv01 = (base + vec2(0.5, 1.5)) * paintTexelSize;
        vec2 uv11 = (base + vec2(1.5, 1.5)) * paintTexelSize;
        vec4 myId = texture2D(islandMask, uv);
        vec4 t00 = texture2D(paintTex, uv00);
        vec4 t10 = texture2D(paintTex, uv10);
        vec4 t01 = texture2D(paintTex, uv01);
        vec4 t11 = texture2D(paintTex, uv11);
        vec4 m00 = texture2D(islandMask, uv00);
        vec4 m10 = texture2D(islandMask, uv10);
        vec4 m01 = texture2D(islandMask, uv01);
        vec4 m11 = texture2D(islandMask, uv11);
        float w00 = (sameIsland(myId, m00) && t00.a > 0.001) ? 1.0 : 0.0;
        float w10 = (sameIsland(myId, m10) && t10.a > 0.001) ? 1.0 : 0.0;
        float w01 = (sameIsland(myId, m01) && t01.a > 0.001) ? 1.0 : 0.0;
        float w11 = (sameIsland(myId, m11) && t11.a > 0.001) ? 1.0 : 0.0;
        float bw00 = (1.0 - f.x) * (1.0 - f.y) * w00;
        float bw10 = f.x * (1.0 - f.y) * w10;
        float bw01 = (1.0 - f.x) * f.y * w01;
        float bw11 = f.x * f.y * w11;
        float total = bw00 + bw10 + bw01 + bw11;
        if (total < 1e-5) return texture2D(paintTex, uv);
        return (t00 * bw00 + t10 * bw10 + t01 * bw01 + t11 * bw11) / total;
      }

      void main() {
        vec3 base = baseColorTint * texture2D(baseColorMap, vUv).rgb;
        vec4 paint = (useIslandGate > 0.5) ? sampleIslandGated(vUv) : texture2D(paintTex, vUv);
        vec3 final = mix(base, paint.rgb, paint.a);
        gl_FragColor = vec4(final, 1.0);
        #include <colorspace_fragment>
      }
    `,g=new Me({vertexShader:Kt,fragmentShader:l,uniforms:{baseColorMap:{value:a.compositeMat.uniforms.baseColorMap.value},baseColorTint:{value:a.compositeMat.uniforms.baseColorTint.value},paintTex:{value:a.paintTexCopy},islandMask:{value:a.compositeMat.uniforms.islandMask.value},paintTexelSize:{value:a.compositeMat.uniforms.paintTexelSize.value},useIslandGate:{value:O?1:0}},depthTest:!1,depthWrite:!1}),b=new mt(2,2),T=new Ce(b,g),H=new Ke;H.add(T);const Q=new ht(-1,1,1,-1,0,1),_=e.renderer.getRenderTarget();e.renderer.setRenderTarget(f),e.renderer.setClearColor(0,1),e.renderer.clear(),e.renderer.render(H,Q),e.renderer.setRenderTarget(_),e.renderer.setClearColor(2236962,1);const N=new Uint8Array(n*n*4);e.renderer.readRenderTargetPixels(f,0,0,n,n,N);const ee=new Uint8ClampedArray(N.length),m=n*4;for(let k=0;k<n;k++){const G=(n-1-k)*m;ee.set(N.subarray(G,G+m),k*m)}const te=document.createElement("canvas");te.width=n,te.height=n;const fe=te.getContext("2d");if(!fe){console.warn("[PaintLab] download: 2D context unavailable"),f.dispose(),g.dispose(),b.dispose();return}fe.putImageData(new ImageData(ee,n,n),0,0),te.toBlob(k=>{if(!k)return;const G=URL.createObjectURL(k),j=document.createElement("a");j.href=G;const c=a.name.replace(/[^a-zA-Z0-9_-]/g,"_").slice(0,40)||"material";j.download=`paint-${c}-${Date.now()}.png`,document.body.appendChild(j),j.click(),document.body.removeChild(j),URL.revokeObjectURL(G)},"image/png"),f.dispose(),g.dispose(),b.dispose()},[O,re]);return t.jsxs("div",{style:S.root,children:[t.jsxs("div",{style:S.controls,children:[t.jsx("input",{ref:d,type:"file",accept:".glb,.gltf",style:{display:"none"},onChange:e=>{const a=e.target.files?.[0];a&&Pa(a)}}),t.jsx("button",{style:S.button,onClick:()=>d.current?.click(),children:"Upload GLB"}),t.jsxs("label",{style:S.field,children:[t.jsx("span",{children:"Color"}),t.jsx("input",{type:"color",value:x,onChange:e=>P(e.target.value),style:S.colorInput}),t.jsx("span",{style:S.mono,children:x.toUpperCase()})]}),t.jsxs("label",{style:S.field,title:"Background color — consumed by the FgBgMix brush setting (Phase 4). When the brush sets FgBgMix > 0, per-stamp color blends toward this picker.",children:[t.jsx("span",{children:"Bg"}),t.jsx("input",{type:"color",value:p,onChange:e=>w(e.target.value),style:S.colorInput})]}),t.jsxs("label",{style:S.field,children:[t.jsxs("span",{children:["Size ",h,"px"]}),t.jsx("input",{type:"range",min:2,max:80,value:h,onChange:e=>C(Number(e.target.value)),style:{width:120}})]}),t.jsxs("label",{style:S.field,title:"Parametric brush from the .brush.json library. Picking one replaces the active brush with its definition; sliders below reflect its base values.",children:[t.jsx("span",{children:"Brush"}),t.jsx("select",{value:tt,onChange:e=>xa(e.target.value),style:{padding:"4px 8px",background:"#3a3a3a",color:"#eaeaea",border:"1px solid rgba(255,255,255,0.08)",borderRadius:4,fontSize:11,cursor:"pointer",minWidth:150},children:Ba.map(e=>t.jsx("optgroup",{label:e.label,children:e.brushes.map(a=>t.jsx("option",{value:a.id,children:a.name},a.id))},e.label))})]}),t.jsxs("div",{style:S.field,title:"Legacy hand-tuned presets — kept for parity with prior PaintLab behavior. Pick a .brush.json above to switch to the parametric engine.",children:[t.jsx("span",{style:{color:"#888"},children:"Legacy"}),t.jsx("div",{style:{display:"flex",gap:0},children:Object.keys(K).map((e,a,n)=>{const f=wa===e,l=a===0,g=a===n.length-1;return t.jsx("button",{onClick:()=>Da(e),style:{padding:"5px 10px",background:f?"#5a5a5a":"#3a3a3a",color:"#eaeaea",border:"1px solid rgba(255,255,255,0.08)",borderRight:g?"1px solid rgba(255,255,255,0.08)":"none",borderTopLeftRadius:l?4:0,borderBottomLeftRadius:l?4:0,borderTopRightRadius:g?4:0,borderBottomRightRadius:g?4:0,fontSize:11,cursor:"pointer"},children:e},e)})})]}),t.jsxs("label",{style:S.field,children:[t.jsxs("span",{children:["Hardness ",Math.round(He*100),"%"]}),t.jsx("input",{type:"range",min:0,max:100,value:Math.round(He*100),onChange:e=>La(Number(e.target.value)/100),style:{width:100}})]}),t.jsxs("label",{style:S.field,children:[t.jsxs("span",{children:["Opacity ",Math.round(Ne*100),"%"]}),t.jsx("input",{type:"range",min:0,max:100,value:Math.round(Ne*100),onChange:e=>Ha(Number(e.target.value)/100),style:{width:100}})]}),t.jsxs("label",{style:S.field,children:[t.jsxs("span",{children:["Flow ",Math.round(Ge*100),"%"]}),t.jsx("input",{type:"range",min:0,max:100,value:Math.round(Ge*100),onChange:e=>Na(Number(e.target.value)/100),style:{width:100}})]}),t.jsxs("label",{style:S.field,title:"Ellipse aspect ratio. 100% = circle. Lower = thinner, calligraphic ellipse aligned to stroke direction on the surface.",children:[t.jsxs("span",{children:["Aspect ",Math.round(We*100),"%"]}),t.jsx("input",{type:"range",min:5,max:100,value:Math.round(We*100),onChange:e=>Ga(Number(e.target.value)/100),style:{width:100}})]}),t.jsxs("label",{style:S.field,children:[t.jsxs("span",{children:["Spacing ",Math.round(ze*100),"%"]}),t.jsx("input",{type:"range",min:0,max:100,value:Math.round(ze*100),onChange:e=>Wa(Number(e.target.value)/100),style:{width:100}})]}),t.jsxs("label",{style:S.field,title:"Lazy mouse: cursor lags behind pointer for smoother strokes.",children:[t.jsxs("span",{children:["Streamline ",Math.round(vt*100),"%"]}),t.jsx("input",{type:"range",min:0,max:99,value:Math.round(vt*100),onChange:e=>za(Number(e.target.value)/100),style:{width:100}})]}),t.jsxs("label",{style:S.field,title:"Faster strokes shrink the brush — pen-like response. 0% = constant size.",children:[t.jsxs("span",{children:["Velocity ",Math.round(bt*100),"%"]}),t.jsx("input",{type:"range",min:0,max:100,value:Math.round(bt*100),onChange:e=>Oa(Number(e.target.value)/100),style:{width:100}})]}),t.jsxs("label",{style:S.field,title:"Start-taper: brush fades in over the first N stamps of a stroke for a hand-drawn feel.",children:[t.jsxs("span",{children:["Taper ",Math.round(xt*100),"%"]}),t.jsx("input",{type:"range",min:0,max:100,value:Math.round(xt*100),onChange:e=>Ea(Number(e.target.value)/100),style:{width:100}})]}),t.jsxs("label",{style:S.field,title:"Brush shrinks where surface curvature is high (around hard edges). 3D-only feature with no 2D equivalent.",children:[t.jsxs("span",{children:["Curvature ",Math.round(yt*100),"%"]}),t.jsx("input",{type:"range",min:0,max:100,value:Math.round(yt*100),onChange:e=>_a(Number(e.target.value)/100),style:{width:100}})]}),t.jsxs("label",{style:S.field,children:[t.jsx("input",{type:"checkbox",checked:O,onChange:e=>U(e.target.checked)}),t.jsx("span",{children:"Seam gate (Solution 1)"})]}),t.jsxs("label",{style:S.field,title:"Render a depth pre-pass each stamp; discard brush fragments that are hidden behind closer geometry. Stops paint from bleeding through to the back of the model.",children:[t.jsx("input",{type:"checkbox",checked:At,onChange:e=>ba(e.target.checked)}),t.jsx("span",{children:"Block backside"})]}),t.jsxs("label",{style:S.field,children:[t.jsxs("span",{children:["Shading ",Math.round(E*100),"%"]}),t.jsx("input",{type:"range",min:0,max:100,value:Math.round(E*100),onChange:e=>ge(Number(e.target.value)/100),style:{width:120}})]}),t.jsxs("label",{style:S.field,children:[t.jsxs("span",{children:["Reflect ",Math.round(oe*100),"%"]}),t.jsx("input",{type:"range",min:0,max:100,value:Math.round(oe*100),onChange:e=>be(Number(e.target.value)/100),style:{width:120}})]}),t.jsxs("label",{style:S.field,title:"Stop paint at hard edges (split-normal seams). 0 = off; higher = stricter angle gate.",children:[t.jsxs("span",{children:["Edge snap ",Math.round(De*100),"%"]}),t.jsx("input",{type:"range",min:0,max:100,value:Math.round(De*100),onChange:e=>Pt(Number(e.target.value)/100),style:{width:120}})]}),t.jsxs("div",{style:S.field,children:[t.jsx("span",{children:"View"}),t.jsx("div",{style:{display:"flex",gap:0},children:["color","metalness","roughness"].map((e,a,n)=>t.jsx("button",{onClick:()=>Z(e),style:{padding:"5px 10px",background:W===e?"#5a5a5a":"#3a3a3a",color:"#eaeaea",border:"1px solid rgba(255,255,255,0.08)",borderRight:a<n.length-1?"none":"1px solid rgba(255,255,255,0.08)",borderTopLeftRadius:a===0?4:0,borderBottomLeftRadius:a===0?4:0,borderTopRightRadius:a===n.length-1?4:0,borderBottomRightRadius:a===n.length-1?4:0,fontSize:11,cursor:"pointer"},children:e==="color"?"Color":e==="metalness"?"Metal":"Rough"},e))})]}),Ft.length>1&&t.jsxs("div",{style:S.field,children:[t.jsx("span",{children:"Material"}),t.jsx("div",{style:{display:"flex",gap:0,flexWrap:"wrap"},children:Ft.map((e,a)=>{const n=a===re,f=a===0,l=a===Ft.length-1;return t.jsxs("button",{onClick:()=>Dt(a),title:e.name,style:{display:"inline-flex",alignItems:"center",gap:6,padding:"4px 10px",background:n?"#5a5a5a":"#3a3a3a",color:"#eaeaea",border:"1px solid rgba(255,255,255,0.08)",borderRight:l?"1px solid rgba(255,255,255,0.08)":"none",borderTopLeftRadius:f?4:0,borderBottomLeftRadius:f?4:0,borderTopRightRadius:l?4:0,borderBottomRightRadius:l?4:0,fontSize:11,cursor:"pointer",maxWidth:140},children:[t.jsx("span",{style:{width:14,height:14,borderRadius:3,background:e.swatchColor,border:"1px solid rgba(0,0,0,0.4)",flexShrink:0}}),t.jsx("span",{style:{overflow:"hidden",textOverflow:"ellipsis",whiteSpace:"nowrap"},children:e.name})]},`${e.slotIndex}-${e.name}`)})})]}),t.jsx("button",{style:{...S.button,background:xe?"#4a4a4a":"#3a3a3a"},onClick:()=>St(e=>!e),title:"Open the parametric brush editor (Phase 7)",children:xe?"Studio ▴":"Studio ▾"}),t.jsx("button",{style:S.button,onClick:Rt,disabled:!B,title:"Undo last stroke (Ctrl+Z / Cmd+Z)",children:"↶ Undo"}),t.jsx("button",{style:S.button,onClick:Lt,disabled:!B,title:"Redo (Ctrl+Shift+Z / Ctrl+Y)",children:"↷ Redo"}),t.jsx("button",{style:S.button,onClick:Xa,disabled:!B,children:"Clear"}),t.jsx("button",{style:S.button,onClick:Ja,disabled:!B,children:"Download Texture"}),t.jsx("span",{style:S.hint,children:"Left-drag: paint · Right-drag: orbit · Wheel: zoom · Ctrl+Z: undo"}),t.jsx("span",{style:S.status,children:A})]}),t.jsxs("div",{style:S.smoothBar,children:[t.jsx("span",{style:S.smoothLabel,children:"Smooth Colors"}),t.jsxs("div",{style:S.field,title:"Gaussian: spatial blur only (washes small colours fastest). Bilateral: also weights by colour similarity — preserves boundaries and small marks. Variance-gated: Gaussian, but reduced where local colour contrast is high.",children:[t.jsx("span",{children:"Method"}),t.jsx("div",{style:{display:"flex",gap:0},children:[{id:"gaussian",label:"Gaussian"},{id:"bilateral",label:"Bilateral"},{id:"variance-gated",label:"Variance"}].map((e,a,n)=>{const f=Oe===e.id,l=a===0,g=a===n.length-1;return t.jsx("button",{onClick:()=>Ma(e.id),style:{padding:"5px 10px",background:f?"#5a5a5a":"#3a3a3a",color:"#eaeaea",border:"1px solid rgba(255,255,255,0.08)",borderRight:g?"1px solid rgba(255,255,255,0.08)":"none",borderTopLeftRadius:l?4:0,borderBottomLeftRadius:l?4:0,borderTopRightRadius:g?4:0,borderBottomRightRadius:g?4:0,fontSize:11,cursor:"pointer"},children:e.label},e.id)})})]}),t.jsxs("label",{style:S.field,title:"How far each pixel blends toward the smoothed value.",children:[t.jsxs("span",{children:["Strength ",Math.round(lt*100),"%"]}),t.jsx("input",{type:"range",min:0,max:100,value:Math.round(lt*100),onChange:e=>Ca(Number(e.target.value)/100),style:{width:100}})]}),t.jsxs("label",{style:S.field,title:"Kernel half-width in texels — larger spreads colour further per pass.",children:[t.jsxs("span",{children:["Kernel ",ct,"px"]}),t.jsx("input",{type:"range",min:1,max:Zt,value:ct,onChange:e=>Ra(Number(e.target.value)),style:{width:100}})]}),t.jsxs("label",{style:S.field,title:"Repeat the pass N times — stacks smoothing without a huge kernel.",children:[t.jsxs("span",{children:["Iterations ",dt,"×"]}),t.jsx("input",{type:"range",min:1,max:6,value:dt,onChange:e=>ka(Number(e.target.value)),style:{width:90}})]}),Oe==="bilateral"&&t.jsxs("label",{style:S.field,title:"Colour sigma — smaller preserves colour boundaries and small marks more aggressively.",children:[t.jsxs("span",{children:["Colour σ ",Mt.toFixed(2)]}),t.jsx("input",{type:"range",min:2,max:60,value:Math.round(Mt*100),onChange:e=>Ta(Number(e.target.value)/100),style:{width:100}})]}),Oe==="variance-gated"&&t.jsxs("label",{style:S.field,title:"Variance threshold — above this much local colour contrast, smoothing is suppressed (edges / small marks preserved).",children:[t.jsxs("span",{children:["Edge thr ",Ct.toFixed(3)]}),t.jsx("input",{type:"range",min:1,max:40,value:Math.round(Ct*200),onChange:e=>ja(Number(e.target.value)/200),style:{width:100}})]}),t.jsx("button",{style:{...S.button,background:"#3f5a3f"},onClick:$a,disabled:!B,title:"Smooth all colours on the active material (island-aware). Reversible with Undo / Ctrl+Z.",children:"Apply Smooth"}),t.jsx("button",{style:S.button,onClick:Rt,disabled:!B,title:"Undo the last action (Ctrl+Z / Cmd+Z)",children:"↶ Undo"})]}),t.jsxs("div",{style:S.viewSplit,children:[t.jsx("canvas",{ref:r,style:S.canvas,onPointerDown:Aa,onPointerMove:Ia,onPointerUp:la,onPointerCancel:la,onPointerLeave:Ua,onContextMenu:e=>e.preventDefault(),onWheel:Fa}),t.jsxs("div",{style:S.rightColumn,children:[t.jsxs("div",{style:S.rightTabs,children:[t.jsx("button",{style:{...S.rightTab,background:xe?"transparent":"#3a3a3a",color:xe?"#979797":"#eaeaea"},onClick:()=>St(!1),children:"UV View"}),t.jsx("button",{style:{...S.rightTab,background:xe?"#3a3a3a":"transparent",color:xe?"#eaeaea":"#979797"},onClick:()=>St(!0),children:"Brush Studio"})]}),t.jsx("canvas",{ref:s,style:{...S.canvasUV,display:xe?"none":"block"}}),xe&&t.jsx(Gr,{brushDef:ae.current,revision:Sa,onMutated:Ht,onPasteJson:Ya,onResetToPreset:Va,onClose:()=>St(!1)})]})]})]})}const S={root:{display:"flex",flexDirection:"column",width:"100%",height:"calc(100vh - 120px)",minHeight:600,background:"#1a1a1a",color:"#eaeaea",borderRadius:6,overflow:"hidden"},controls:{display:"flex",alignItems:"center",gap:16,padding:"10px 14px",background:"#232323",fontSize:12,borderBottom:"1px solid rgba(255,255,255,0.06)",flexWrap:"wrap"},smoothBar:{display:"flex",alignItems:"center",gap:14,padding:"8px 14px",background:"#1f1f1f",fontSize:12,borderBottom:"1px solid rgba(255,255,255,0.06)",flexWrap:"wrap"},smoothLabel:{fontWeight:600,letterSpacing:.3,color:"#eaeaea",marginRight:4},button:{padding:"6px 12px",background:"#3a3a3a",color:"#eaeaea",border:"1px solid rgba(255,255,255,0.08)",borderRadius:4,fontSize:12,cursor:"pointer"},field:{display:"inline-flex",alignItems:"center",gap:8},colorInput:{width:28,height:22,padding:0,border:"1px solid rgba(255,255,255,0.12)",borderRadius:3,background:"transparent",cursor:"pointer"},mono:{fontFamily:"ui-monospace, SFMono-Regular, Menlo, monospace",color:"#b7b7b7"},hint:{color:"#979797",fontSize:11},status:{marginLeft:"auto",color:"#979797",fontSize:11},viewSplit:{flex:1,display:"flex",minHeight:0,minWidth:0,overflow:"hidden"},canvas:{flex:1,height:"100%",minWidth:0,display:"block",touchAction:"none",borderRight:"1px solid rgba(255,255,255,0.06)"},rightColumn:{flex:1,display:"flex",flexDirection:"column",minWidth:0,minHeight:0},rightTabs:{display:"flex",borderBottom:"1px solid rgba(255,255,255,0.06)",background:"#1a1a1a",flexShrink:0},rightTab:{flex:1,padding:"6px 10px",border:0,borderRight:"1px solid rgba(255,255,255,0.06)",fontSize:11,cursor:"pointer",fontWeight:600,letterSpacing:.3},canvasUV:{flex:1,minWidth:0,minHeight:0,display:"block",touchAction:"none"}};export{kn as PaintLab};
