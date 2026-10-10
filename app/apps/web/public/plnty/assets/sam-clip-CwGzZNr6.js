const __vite__mapDeps=(i,m=__vite__mapDeps,d=(m.f||(m.f=["assets/index-DLuMKoLC.js","assets/__vite-browser-external-ke9ZaMpT.js"])))=>i.map(i=>d[i]);
import{mP as O,mQ as P,mR as G,mS as z,x as $,ao as N}from"./main-DamfXklH.js";import{d as M}from"./react-C9WEJgbf.js";const k=t=>Number.isInteger(t)?t.toFixed(1):String(t),U=`vec3(${P.map(k).join(", ")})`,ot=`
attribute vec2 a_position;
varying vec2 v_uv;
void main() {
  // a_position is in NDC [-1, 1]. Map to UV [0, 1] and flip Y so the texture
  // (which has origin top-left in canvas convention) reads upright.
  v_uv = a_position * 0.5 + 0.5;
  v_uv.y = 1.0 - v_uv.y;
  gl_Position = vec4(a_position, 0.0, 1.0);
}
`,it=`
precision highp float;
varying vec2 v_uv;
uniform sampler2D u_src;
uniform vec2 u_resolution;
uniform float u_time;
uniform float u_grainI;
uniform float u_grainSize;
uniform float u_grainColor;
uniform float u_caAmount;
uniform float u_caEdgeMask;
uniform float u_pop;
uniform float u_boost;
// Color Correct grade controls applied BEFORE the existing CA / pop / boost /
// grain chain so the wheel pick + master sliders set the look, and finishing
// effects sit on top.
uniform float u_exposure;
uniform float u_masterSat;
uniform float u_masterContrast;
uniform vec3  u_lggShadowsRgb;
uniform float u_lggShadowsY;
uniform vec3  u_lggMidtonesRgb;
uniform float u_lggMidtonesY;
uniform vec3  u_lggHighlightsRgb;
uniform float u_lggHighlightsY;

// Jarzynski-style "hash without sine". Stays float32-stable at high resolutions
// (compresses input to [0,1) before any large multiplies).
float hash(vec2 p) {
  vec3 p3 = fract(vec3(p.xyx) * 0.1031);
  p3 += dot(p3, p3.yzx + 33.33);
  return fract((p3.x + p3.y) * p3.z);
}

float valueNoise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  float a = hash(i);
  float b = hash(i + vec2(1.0, 0.0));
  float c = hash(i + vec2(0.0, 1.0));
  float d = hash(i + vec2(1.0, 1.0));
  vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

void main() {
  vec2 uv = v_uv;
  vec2 toCenter = uv - vec2(0.5);
  float dist = length(toCenter);

  // CA — radial RGB split, masked toward edges so corrections aren't fringe-heavy.
  float edgeT = mix(0.0, 0.5, u_caEdgeMask);
  float edgeMask = smoothstep(edgeT, 0.707, dist);
  float caStrength = u_caAmount * edgeMask;
  vec2 dir = (dist > 1e-4) ? (toCenter / dist) : vec2(1.0, 0.0);
  float maxOffsetPx = 12.0;
  vec2 maxOffset = (dir * maxOffsetPx) / u_resolution;

  vec3 col;
  col.r = texture2D(u_src, uv + maxOffset * caStrength).r;
  col.g = texture2D(u_src, uv).g;
  col.b = texture2D(u_src, uv - maxOffset * caStrength).b;

  // ── Color Correct grade chain (runs BEFORE pop/boost/grain) ──
  // Exposure first — operates uniformly on the unmodified RGB.
  col *= pow(2.0, u_exposure);

  // 3-way LGG: per-tonal-range RGB offsets + luma adjust. Luminance is
  // computed once on the CA'd colour so shifts are perceptually weighted.
  // The constants are the Grade layer's too (visual-fx/grade-math.ts).
  float gradeLum = dot(col, ${U});
  float shadowMask = smoothstep(0.5, 0.0, gradeLum);
  float highlightMask = smoothstep(0.5, 1.0, gradeLum);
  float midMask = max(0.0, 1.0 - shadowMask - highlightMask);

  const float rgbScale = ${k(G)};
  col += (u_lggShadowsRgb * shadowMask
        + u_lggMidtonesRgb * midMask
        + u_lggHighlightsRgb * highlightMask) * rgbScale;

  const float lumaScale = ${k(z)};
  float lumaShift = u_lggShadowsY * shadowMask
                  + u_lggMidtonesY * midMask
                  + u_lggHighlightsY * highlightMask;
  col += col * (lumaShift * lumaScale);

  // Master saturation.
  float masterLum = dot(col, ${U});
  col = mix(vec3(masterLum), col, 1.0 + u_masterSat);

  // Master contrast — linear stretch around 0.5.
  col = (col - vec3(0.5)) * (1.0 + u_masterContrast) + vec3(0.5);

  col = clamp(col, 0.0, 1.0);

  // Pop — saturation lift + S-curve contrast, single combined dial.
  float lum = dot(col, vec3(0.299, 0.587, 0.114));
  vec3 satted = mix(vec3(lum), col, 1.0 + u_pop * 0.6);
  vec3 contrasted = mix(satted, smoothstep(vec3(0.0), vec3(1.0), satted), u_pop * 0.3);
  col = contrasted;

  // Boost — midtone lift + highlight extension.
  float midWeight = 1.0 - abs(lum * 2.0 - 1.0);
  col += col * (u_boost * 0.4 * midWeight);
  col = mix(col, col * 1.15, u_boost * smoothstep(0.6, 1.0, lum));
  col = clamp(col, 0.0, 1.0);

  // Grain on top — corrections don't soften the texture.
  vec2 grainUv = uv * u_resolution / max(u_grainSize, 0.5) / 4.0;
  grainUv = mod(grainUv, 256.0);
  float lf = valueNoise(grainUv + u_time);
  float hf = hash(grainUv * 8.0 + u_time * 2.0);
  float grainMono = (lf * 0.5 + hf * 0.5) - 0.5;

  if (u_grainColor > 0.5) {
    vec3 grainRGB = vec3(
      hash(grainUv + vec2(u_time + 0.1, 0.37)),
      hash(grainUv + vec2(u_time + 0.7, 1.13)),
      hash(grainUv + vec2(u_time + 0.3, 2.71))
    ) - 0.5;
    col += grainRGB * u_grainI;
  } else {
    col += grainMono * u_grainI;
  }

  col = clamp(col, 0.0, 1.0);
  gl_FragColor = vec4(col, 1.0);
}
`,S={r:0,g:0,b:0,y:0},Y={shadows:{...S},midtones:{...S},highlights:{...S}};function p(t,e,o,i){const a=typeof t=="number"?t:typeof t=="string"?parseFloat(t):NaN;return Number.isFinite(a)?Math.max(o,Math.min(i,a)):e}function nt(t,e){const o=typeof t=="number"?t:typeof t=="string"?parseInt(t,10):NaN;return Number.isFinite(o)?o:e}function H(t,e){return typeof t=="boolean"?t:e}function x(t){if(typeof t!="object"||t===null)return{...S};const e=t;return{r:typeof e.r=="number"&&Number.isFinite(e.r)?e.r:0,g:typeof e.g=="number"&&Number.isFinite(e.g)?e.g:0,b:typeof e.b=="number"&&Number.isFinite(e.b)?e.b:0,y:typeof e.y=="number"&&Number.isFinite(e.y)?e.y:0}}function j(t){if(typeof t!="object"||t===null)return Y;const e=t;return{shadows:x(e.shadows),midtones:x(e.midtones),highlights:x(e.highlights)}}function rt(t,e){const o=O(t,e);return{grainIntensity:o?p(t.grainIntensity,.13,0,1):0,grainSize:p(t.grainSize,.7,.5,4),grainColor:H(t.grainColor,!0),caAmount:o?p(t.caAmount,.87,0,1):0,caEdgeMask:p(t.caEdgeMask,.57,0,1),pop:p(t.pop,.08,0,1),boost:p(t.boost,.32,0,1),exposure:p(t.exposure,0,-2,2),masterSat:p(t.masterSat,0,-1,1),masterContrast:p(t.masterContrast,0,-1,1),lgg:j(t.lgg)}}const I=4096;function at(t,e,o){const i=Math.min(t,e),a=Math.max(t,e);let n=o>0&&i>o?o/i:1;if(a*n>I&&(n=I/a),n>=1)return{width:t&-2,height:e&-2};const r=Math.round(t*n)&-2,s=Math.round(e*n)&-2;return{width:r,height:s}}function st(t,e,o){const i=D(t,t.VERTEX_SHADER,e),a=D(t,t.FRAGMENT_SHADER,o),n=t.createProgram();if(!n)throw new Error("Color Correct: failed to create GL program.");if(t.attachShader(n,i),t.attachShader(n,a),t.linkProgram(n),!t.getProgramParameter(n,t.LINK_STATUS)){const r=t.getProgramInfoLog(n)??"(no info)";throw t.deleteProgram(n),new Error(`Color Correct: GL link failed -- ${r}`)}return t.deleteShader(i),t.deleteShader(a),n}function D(t,e,o){const i=t.createShader(e);if(!i)throw new Error("Color Correct: failed to create GL shader.");if(t.shaderSource(i,o),t.compileShader(i),!t.getShaderParameter(i,t.COMPILE_STATUS)){const a=t.getShaderInfoLog(i)??"(no info)";throw t.deleteShader(i),new Error(`Color Correct: GL shader compile failed -- ${a}`)}return i}function ut(t,e){return{u_src:t.getUniformLocation(e,"u_src"),u_resolution:t.getUniformLocation(e,"u_resolution"),u_time:t.getUniformLocation(e,"u_time"),u_grainI:t.getUniformLocation(e,"u_grainI"),u_grainSize:t.getUniformLocation(e,"u_grainSize"),u_grainColor:t.getUniformLocation(e,"u_grainColor"),u_caAmount:t.getUniformLocation(e,"u_caAmount"),u_caEdgeMask:t.getUniformLocation(e,"u_caEdgeMask"),u_pop:t.getUniformLocation(e,"u_pop"),u_boost:t.getUniformLocation(e,"u_boost"),u_exposure:t.getUniformLocation(e,"u_exposure"),u_masterSat:t.getUniformLocation(e,"u_masterSat"),u_masterContrast:t.getUniformLocation(e,"u_masterContrast"),u_lggShadowsRgb:t.getUniformLocation(e,"u_lggShadowsRgb"),u_lggShadowsY:t.getUniformLocation(e,"u_lggShadowsY"),u_lggMidtonesRgb:t.getUniformLocation(e,"u_lggMidtonesRgb"),u_lggMidtonesY:t.getUniformLocation(e,"u_lggMidtonesY"),u_lggHighlightsRgb:t.getUniformLocation(e,"u_lggHighlightsRgb"),u_lggHighlightsY:t.getUniformLocation(e,"u_lggHighlightsY")}}function ct(t,e,o){t.uniform1f(e.u_grainI,o.grainIntensity),t.uniform1f(e.u_grainSize,o.grainSize),t.uniform1f(e.u_grainColor,o.grainColor?1:0),t.uniform1f(e.u_caAmount,o.caAmount),t.uniform1f(e.u_caEdgeMask,o.caEdgeMask),t.uniform1f(e.u_pop,o.pop),t.uniform1f(e.u_boost,o.boost)}function ft(t,e,o){t.uniform1f(e.u_exposure,o.exposure),t.uniform1f(e.u_masterSat,o.masterSat),t.uniform1f(e.u_masterContrast,o.masterContrast),t.uniform3f(e.u_lggShadowsRgb,o.lgg.shadows.r,o.lgg.shadows.g,o.lgg.shadows.b),t.uniform1f(e.u_lggShadowsY,o.lgg.shadows.y),t.uniform3f(e.u_lggMidtonesRgb,o.lgg.midtones.r,o.lgg.midtones.g,o.lgg.midtones.b),t.uniform1f(e.u_lggMidtonesY,o.lgg.midtones.y),t.uniform3f(e.u_lggHighlightsRgb,o.lgg.highlights.r,o.lgg.highlights.g,o.lgg.highlights.b),t.uniform1f(e.u_lggHighlightsY,o.lgg.highlights.y)}const v=2,B=127,b=new Uint8Array(5);function W(t,e){const o=t.length;for(let i=0;i<o;i++){const a=t[i][e];let n=i;for(;n>0&&b[n-1]>a;)b[n]=b[n-1],n--;b[n]=a}return b[o>>1]}const L=new Int8Array(511);for(let t=-255;t<=255;t++)L[t+255]=Math.round(t*B/255);function X(t,e,o,i){const a=o*i,n=new Int8Array(a);if(t.length===5){const r=t[0],s=t[1],c=t[2],f=t[3],m=t[4];for(let l=0;l<a;l++){let h=r[l],d=s[l],u=c[l],_=f[l],g;h>d&&(g=h,h=d,d=g),u>_&&(g=u,u=_,_=g),h>u&&(g=h,h=u,u=g,g=d,d=_,_=g),h=m[l],h>d&&(g=h,h=d,d=g),h>u&&(g=h,h=u,u=g,g=d,d=_,_=g),n[l]=L[(d<u?d:u)-e[l]+255]}}else if(t.length>1)for(let r=0;r<a;r++)n[r]=L[W(t,r)-e[r]+255];return{data:n,width:o,height:i}}class ht{constructor(e,o,i="all"){this.width=e,this.height=o,this.retain=i}frames=new Map;last=null;set(e,o){if(o.length!==this.width*this.height)throw new Error(`LumaFrames: frame ${e} is ${o.length} bytes, expected ${this.width*this.height}.`);this.frames.set(e,o)}finish(e){this.last=e}get size(){return this.frames.size}deltaAt(e){const o=this.frames.get(e);if(!o||this.last===null&&!this.frames.has(e+v))return null;const i=this.last===null?e+v:Math.min(this.last,e+v),a=[];for(let r=Math.max(0,e-v);r<=i;r++){const s=this.frames.get(r);s&&a.push(s)}const n=X(a,o,this.width,this.height);if(this.retain==="window")for(const r of this.frames.keys())r<e-v&&this.frames.delete(r);return n}}function lt(t,e,o){if(t.width===e&&t.height===o)return t;const{data:i,width:a,height:n}=t,r=new Int8Array(e*o);for(let s=0;s<o;s++){const c=Math.min(n-1,Math.max(0,(s+.5)*n/o-.5)),f=c|0,m=Math.min(n-1,f+1),l=c-f;for(let h=0;h<e;h++){const d=Math.min(a-1,Math.max(0,(h+.5)*a/e-.5)),u=d|0,_=Math.min(a-1,u+1),g=d-u,T=i[f*a+u]*(1-g)+i[f*a+_]*g,F=i[m*a+u]*(1-g)+i[m*a+_]*g;r[s*e+h]=Math.round(T*(1-l)+F*l)}}return{data:r,width:e,height:o}}const y=$("VisualFx");function A(t){return new ImageData(new Uint8ClampedArray(t.buffer),t.width,t.height)}class V{worker=null;nextId=0;pending=new Map;disposed=!1;getWorker(){if(this.disposed)throw new Error("VisualFxWorker has been disposed.");if(this.worker)return this.worker;const e=new Worker(new URL("/assets/visual-fx-worker-DrSuI1GY.js",import.meta.url),{type:"module"});return e.onmessage=o=>{const{id:i,result:a,error:n}=o.data,r=this.pending.get(i);r&&(this.pending.delete(i),n!==void 0?r.reject(new Error(n)):r.resolve(a))},e.onerror=o=>{const i=new Error(`Visual FX worker error: ${o.message}`);y.error("Worker crashed, rejecting",this.pending.size,"pending job(s)"),this.rejectAll(i);try{e.terminate()}catch{}this.worker===e&&(this.worker=null)},this.worker=e,e}rejectAll(e){for(const[,o]of this.pending)o.reject(e);this.pending.clear()}send(e,o){return new Promise((i,a)=>{const n=this.nextId++;this.pending.set(n,{resolve:i,reject:a}),this.getWorker().postMessage({id:n,command:e},o)})}async apply(e,o,i,a){const n=await this.send({type:"apply",effectId:e,values:o,width:i.width,height:i.height,buffer:i.data.buffer,frame:a},[i.data.buffer]);return A(n)}async applyMany(e,o,i){if(e.length===0)return[];const a=o.data.slice().buffer;return(await this.send({type:"apply-many",specs:e,width:o.width,height:o.height,buffer:a,frame:i},[a])).map(r=>({effectId:r.effectId,image:r.frame?A(r.frame):void 0,error:r.error}))}async applyChain(e,o,i,a,n,r,s,c){const f=[o.data.buffer];let m;if(n)if(n.width!==o.width||n.height!==o.height)y.warn(`Dropping a steady correction of size ${n.width}x${n.height} for a frame of size ${o.width}x${o.height}.`);else{const{data:u}=n,_=u.byteOffset===0&&u.byteLength===u.buffer.byteLength?u.buffer:u.slice().buffer;m={buffer:_},f.push(_)}let l;if(r)if(r.width!==o.width||r.height!==o.height)y.warn(`Dropping a depth plane of size ${r.width}x${r.height} for a frame of size ${o.width}x${o.height}.`);else{const u=r.data.slice().buffer;l={buffer:u},f.push(u)}let h;s&&s.some(Boolean)&&(h=s.map(u=>{if(!u)return null;if(u.length!==o.width*o.height)return y.warn(`Dropping a SAM plane of ${u.length} bytes for a frame of size ${o.width}x${o.height}.`),null;const _=u.slice().buffer;return f.push(_),{buffer:_}}));const d=await this.send({type:"apply-chain",layers:e,width:o.width,height:o.height,buffer:o.data.buffer,frame:i,want:a,steady:m,depth:l,sam:h,cacheKey:c},f);return A(d)}dispose(){this.disposed||(this.disposed=!0,this.rejectAll(new Error("Visual FX worker disposed.")),this.worker?.terminate(),this.worker=null)}}let E=null;function gt(){return E||(E=new V),E}const q=8,w=new Map;function J(t){const e=w.get(t);if(e)return w.delete(t),w.set(t,e),e;const o=(async()=>{const i=await N(t);if(!i)throw new Error(`SAM matte: could not reach ${t}.`);try{return await(await fetch(i)).blob()}finally{URL.revokeObjectURL(i)}})();for(o.catch(()=>{w.get(t)===o&&w.delete(t)}),w.set(t,o);w.size>q;){const i=w.keys().next().value;if(i===void 0)break;w.delete(i)}return o}async function C(t){const e=await M(()=>import("./index-DLuMKoLC.js"),__vite__mapDeps([0,1])),o=new e.Input({source:new e.BlobSource(await J(t)),formats:e.ALL_FORMATS}),i=await o.getPrimaryVideoTrack();if(!i)throw o.dispose(),new Error("SAM matte: the matte clip has no picture.");const a=await i.getFirstTimestamp(),n=Math.max(0,await o.computeDuration([i])-a);return{input:o,track:i,start:a,duration:n}}function R(t,e,o,i){i.width!==e&&(i.width=e),i.height!==o&&(i.height=o);const a=i.getContext("2d",{willReadFrequently:!0});if(!a)throw new Error("SAM matte: no 2D context to read the matte.");a.drawImage(t,0,0,e,o);const n=a.getImageData(0,0,e,o).data,r=new Uint8Array(e*o);for(let s=0;s<r.length;s++)r[s]=n[s*4]>127?255:0;return r}function K(t,e,o){const i=new Map,a=r=>{r.out&&r.reads===0&&r.ready.then(e).catch(()=>{})},n=r=>{const s=i.get(r);if(s)return i.delete(r),i.set(r,s),s;const c={ready:t(r),reads:0,out:!1};for(i.set(r,c),c.ready.catch(()=>{i.get(r)===c&&i.delete(r)});i.size>o;){const f=i.keys().next().value;if(f===void 0)break;const m=i.get(f);i.delete(f),m.out=!0,a(m)}return c};return{async read(r,s){const c=n(r);c.reads+=1;try{return await s(await c.ready)}finally{c.reads-=1,a(c)}}}}const Q=K(async t=>{const e=await C(t),o=await M(()=>import("./index-DLuMKoLC.js"),__vite__mapDeps([0,1]));return{matte:e,sink:new o.CanvasSink(e.track,{poolSize:0})}},({matte:t})=>t.input.dispose(),6),Z=typeof OffscreenCanvas>"u"?null:new OffscreenCanvas(1,1);async function dt(t,e,o,i){const a=Z;if(!a)throw new Error("SAM matte: this browser cannot read a matte clip.");return Q.read(t,async({matte:n,sink:r})=>{const s=await r.getCanvas(n.start+Math.max(0,e))??await r.getCanvas(n.start);if(!s)throw new Error("SAM matte: the matte clip has no frame there.");return R(s.canvas,o,i,a)})}async function mt(t,e,o){const i=await C(t),a=await M(()=>import("./index-DLuMKoLC.js"),__vite__mapDeps([0,1])),n=new a.CanvasSink(i.track,{poolSize:0}).canvases(),r=new OffscreenCanvas(e,o),s=await n.next();if(s.done)throw i.input.dispose(),new Error("SAM matte: the matte clip has no frames.");let c=s.value,f=await n.next(),m=null;return{async planeAt(l){const h=i.start+l+.001;for(;!f.done&&f.value.timestamp<=h;)c=f.value,m=null,f=await n.next();return m??=R(c.canvas,e,o,r),m},dispose(){n.return(void 0).catch(()=>{}),i.input.dispose()}}}async function _t(t,e=9){let o=null;try{const i=await C(t);o=i;const a=await M(()=>import("./index-DLuMKoLC.js"),__vite__mapDeps([0,1])),n=new a.CanvasSink(i.track,{width:96,poolSize:0}),r=Array.from({length:e},(c,f)=>i.start+i.duration*(f+.5)/e),s=new OffscreenCanvas(96,96);for await(const c of n.canvasesAtTimestamps(r)){if(!c)continue;const f=Math.max(1,Math.round(c.canvas.width)),m=Math.max(1,Math.round(c.canvas.height));if(R(c.canvas,f,m,s).some(l=>l>0))return!0}return!1}catch{return!0}finally{o?.input.dispose()}}export{it as F,ht as L,V,ot as a,ft as b,st as c,H as d,at as e,dt as f,_t as g,K as h,ut as l,mt as o,nt as p,rt as r,gt as s,lt as u,ct as w};
