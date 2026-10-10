import{x as we}from"./main-DamfXklH.js";import{d as Ut}from"./react-C9WEJgbf.js";import{p as Jt,al as Fe,bp as wo,f as Hr,U as jr,X as Ht,$ as ar,Z as So,ab as Po,h as Co,D as Eo,au as To,M as Bo,S as Fo,C as Mo,k as Zr}from"./three-CbhGz-nM.js";const ur=we("WebGPU");class zo{context=null;initPromise=null;deviceLostCallbacks=new Set;async checkSupport(){if(typeof navigator>"u"||!navigator.gpu)return{supported:!1,reason:"webgpu-unavailable"};try{const e=await navigator.gpu.requestAdapter({powerPreference:"high-performance"});if(!e)return{supported:!1,reason:"no-adapter"};const i=e.limits,o=4096*64*4;if(i.maxStorageBufferBindingSize<o)return{supported:!1,reason:"insufficient-limits",adapter:e};const n=this.getMaxAllowedResolution(i),u=this.getRecommendedResolution(n);return{supported:!0,adapter:e,recommendedResolution:u,maxAllowedResolution:n}}catch(e){return console.error("[WebGPU] Support check failed:",e),{supported:!1,reason:"webgpu-unavailable"}}}async initialize(){return this.context?this.context:this.initPromise?this.initPromise:(this.initPromise=this.doInitialize(),this.initPromise)}async doInitialize(){const e=await this.checkSupport();if(!e.supported||!e.adapter)return console.warn("[WebGPU] Not supported:",e.reason),null;try{const o=Math.min(e.adapter.limits.maxComputeInvocationsPerWorkgroup,512);if(o<512)return console.warn(`[WebGPU] Adapter only supports ${o} invocations per workgroup, need 512`),null;const n=await e.adapter.requestDevice({requiredLimits:{maxStorageBufferBindingSize:e.adapter.limits.maxStorageBufferBindingSize,maxBufferSize:e.adapter.limits.maxBufferSize,maxComputeWorkgroupSizeX:8,maxComputeWorkgroupSizeY:8,maxComputeWorkgroupSizeZ:8,maxComputeInvocationsPerWorkgroup:512}});return n.lost.then(u=>{console.error("[WebGPU] Device lost:",u.message),this.handleDeviceLoss()}),n.onuncapturederror=u=>{console.error("[WebGPU] Uncaptured error:",u.error)},this.context={adapter:e.adapter,device:n,limits:n.limits,features:n.features},ur.info("Context initialized successfully"),ur.info("Max storage buffer:",this.formatBytes(n.limits.maxStorageBufferBindingSize)),ur.info("Recommended resolution:",e.recommendedResolution),this.context}catch(i){return console.error("[WebGPU] Device request failed:",i),null}}getContext(){return this.context}isAvailable(){return this.context!==null}onDeviceLost(e){return this.deviceLostCallbacks.add(e),()=>{this.deviceLostCallbacks.delete(e)}}destroy(){this.context&&(this.context.device.destroy(),this.context=null),this.initPromise=null,this.deviceLostCallbacks.clear()}handleDeviceLoss(){this.context=null,this.initPromise=null;for(const e of this.deviceLostCallbacks)try{e()}catch(i){console.error("[WebGPU] Device loss callback error:",i)}}getMaxAllowedResolution(e){const i=e.maxStorageBufferBindingSize;return i>=512*512*512*4?512:i>=256*256*256*4?256:i>=16384*128*4?128:64}getRecommendedResolution(e){return e>=128?128:64}formatBytes(e){return e>=1024*1024*1024?`${(e/(1024*1024*1024)).toFixed(1)}GB`:e>=1024*1024?`${(e/(1024*1024)).toFixed(1)}MB`:e>=1024?`${(e/1024).toFixed(1)}KB`:`${e}B`}}const Gn=new zo,Xr=1448629062,Kr=1,Uo={axis:"off",radialSegments:8},ko={x:1,y:1,z:1,w:1},ft=24,Jr=80,Qr=0,Ao=1,Io=2,Ro=3,Do=4,st=8,Bt=we("SDFVolume");class Ge{context;_resolution;_buffer;_stagingBuffer=null;_editCount=0;_readbackInProgress=null;_bounds=null;constructor(e,i){this.context=e,this._resolution=i.resolution;const n=this._resolution**3*4;this._buffer=e.device.createBuffer({label:`sdf-volume-${this._resolution}`,size:n,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC|GPUBufferUsage.COPY_DST,mappedAtCreation:!0});const u=i.initialValue??1;new Float32Array(this._buffer.getMappedRange()).fill(u),this._buffer.unmap(),Bt.info(`Created ${this._resolution}^3 volume (${(n/1024/1024).toFixed(1)}MB)`)}get resolution(){return this._resolution}get buffer(){return this._buffer}get editCount(){return this._editCount}get voxelCount(){return this._resolution**3}get bufferSize(){return this.voxelCount*4}get bounds(){return this._bounds}set bounds(e){this._bounds=e}incrementEditCount(){this._editCount++}async initializeSphere(){const e=this._resolution/2,i=this._resolution*.15,o=this.context.device.createBuffer({size:this.bufferSize,usage:GPUBufferUsage.MAP_WRITE|GPUBufferUsage.COPY_SRC,mappedAtCreation:!0}),n=new Float32Array(o.getMappedRange());let u=0;for(let d=0;d<this._resolution;d++)for(let f=0;f<this._resolution;f++)for(let g=0;g<this._resolution;g++){const x=g-e,v=f-e,_=d-e,E=Math.sqrt(x*x+v*v+_*_);n[u++]=E-i}o.unmap();const c=this.context.device.createCommandEncoder();c.copyBufferToBuffer(o,0,this._buffer,0,this.bufferSize),this.context.device.queue.submit([c.finish()]),await this.context.device.queue.onSubmittedWorkDone(),o.destroy(),Bt.info(`Initialized with sphere (radius=${i.toFixed(1)})`)}async readback(){if(this._readbackInProgress)return this._readbackInProgress;this._readbackInProgress=this._readbackInternal();try{return await this._readbackInProgress}finally{this._readbackInProgress=null}}async _readbackInternal(){this._stagingBuffer||(this._stagingBuffer=this.context.device.createBuffer({size:this.bufferSize,usage:GPUBufferUsage.MAP_READ|GPUBufferUsage.COPY_DST}));const e=this.context.device.createCommandEncoder();e.copyBufferToBuffer(this._buffer,0,this._stagingBuffer,0,this.bufferSize),this.context.device.queue.submit([e.finish()]),await this._stagingBuffer.mapAsync(GPUMapMode.READ);const i=new Float32Array(this._stagingBuffer.getMappedRange().slice(0));return this._stagingBuffer.unmap(),i}async upload(e){if(e.length!==this.voxelCount)throw new Error(`Data length ${e.length} does not match voxel count ${this.voxelCount}`);this.context.device.queue.writeBuffer(this._buffer,0,e.buffer,e.byteOffset,e.byteLength),await this.context.device.queue.onSubmittedWorkDone()}async createSnapshot(){return this.readback()}async restoreSnapshot(e){await this.upload(e)}async serialize(e=!0){const i=await this.readback();let o=0;e&&(o|=1),this._bounds&&(o|=2);const n=new ArrayBuffer(ft),u=new DataView(n);u.setUint32(0,Xr,!0),u.setUint32(4,Kr,!0),u.setUint32(8,this._resolution,!0),u.setUint32(12,this._editCount,!0),u.setUint32(16,o,!0),u.setUint32(20,0,!0);let c=null;if(this._bounds){const x=new ArrayBuffer(24),v=new DataView(x);v.setFloat32(0,this._bounds.min.x,!0),v.setFloat32(4,this._bounds.min.y,!0),v.setFloat32(8,this._bounds.min.z,!0),v.setFloat32(12,this._bounds.max.x,!0),v.setFloat32(16,this._bounds.max.y,!0),v.setFloat32(20,this._bounds.max.z,!0),c=new Uint8Array(x)}let d;if(e){const{deflateSync:x}=await Ut(async()=>{const{deflateSync:_}=await import("./browser-CL2bEFe4.js");return{deflateSync:_}},[]),v=new Uint8Array(i.buffer);d=x(v,{level:6}),Bt.info(`Compressed ${(v.length/1024).toFixed(0)}KB → ${(d.length/1024).toFixed(0)}KB`)}else d=new Uint8Array(i.buffer);const f=c?24:0,g=new Uint8Array(ft+f+d.length);return g.set(new Uint8Array(n),0),c&&g.set(c,ft),g.set(d,ft+f),g.buffer}static async deserialize(e,i){const o=new DataView(i,0,ft),n=o.getUint32(0,!0);if(n!==Xr)throw new Error(`Invalid volume magic: 0x${n.toString(16)}`);const u=o.getUint32(4,!0);if(u!==Kr)throw new Error(`Unsupported volume version: ${u}`);const c=o.getUint32(8,!0),d=o.getUint32(12,!0),f=o.getUint32(16,!0),g=(f&1)!==0,x=(f&2)!==0,v=new Ge(e,{resolution:c,initialValue:1});v._editCount=d;let _=ft;if(x){const P=new DataView(i,ft,24);v._bounds={min:{x:P.getFloat32(0,!0),y:P.getFloat32(4,!0),z:P.getFloat32(8,!0)},max:{x:P.getFloat32(12,!0),y:P.getFloat32(16,!0),z:P.getFloat32(20,!0)}},_+=24}const E=new Uint8Array(i,_);let b;if(g){const{inflateSync:P}=await Ut(async()=>{const{inflateSync:z}=await import("./browser-CL2bEFe4.js");return{inflateSync:z}},[]),B=P(E);b=new Float32Array(B.buffer)}else b=new Float32Array(E.buffer,E.byteOffset,E.length/4);return await v.upload(b),Bt.info(`Deserialized ${c}^3 volume (editCount=${d}, hasBounds=${x})`),v}dispose(){this._buffer.destroy(),this._stagingBuffer&&(this._stagingBuffer.destroy(),this._stagingBuffer=null),Bt.info("Disposed")}}const Ft=we("ColorVolume"),ei=1129270358,ti=2,Mt=24;function Oo(w){return Math.max(0,Math.min(1,w))}function it(w){return Math.round(Oo(w)*255)}class qe{context;_resolution;_buffer;_stagingBuffer=null;_readbackInProgress=null;constructor(e,i){this.context=e,this._resolution=i.resolution;const o=this._resolution**3,n=o*4;this._buffer=e.device.createBuffer({label:`color-volume-${this._resolution}`,size:n,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC|GPUBufferUsage.COPY_DST,mappedAtCreation:!0});const u=i.initialColor??{r:0,g:0,b:0,a:0},c=it(u.r),d=it(u.g),f=it(u.b),g=it(u.a),x=new Uint8Array(this._buffer.getMappedRange());for(let v=0;v<o;v++){const _=v*4;x[_+0]=c,x[_+1]=d,x[_+2]=f,x[_+3]=g}this._buffer.unmap(),Ft.info(`Created ${this._resolution}^3 volume (${(n/1024/1024).toFixed(1)}MB, RGBA8)`)}get resolution(){return this._resolution}get buffer(){return this._buffer}get voxelCount(){return this._resolution**3}get bufferSize(){return this.voxelCount*4}async readback(){if(this._readbackInProgress)return this._readbackInProgress;this._readbackInProgress=this._readbackInternal();try{return await this._readbackInProgress}finally{this._readbackInProgress=null}}async _readbackInternal(){this._stagingBuffer||(this._stagingBuffer=this.context.device.createBuffer({size:this.bufferSize,usage:GPUBufferUsage.MAP_READ|GPUBufferUsage.COPY_DST}));const e=this.context.device.createCommandEncoder();e.copyBufferToBuffer(this._buffer,0,this._stagingBuffer,0,this.bufferSize),this.context.device.queue.submit([e.finish()]),await this._stagingBuffer.mapAsync(GPUMapMode.READ);const i=new Uint8Array(this._stagingBuffer.getMappedRange().slice(0));return this._stagingBuffer.unmap(),i}async upload(e){const i=this.voxelCount*4;if(e.length!==i)throw new Error(`Data length ${e.length} does not match expected ${i}`);this.context.device.queue.writeBuffer(this._buffer,0,e.buffer,e.byteOffset,e.byteLength),await this.context.device.queue.onSubmittedWorkDone()}async createSnapshot(){return this.readback()}async restoreSnapshot(e){await this.upload(e)}async fill(e){const i=new Uint8Array(this.voxelCount*4),o=it(e.r),n=it(e.g),u=it(e.b),c=it(e.a);for(let d=0;d<this.voxelCount;d++){const f=d*4;i[f+0]=o,i[f+1]=n,i[f+2]=u,i[f+3]=c}await this.upload(i)}async serialize(e=!0){const i=await this.readback(),o=new ArrayBuffer(Mt),n=new DataView(o);n.setUint32(0,ei,!0),n.setUint32(4,ti,!0),n.setUint32(8,this._resolution,!0),n.setUint32(12,0,!0),n.setUint32(16,e?1:0,!0),n.setUint32(20,0,!0);let u;if(e){const{deflateSync:d}=await Ut(async()=>{const{deflateSync:f}=await import("./browser-CL2bEFe4.js");return{deflateSync:f}},[]);u=d(i,{level:6}),Ft.info(`Compressed ${(i.length/1024).toFixed(0)}KB -> ${(u.length/1024).toFixed(0)}KB`)}else u=i;const c=new Uint8Array(Mt+u.length);return c.set(new Uint8Array(o),0),c.set(u,Mt),c.buffer}static async deserialize(e,i){const o=new DataView(i,0,Mt),n=o.getUint32(0,!0);if(n!==ei)throw new Error(`Invalid color volume magic: 0x${n.toString(16)}`);const u=o.getUint32(4,!0);if(u!==1&&u!==ti)throw new Error(`Unsupported color volume version: ${u}`);const c=o.getUint32(8,!0),f=(o.getUint32(16,!0)&1)!==0,g=new qe(e,{resolution:c}),x=Mt,v=new Uint8Array(i,x);let _;if(f){const{inflateSync:b}=await Ut(async()=>{const{inflateSync:P}=await import("./browser-CL2bEFe4.js");return{inflateSync:P}},[]);_=b(v)}else _=new Uint8Array(v);let E;if(u===1){const b=new Float32Array(_.buffer,_.byteOffset,_.byteLength/4);E=new Uint8Array(b.length);for(let P=0;P<b.length;P++)E[P]=Math.round(Math.max(0,Math.min(1,b[P]))*255);Ft.info(`Deserialized v1 (Float32) → migrated to v2 (Uint8) at ${c}^3`)}else E=_,Ft.info(`Deserialized v2 (Uint8) at ${c}^3`);return await g.upload(E),g}dispose(){this._buffer.destroy(),this._stagingBuffer&&(this._stagingBuffer.destroy(),this._stagingBuffer=null),Ft.info("Disposed")}}const jt=we("BoundsManager"),Vo=.1,Lo=.05;class Go{warningMargin;criticalMargin;lastWarning=null;eventListeners=new Set;constructor(e={}){this.warningMargin=e.warningMargin??Vo,this.criticalMargin=e.criticalMargin??Lo,jt.info(`Created with warningMargin=${this.warningMargin} criticalMargin=${this.criticalMargin}`)}setWarningMargin(e){this.warningMargin=Math.max(0,Math.min(.5,e))}setCriticalMargin(e){this.criticalMargin=Math.max(0,Math.min(.5,e))}checkCallCount=0;checkPosition(e,i=0){this.checkCallCount++;const o=this.checkCallCount<=3||this.checkCallCount%20===0;o&&jt(`checkPosition #${this.checkCallCount}: pos=(${e.x.toFixed(3)}, ${e.y.toFixed(3)}, ${e.z.toFixed(3)}) brushRadius=${i.toFixed(4)}`);const n={minX:e.x-i,maxX:1-e.x-i,minY:e.y-i,maxY:1-e.y-i,minZ:e.z-i,maxZ:1-e.z-i},u=Math.min(n.minX,n.maxX,n.minY,n.maxY,n.minZ,n.maxZ);u<this.warningMargin&&o&&jt(`NEAR EDGE! minProx=${u.toFixed(3)} < warningMargin=${this.warningMargin}`);const c={minX:n.minX<this.warningMargin,maxX:n.maxX<this.warningMargin,minY:n.minY<this.warningMargin,maxY:n.maxY<this.warningMargin,minZ:n.minZ<this.warningMargin,maxZ:n.maxZ<this.warningMargin},d=Object.values(c).some(b=>b);let f=null,g=1/0;const x=Object.entries(n);for(const[b,P]of x)P<g&&(g=P,f={edge:b,distance:P});let v=null;if(d){v={x:0,y:0,z:0},c.minX&&(v.x-=1),c.maxX&&(v.x+=1),c.minY&&(v.y-=1),c.maxY&&(v.y+=1),c.minZ&&(v.z-=1),c.maxZ&&(v.z+=1);const b=Math.sqrt(v.x**2+v.y**2+v.z**2);b>0?(v.x/=b,v.y/=b,v.z/=b):v=null}let _=null;if(d&&f){const b=f.distance<this.criticalMargin,P={minX:"-X",maxX:"+X",minY:"-Y",maxY:"+Y",minZ:"-Z",maxZ:"+Z"},B=Math.round(f.distance*100);b?_=`Critical: ${B}% from ${P[f.edge]} edge. Expand bounds to continue.`:_=`Warning: ${B}% from ${P[f.edge]} edge.`}const E={isNearEdge:d,atRisk:c,closestEdge:f,suggestedExpansion:v,message:_};return this.emitIfChanged(E),this.lastWarning=E,E}wouldExceedBounds(e,i){const o=i+this.criticalMargin;return e.x-o<0||e.x+o>1||e.y-o<0||e.y+o>1||e.z-o<0||e.z+o>1}getLastWarning(){return this.lastWarning}clear(){if(this.lastWarning?.isNearEdge){const e={isNearEdge:!1,atRisk:{minX:!1,maxX:!1,minY:!1,maxY:!1,minZ:!1,maxZ:!1},closestEdge:null,suggestedExpansion:null,message:null};this.emitEvent({type:"clear",warning:e}),this.lastWarning=e}}calculateExpandedResolution(e,i){const o=e*2;return Math.min(256,o)}calculateDataOffset(e,i,o){const n=i-e;return{x:o.x<0?n:o.x>0?0:Math.floor(n/2),y:o.y<0?n:o.y>0?0:Math.floor(n/2),z:o.z<0?n:o.z>0?0:Math.floor(n/2)}}on(e){return this.eventListeners.add(e),()=>{this.eventListeners.delete(e)}}emitEvent(e){jt.info(`Emitting event: ${e.type}`,e.warning.message);for(const i of this.eventListeners)try{i(e)}catch(o){console.error("[BoundsManager] Event callback error:",o)}}emitIfChanged(e){const i=this.lastWarning,o=i?.closestEdge?i.closestEdge.distance<this.criticalMargin:!1,n=e.closestEdge?e.closestEdge.distance<this.criticalMargin:!1,u=i?.isNearEdge??!1,c=e.isNearEdge;!u&&!o&&c&&!n?this.emitEvent({type:"warning",warning:e}):!o&&n?this.emitEvent({type:"critical",warning:e}):(u||o)&&!c&&!n?this.emitEvent({type:"clear",warning:e}):c&&i?.closestEdge?.edge!==e.closestEdge?.edge&&this.emitEvent({type:"warning",warning:e})}dispose(){this.eventListeners.clear(),this.lastWarning=null}}const kt=`
fn packRGBA8(c: vec4<f32>) -> u32 {
  let r = u32(clamp(c.x, 0.0, 1.0) * 255.0 + 0.5);
  let g = u32(clamp(c.y, 0.0, 1.0) * 255.0 + 0.5);
  let b = u32(clamp(c.z, 0.0, 1.0) * 255.0 + 0.5);
  let a = u32(clamp(c.w, 0.0, 1.0) * 255.0 + 0.5);
  return r | (g << 8u) | (b << 16u) | (a << 24u);
}

fn unpackRGBA8(p: u32) -> vec4<f32> {
  let r = f32(p & 0xFFu) / 255.0;
  let g = f32((p >> 8u) & 0xFFu) / 255.0;
  let b = f32((p >> 16u) & 0xFFu) / 255.0;
  let a = f32((p >> 24u) & 0xFFu) / 255.0;
  return vec4<f32>(r, g, b, a);
}
`,$o=`
${kt}
/**
 * Brush Application Compute Shader
 *
 * Applies brush operations (add/subtract/smooth) to SDF volume.
 * Also writes brush color to color volume when sculpting.
 * Supports symmetry modes (X/Y/Z mirror and radial).
 * Each invocation processes one voxel.
 */

// Brush tool types
const TOOL_BRUSH: u32 = 0u;    // Add material (union)
const TOOL_PULL: u32 = 1u;     // Remove material (subtract)
const TOOL_SMOOTH: u32 = 2u;   // Smooth surface (blur)
const TOOL_INFLATE: u32 = 3u;  // Offset SDF outward (push existing surface outward)
const TOOL_DEFLATE: u32 = 4u;  // Offset SDF inward (shrink existing surface)
const TOOL_FLATTEN: u32 = 5u;  // Press surface toward local average (planar)
const TOOL_PINCH: u32 = 6u;    // Lower SDF proportional to (1 - dist/radius)
const TOOL_CREASE: u32 = 7u;   // Sharper pinch — narrower falloff, deeper bite
const TOOL_CARVE: u32 = 8u;    // Sharper pull — smaller smin smoothK (hard edge)

// Symmetry modes
const SYMMETRY_OFF: u32 = 0u;
const SYMMETRY_X: u32 = 1u;
const SYMMETRY_Y: u32 = 2u;
const SYMMETRY_Z: u32 = 3u;
const SYMMETRY_RADIAL: u32 = 4u;

// Layout must match BrushPipeline uniform writes (80 bytes total)
struct BrushUniforms {
  // Brush center position in volume space [0, resolution) - offset 0
  position: vec3<f32>,
  _pad0: f32,
  // Brush radius in voxels - offset 16
  radius: f32,
  // Brush strength (0-1) - offset 20
  strength: f32,
  // Tool type (0=brush, 1=pull, 2=smooth, 3=inflate, 4=deflate,
  // 5=flatten, 6=pinch, 7=crease, 8=carve) - offset 24
  tool: u32,
  // Edge falloff hardness 0..1 - offset 28.
  // 0 = soft/wide taper; 1 = crisp edge with minimal taper.
  hardness: f32,
  // Volume resolution - offset 32
  resolution: u32,
  // Symmetry axis (0=off, 1=x, 2=y, 3=z, 4=radial) - offset 36
  symmetry_axis: u32,
  // Radial symmetry segments - offset 40
  symmetry_segments: u32,
  _pad2: u32,
  // Brush color (RGBA) - offset 48
  color: vec4<f32>,
  // Reserved padding to 80 bytes - offset 64
  _pad3: vec4<f32>,
}

@group(0) @binding(0) var<storage, read_write> volume: array<f32>;
@group(0) @binding(1) var<uniform> brush: BrushUniforms;
// Packed RGBA8: one u32 per voxel. Read with unpackRGBA8, write with packRGBA8.
@group(0) @binding(2) var<storage, read_write> colorVolume: array<u32>;

// Convert 3D coordinates to 1D index (one element per voxel — works for both
// SDF (f32) and packed color (u32) since both store one element per voxel).
fn coordToIndex(x: u32, y: u32, z: u32, res: u32) -> u32 {
  return x + y * res + z * res * res;
}

// Signed distance to sphere
fn sdSphere(p: vec3<f32>, center: vec3<f32>, radius: f32) -> f32 {
  return length(p - center) - radius;
}

// Smooth minimum for union (adding material)
fn sminPoly(a: f32, b: f32, k: f32) -> f32 {
  let h = max(k - abs(a - b), 0.0) / k;
  return min(a, b) - h * h * k * 0.25;
}

// Smooth maximum for subtraction (removing material)
fn smaxPoly(a: f32, b: f32, k: f32) -> f32 {
  return -sminPoly(-a, -b, k);
}

// Mirror position across an axis for symmetry
fn mirrorPosition(pos: vec3<f32>, axis: u32, res: u32) -> vec3<f32> {
  let center = f32(res) / 2.0;
  var mirrored = pos;

  switch axis {
    case SYMMETRY_X: {
      mirrored.x = 2.0 * center - pos.x;
    }
    case SYMMETRY_Y: {
      mirrored.y = 2.0 * center - pos.y;
    }
    case SYMMETRY_Z: {
      mirrored.z = 2.0 * center - pos.z;
    }
    default: {}
  }

  return mirrored;
}

// Calculate distance to brush including symmetry
fn getMinBrushDistance(voxelPos: vec3<f32>, brushPos: vec3<f32>, radius: f32, axis: u32, segments: u32, res: u32) -> f32 {
  var minDist = length(voxelPos - brushPos);

  // Handle symmetry modes
  if (axis == SYMMETRY_X || axis == SYMMETRY_Y || axis == SYMMETRY_Z) {
    // Mirror symmetry - check distance to mirrored brush position
    let mirroredBrush = mirrorPosition(brushPos, axis, res);
    let mirrorDist = length(voxelPos - mirroredBrush);
    minDist = min(minDist, mirrorDist);
  } else if (axis == SYMMETRY_RADIAL) {
    // Radial symmetry around Y axis
    let center = f32(res) / 2.0;
    let centerVec = vec3<f32>(center, 0.0, center);

    // Get brush offset from center (ignoring Y)
    let brushOffset = vec2<f32>(brushPos.x - center, brushPos.z - center);
    let brushRadius2D = length(brushOffset);
    let brushAngle = atan2(brushOffset.y, brushOffset.x);

    // Check each radial copy
    let angleStep = 6.283185307 / f32(segments); // 2π / segments
    for (var i = 1u; i < segments; i++) {
      let angle = brushAngle + angleStep * f32(i);
      let rotatedPos = vec3<f32>(
        center + brushRadius2D * cos(angle),
        brushPos.y,
        center + brushRadius2D * sin(angle)
      );
      let dist = length(voxelPos - rotatedPos);
      minDist = min(minDist, dist);
    }
  }

  return minDist;
}

// Apply brush operation at a position (returns new SDF value)
fn applyBrushOp(voxelPos: vec3<f32>, brushPos: vec3<f32>, currentValue: f32, res: u32) -> f32 {
  let dist = length(voxelPos - brushPos);

  // Skip if too far from brush
  if (dist > brush.radius * 2.0) {
    return currentValue;
  }

  // Calculate brush SDF (sphere)
  let brushSdf = sdSphere(voxelPos, brushPos, brush.radius);

  // Edge taper driven by brush.hardness (0 = full feather across the
  // whole brush, 1 = crisp edge at brush.radius). Keeping the falloff
  // band INSIDE brush.radius means raising taper narrows the visible
  // stroke instead of widening it.
  let outerEdge = brush.radius;
  let innerEdge = brush.radius * brush.hardness;
  let falloff = smoothstep(outerEdge, innerEdge, dist);
  let effectiveStrength = brush.strength * falloff;

  var newValue = currentValue;

  switch brush.tool {
    case TOOL_BRUSH: {
      let smoothK = brush.radius * 0.3;
      let blended = sminPoly(currentValue, brushSdf, smoothK);
      newValue = mix(currentValue, blended, effectiveStrength);
    }
    case TOOL_PULL: {
      let smoothK = brush.radius * 0.3;
      let blended = smaxPoly(currentValue, -brushSdf, smoothK);
      newValue = mix(currentValue, blended, effectiveStrength);
    }
    case TOOL_SMOOTH: {
      // Phase G — dropped the abs(currentValue) < radius * 0.5 half of the
      // gate so smooth visibly affects voxels deeper inside the surface, not
      // just the iso-shell. Still localised by dist < brush.radius.
      if (dist < brush.radius) {
        var sum = 0.0;
        var count = 0.0;

        for (var dz: i32 = -1; dz <= 1; dz++) {
          for (var dy: i32 = -1; dy <= 1; dy++) {
            for (var dx: i32 = -1; dx <= 1; dx++) {
              let nx = i32(voxelPos.x) + dx;
              let ny = i32(voxelPos.y) + dy;
              let nz = i32(voxelPos.z) + dz;

              if (nx >= 0 && nx < i32(res) && ny >= 0 && ny < i32(res) && nz >= 0 && nz < i32(res)) {
                let nIdx = coordToIndex(u32(nx), u32(ny), u32(nz), res);
                sum += volume[nIdx];
                count += 1.0;
              }
            }
          }
        }

        let avgValue = sum / count;
        newValue = mix(currentValue, avgValue, effectiveStrength * 0.3);
      }
    }
    case TOOL_INFLATE: {
      // Push existing surface outward along its normal by lowering SDF.
      // Negative SDF = inside, so subtracting moves the iso-surface
      // outward. The surface-proximity weight is the key contract: voxels
      // far from any material (in empty space) get weight 0 → dragging
      // through air won't lay down a trail. Voxels near the surface get
      // full effect → holding at a spot puffs out a blob. Distinct from
      // TOOL_BRUSH which unions a sphere of new material.
      if (dist < brush.radius) {
        let proximity = clamp(1.0 - max(currentValue, 0.0) / brush.radius, 0.0, 1.0);
        newValue = currentValue - effectiveStrength * 0.3 * proximity;
      }
    }
    case TOOL_DEFLATE: {
      // Inverse of inflate: raise SDF so the iso-surface retreats inward.
      // Surface-proximity weight gates from the inside — voxels deep
      // inside the material (where there's no nearby surface to move)
      // get weight 0 → no effect. Near-surface voxels get full effect →
      // the surface shrinks at the brush without disturbing the interior.
      if (dist < brush.radius) {
        let proximity = clamp(1.0 + min(currentValue, 0.0) / brush.radius, 0.0, 1.0);
        newValue = currentValue + effectiveStrength * 0.3 * proximity;
      }
    }
    case TOOL_FLATTEN: {
      // Local neighbourhood-average smoothing of the SDF, attenuated by
      // 0.3 (same as TOOL_SMOOTH). The attenuation is what makes flatten
      // self-arresting: when the patch is already flat the local average
      // matches the current value and the mix is a no-op. Without the
      // attenuation, repeated stamps push past flat and gouge holes.
      // Plane-fit flatten is a v2 task.
      if (dist < brush.radius) {
        var sum = 0.0;
        var count = 0.0;

        for (var dz: i32 = -1; dz <= 1; dz++) {
          for (var dy: i32 = -1; dy <= 1; dy++) {
            for (var dx: i32 = -1; dx <= 1; dx++) {
              let nx = i32(voxelPos.x) + dx;
              let ny = i32(voxelPos.y) + dy;
              let nz = i32(voxelPos.z) + dz;

              if (nx >= 0 && nx < i32(res) && ny >= 0 && ny < i32(res) && nz >= 0 && nz < i32(res)) {
                let nIdx = coordToIndex(u32(nx), u32(ny), u32(nz), res);
                sum += volume[nIdx];
                count += 1.0;
              }
            }
          }
        }

        let avgValue = sum / count;
        newValue = mix(currentValue, avgValue, effectiveStrength * 0.3);
      }
    }
    case TOOL_PINCH: {
      // Pinch the surface INWARD toward the brush (fingers-squeezing-walls
      // gesture). Raising SDF makes the iso-surface retreat away from the
      // brush position; the center-weighted (1 - dist/radius) bias makes
      // it retreat most at the brush, producing a soft inward dimple. For
      // a thin wall, both faces retreat toward each other = walls pinch
      // together. Surface-band proximity weight (|SDF| < radius) keeps
      // the effect on the surface shell — empty space and deep interior
      // are untouched.
      if (dist < brush.radius) {
        let pinchFactor = 1.0 - dist / brush.radius;
        let proximity = clamp(1.0 - abs(currentValue) / brush.radius, 0.0, 1.0);
        newValue = currentValue + effectiveStrength * 0.3 * pinchFactor * proximity;
      }
    }
    case TOOL_CREASE: {
      // Sharper sibling of pinch — narrower footprint (60% radius) and 2×
      // per-stamp magnitude. Produces a thin deep inward trench rather
      // than pinch's soft dimple. Same surface-band gate as pinch.
      let creaseRadius = brush.radius * 0.6;
      if (dist < creaseRadius) {
        let creaseFactor = 1.0 - dist / creaseRadius;
        let proximity = clamp(1.0 - abs(currentValue) / brush.radius, 0.0, 1.0);
        newValue = currentValue + effectiveStrength * 0.6 * creaseFactor * proximity;
      }
    }
    case TOOL_CARVE: {
      // Same subtract shape as TOOL_PULL but with a tighter smin smoothK
      // (0.1× radius vs 0.3×). The narrow blend band gives carve its
      // chisel-edge character — Pull's bite is softer.
      let smoothK = brush.radius * 0.1;
      let blended = smaxPoly(currentValue, -brushSdf, smoothK);
      newValue = mix(currentValue, blended, effectiveStrength);
    }
    default: {}
  }

  return newValue;
}

@compute @workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let res = brush.resolution;

  // Bounds check
  if (global_id.x >= res || global_id.y >= res || global_id.z >= res) {
    return;
  }

  let idx = coordToIndex(global_id.x, global_id.y, global_id.z, res);
  let currentValue = volume[idx];

  // Position in volume space
  let voxelPos = vec3<f32>(global_id);

  // Get minimum distance considering symmetry
  let minDist = getMinBrushDistance(voxelPos, brush.position, brush.radius, brush.symmetry_axis, brush.symmetry_segments, res);

  // Skip if too far from any brush position
  if (minDist > brush.radius * 2.0) {
    return;
  }

  var newValue = currentValue;

  // Apply brush at primary position
  newValue = applyBrushOp(voxelPos, brush.position, newValue, res);

  // Apply symmetry
  if (brush.symmetry_axis == SYMMETRY_X || brush.symmetry_axis == SYMMETRY_Y || brush.symmetry_axis == SYMMETRY_Z) {
    let mirroredBrush = mirrorPosition(brush.position, brush.symmetry_axis, res);
    newValue = applyBrushOp(voxelPos, mirroredBrush, newValue, res);
  } else if (brush.symmetry_axis == SYMMETRY_RADIAL) {
    let center = f32(res) / 2.0;
    let brushOffset = vec2<f32>(brush.position.x - center, brush.position.z - center);
    let brushRadius2D = length(brushOffset);
    let brushAngle = atan2(brushOffset.y, brushOffset.x);
    let angleStep = 6.283185307 / f32(brush.symmetry_segments);

    for (var i = 1u; i < brush.symmetry_segments; i++) {
      let angle = brushAngle + angleStep * f32(i);
      let rotatedPos = vec3<f32>(
        center + brushRadius2D * cos(angle),
        brush.position.y,
        center + brushRadius2D * sin(angle)
      );
      newValue = applyBrushOp(voxelPos, rotatedPos, newValue, res);
    }
  }

  volume[idx] = newValue;

  // Paint color into the color volume. Branches by tool:
  //
  // TOOL_BRUSH (add): "where new material appears, paint it" — gated on
  // BOTH (a) SDF actually changed AND (b) newValue near/inside the surface
  // (< 0.5). Preserves the legacy add-brush semantics.
  //
  // TOOL_PULL (subtract): user wants the carved cavity to inherit the
  // brush color "like the scribble tool". The naive gate (paint only
  // changed voxels) FAILS for two reasons:
  //   1. The cavity wall vertices sit at the iso=0 boundary between
  //      carved voxels (SDF positive) and SURROUNDING SOLID voxels (SDF
  //      unchanged). Triplanar sampling weights nearby voxels — most are
  //      in the solid region. So we need to paint the surrounding solid
  //      voxels too, not just the carved ones.
  //   2. Carved voxels' newValue often exceeds 0.5 (deeply outside),
  //      excluded by the 'less than 0.5' gate.
  // Fix: for PULL, paint the WHOLE brush sphere (radius * 1.5 — covers
  // carved interior + surrounding solid boundary) using a distance falloff
  // unrelated to the SDF change. minDist already accounts for symmetry.
  if (brush.tool == TOOL_BRUSH) {
    // Paint across the brush footprint at/inside the new surface so a new
    // colour covers SOLIDLY — even where the brush adds no material (i.e.
    // recolouring an existing or smoothed-flat surface). The old gate painted
    // ONLY where the SDF changed (paintStrength = change*2), so recolours
    // barely registered and blended with the prior colour → mottled speckling
    // (blue-on-white). We keep a bias toward freshly-added material
    // (changeStrength) so a growing blob's leading edge still takes crisp
    // colour. minDist + brush.radius mirror the PULL branch's footprint paint.
    let change = abs(newValue - currentValue);
    let changeStrength = min(change * 2.0, 1.0);
    // Footprint coverage: solid at the brush centre, soft at the rim. Gated to
    // near/inside the surface (newValue < 0.5) so the empty exterior the brush
    // sphere also spans stays unpainted.
    let coverage = select(0.0, clamp(1.0 - minDist / brush.radius, 0.0, 1.0), newValue < 0.5);
    let paintStrength = max(changeStrength, coverage);
    if (paintStrength > 0.0) {
      let current = unpackRGBA8(colorVolume[idx]);
      let mixed = vec4<f32>(
        mix(current.x, brush.color.x, paintStrength),
        mix(current.y, brush.color.y, paintStrength),
        mix(current.z, brush.color.z, paintStrength),
        max(current.w, paintStrength)
      );
      colorVolume[idx] = packRGBA8(mixed);
    }
  } else if (brush.tool == TOOL_PULL) {
    let paintRadius = brush.radius * 1.5;
    let falloff = clamp(1.0 - minDist / paintRadius, 0.0, 1.0);
    if (falloff > 0.0) {
      let paintStrength = min(falloff * 2.0, 1.0);
      let current = unpackRGBA8(colorVolume[idx]);
      let mixed = vec4<f32>(
        mix(current.x, brush.color.x, paintStrength),
        mix(current.y, brush.color.y, paintStrength),
        mix(current.z, brush.color.z, paintStrength),
        max(current.w, paintStrength)
      );
      colorVolume[idx] = packRGBA8(mixed);
    }
  } else if (brush.tool != TOOL_SMOOTH && newValue != currentValue && newValue < 0.5) {
    // INFLATE / DEFLATE / FLATTEN / PINCH / CREASE / CARVE — lock the
    // brush color into voxels these tools modified near the surface.
    // Without this branch the tools modify SDF but leave colorVolume
    // alpha at zero, so the mesh-extract unpainted-region fallback
    // governs the displayed color entirely — every picker change
    // retroactively recolored shape-tool strokes. SMOOTH is excluded
    // because its color blending runs separately through
    // colorSmoothLocalPipeline (Sculptor.addStrokePoint), and PULL is
    // handled by the dedicated branch above.
    let change = abs(newValue - currentValue);
    let paintStrength = min(change * 2.0, 1.0);
    let current = unpackRGBA8(colorVolume[idx]);
    let mixed = vec4<f32>(
      mix(current.x, brush.color.x, paintStrength),
      mix(current.y, brush.color.y, paintStrength),
      mix(current.z, brush.color.z, paintStrength),
      max(current.w, paintStrength)
    );
    colorVolume[idx] = packRGBA8(mixed);
  }
}
`,No=`
${kt}
/**
 * Color Smoothing Compute Shader
 *
 * Applies Gaussian blur to the color volume to reduce color jitter
 * and create smoother painted surfaces.
 * Each invocation processes one voxel.
 */

struct SmoothUniforms {
  // Volume resolution
  resolution: u32,
  // Kernel size (1 = 3x3x3, 2 = 5x5x5)
  kernelSize: u32,
  // Smoothing strength (0-1): how much to blend with neighbors
  strength: f32,
  _pad: u32,
}

// Packed RGBA8: one u32 per voxel. Read with unpackRGBA8, write with packRGBA8.
@group(0) @binding(0) var<storage, read> colorInput: array<u32>;
@group(0) @binding(1) var<storage, read_write> colorOutput: array<u32>;
@group(0) @binding(2) var<uniform> params: SmoothUniforms;

// Convert 3D coordinates to 1D index
fn coordToIndex(x: u32, y: u32, z: u32, res: u32) -> u32 {
  return x + y * res + z * res * res;
}

// Gaussian weight for 3D distance
fn gaussianWeight(dx: i32, dy: i32, dz: i32, sigma: f32) -> f32 {
  let distSq = f32(dx * dx + dy * dy + dz * dz);
  return exp(-distSq / (2.0 * sigma * sigma));
}

@compute @workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let res = params.resolution;

  // Bounds check
  if (global_id.x >= res || global_id.y >= res || global_id.z >= res) {
    return;
  }

  let idx = coordToIndex(global_id.x, global_id.y, global_id.z, res);
  let currentColor = unpackRGBA8(colorInput[idx]);

  // Skip if alpha is very low (empty voxel)
  if (currentColor.a < 0.01) {
    colorOutput[idx] = packRGBA8(currentColor);
    return;
  }

  // Kernel radius
  let radius = i32(params.kernelSize);
  let sigma = f32(radius) * 0.5 + 0.5; // Sigma based on kernel size

  var weightedSum = vec4<f32>(0.0);
  var totalWeight = 0.0;

  // Sample neighborhood
  for (var dz: i32 = -radius; dz <= radius; dz++) {
    for (var dy: i32 = -radius; dy <= radius; dy++) {
      for (var dx: i32 = -radius; dx <= radius; dx++) {
        let nx = i32(global_id.x) + dx;
        let ny = i32(global_id.y) + dy;
        let nz = i32(global_id.z) + dz;

        // Bounds check for neighbor
        if (nx >= 0 && nx < i32(res) && ny >= 0 && ny < i32(res) && nz >= 0 && nz < i32(res)) {
          let neighborIdx = coordToIndex(u32(nx), u32(ny), u32(nz), res);
          let neighborColor = unpackRGBA8(colorInput[neighborIdx]);

          // Only include neighbors with meaningful alpha
          if (neighborColor.a > 0.01) {
            let weight = gaussianWeight(dx, dy, dz, sigma);
            weightedSum += neighborColor * weight;
            totalWeight += weight;
          }
        }
      }
    }
  }

  // Compute blurred color
  var blurredColor = currentColor;
  if (totalWeight > 0.0) {
    blurredColor = weightedSum / totalWeight;
    // Preserve original alpha
    blurredColor.a = currentColor.a;
  }

  // Blend based on strength
  let finalColor = mix(currentColor, blurredColor, params.strength);
  colorOutput[idx] = packRGBA8(finalColor);
}
`,Wo=`
/**
 * SDF Filter Compute Shader
 *
 * Smooths the SDF volume globally with one of four kernels:
 *   mode 0: Gaussian   — distance-weighted average (soft falloff)
 *   mode 1: Mean / Box — uniform-weighted average
 *   mode 2: Median     — middle value of the 3³ neighborhood (edge-preserving)
 *   mode 3: Bilateral  — Gaussian weighted by value-similarity (preserves shape edges)
 *
 * All modes use a 3×3×3 kernel (27 samples). Blends the filtered value with
 * the original via 'strength' (0 = no change, 1 = full filter). Operates on
 * f32 SDF voxels.
 */

struct FilterUniforms {
  resolution: u32,
  mode: u32,        // 0=Gaussian, 1=Mean, 2=Median, 3=Bilateral
  strength: f32,    // 0..1 blend toward filtered
  valueSigma: f32,  // bilateral value-similarity sigma (only used by mode 3)
}

@group(0) @binding(0) var<storage, read> sdfInput: array<f32>;
@group(0) @binding(1) var<storage, read_write> sdfOutput: array<f32>;
@group(0) @binding(2) var<uniform> params: FilterUniforms;

fn coordToIndex(x: u32, y: u32, z: u32, res: u32) -> u32 {
  return x + y * res + z * res * res;
}

@compute @workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let res = params.resolution;
  if (global_id.x >= res || global_id.y >= res || global_id.z >= res) {
    return;
  }

  let idx = coordToIndex(global_id.x, global_id.y, global_id.z, res);
  let current = sdfInput[idx];

  // Collect the 3×3×3 neighborhood (clamped at boundaries — replicates the
  // edge value rather than introducing zero-bias). Stored flat for median.
  var samples: array<f32, 27>;
  var count: u32 = 0u;
  let cx = i32(global_id.x);
  let cy = i32(global_id.y);
  let cz = i32(global_id.z);
  let resI = i32(res);

  for (var dz: i32 = -1; dz <= 1; dz = dz + 1) {
    for (var dy: i32 = -1; dy <= 1; dy = dy + 1) {
      for (var dx: i32 = -1; dx <= 1; dx = dx + 1) {
        let nx = clamp(cx + dx, 0, resI - 1);
        let ny = clamp(cy + dy, 0, resI - 1);
        let nz = clamp(cz + dz, 0, resI - 1);
        samples[count] = sdfInput[coordToIndex(u32(nx), u32(ny), u32(nz), res)];
        count = count + 1u;
      }
    }
  }

  var filtered: f32 = current;

  if (params.mode == 0u) {
    // Gaussian: sigma=1 over a radius-1 kernel. Precomputed weights:
    //   center (0,0,0)  → 1.0
    //   face   (±1,0,0) → e^(-0.5)        ≈ 0.6065  (× 6 faces)
    //   edge   (±1,±1,0)→ e^(-1.0)        ≈ 0.3679  (× 12 edges)
    //   corner (±1,±1,±1) → e^(-1.5)      ≈ 0.2231  (× 8 corners)
    var sum: f32 = 0.0;
    var totalW: f32 = 0.0;
    var sIdx: u32 = 0u;
    for (var dz: i32 = -1; dz <= 1; dz = dz + 1) {
      for (var dy: i32 = -1; dy <= 1; dy = dy + 1) {
        for (var dx: i32 = -1; dx <= 1; dx = dx + 1) {
          let d2 = f32(dx * dx + dy * dy + dz * dz);
          let w = exp(-0.5 * d2);
          sum = sum + samples[sIdx] * w;
          totalW = totalW + w;
          sIdx = sIdx + 1u;
        }
      }
    }
    filtered = sum / totalW;

  } else if (params.mode == 1u) {
    // Mean: uniform average over all 27 samples.
    var sum: f32 = 0.0;
    for (var i: u32 = 0u; i < 27u; i = i + 1u) {
      sum = sum + samples[i];
    }
    filtered = sum / 27.0;

  } else if (params.mode == 2u) {
    // Median: insertion-sort 27 values, pick index 13. Insertion sort is
    // O(n²) but n=27 is tiny — ~700 ops per voxel, fine for a one-shot pass.
    for (var i: u32 = 1u; i < 27u; i = i + 1u) {
      let key = samples[i];
      var j: i32 = i32(i) - 1;
      loop {
        if (j < 0) { break; }
        if (samples[u32(j)] <= key) { break; }
        samples[u32(j) + 1u] = samples[u32(j)];
        j = j - 1;
      }
      samples[u32(j + 1)] = key;
    }
    filtered = samples[13];

  } else {
    // Bilateral: Gaussian spatial weight × Gaussian value-similarity weight.
    // Preserves boundaries between inside/outside (large SDF jumps don't
    // pull each other together). valueSigma controls how aggressively
    // dissimilar voxels are excluded — small sigma = sharp edges.
    var sum: f32 = 0.0;
    var totalW: f32 = 0.0;
    var sIdx: u32 = 0u;
    let vSigma = max(0.0001, params.valueSigma);
    for (var dz: i32 = -1; dz <= 1; dz = dz + 1) {
      for (var dy: i32 = -1; dy <= 1; dy = dy + 1) {
        for (var dx: i32 = -1; dx <= 1; dx = dx + 1) {
          let d2 = f32(dx * dx + dy * dy + dz * dz);
          let wSpace = exp(-0.5 * d2);
          let dv = samples[sIdx] - current;
          let wValue = exp(-(dv * dv) / (2.0 * vSigma * vSigma));
          let w = wSpace * wValue;
          sum = sum + samples[sIdx] * w;
          totalW = totalW + w;
          sIdx = sIdx + 1u;
        }
      }
    }
    filtered = sum / totalW;
  }

  sdfOutput[idx] = mix(current, filtered, params.strength);
}
`,Yo=`
${kt}
/**
 * Paint Expand Compute Shader
 *
 * Dilates the painted region of the colour volume outward by one voxel per
 * pass. For every UNPAINTED voxel (alpha < threshold), look at the 6 face
 * neighbours; if any neighbour is painted, copy its RGB and set this
 * voxel's alpha to a small "halo" value (so the next pass treats it as
 * painted, but a real brush stroke can still overwrite it cleanly).
 *
 * Painted voxels (alpha >= threshold) pass through untouched — this pass
 * never modifies the user's actual colours, only fills empty space around
 * them. Used by Sculptor.filterSDF as a post-pass so vertex sampling at
 * the new iso-surface positions doesn't fall off the painted-region cliff
 * that produced visible blocky colour patches.
 *
 * 6-neighbour kernel (not 26): each pass moves the front by exactly one
 * voxel, so N passes = N-voxel reach. Composes naturally with the SDF
 * filter's iteration count.
 */

struct ExpandUniforms {
  resolution: u32,
  // Voxels with alpha < this are considered unpainted (eligible to be
  // filled). Voxels with alpha >= this are treated as painted (sources).
  threshold: f32,
  // Alpha to write into newly-filled voxels. Below 1.0 so a future brush
  // stamp can overwrite without trilinear muddiness; above 'threshold' so
  // the next dilation pass treats this voxel as a source.
  haloAlpha: f32,
  _pad: u32,
}

// Packed RGBA8: one u32 per voxel. Read with unpackRGBA8, write with packRGBA8.
@group(0) @binding(0) var<storage, read> colorInput: array<u32>;
@group(0) @binding(1) var<storage, read_write> colorOutput: array<u32>;
@group(0) @binding(2) var<uniform> params: ExpandUniforms;

fn coordToIndex(x: u32, y: u32, z: u32, res: u32) -> u32 {
  return x + y * res + z * res * res;
}

@compute @workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let res = params.resolution;
  if (global_id.x >= res || global_id.y >= res || global_id.z >= res) {
    return;
  }

  let idx = coordToIndex(global_id.x, global_id.y, global_id.z, res);
  let currentPacked = colorInput[idx];
  let current = unpackRGBA8(currentPacked);

  // Already painted → leave alone. Only unpainted voxels get filled.
  if (current.a >= params.threshold) {
    colorOutput[idx] = currentPacked;
    return;
  }

  // Look at 6 face neighbours. First painted one wins (averaging multiple
  // sources tends to mud the boundary; nearest-painted gives a cleaner
  // halo). Order: -x, +x, -y, +y, -z, +z.
  let cx = i32(global_id.x);
  let cy = i32(global_id.y);
  let cz = i32(global_id.z);
  let resI = i32(res);

  var picked = vec3<f32>(0.0);
  var found = false;

  // -x
  if (!found && cx > 0) {
    let n = unpackRGBA8(colorInput[coordToIndex(u32(cx - 1), u32(cy), u32(cz), res)]);
    if (n.a >= params.threshold) { picked = n.rgb; found = true; }
  }
  // +x
  if (!found && cx < resI - 1) {
    let n = unpackRGBA8(colorInput[coordToIndex(u32(cx + 1), u32(cy), u32(cz), res)]);
    if (n.a >= params.threshold) { picked = n.rgb; found = true; }
  }
  // -y
  if (!found && cy > 0) {
    let n = unpackRGBA8(colorInput[coordToIndex(u32(cx), u32(cy - 1), u32(cz), res)]);
    if (n.a >= params.threshold) { picked = n.rgb; found = true; }
  }
  // +y
  if (!found && cy < resI - 1) {
    let n = unpackRGBA8(colorInput[coordToIndex(u32(cx), u32(cy + 1), u32(cz), res)]);
    if (n.a >= params.threshold) { picked = n.rgb; found = true; }
  }
  // -z
  if (!found && cz > 0) {
    let n = unpackRGBA8(colorInput[coordToIndex(u32(cx), u32(cy), u32(cz - 1), res)]);
    if (n.a >= params.threshold) { picked = n.rgb; found = true; }
  }
  // +z
  if (!found && cz < resI - 1) {
    let n = unpackRGBA8(colorInput[coordToIndex(u32(cx), u32(cy), u32(cz + 1), res)]);
    if (n.a >= params.threshold) { picked = n.rgb; found = true; }
  }

  if (found) {
    colorOutput[idx] = packRGBA8(vec4<f32>(picked, params.haloAlpha));
  } else {
    // No painted neighbour — leave unpainted.
    colorOutput[idx] = currentPacked;
  }
}
`,qo=`
struct InitParams {
  resolution: u32,
  _pad0: u32,
  _pad1: u32,
  _pad2: u32,
}

@group(0) @binding(0) var<storage, read_write> dst: array<f32>;
// Packed RGBA8: one u32 per voxel. 0u = (0,0,0,0) = unpainted.
@group(0) @binding(1) var<storage, read_write> dst_color: array<u32>;
@group(0) @binding(2) var<uniform> params: InitParams;

@compute @workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let res = params.resolution;
  if (gid.x >= res || gid.y >= res || gid.z >= res) { return; }
  let idx = gid.x + gid.y * res + gid.z * res * res;
  // SDF identity for union = 1.0 (outside).
  dst[idx] = 1.0;
  // Color identity = (0,0,0,0). alpha=0 marks "unpainted" so the noise
  // composite + vertex sampler treat the voxel as having no brush color.
  // Packed RGBA8: u32 0 = all bytes zero = (0,0,0,0).
  dst_color[idx] = 0u;
}
`,Ho=`
struct UnionParams {
  resolution: u32,
  offset_x: i32,    // src sample offset in voxels (read src at gid - offset)
  offset_y: i32,
  offset_z: i32,
  _pad0: u32,
  _pad1: u32,
  _pad2: u32,
  _pad3: u32,
}

@group(0) @binding(0) var<storage, read_write> dst: array<f32>;
// Packed RGBA8: one u32 per voxel. Straight u32 copy preserves all 4 channels.
@group(0) @binding(1) var<storage, read_write> dst_color: array<u32>;
@group(0) @binding(2) var<storage, read> src: array<f32>;
@group(0) @binding(3) var<storage, read> src_color: array<u32>;
@group(0) @binding(4) var<uniform> params: UnionParams;

@compute @workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let res = params.resolution;
  if (gid.x >= res || gid.y >= res || gid.z >= res) { return; }
  let idx = gid.x + gid.y * res + gid.z * res * res;

  // Union with translated src. src is read at \`gid - offset\` (voxel units).
  // Out-of-bounds reads return identity values so a shifted layer doesn't
  // tile or wrap: 1.0 for SDF (= outside), 0u for color (packed (0,0,0,0)).
  let src_x = i32(gid.x) - params.offset_x;
  let src_y = i32(gid.y) - params.offset_y;
  let src_z = i32(gid.z) - params.offset_z;
  var src_val: f32 = 1.0;
  var src_col: u32 = 0u;
  if (src_x >= 0 && src_x < i32(res) &&
      src_y >= 0 && src_y < i32(res) &&
      src_z >= 0 && src_z < i32(res)) {
    let src_idx = u32(src_x) + u32(src_y) * res + u32(src_z) * res * res;
    src_val = src[src_idx];
    src_col = src_color[src_idx];
  }

  // Phase A.11d color-isolation. SDF still uses min() (commutative). For
  // color, the brush layer whose SDF is the smallest at this voxel "wins"
  // ownership — its color appears in the composite. Voxels where src is
  // not actively contributing (src >= dst) keep the existing dst_color so
  // earlier-iterated layers' colors aren't clobbered by later layers that
  // don't reach this voxel.
  if (src_val < dst[idx]) {
    dst[idx] = src_val;
    dst_color[idx] = src_col;
  }
}
`,jo=`
${kt}
/**
 * Noise Composite Compute Shader
 *
 * Reads baseSdf, composes N parametric noise stamps on top via union (min),
 * writes compositeSdf. Each invocation processes one voxel.
 *
 * Stamp model: value-noise FBM × spherical falloff envelope. SDF contribution
 * is negative (= inside, matches the project's "negative = solid" convention).
 *
 * MAX_LAYERS = 8 in v1 — see TASK-VOXEL-SCULPT-NOISE-FIELD.md § Decisions
 * LOCKED. Bumping requires a corresponding bump in COMPOSITE_UNIFORM_SIZE
 * and NOISE_LAYER_STRIDE on the JS side (see noise/types.ts).
 */

const MAX_LAYERS: u32 = 8u;

// Noise type enum — must match noise/types.ts § NOISE_TYPE_* constants.
// Adding a new type requires bumping BOTH sides + adding a case in the
// sample_noise dispatch below.
const NOISE_TYPE_SMOOTH_FBM: u32        = 0u;
const NOISE_TYPE_RIDGE: u32             = 1u;
const NOISE_TYPE_DOMAIN_WARPED_FBM: u32 = 2u;
const NOISE_TYPE_WORLEY_F1: u32         = 3u;
const NOISE_TYPE_SMOOTH_VORONOI: u32    = 4u;
const NOISE_TYPE_VORONOI_EDGES: u32     = 5u;

// One stamp. Layout matches noise/types.ts § NOISE_LAYER_STRIDE (64 bytes).
// Stride is 64 because the color (vec3<f32>) at offset 48 needs 16-byte
// alignment, and trailing pad keeps the array element multiple-of-16.
// \`size\` packs into the position vec3 tail (offset 12).
struct NoiseLayer {
  position: vec3<f32>,   // offset  0..11 — WORLD coords [-halfExtent, +halfExtent]
  size: f32,             // offset 12..15 — falloff radius in world units
  scale: f32,            // offset 16..19 — base frequency / cell density
  persistence: f32,      // offset 20..23 — FBM amplitude falloff per octave
  strength: f32,         // offset 24..27 — SDF contribution magnitude
  seed: f32,             // offset 28..31 — hashed per-axis below
  octaves: u32,          // offset 32..35 — 1..6
  enabled: u32,          // offset 36..39 — 0 = skip, non-zero = include
  noiseType: u32,        // offset 40..43 — Phase A.8 — see NOISE_TYPE_* above
  _pad: u32,             // offset 44..47 — reserved
  color: vec3<f32>,      // offset 48..59 — Phase A.11a — per-stamp RGB [0,1]³
  _pad2: f32,            // offset 60..63 — keeps 64-byte stride
}

struct CompositeUniforms {
  layerCount: u32,                       // offset  0
  volumeExtent: f32,                     // offset  4 — shipped from SculptCanvas
  resolution: u32,                       // offset  8
  _pad: u32,                             // offset 12 — reserved
  layers: array<NoiseLayer, MAX_LAYERS>, // offset 16..527 (8 × 64 = 512 bytes)
}

@group(0) @binding(0) var<storage, read> baseSdf: array<f32>;
@group(0) @binding(1) var<storage, read_write> compositeSdf: array<f32>;
@group(0) @binding(2) var<uniform> params: CompositeUniforms;
// Packed RGBA8: one u32 per voxel. Read with unpackRGBA8, write with packRGBA8.
@group(0) @binding(3) var<storage, read_write> colorVolume: array<u32>;

fn coordToIndex(x: u32, y: u32, z: u32, res: u32) -> u32 {
  return x + y * res + z * res * res;
}

// Cheap 3D hash → [0, 1]. Schechter & Bridson 2008-ish — adequate for FBM
// lookups, no gradient table needed. Used by valueNoise3 below.
fn hash13(p: vec3<f32>) -> f32 {
  var p3 = fract(p * vec3<f32>(0.1031));
  p3 = p3 + dot(p3, p3.yzx + 33.33);
  return fract((p3.x + p3.y) * p3.z);
}

// Value noise in [-1, 1]. Cube-aligned at very low octaves but FBM with
// octaves >= 2 hides it. If Phase A.7 visual check finds the cube-alignment
// too obvious, swap this single function for simplex noise without touching
// any caller (FBM, composite, JS-side pipeline all stay the same).
fn valueNoise3(p: vec3<f32>) -> f32 {
  let i = floor(p);
  let f = fract(p);
  let u = f * f * (3.0 - 2.0 * f);  // smoothstep

  // 8 corner values, each mapped from [0,1] → [-1,1]
  let c000 = hash13(i + vec3<f32>(0.0, 0.0, 0.0)) * 2.0 - 1.0;
  let c100 = hash13(i + vec3<f32>(1.0, 0.0, 0.0)) * 2.0 - 1.0;
  let c010 = hash13(i + vec3<f32>(0.0, 1.0, 0.0)) * 2.0 - 1.0;
  let c110 = hash13(i + vec3<f32>(1.0, 1.0, 0.0)) * 2.0 - 1.0;
  let c001 = hash13(i + vec3<f32>(0.0, 0.0, 1.0)) * 2.0 - 1.0;
  let c101 = hash13(i + vec3<f32>(1.0, 0.0, 1.0)) * 2.0 - 1.0;
  let c011 = hash13(i + vec3<f32>(0.0, 1.0, 1.0)) * 2.0 - 1.0;
  let c111 = hash13(i + vec3<f32>(1.0, 1.0, 1.0)) * 2.0 - 1.0;

  let x00 = mix(c000, c100, u.x);
  let x10 = mix(c010, c110, u.x);
  let x01 = mix(c001, c101, u.x);
  let x11 = mix(c011, c111, u.x);
  let y0 = mix(x00, x10, u.y);
  let y1 = mix(x01, x11, u.y);
  return mix(y0, y1, u.z);
}

// Fractal sum of value noise. Lacunarity locked at 2.0 (frequency doubles
// per octave). Persistence (amplitude falloff per octave) is per-stamp.
// Normalized to ~[-1, 1] by dividing by total amplitude.
fn fbm(p: vec3<f32>, octaves: u32, persistence: f32) -> f32 {
  var sum: f32 = 0.0;
  var amp: f32 = 1.0;
  var freq: f32 = 1.0;
  var totalAmp: f32 = 0.0;
  let octavesClamped = clamp(octaves, 1u, 6u);

  for (var i: u32 = 0u; i < octavesClamped; i = i + 1u) {
    sum = sum + amp * valueNoise3(p * freq);
    totalAmp = totalAmp + amp;
    amp = amp * persistence;
    freq = freq * 2.0;
  }

  if (totalAmp <= 0.0) { return 0.0; }
  return sum / totalAmp;
}

// ─── Phase A.8 — Higher-quality hash + cellular/Voronoi/Ridge/Warped ────
//
// Foundation: PCG3D (Jarzynski/Olano 2020) — significantly better 3D hash
// than the fract-sin family hash13 used above. Every cellular variant
// benefits. The catalog (claudedocs/research/noise-shapes-visual-catalog.html)
// flags this as Day-1 of the noise sprint.

fn pcg3d(v: vec3<u32>) -> vec3<u32> {
  var x = v * 1664525u + 1013904223u;
  x.x = x.x + x.y * x.z;
  x.y = x.y + x.z * x.x;
  x.z = x.z + x.x * x.y;
  x = x ^ (x >> vec3<u32>(16u, 16u, 16u));
  x.x = x.x + x.y * x.z;
  x.y = x.y + x.z * x.x;
  x.z = x.z + x.x * x.y;
  return x;
}

// 3D hash → vec3<f32> in [0, 1]³ from a cell-integer-valued input.
// Bitcast of the floor()'d vec3 to u32 reinterprets the IEEE bits — works
// for negative inputs too since PCG3D treats its input as opaque bits.
fn hash3(cell: vec3<f32>) -> vec3<f32> {
  let u = bitcast<vec3<u32>>(cell);
  let h = pcg3d(u);
  return vec3<f32>(h) * (1.0 / 4294967296.0);
}

// Ridge FBM — squared (1 - |noise|) per octave. Produces sharp veined
// ridges instead of smooth rolling forms. Catalog § "razor-edged crystal"
// adjacency but in stochastic form (Voronoi Edges is the deterministic
// crystalline counterpart).
fn fbm_ridge(p: vec3<f32>, octaves: u32, persistence: f32) -> f32 {
  var sum: f32 = 0.0;
  var amp: f32 = 1.0;
  var freq: f32 = 1.0;
  var totalAmp: f32 = 0.0;
  let octavesClamped = clamp(octaves, 1u, 6u);
  for (var i: u32 = 0u; i < octavesClamped; i = i + 1u) {
    let v = valueNoise3(p * freq);
    let ridged = 1.0 - abs(v);  // [0, 1] — peaks where v=0
    sum = sum + amp * ridged * ridged;
    totalAmp = totalAmp + amp;
    amp = amp * persistence;
    freq = freq * 2.0;
  }
  if (totalAmp <= 0.0) { return 0.0; }
  // Remap [0, 1] → [-1, 1] for the SDF displacement convention.
  return (sum / totalAmp) * 2.0 - 1.0;
}

// Domain-warped FBM (Quilez 2002, 3-level recursive). Each level perturbs
// the sample position of the next, producing flowing marbled / painterly
// surfaces. Expensive: 7 fbm calls per voxel = octaves × 7 noise samples.
// Catalog § "marbled, hand-painted swirls (ink wash)".
fn fbm_warped(p: vec3<f32>, octaves: u32, persistence: f32) -> f32 {
  let q = vec3<f32>(
    fbm(p, octaves, persistence),
    fbm(p + vec3<f32>(5.2, 1.3, 2.8), octaves, persistence),
    fbm(p + vec3<f32>(3.7, 8.4, 4.5), octaves, persistence),
  );
  let r = vec3<f32>(
    fbm(p + 4.0 * q, octaves, persistence),
    fbm(p + 4.0 * q + vec3<f32>(1.7, 9.2, 6.3), octaves, persistence),
    fbm(p + 4.0 * q + vec3<f32>(8.3, 2.8, 1.4), octaves, persistence),
  );
  return fbm(p + 4.0 * r, octaves, persistence);
}

// Worley F1 (Worley 1996) — distance to nearest jittered cell point. Used
// for cellular / foam / stone-like surfaces. The 3×3×3 neighbour scan is
// the canonical correctness window for 3D Worley. Catalog § "Cellular foam
// / pollen / filigree".
//
// Returns a signed value in roughly [-1, 1] for SDF displacement: small
// minDist (= near cell center) → negative (carve a dimple); large minDist
// (= near cell boundary) → positive (push out a ridge). The 1.0 normalizer
// matches the max F1 distance for jittered cells in a unit grid.
fn worley_f1(p: vec3<f32>) -> f32 {
  let cell = floor(p);
  let frac = p - cell;
  var minD2: f32 = 4.0;
  for (var dz: i32 = -1; dz <= 1; dz = dz + 1) {
    for (var dy: i32 = -1; dy <= 1; dy = dy + 1) {
      for (var dx: i32 = -1; dx <= 1; dx = dx + 1) {
        let offset = vec3<f32>(f32(dx), f32(dy), f32(dz));
        let jitter = hash3(cell + offset);
        let site = offset + jitter;
        let dv = site - frac;
        let d2 = dot(dv, dv);
        minD2 = min(minD2, d2);
      }
    }
  }
  let d = sqrt(minD2);
  // Remap to [-1, 1]: d ranges roughly [0, 1.1] for jittered grid; 0.5 is
  // the natural midpoint between cell center and cell boundary.
  return clamp(d * 2.0 - 1.0, -1.0, 1.0);
}

// Smooth Voronoi (Quilez — iquilezles.org/articles/smoothvoronoi). Replaces
// the hard min() with an exponential soft-min, producing wet bubble / foam
// / organic-skin character. The 32.0 smoothness factor matches Quilez's
// canonical demo. Catalog § "Wet bubbles, foam, organic skin".
fn smooth_voronoi(p: vec3<f32>) -> f32 {
  let cell = floor(p);
  let frac = p - cell;
  var r: f32 = 0.0;
  let smoothness: f32 = 32.0;
  for (var dz: i32 = -1; dz <= 1; dz = dz + 1) {
    for (var dy: i32 = -1; dy <= 1; dy = dy + 1) {
      for (var dx: i32 = -1; dx <= 1; dx = dx + 1) {
        let offset = vec3<f32>(f32(dx), f32(dy), f32(dz));
        let jitter = hash3(cell + offset);
        let site = offset + jitter;
        let d = length(site - frac);
        r = r + exp(-smoothness * d);
      }
    }
  }
  // -log(r) / smoothness approximates the min distance with smooth wells.
  let d = -log(max(r, 0.0001)) / smoothness;
  return clamp(d * 2.0 - 1.0, -1.0, 1.0);
}

// Voronoi Edges (Quilez — iquilezles.org/articles/voronoilines). Two-pass:
// (1) find closest cell, (2) measure distance to the bisector plane
// between p's closest cell and each other cell. Razor-edged crystalline
// look — geode / shattered glass / mosaic. Catalog § "razor-edged crystal".
//
// The 2-cell radius search (5×5×5 = 125 iterations) is required for the
// bisector to be correct — neighbour-of-neighbour cells can still own the
// closest bisector edge. Expensive but cleaner than the 3×3×3 approximation.
fn voronoi_edges(p: vec3<f32>) -> f32 {
  let cell = floor(p);
  let frac = p - cell;

  // Pass 1: find closest cell.
  var minD2: f32 = 4.0;
  var closestSite: vec3<f32> = vec3<f32>(0.0);
  for (var dz: i32 = -1; dz <= 1; dz = dz + 1) {
    for (var dy: i32 = -1; dy <= 1; dy = dy + 1) {
      for (var dx: i32 = -1; dx <= 1; dx = dx + 1) {
        let offset = vec3<f32>(f32(dx), f32(dy), f32(dz));
        let jitter = hash3(cell + offset);
        let site = offset + jitter;
        let dv = site - frac;
        let d2 = dot(dv, dv);
        if (d2 < minD2) {
          minD2 = d2;
          closestSite = site;
        }
      }
    }
  }

  // Pass 2: minimum distance to bisector plane.
  var minEdge: f32 = 4.0;
  for (var dz: i32 = -2; dz <= 2; dz = dz + 1) {
    for (var dy: i32 = -2; dy <= 2; dy = dy + 1) {
      for (var dx: i32 = -2; dx <= 2; dx = dx + 1) {
        let offset = vec3<f32>(f32(dx), f32(dy), f32(dz));
        let jitter = hash3(cell + offset);
        let site = offset + jitter;
        let r = site - closestSite;
        let r2 = dot(r, r);
        if (r2 > 0.00001) {
          // Distance from frac to the bisector plane between closestSite and site.
          let mid = 0.5 * (closestSite + site);
          let n = r / sqrt(r2);
          let edgeDist = abs(dot(frac - mid, n));
          minEdge = min(minEdge, edgeDist);
        }
      }
    }
  }
  // Small minEdge (on cell wall) → +1 displacement (push out a ridge).
  // Large minEdge (deep inside cell) → -1 displacement (carve a dimple).
  // The * 3.0 scales the natural [0, ~0.5] edge-distance range into the
  // [-1, 1] displacement range; clamp keeps it bounded.
  return clamp(1.0 - minEdge * 3.0 * 2.0, -1.0, 1.0);
}

// ─── Dispatch ───────────────────────────────────────────────────────────
//
// Returns noise value in [-1, 1] for SDF displacement (matches the shipped
// formula in main). Each type's internal output is remapped to this range
// inside its function.

fn sample_noise(noiseType: u32, p: vec3<f32>, octaves: u32, persistence: f32) -> f32 {
  switch noiseType {
    case 0u: { return fbm(p, octaves, persistence); }
    case 1u: { return fbm_ridge(p, octaves, persistence); }
    case 2u: { return fbm_warped(p, octaves, persistence); }
    case 3u: { return worley_f1(p); }
    case 4u: { return smooth_voronoi(p); }
    case 5u: { return voronoi_edges(p); }
    default: { return fbm(p, octaves, persistence); }
  }
}

@compute @workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let res = params.resolution;
  if (gid.x >= res || gid.y >= res || gid.z >= res) { return; }

  let idx = coordToIndex(gid.x, gid.y, gid.z, res);
  let p = vec3<f32>(gid) / f32(res);  // normalized [0, 1]³
  // Phase A.11c — baseSdf is now the brush-union output (built by Sculptor's
  // BrushUnionPipeline from per-layer brush SDFs). Per-brush-layer enable
  // flags are handled JS-side: disabled brush layers don't contribute to the
  // union, so baseSdf here already reflects the visible-brush state.
  var sdf: f32 = baseSdf[idx];

  // Phase A.11a — track which layer (if any) contributes material at this
  // voxel and what color to write. Last enabled layer to claim the voxel
  // wins (matches the SDF min() last-write semantic).
  var noiseColor: vec3<f32> = vec3<f32>(0.0);
  var anyNoise: bool = false;

  let halfExtent = params.volumeExtent * 0.5;
  let lc = min(params.layerCount, MAX_LAYERS);

  for (var i: u32 = 0u; i < lc; i = i + 1u) {
    let L = params.layers[i];
    if (L.enabled == 0u) { continue; }

    // World coords [-halfExtent, +halfExtent] → normalized [0, 1].
    // (Direct world→normalized; fixes the spec's original off-by-0.125 map
    // that assumed position was in [-1, +1] — see TASK § audit findings.)
    let center = (L.position + vec3<f32>(halfExtent)) / params.volumeExtent;
    let d = length(p - center);
    let r = L.size / params.volumeExtent;

    // Outer influence radius — allows noise-driven bumps to extend up to
    // 30% past the nominal stamp radius. Voxels beyond are unaffected
    // (baseSdf preserved by the early-continue below).
    let r_outer = r * 1.3;
    if (d > r_outer) { continue; }

    // Hash the seed into a 3-component offset via distinct prime multipliers
    // per axis. Plain vec3(seed,seed,seed) correlates all stamps along the
    // (1,1,1) noise-lattice diagonal — sequential seeds would look near-
    // identical. Primes 17/31/47 keep axis offsets uncorrelated.
    let seedOffset = vec3<f32>(L.seed * 17.0, L.seed * 31.0, L.seed * 47.0);
    // Phase A.8 — per-layer noise type dispatch. sample_noise returns [-1, 1]
    // for all 6 types; the displacement formula below is type-agnostic.
    let n = sample_noise(L.noiseType, p * L.scale * 8.0 + seedOffset, L.octaves, L.persistence);

    // Noisy sphere SDF: displace the sphere boundary by signed FBM noise.
    //   n > 0 → surface pushes OUTWARD (bump)
    //   n < 0 → surface pulls INWARD  (dimple)
    // Amplitude scales with both strength and stamp radius — bigger stamps
    // get proportionally bigger bumps. The 0.5 keeps max displacement at
    // ±strength*r*0.5: at default strength=0.8 that's ±0.4r (clearly broken
    // organic form); at max strength=1.5 it's ±0.75r (wild).
    //
    // Phase A.7 fix (2026-05-25): replaces the original "density-gated by
    // noise inside a spherical envelope" formulation, which only carved
    // material where noise ≈ -1 (rare) and rendered as a textured-but-still-
    // spherical blob. The displacement formulation makes signed noise drive
    // the surface position directly — bumps AND dimples follow the noise
    // field, producing the broken organic character the spec called for.
    let sphereSdf = d - r;
    let displacement = n * L.strength * r * 0.5;
    let layer_sdf = sphereSdf - displacement;

    // Track color attribution: if this layer contributes material at this
    // voxel (layer_sdf < 0 means "inside this layer's iso-surface"), its
    // color becomes the candidate. Last enabled contributing layer wins.
    if (layer_sdf < 0.0) {
      noiseColor = L.color;
      anyNoise = true;
    }

    sdf = min(sdf, layer_sdf);
  }

  compositeSdf[idx] = sdf;

  // Phase A.11a — color attribution write-back. Three cases:
  //   - colorVolume[idx].a > 0.05 → brush-painted, preserve (brush wins on overlap).
  //     The brushApplyShader writes alpha via mix(currentAlpha, brushAlpha,
  //     paintStrength) so partial-strength strokes produce voxels with
  //     alpha in (0, 1). A strict >= 0.99 gate (the prior value) treated
  //     soft brush edges as "unpainted" and the else-branch below cleared
  //     them on every composite tick — visible as brush color disappearing
  //     whenever a noise stamp was added. > 0.05 matches paint-expand's
  //     "any brush contribution counts" convention.
  //   - anyNoise → write the top contributing layer's color at α=0.95
  //     (distinct from brush, only reached when this voxel has no brush
  //     contribution at all).
  //   - else → clear (handles "delete a stamp": voxels previously owned by
  //     that stamp now have no claimant, must return to unpainted state so
  //     ghost color doesn't linger after deletion).
  let existing = unpackRGBA8(colorVolume[idx]);
  let isBrushPainted = existing.a > 0.05;
  if (!isBrushPainted) {
    if (anyNoise) {
      colorVolume[idx] = packRGBA8(vec4<f32>(noiseColor, 0.95));
    } else {
      colorVolume[idx] = 0u;
    }
  }
}
`,Zo=`
${kt}
/**
 * Local Color Smoothing Compute Shader (Phase G)
 *
 * Smooths the color volume in a brush-radius sphere around brush.position.
 * Used by the Smooth tool (Shift+drag) to blend nearby colors in the same
 * stroke as SDF smoothing.
 *
 * Honors the alpha-tagged painted-ness contract (Phase F Issue 2):
 * - Voxels with alpha < 0.01 are unpainted and skipped (preserve unpainted state).
 * - Neighbors with alpha < 0.01 don't contribute to the weighted average
 *   (unpainted shouldn't dilute painted's saturation).
 * - Original alpha is preserved on write.
 */

struct LocalSmoothUniforms {
  resolution: u32,
  kernelSize: u32,        // 1 = 3x3x3, 2 = 5x5x5
  strength: f32,          // 0-1 base blend factor
  hardness: f32,          // 0-1 falloff curve
  brushPos: vec3<f32>,    // brush center in voxel space [0, resolution)
  brushRadius: f32,       // in voxel units
}

// Packed RGBA8: one u32 per voxel. Read with unpackRGBA8, write with packRGBA8.
@group(0) @binding(0) var<storage, read> colorInput: array<u32>;
@group(0) @binding(1) var<storage, read_write> colorOutput: array<u32>;
@group(0) @binding(2) var<uniform> params: LocalSmoothUniforms;

fn coordToIndex(x: u32, y: u32, z: u32, res: u32) -> u32 {
  return x + y * res + z * res * res;
}

fn gaussianWeight(dx: i32, dy: i32, dz: i32, sigma: f32) -> f32 {
  let distSq = f32(dx * dx + dy * dy + dz * dz);
  return exp(-distSq / (2.0 * sigma * sigma));
}

@compute @workgroup_size(8, 8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let res = params.resolution;

  if (global_id.x >= res || global_id.y >= res || global_id.z >= res) {
    return;
  }

  let idx = coordToIndex(global_id.x, global_id.y, global_id.z, res);
  let currentPacked = colorInput[idx];
  let currentColor = unpackRGBA8(currentPacked);

  // Skip unpainted voxels — preserve unpainted state.
  if (currentColor.a < 0.01) {
    colorOutput[idx] = currentPacked;
    return;
  }

  // Out-of-brush voxels: pass through unchanged.
  let voxelPos = vec3<f32>(global_id);
  let dist = distance(voxelPos, params.brushPos);
  if (dist > params.brushRadius) {
    colorOutput[idx] = currentPacked;
    return;
  }

  // Per-voxel falloff (smoothstep from outer to inner edge per hardness).
  let outerEdge = mix(params.brushRadius * 1.5, params.brushRadius * 1.0, params.hardness);
  let innerEdge = mix(params.brushRadius * 0.5, params.brushRadius * 0.9, params.hardness);
  let falloff = smoothstep(outerEdge, innerEdge, dist);
  let effectiveStrength = params.strength * falloff;

  // Gaussian blur over neighborhood, alpha-weighted.
  let radius = i32(params.kernelSize);
  let sigma = f32(radius) * 0.5 + 0.5;

  var weightedSum = vec4<f32>(0.0);
  var totalWeight = 0.0;

  for (var dz: i32 = -radius; dz <= radius; dz++) {
    for (var dy: i32 = -radius; dy <= radius; dy++) {
      for (var dx: i32 = -radius; dx <= radius; dx++) {
        let nx = i32(global_id.x) + dx;
        let ny = i32(global_id.y) + dy;
        let nz = i32(global_id.z) + dz;

        if (nx >= 0 && nx < i32(res) && ny >= 0 && ny < i32(res) && nz >= 0 && nz < i32(res)) {
          let neighborIdx = coordToIndex(u32(nx), u32(ny), u32(nz), res);
          let neighborColor = unpackRGBA8(colorInput[neighborIdx]);

          // Only painted neighbors contribute (alpha-tagged painted-ness).
          if (neighborColor.a > 0.01) {
            let weight = gaussianWeight(dx, dy, dz, sigma);
            weightedSum += neighborColor * weight;
            totalWeight += weight;
          }
        }
      }
    }
  }

  var blurredColor = currentColor;
  if (totalWeight > 0.0) {
    blurredColor = weightedSum / totalWeight;
  }
  // Preserve original alpha — do not downgrade painted-ness.
  blurredColor.a = currentColor.a;

  colorOutput[idx] = packRGBA8(mix(currentColor, blurredColor, effectiveStrength));
}
`,Xo=`
/**
 * Volume Resampling Shader
 *
 * Copies SDF/Color data from a smaller volume to a larger one,
 * placing the old data at an offset to allow expansion in any direction.
 *
 * The shader processes voxels in the DESTINATION volume.
 * For each destination voxel, it calculates the corresponding source voxel
 * (if any) and copies the value. Voxels outside the source region get
 * the default value (1.0 for SDF = outside, or default color).
 */

// Uniforms
struct ResampleParams {
    // Source resolution (e.g., 64, 128)
    src_res: u32,
    // Destination resolution (e.g., 128, 256)
    dst_res: u32,
    // Offset in destination volume where source data starts (x)
    offset_x: i32,
    // Offset in destination volume where source data starts (y)
    offset_y: i32,
    // Offset in destination volume where source data starts (z)
    offset_z: i32,
    // Default value for empty voxels (1.0 for SDF outside)
    default_value: f32,
    // Padding for alignment
    _pad0: u32,
    _pad1: u32,
}

@group(0) @binding(0) var<storage, read> src_volume: array<f32>;
@group(0) @binding(1) var<storage, read_write> dst_volume: array<f32>;
@group(0) @binding(2) var<uniform> params: ResampleParams;

/**
 * Convert 3D coordinates to 1D buffer index.
 */
fn idx3d(x: u32, y: u32, z: u32, res: u32) -> u32 {
    return x + y * res + z * res * res;
}

@compute @workgroup_size(8, 8, 8)
fn resample_sdf(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dst_x = gid.x;
    let dst_y = gid.y;
    let dst_z = gid.z;

    // Bounds check for destination
    if (dst_x >= params.dst_res || dst_y >= params.dst_res || dst_z >= params.dst_res) {
        return;
    }

    // Calculate corresponding source coordinates
    let src_x = i32(dst_x) - params.offset_x;
    let src_y = i32(dst_y) - params.offset_y;
    let src_z = i32(dst_z) - params.offset_z;

    // Check if this destination voxel maps to a valid source voxel
    let src_res_i = i32(params.src_res);
    let in_source = src_x >= 0 && src_x < src_res_i &&
                    src_y >= 0 && src_y < src_res_i &&
                    src_z >= 0 && src_z < src_res_i;

    // Get destination index
    let dst_idx = idx3d(dst_x, dst_y, dst_z, params.dst_res);

    if (in_source) {
        // Copy from source
        let src_idx = idx3d(u32(src_x), u32(src_y), u32(src_z), params.src_res);
        dst_volume[dst_idx] = src_volume[src_idx];
    } else {
        // Outside source bounds - use default value
        dst_volume[dst_idx] = params.default_value;
    }
}
`,Ko=`
/**
 * Color Volume Resampling Shader
 *
 * Same logic as volumeResampleShader but handles RGBA (vec4) per voxel.
 */

struct ResampleParams {
    src_res: u32,
    dst_res: u32,
    offset_x: i32,
    offset_y: i32,
    offset_z: i32,
    default_value: f32, // unused for color, but keeps struct consistent
    _pad0: u32,
    _pad1: u32,
}

// Packed RGBA8: one u32 per voxel. Straight u32 copy preserves all 4 channels.
@group(0) @binding(0) var<storage, read> src_color: array<u32>;
@group(0) @binding(1) var<storage, read_write> dst_color: array<u32>;
@group(0) @binding(2) var<uniform> params: ResampleParams;

fn idx3d(x: u32, y: u32, z: u32, res: u32) -> u32 {
    return x + y * res + z * res * res;
}

@compute @workgroup_size(8, 8, 8)
fn resample_color(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dst_x = gid.x;
    let dst_y = gid.y;
    let dst_z = gid.z;

    if (dst_x >= params.dst_res || dst_y >= params.dst_res || dst_z >= params.dst_res) {
        return;
    }

    let src_x = i32(dst_x) - params.offset_x;
    let src_y = i32(dst_y) - params.offset_y;
    let src_z = i32(dst_z) - params.offset_z;

    let src_res_i = i32(params.src_res);
    let in_source = src_x >= 0 && src_x < src_res_i &&
                    src_y >= 0 && src_y < src_res_i &&
                    src_z >= 0 && src_z < src_res_i;

    let dst_idx = idx3d(dst_x, dst_y, dst_z, params.dst_res);

    if (in_source) {
        let src_idx = idx3d(u32(src_x), u32(src_y), u32(src_z), params.src_res);
        dst_color[dst_idx] = src_color[src_idx];
    } else {
        // Default gray color for empty voxels: packRGBA8(0.8, 0.8, 0.8, 1.0).
        // Pre-computed: round(0.8 * 255) = 204 = 0xCC, alpha = 255 = 0xFF.
        // u32 = (A << 24) | (B << 16) | (G << 8) | R = 0xFFCCCCCCu.
        dst_color[dst_idx] = 0xFFCCCCCCu;
    }
}
`,Zt=we("VolumeResizer"),lr=32;class Jo{context;sdfPipeline=null;colorPipeline=null;uniformBuffer=null;initialized=!1;constructor(e){this.context=e}async initialize(){if(this.initialized)return;const e=this.context.device.createShaderModule({label:"VolumeResizer SDF Shader",code:Xo});this.sdfPipeline=await this.context.device.createComputePipelineAsync({label:"VolumeResizer SDF Pipeline",layout:"auto",compute:{module:e,entryPoint:"resample_sdf"}});const i=this.context.device.createShaderModule({label:"VolumeResizer Color Shader",code:Ko});this.colorPipeline=await this.context.device.createComputePipelineAsync({label:"VolumeResizer Color Pipeline",layout:"auto",compute:{module:i,entryPoint:"resample_color"}}),this.uniformBuffer=this.context.device.createBuffer({label:"VolumeResizer Uniform Buffer",size:lr,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST}),this.initialized=!0,Zt.info("Initialized")}async resizeSDF(e,i,o){if(!this.initialized||!this.sdfPipeline||!this.uniformBuffer)throw new Error("VolumeResizer not initialized");const n=new ArrayBuffer(lr),u=new DataView(n);u.setUint32(0,o.srcResolution,!0),u.setUint32(4,o.dstResolution,!0),u.setInt32(8,o.offset.x,!0),u.setInt32(12,o.offset.y,!0),u.setInt32(16,o.offset.z,!0),u.setFloat32(20,o.defaultValue??1,!0),u.setUint32(24,0,!0),u.setUint32(28,0,!0),this.context.device.queue.writeBuffer(this.uniformBuffer,0,n);const c=this.context.device.createBindGroup({label:"VolumeResizer SDF BindGroup",layout:this.sdfPipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:e}},{binding:1,resource:{buffer:i}},{binding:2,resource:{buffer:this.uniformBuffer}}]}),d=this.context.device.createCommandEncoder(),f=d.beginComputePass();f.setPipeline(this.sdfPipeline),f.setBindGroup(0,c);const g=8,x=Math.ceil(o.dstResolution/g),v=Math.ceil(o.dstResolution/g),_=Math.ceil(o.dstResolution/g);f.dispatchWorkgroups(x,v,_),f.end(),this.context.device.queue.submit([d.finish()]),await this.context.device.queue.onSubmittedWorkDone(),Zt.info(`Resized SDF ${o.srcResolution}³ → ${o.dstResolution}³`)}async resizeColor(e,i,o){if(!this.initialized||!this.colorPipeline||!this.uniformBuffer)throw new Error("VolumeResizer not initialized");const n=new ArrayBuffer(lr),u=new DataView(n);u.setUint32(0,o.srcResolution,!0),u.setUint32(4,o.dstResolution,!0),u.setInt32(8,o.offset.x,!0),u.setInt32(12,o.offset.y,!0),u.setInt32(16,o.offset.z,!0),u.setFloat32(20,0,!0),u.setUint32(24,0,!0),u.setUint32(28,0,!0),this.context.device.queue.writeBuffer(this.uniformBuffer,0,n);const c=this.context.device.createBindGroup({label:"VolumeResizer Color BindGroup",layout:this.colorPipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:e}},{binding:1,resource:{buffer:i}},{binding:2,resource:{buffer:this.uniformBuffer}}]}),d=this.context.device.createCommandEncoder(),f=d.beginComputePass();f.setPipeline(this.colorPipeline),f.setBindGroup(0,c);const g=8,x=Math.ceil(o.dstResolution/g),v=Math.ceil(o.dstResolution/g),_=Math.ceil(o.dstResolution/g);f.dispatchWorkgroups(x,v,_),f.end(),this.context.device.queue.submit([d.finish()]),await this.context.device.queue.onSubmittedWorkDone(),Zt.info(`Resized Color ${o.srcResolution}³ → ${o.dstResolution}³`)}dispose(){this.uniformBuffer&&(this.uniformBuffer.destroy(),this.uniformBuffer=null),this.sdfPipeline=null,this.colorPipeline=null,this.initialized=!1,Zt.info("Disposed")}}function Qo(w,e){const i=e-w,o=Math.floor(i/2);return{x:o,y:o,z:o}}function en(w,e,i){const o=e-w;return{x:i.x<0?o:i.x>0?0:Math.floor(o/2),y:i.y<0?o:i.y>0?0:Math.floor(o/2),z:i.z<0?o:i.z>0?0:Math.floor(o/2)}}const cr=we("BrushPipeline"),ri=0,tn=1,rn=2,sn=3,on=4,nn=5,an=6,un=7,ln=8;class cn{context;pipeline=null;uniformBuffer;bindGroupLayout;pipelineLayout;bindGroupCache=new Map;dummyColorBuffer=null;uniformData;uniformView;pendingCommands=[];lastSubmitTime=0;BATCH_INTERVAL_MS=16;symmetry={...Uo};constructor(e){this.context=e,this.uniformBuffer=e.device.createBuffer({size:Jr,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST}),this.uniformData=new ArrayBuffer(Jr),this.uniformView=new DataView(this.uniformData),this.bindGroupLayout=e.device.createBindGroupLayout({entries:[{binding:0,visibility:GPUShaderStage.COMPUTE,buffer:{type:"storage"}},{binding:1,visibility:GPUShaderStage.COMPUTE,buffer:{type:"uniform"}},{binding:2,visibility:GPUShaderStage.COMPUTE,buffer:{type:"storage"}}]}),this.pipelineLayout=e.device.createPipelineLayout({bindGroupLayouts:[this.bindGroupLayout]})}async initialize(){if(this.pipeline)return;const e=this.context.device.createShaderModule({code:$o});this.pipeline=await this.context.device.createComputePipelineAsync({layout:this.pipelineLayout,compute:{module:e,entryPoint:"main"}}),cr("Initialized")}getDummyColorBuffer(e){if(!this.dummyColorBuffer){const i=e**3*4;this.dummyColorBuffer=this.context.device.createBuffer({size:i,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST})}return this.dummyColorBuffer}apply(e,i,o,n){if(!this.pipeline)throw new Error("BrushPipeline not initialized");const u={x:o.x*e.resolution,y:o.y*e.resolution,z:o.z*e.resolution},c=i.size*e.resolution,d=this.uniformView;d.setFloat32(0,u.x,!0),d.setFloat32(4,u.y,!0),d.setFloat32(8,u.z,!0),d.setFloat32(12,0,!0),d.setFloat32(16,c,!0),d.setFloat32(20,i.strength,!0),d.setUint32(24,this.toolToInt(i.tool),!0),d.setFloat32(28,i.hardness??.5,!0),d.setUint32(32,e.resolution,!0),d.setUint32(36,this.symmetryAxisToInt(this.symmetry.axis),!0),d.setUint32(40,this.symmetry.radialSegments,!0),d.setUint32(44,0,!0),d.setFloat32(48,i.color.x,!0),d.setFloat32(52,i.color.y,!0),d.setFloat32(56,i.color.z,!0),d.setFloat32(60,i.color.w,!0),d.setFloat32(64,0,!0),d.setFloat32(68,0,!0),d.setFloat32(72,0,!0),d.setFloat32(76,0,!0),this.context.device.queue.writeBuffer(this.uniformBuffer,0,this.uniformData);const f=n?.buffer??this.getDummyColorBuffer(e.resolution);let g=this.bindGroupCache.get(e.buffer),x=g?.get(f);x||(x=this.context.device.createBindGroup({layout:this.bindGroupLayout,entries:[{binding:0,resource:{buffer:e.buffer}},{binding:1,resource:{buffer:this.uniformBuffer}},{binding:2,resource:{buffer:f}}]}),g||(g=new Map,this.bindGroupCache.set(e.buffer,g)),g.set(f,x));const v=this.context.device.createCommandEncoder(),_=v.beginComputePass();_.setPipeline(this.pipeline),_.setBindGroup(0,x);const E=Math.ceil(e.resolution/st);_.dispatchWorkgroups(E,E,E),_.end();const b=performance.now();this.pendingCommands.push(v.finish()),(b-this.lastSubmitTime>=this.BATCH_INTERVAL_MS||this.pendingCommands.length>=3)&&this.flush()}flush(){this.pendingCommands.length>0&&(this.context.device.queue.submit(this.pendingCommands),this.pendingCommands=[],this.lastSubmitTime=performance.now())}async applyStroke(e,i,o,n){for(const u of o)this.apply(e,i,u,n);this.flush(),await this.context.device.queue.onSubmittedWorkDone()}clearBindGroupCache(){this.bindGroupCache.clear(),cr("Bind group cache cleared")}dispose(){this.flush(),this.uniformBuffer.destroy(),this.dummyColorBuffer&&(this.dummyColorBuffer.destroy(),this.dummyColorBuffer=null),this.bindGroupCache.clear(),this.pipeline=null,cr("Disposed")}toolToInt(e){switch(e){case"brush":return ri;case"pull":return tn;case"smooth":return rn;case"inflate":return sn;case"deflate":return on;case"flatten":return nn;case"pinch":return an;case"crease":return un;case"carve":return ln;default:return ri}}symmetryAxisToInt(e){switch(e){case"off":return Qr;case"x":return Ao;case"y":return Io;case"z":return Ro;case"radial":return Do;default:return Qr}}setSymmetry(e){this.symmetry={...this.symmetry,...e},this.symmetry.radialSegments=Math.max(2,Math.min(16,this.symmetry.radialSegments))}getSymmetry(){return{...this.symmetry}}cycleSymmetryAxis(){const e=["off","x","y","z"],o=(e.indexOf(this.symmetry.axis)+1)%e.length;return this.symmetry.axis=e[o],this.symmetry.axis}cycleSymmetryMode(){const e=["off","x","y","z","radial"],o=(e.indexOf(this.symmetry.axis)+1)%e.length;return this.symmetry.axis=e[o],this.symmetry.axis}}const ii=we("ColorSmoothPipeline"),si=16;class dn{context;pipeline=null;uniformBuffer;bindGroupLayout;pipelineLayout;tempBuffer=null;tempBufferSize=0;uniformData;uniformView;constructor(e){this.context=e,this.uniformBuffer=e.device.createBuffer({size:si,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST}),this.uniformData=new ArrayBuffer(si),this.uniformView=new DataView(this.uniformData),this.bindGroupLayout=e.device.createBindGroupLayout({entries:[{binding:0,visibility:GPUShaderStage.COMPUTE,buffer:{type:"read-only-storage"}},{binding:1,visibility:GPUShaderStage.COMPUTE,buffer:{type:"storage"}},{binding:2,visibility:GPUShaderStage.COMPUTE,buffer:{type:"uniform"}}]}),this.pipelineLayout=e.device.createPipelineLayout({bindGroupLayouts:[this.bindGroupLayout]})}async initialize(){if(this.pipeline)return;const e=this.context.device.createShaderModule({code:No});this.pipeline=await this.context.device.createComputePipelineAsync({layout:this.pipelineLayout,compute:{module:e,entryPoint:"main"}}),ii.info("Initialized")}ensureTempBuffer(e){const i=e**3*4;return(!this.tempBuffer||this.tempBufferSize<i)&&(this.tempBuffer&&this.tempBuffer.destroy(),this.tempBuffer=this.context.device.createBuffer({size:i,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC|GPUBufferUsage.COPY_DST}),this.tempBufferSize=i),this.tempBuffer}async smooth(e,i={}){if(!this.pipeline)throw new Error("ColorSmoothPipeline not initialized");const{kernelSize:o=1,strength:n=.5,iterations:u=1}=i,c=e.resolution,d=this.ensureTempBuffer(c),f=this.uniformView;f.setUint32(0,c,!0),f.setUint32(4,Math.max(1,Math.min(2,o)),!0),f.setFloat32(8,Math.max(0,Math.min(1,n)),!0),f.setUint32(12,0,!0),this.context.device.queue.writeBuffer(this.uniformBuffer,0,this.uniformData);const g=Math.ceil(c/st);for(let x=0;x<u;x++){const v=x%2===0,_=v?e.buffer:d,E=v?d:e.buffer,b=this.context.device.createBindGroup({layout:this.bindGroupLayout,entries:[{binding:0,resource:{buffer:_}},{binding:1,resource:{buffer:E}},{binding:2,resource:{buffer:this.uniformBuffer}}]}),P=this.context.device.createCommandEncoder(),B=P.beginComputePass();B.setPipeline(this.pipeline),B.setBindGroup(0,b),B.dispatchWorkgroups(g,g,g),B.end(),this.context.device.queue.submit([P.finish()])}if(u%2===1){const x=this.context.device.createCommandEncoder(),v=c**3*4;x.copyBufferToBuffer(d,0,e.buffer,0,v),this.context.device.queue.submit([x.finish()])}await this.context.device.queue.onSubmittedWorkDone()}dispose(){this.uniformBuffer.destroy(),this.tempBuffer&&(this.tempBuffer.destroy(),this.tempBuffer=null),this.pipeline=null,ii.info("Disposed")}}const oi=we("ColorSmoothLocalPipeline"),ni=32;class fn{context;pipeline=null;uniformBuffer;bindGroupLayout;pipelineLayout;tempBuffer=null;tempBufferSize=0;uniformData;uniformView;constructor(e){this.context=e,this.uniformBuffer=e.device.createBuffer({size:ni,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST}),this.uniformData=new ArrayBuffer(ni),this.uniformView=new DataView(this.uniformData),this.bindGroupLayout=e.device.createBindGroupLayout({entries:[{binding:0,visibility:GPUShaderStage.COMPUTE,buffer:{type:"read-only-storage"}},{binding:1,visibility:GPUShaderStage.COMPUTE,buffer:{type:"storage"}},{binding:2,visibility:GPUShaderStage.COMPUTE,buffer:{type:"uniform"}}]}),this.pipelineLayout=e.device.createPipelineLayout({bindGroupLayouts:[this.bindGroupLayout]})}async initialize(){if(this.pipeline)return;const e=this.context.device.createShaderModule({code:Zo});this.pipeline=await this.context.device.createComputePipelineAsync({layout:this.pipelineLayout,compute:{module:e,entryPoint:"main"}}),oi.info("Initialized")}ensureTempBuffer(e){const i=e**3*4;return(!this.tempBuffer||this.tempBufferSize<i)&&(this.tempBuffer&&this.tempBuffer.destroy(),this.tempBuffer=this.context.device.createBuffer({size:i,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC|GPUBufferUsage.COPY_DST}),this.tempBufferSize=i),this.tempBuffer}async apply(e,i){if(!this.pipeline)throw new Error("ColorSmoothLocalPipeline not initialized");const{brushPos:o,brushRadius:n,hardness:u,strength:c,kernelSize:d=1}=i,f=e.resolution,g=this.ensureTempBuffer(f),x=o.x*f,v=o.y*f,_=o.z*f,E=n*f,b=this.uniformView;b.setUint32(0,f,!0),b.setUint32(4,Math.max(1,Math.min(2,d)),!0),b.setFloat32(8,Math.max(0,Math.min(1,c)),!0),b.setFloat32(12,Math.max(0,Math.min(1,u)),!0),b.setFloat32(16,x,!0),b.setFloat32(20,v,!0),b.setFloat32(24,_,!0),b.setFloat32(28,E,!0),this.context.device.queue.writeBuffer(this.uniformBuffer,0,this.uniformData);const P=Math.ceil(f/st),B=this.context.device.createBindGroup({layout:this.bindGroupLayout,entries:[{binding:0,resource:{buffer:e.buffer}},{binding:1,resource:{buffer:g}},{binding:2,resource:{buffer:this.uniformBuffer}}]}),z=this.context.device.createCommandEncoder(),A=z.beginComputePass();A.setPipeline(this.pipeline),A.setBindGroup(0,B),A.dispatchWorkgroups(P,P,P),A.end();const k=f**3*4;z.copyBufferToBuffer(g,0,e.buffer,0,k),this.context.device.queue.submit([z.finish()])}dispose(){this.uniformBuffer.destroy(),this.tempBuffer&&(this.tempBuffer.destroy(),this.tempBuffer=null),this.pipeline=null,oi.info("Disposed")}}const ai=we("SDFFilterPipeline"),hn={gaussian:0,mean:1,median:2,bilateral:3},ui=16;class pn{context;pipeline=null;uniformBuffer;bindGroupLayout;pipelineLayout;tempBuffer=null;tempBufferSize=0;uniformData;uniformView;constructor(e){this.context=e,this.uniformBuffer=e.device.createBuffer({size:ui,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST}),this.uniformData=new ArrayBuffer(ui),this.uniformView=new DataView(this.uniformData),this.bindGroupLayout=e.device.createBindGroupLayout({entries:[{binding:0,visibility:GPUShaderStage.COMPUTE,buffer:{type:"read-only-storage"}},{binding:1,visibility:GPUShaderStage.COMPUTE,buffer:{type:"storage"}},{binding:2,visibility:GPUShaderStage.COMPUTE,buffer:{type:"uniform"}}]}),this.pipelineLayout=e.device.createPipelineLayout({bindGroupLayouts:[this.bindGroupLayout]})}async initialize(){if(this.pipeline)return;const e=this.context.device.createShaderModule({code:Wo});this.pipeline=await this.context.device.createComputePipelineAsync({layout:this.pipelineLayout,compute:{module:e,entryPoint:"main"}}),ai.info("Initialized")}ensureTempBuffer(e){const i=e**3*4;return(!this.tempBuffer||this.tempBufferSize<i)&&(this.tempBuffer&&this.tempBuffer.destroy(),this.tempBuffer=this.context.device.createBuffer({size:i,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC|GPUBufferUsage.COPY_DST}),this.tempBufferSize=i),this.tempBuffer}async filter(e,i){if(!this.pipeline)throw new Error("SDFFilterPipeline not initialized");const o=hn[i.mode],n=mn(i.strength??.5),u=Math.max(1,Math.min(20,i.iterations??1)),c=Math.max(.001,i.valueSigma??.05),d=e.resolution,f=this.ensureTempBuffer(d),g=this.uniformView;g.setUint32(0,d,!0),g.setUint32(4,o,!0),g.setFloat32(8,n,!0),g.setFloat32(12,c,!0),this.context.device.queue.writeBuffer(this.uniformBuffer,0,this.uniformData);const x=Math.ceil(d/st);for(let v=0;v<u;v++){const _=v%2===0,E=_?e.buffer:f,b=_?f:e.buffer,P=this.context.device.createBindGroup({layout:this.bindGroupLayout,entries:[{binding:0,resource:{buffer:E}},{binding:1,resource:{buffer:b}},{binding:2,resource:{buffer:this.uniformBuffer}}]}),B=this.context.device.createCommandEncoder(),z=B.beginComputePass();z.setPipeline(this.pipeline),z.setBindGroup(0,P),z.dispatchWorkgroups(x,x,x),z.end(),this.context.device.queue.submit([B.finish()])}if(u%2===1){const v=this.context.device.createCommandEncoder(),_=d**3*4;v.copyBufferToBuffer(f,0,e.buffer,0,_),this.context.device.queue.submit([v.finish()])}await this.context.device.queue.onSubmittedWorkDone()}dispose(){this.uniformBuffer.destroy(),this.tempBuffer&&(this.tempBuffer.destroy(),this.tempBuffer=null),this.pipeline=null,ai.info("Disposed")}}function mn(w){return Math.max(0,Math.min(1,w))}const li=we("PaintExpandPipeline"),ci=16;class gn{context;pipeline=null;uniformBuffer;bindGroupLayout;pipelineLayout;tempBuffer=null;tempBufferSize=0;uniformData;uniformView;constructor(e){this.context=e,this.uniformBuffer=e.device.createBuffer({size:ci,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST}),this.uniformData=new ArrayBuffer(ci),this.uniformView=new DataView(this.uniformData),this.bindGroupLayout=e.device.createBindGroupLayout({entries:[{binding:0,visibility:GPUShaderStage.COMPUTE,buffer:{type:"read-only-storage"}},{binding:1,visibility:GPUShaderStage.COMPUTE,buffer:{type:"storage"}},{binding:2,visibility:GPUShaderStage.COMPUTE,buffer:{type:"uniform"}}]}),this.pipelineLayout=e.device.createPipelineLayout({bindGroupLayouts:[this.bindGroupLayout]})}async initialize(){if(this.pipeline)return;const e=this.context.device.createShaderModule({code:Yo});this.pipeline=await this.context.device.createComputePipelineAsync({layout:this.pipelineLayout,compute:{module:e,entryPoint:"main"}}),li.info("Initialized")}ensureTempBuffer(e){const i=e**3*4;return(!this.tempBuffer||this.tempBufferSize<i)&&(this.tempBuffer&&this.tempBuffer.destroy(),this.tempBuffer=this.context.device.createBuffer({size:i,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC|GPUBufferUsage.COPY_DST}),this.tempBufferSize=i),this.tempBuffer}async dilate(e,i={}){if(!this.pipeline)throw new Error("PaintExpandPipeline not initialized");const o=Math.max(1,Math.min(20,i.iterations??1)),n=Math.max(.001,i.threshold??.05),u=Math.max(.01,Math.min(1,i.haloAlpha??.5)),c=e.resolution,d=this.ensureTempBuffer(c),f=this.uniformView;f.setUint32(0,c,!0),f.setFloat32(4,n,!0),f.setFloat32(8,u,!0),f.setUint32(12,0,!0),this.context.device.queue.writeBuffer(this.uniformBuffer,0,this.uniformData);const g=Math.ceil(c/st);for(let x=0;x<o;x++){const v=x%2===0,_=v?e.buffer:d,E=v?d:e.buffer,b=this.context.device.createBindGroup({layout:this.bindGroupLayout,entries:[{binding:0,resource:{buffer:_}},{binding:1,resource:{buffer:E}},{binding:2,resource:{buffer:this.uniformBuffer}}]}),P=this.context.device.createCommandEncoder(),B=P.beginComputePass();B.setPipeline(this.pipeline),B.setBindGroup(0,b),B.dispatchWorkgroups(g,g,g),B.end(),this.context.device.queue.submit([P.finish()])}if(o%2===1){const x=this.context.device.createCommandEncoder(),v=c**3*4;x.copyBufferToBuffer(d,0,e.buffer,0,v),this.context.device.queue.submit([x.finish()])}await this.context.device.queue.onSubmittedWorkDone()}dispose(){this.uniformBuffer.destroy(),this.tempBuffer&&(this.tempBuffer.destroy(),this.tempBuffer=null),this.pipeline=null,li.info("Disposed")}}const pr=new Uint32Array([0,265,515,778,1030,1295,1541,1804,2060,2309,2575,2822,3082,3331,3593,3840,400,153,915,666,1430,1183,1941,1692,2460,2197,2975,2710,3482,3219,3993,3728,560,825,51,314,1590,1855,1077,1340,2620,2869,2111,2358,3642,3891,3129,3376,928,681,419,170,1958,1711,1445,1196,2988,2725,2479,2214,4010,3747,3497,3232,1120,1385,1635,1898,102,367,613,876,3180,3429,3695,3942,2154,2403,2665,2912,1520,1273,2035,1786,502,255,1013,764,3580,3317,4095,3830,2554,2291,3065,2800,1616,1881,1107,1370,598,863,85,348,3676,3925,3167,3414,2650,2899,2137,2384,1984,1737,1475,1226,966,719,453,204,4044,3781,3535,3270,3018,2755,2505,2240,2240,2505,2755,3018,3270,3535,3781,4044,204,453,719,966,1226,1475,1737,1984,2384,2137,2899,2650,3414,3167,3925,3676,348,85,863,598,1370,1107,1881,1616,2800,3065,2291,2554,3830,4095,3317,3580,764,1013,255,502,1786,2035,1273,1520,2912,2665,2403,2154,3942,3695,3429,3180,876,613,367,102,1898,1635,1385,1120,3232,3497,3747,4010,2214,2479,2725,2988,1196,1445,1711,1958,170,419,681,928,3376,3129,3891,3642,2358,2111,2869,2620,1340,1077,1855,1590,314,51,825,560,3728,3993,3219,3482,2710,2975,2197,2460,1692,1941,1183,1430,666,915,153,400,3840,3593,3331,3082,2822,2575,2309,2060,1804,1541,1295,1030,778,515,265,0]),xt=new Int32Array([-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,0,8,3,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,0,1,9,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,1,8,3,9,8,1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,1,2,10,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,0,8,3,1,2,10,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,9,2,10,0,2,9,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,2,8,3,2,10,8,10,9,8,-1,-1,-1,-1,-1,-1,-1,3,11,2,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,0,11,2,8,11,0,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,1,9,0,2,3,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,1,11,2,1,9,11,9,8,11,-1,-1,-1,-1,-1,-1,-1,3,10,1,11,10,3,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,0,10,1,0,8,10,8,11,10,-1,-1,-1,-1,-1,-1,-1,3,9,0,3,11,9,11,10,9,-1,-1,-1,-1,-1,-1,-1,9,8,10,10,8,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,4,7,8,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,4,3,0,7,3,4,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,0,1,9,8,4,7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,4,1,9,4,7,1,7,3,1,-1,-1,-1,-1,-1,-1,-1,1,2,10,8,4,7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,3,4,7,3,0,4,1,2,10,-1,-1,-1,-1,-1,-1,-1,9,2,10,9,0,2,8,4,7,-1,-1,-1,-1,-1,-1,-1,2,10,9,2,9,7,2,7,3,7,9,4,-1,-1,-1,-1,8,4,7,3,11,2,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,11,4,7,11,2,4,2,0,4,-1,-1,-1,-1,-1,-1,-1,9,0,1,8,4,7,2,3,11,-1,-1,-1,-1,-1,-1,-1,4,7,11,9,4,11,9,11,2,9,2,1,-1,-1,-1,-1,3,10,1,3,11,10,7,8,4,-1,-1,-1,-1,-1,-1,-1,1,11,10,1,4,11,1,0,4,7,11,4,-1,-1,-1,-1,4,7,8,9,0,11,9,11,10,11,0,3,-1,-1,-1,-1,4,7,11,4,11,9,9,11,10,-1,-1,-1,-1,-1,-1,-1,9,5,4,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,9,5,4,0,8,3,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,0,5,4,1,5,0,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,8,5,4,8,3,5,3,1,5,-1,-1,-1,-1,-1,-1,-1,1,2,10,9,5,4,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,3,0,8,1,2,10,4,9,5,-1,-1,-1,-1,-1,-1,-1,5,2,10,5,4,2,4,0,2,-1,-1,-1,-1,-1,-1,-1,2,10,5,3,2,5,3,5,4,3,4,8,-1,-1,-1,-1,9,5,4,2,3,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,0,11,2,0,8,11,4,9,5,-1,-1,-1,-1,-1,-1,-1,0,5,4,0,1,5,2,3,11,-1,-1,-1,-1,-1,-1,-1,2,1,5,2,5,8,2,8,11,4,8,5,-1,-1,-1,-1,10,3,11,10,1,3,9,5,4,-1,-1,-1,-1,-1,-1,-1,4,9,5,0,8,1,8,10,1,8,11,10,-1,-1,-1,-1,5,4,0,5,0,11,5,11,10,11,0,3,-1,-1,-1,-1,5,4,8,5,8,10,10,8,11,-1,-1,-1,-1,-1,-1,-1,9,7,8,5,7,9,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,9,3,0,9,5,3,5,7,3,-1,-1,-1,-1,-1,-1,-1,0,7,8,0,1,7,1,5,7,-1,-1,-1,-1,-1,-1,-1,1,5,3,3,5,7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,9,7,8,9,5,7,10,1,2,-1,-1,-1,-1,-1,-1,-1,10,1,2,9,5,0,5,3,0,5,7,3,-1,-1,-1,-1,8,0,2,8,2,5,8,5,7,10,5,2,-1,-1,-1,-1,2,10,5,2,5,3,3,5,7,-1,-1,-1,-1,-1,-1,-1,7,9,5,7,8,9,3,11,2,-1,-1,-1,-1,-1,-1,-1,9,5,7,9,7,2,9,2,0,2,7,11,-1,-1,-1,-1,2,3,11,0,1,8,1,7,8,1,5,7,-1,-1,-1,-1,11,2,1,11,1,7,7,1,5,-1,-1,-1,-1,-1,-1,-1,9,5,8,8,5,7,10,1,3,10,3,11,-1,-1,-1,-1,5,7,0,5,0,9,7,11,0,1,0,10,11,10,0,-1,11,10,0,11,0,3,10,5,0,8,0,7,5,7,0,-1,11,10,5,7,11,5,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,10,6,5,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,0,8,3,5,10,6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,9,0,1,5,10,6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,1,8,3,1,9,8,5,10,6,-1,-1,-1,-1,-1,-1,-1,1,6,5,2,6,1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,1,6,5,1,2,6,3,0,8,-1,-1,-1,-1,-1,-1,-1,9,6,5,9,0,6,0,2,6,-1,-1,-1,-1,-1,-1,-1,5,9,8,5,8,2,5,2,6,3,2,8,-1,-1,-1,-1,2,3,11,10,6,5,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,11,0,8,11,2,0,10,6,5,-1,-1,-1,-1,-1,-1,-1,0,1,9,2,3,11,5,10,6,-1,-1,-1,-1,-1,-1,-1,5,10,6,1,9,2,9,11,2,9,8,11,-1,-1,-1,-1,6,3,11,6,5,3,5,1,3,-1,-1,-1,-1,-1,-1,-1,0,8,11,0,11,5,0,5,1,5,11,6,-1,-1,-1,-1,3,11,6,0,3,6,0,6,5,0,5,9,-1,-1,-1,-1,6,5,9,6,9,11,11,9,8,-1,-1,-1,-1,-1,-1,-1,5,10,6,4,7,8,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,4,3,0,4,7,3,6,5,10,-1,-1,-1,-1,-1,-1,-1,1,9,0,5,10,6,8,4,7,-1,-1,-1,-1,-1,-1,-1,10,6,5,1,9,7,1,7,3,7,9,4,-1,-1,-1,-1,6,1,2,6,5,1,4,7,8,-1,-1,-1,-1,-1,-1,-1,1,2,5,5,2,6,3,0,4,3,4,7,-1,-1,-1,-1,8,4,7,9,0,5,0,6,5,0,2,6,-1,-1,-1,-1,7,3,9,7,9,4,3,2,9,5,9,6,2,6,9,-1,3,11,2,7,8,4,10,6,5,-1,-1,-1,-1,-1,-1,-1,5,10,6,4,7,2,4,2,0,2,7,11,-1,-1,-1,-1,0,1,9,4,7,8,2,3,11,5,10,6,-1,-1,-1,-1,9,2,1,9,11,2,9,4,11,7,11,4,5,10,6,-1,8,4,7,3,11,5,3,5,1,5,11,6,-1,-1,-1,-1,5,1,11,5,11,6,1,0,11,7,11,4,0,4,11,-1,0,5,9,0,6,5,0,3,6,11,6,3,8,4,7,-1,6,5,9,6,9,11,4,7,9,7,11,9,-1,-1,-1,-1,10,4,9,6,4,10,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,4,10,6,4,9,10,0,8,3,-1,-1,-1,-1,-1,-1,-1,10,0,1,10,6,0,6,4,0,-1,-1,-1,-1,-1,-1,-1,8,3,1,8,1,6,8,6,4,6,1,10,-1,-1,-1,-1,1,4,9,1,2,4,2,6,4,-1,-1,-1,-1,-1,-1,-1,3,0,8,1,2,9,2,4,9,2,6,4,-1,-1,-1,-1,0,2,4,4,2,6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,8,3,2,8,2,4,4,2,6,-1,-1,-1,-1,-1,-1,-1,10,4,9,10,6,4,11,2,3,-1,-1,-1,-1,-1,-1,-1,0,8,2,2,8,11,4,9,10,4,10,6,-1,-1,-1,-1,3,11,2,0,1,6,0,6,4,6,1,10,-1,-1,-1,-1,6,4,1,6,1,10,4,8,1,2,1,11,8,11,1,-1,9,6,4,9,3,6,9,1,3,11,6,3,-1,-1,-1,-1,8,11,1,8,1,0,11,6,1,9,1,4,6,4,1,-1,3,11,6,3,6,0,0,6,4,-1,-1,-1,-1,-1,-1,-1,6,4,8,11,6,8,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,7,10,6,7,8,10,8,9,10,-1,-1,-1,-1,-1,-1,-1,0,7,3,0,10,7,0,9,10,6,7,10,-1,-1,-1,-1,10,6,7,1,10,7,1,7,8,1,8,0,-1,-1,-1,-1,10,6,7,10,7,1,1,7,3,-1,-1,-1,-1,-1,-1,-1,1,2,6,1,6,8,1,8,9,8,6,7,-1,-1,-1,-1,2,6,9,2,9,1,6,7,9,0,9,3,7,3,9,-1,7,8,0,7,0,6,6,0,2,-1,-1,-1,-1,-1,-1,-1,7,3,2,6,7,2,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,2,3,11,10,6,8,10,8,9,8,6,7,-1,-1,-1,-1,2,0,7,2,7,11,0,9,7,6,7,10,9,10,7,-1,1,8,0,1,7,8,1,10,7,6,7,10,2,3,11,-1,11,2,1,11,1,7,10,6,1,6,7,1,-1,-1,-1,-1,8,9,6,8,6,7,9,1,6,11,6,3,1,3,6,-1,0,9,1,11,6,7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,7,8,0,7,0,6,3,11,0,11,6,0,-1,-1,-1,-1,7,11,6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,7,6,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,3,0,8,11,7,6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,0,1,9,11,7,6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,8,1,9,8,3,1,11,7,6,-1,-1,-1,-1,-1,-1,-1,10,1,2,6,11,7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,1,2,10,3,0,8,6,11,7,-1,-1,-1,-1,-1,-1,-1,2,9,0,2,10,9,6,11,7,-1,-1,-1,-1,-1,-1,-1,6,11,7,2,10,3,10,8,3,10,9,8,-1,-1,-1,-1,7,2,3,6,2,7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,7,0,8,7,6,0,6,2,0,-1,-1,-1,-1,-1,-1,-1,2,7,6,2,3,7,0,1,9,-1,-1,-1,-1,-1,-1,-1,1,6,2,1,8,6,1,9,8,8,7,6,-1,-1,-1,-1,10,7,6,10,1,7,1,3,7,-1,-1,-1,-1,-1,-1,-1,10,7,6,1,7,10,1,8,7,1,0,8,-1,-1,-1,-1,0,3,7,0,7,10,0,10,9,6,10,7,-1,-1,-1,-1,7,6,10,7,10,8,8,10,9,-1,-1,-1,-1,-1,-1,-1,6,8,4,11,8,6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,3,6,11,3,0,6,0,4,6,-1,-1,-1,-1,-1,-1,-1,8,6,11,8,4,6,9,0,1,-1,-1,-1,-1,-1,-1,-1,9,4,6,9,6,3,9,3,1,11,3,6,-1,-1,-1,-1,6,8,4,6,11,8,2,10,1,-1,-1,-1,-1,-1,-1,-1,1,2,10,3,0,11,0,6,11,0,4,6,-1,-1,-1,-1,4,11,8,4,6,11,0,2,9,2,10,9,-1,-1,-1,-1,10,9,3,10,3,2,9,4,3,11,3,6,4,6,3,-1,8,2,3,8,4,2,4,6,2,-1,-1,-1,-1,-1,-1,-1,0,4,2,4,6,2,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,1,9,0,2,3,4,2,4,6,4,3,8,-1,-1,-1,-1,1,9,4,1,4,2,2,4,6,-1,-1,-1,-1,-1,-1,-1,8,1,3,8,6,1,8,4,6,6,10,1,-1,-1,-1,-1,10,1,0,10,0,6,6,0,4,-1,-1,-1,-1,-1,-1,-1,4,6,3,4,3,8,6,10,3,0,3,9,10,9,3,-1,10,9,4,6,10,4,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,4,9,5,7,6,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,0,8,3,4,9,5,11,7,6,-1,-1,-1,-1,-1,-1,-1,5,0,1,5,4,0,7,6,11,-1,-1,-1,-1,-1,-1,-1,11,7,6,8,3,4,3,5,4,3,1,5,-1,-1,-1,-1,9,5,4,10,1,2,7,6,11,-1,-1,-1,-1,-1,-1,-1,6,11,7,1,2,10,0,8,3,4,9,5,-1,-1,-1,-1,7,6,11,5,4,10,4,2,10,4,0,2,-1,-1,-1,-1,3,4,8,3,5,4,3,2,5,10,5,2,11,7,6,-1,7,2,3,7,6,2,5,4,9,-1,-1,-1,-1,-1,-1,-1,9,5,4,0,8,6,0,6,2,6,8,7,-1,-1,-1,-1,3,6,2,3,7,6,1,5,0,5,4,0,-1,-1,-1,-1,6,2,8,6,8,7,2,1,8,4,8,5,1,5,8,-1,9,5,4,10,1,6,1,7,6,1,3,7,-1,-1,-1,-1,1,6,10,1,7,6,1,0,7,8,7,0,9,5,4,-1,4,0,10,4,10,5,0,3,10,6,10,7,3,7,10,-1,7,6,10,7,10,8,5,4,10,4,8,10,-1,-1,-1,-1,6,9,5,6,11,9,11,8,9,-1,-1,-1,-1,-1,-1,-1,3,6,11,0,6,3,0,5,6,0,9,5,-1,-1,-1,-1,0,11,8,0,5,11,0,1,5,5,6,11,-1,-1,-1,-1,6,11,3,6,3,5,5,3,1,-1,-1,-1,-1,-1,-1,-1,1,2,10,9,5,11,9,11,8,11,5,6,-1,-1,-1,-1,0,11,3,0,6,11,0,9,6,5,6,9,1,2,10,-1,11,8,5,11,5,6,8,0,5,10,5,2,0,2,5,-1,6,11,3,6,3,5,2,10,3,10,5,3,-1,-1,-1,-1,5,8,9,5,2,8,5,6,2,3,8,2,-1,-1,-1,-1,9,5,6,9,6,0,0,6,2,-1,-1,-1,-1,-1,-1,-1,1,5,8,1,8,0,5,6,8,3,8,2,6,2,8,-1,1,5,6,2,1,6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,1,3,6,1,6,10,3,8,6,5,6,9,8,9,6,-1,10,1,0,10,0,6,9,5,0,5,6,0,-1,-1,-1,-1,0,3,8,5,6,10,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,10,5,6,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,11,5,10,7,5,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,11,5,10,11,7,5,8,3,0,-1,-1,-1,-1,-1,-1,-1,5,11,7,5,10,11,1,9,0,-1,-1,-1,-1,-1,-1,-1,10,7,5,10,11,7,9,8,1,8,3,1,-1,-1,-1,-1,11,1,2,11,7,1,7,5,1,-1,-1,-1,-1,-1,-1,-1,0,8,3,1,2,7,1,7,5,7,2,11,-1,-1,-1,-1,9,7,5,9,2,7,9,0,2,2,11,7,-1,-1,-1,-1,7,5,2,7,2,11,5,9,2,3,2,8,9,8,2,-1,2,5,10,2,3,5,3,7,5,-1,-1,-1,-1,-1,-1,-1,8,2,0,8,5,2,8,7,5,10,2,5,-1,-1,-1,-1,9,0,1,5,10,3,5,3,7,3,10,2,-1,-1,-1,-1,9,8,2,9,2,1,8,7,2,10,2,5,7,5,2,-1,1,3,5,3,7,5,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,0,8,7,0,7,1,1,7,5,-1,-1,-1,-1,-1,-1,-1,9,0,3,9,3,5,5,3,7,-1,-1,-1,-1,-1,-1,-1,9,8,7,5,9,7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,5,8,4,5,10,8,10,11,8,-1,-1,-1,-1,-1,-1,-1,5,0,4,5,11,0,5,10,11,11,3,0,-1,-1,-1,-1,0,1,9,8,4,10,8,10,11,10,4,5,-1,-1,-1,-1,10,11,4,10,4,5,11,3,4,9,4,1,3,1,4,-1,2,5,1,2,8,5,2,11,8,4,5,8,-1,-1,-1,-1,0,4,11,0,11,3,4,5,11,2,11,1,5,1,11,-1,0,2,5,0,5,9,2,11,5,4,5,8,11,8,5,-1,9,4,5,2,11,3,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,2,5,10,3,5,2,3,4,5,3,8,4,-1,-1,-1,-1,5,10,2,5,2,4,4,2,0,-1,-1,-1,-1,-1,-1,-1,3,10,2,3,5,10,3,8,5,4,5,8,0,1,9,-1,5,10,2,5,2,4,1,9,2,9,4,2,-1,-1,-1,-1,8,4,5,8,5,3,3,5,1,-1,-1,-1,-1,-1,-1,-1,0,4,5,1,0,5,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,8,4,5,8,5,3,9,0,5,0,3,5,-1,-1,-1,-1,9,4,5,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,4,11,7,4,9,11,9,10,11,-1,-1,-1,-1,-1,-1,-1,0,8,3,4,9,7,9,11,7,9,10,11,-1,-1,-1,-1,1,10,11,1,11,4,1,4,0,7,4,11,-1,-1,-1,-1,3,1,4,3,4,8,1,10,4,7,4,11,10,11,4,-1,4,11,7,9,11,4,9,2,11,9,1,2,-1,-1,-1,-1,9,7,4,9,11,7,9,1,11,2,11,1,0,8,3,-1,11,7,4,11,4,2,2,4,0,-1,-1,-1,-1,-1,-1,-1,11,7,4,11,4,2,8,3,4,3,2,4,-1,-1,-1,-1,2,9,10,2,7,9,2,3,7,7,4,9,-1,-1,-1,-1,9,10,7,9,7,4,10,2,7,8,7,0,2,0,7,-1,3,7,10,3,10,2,7,4,10,1,10,0,4,0,10,-1,1,10,2,8,7,4,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,4,9,1,4,1,7,7,1,3,-1,-1,-1,-1,-1,-1,-1,4,9,1,4,1,7,0,8,1,8,7,1,-1,-1,-1,-1,4,0,3,7,4,3,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,4,8,7,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,9,10,8,10,11,8,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,3,0,9,3,9,11,11,9,10,-1,-1,-1,-1,-1,-1,-1,0,1,10,0,10,8,8,10,11,-1,-1,-1,-1,-1,-1,-1,3,1,10,11,3,10,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,1,2,11,1,11,9,9,11,8,-1,-1,-1,-1,-1,-1,-1,3,0,9,3,9,11,1,2,9,2,11,9,-1,-1,-1,-1,0,2,11,8,0,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,3,2,11,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,2,3,8,2,8,10,10,8,9,-1,-1,-1,-1,-1,-1,-1,9,10,2,0,9,2,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,2,3,8,2,8,10,0,1,8,1,10,8,-1,-1,-1,-1,1,10,2,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,1,3,8,9,1,8,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,0,9,1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,0,3,8,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1]),vn=[[0,1],[1,2],[2,3],[3,0],[4,5],[5,6],[6,7],[7,4],[0,4],[1,5],[2,6],[3,7]],yn=[[0,0,0],[1,0,0],[1,1,0],[0,1,0],[0,0,1],[1,0,1],[1,1,1],[0,1,1]];class zt{resolution;isoValue;smoothNormals;boundaryMargin;vertexMap=new Map;positions=[];normals=[];normalCounts=[];indices=[];constructor(e,i={}){this.resolution=e,this.isoValue=i.isoValue??.001,this.smoothNormals=i.smoothNormals??!0,this.boundaryMargin=i.boundaryMargin??1}vertexKey(e,i,o){const u=Math.round(e*1e6),c=Math.round(i*1e6),d=Math.round(o*1e6);return`${u},${c},${d}`}addVertex(e,i,o,n,u,c,d,f){const g=e.x*o+c,x=e.y*n+d,v=e.z*u+f,_=this.vertexKey(g,x,v),E=this.vertexMap.get(_);if(E!==void 0){const P=E*3;return this.normals[P]+=i.x,this.normals[P+1]+=i.y,this.normals[P+2]+=i.z,this.normalCounts[E]++,E}const b=this.positions.length/3;return this.vertexMap.set(_,b),this.positions.push(g,x,v),this.normals.push(i.x,i.y,i.z),this.normalCounts.push(1),b}finalizeNormals(){const e=this.positions.length/3;for(let i=0;i<e;i++){const o=i*3;let n=this.normals[o],u=this.normals[o+1],c=this.normals[o+2];const d=Math.sqrt(n*n+u*u+c*c);d>1e-4?(this.normals[o]=n/d,this.normals[o+1]=u/d,this.normals[o+2]=c/d):(this.normals[o]=0,this.normals[o+1]=1,this.normals[o+2]=0)}}extract(e,i){const o=this.resolution,n=this.isoValue;this.vertexMap.clear(),this.positions=[],this.normals=[],this.normalCounts=[],this.indices=[];let u,c,d,f,g,x;if(i){const A=i.max.x-i.min.x,k=i.max.y-i.min.y,M=i.max.z-i.min.z;u=A/o,c=k/o,d=M/o,f=i.min.x,g=i.min.y,x=i.min.z}else u=c=d=1/o,f=g=x=0;const v={x:1/0,y:1/0,z:1/0},_={x:-1/0,y:-1/0,z:-1/0};for(let A=0;A<o-1;A++)for(let k=0;k<o-1;k++)for(let M=0;M<o-1;M++)this.processCell(e,M,k,A,n,v,_,u,c,d,f,g,x);this.finalizeNormals();const E=new Float32Array(this.positions),b=new Float32Array(this.normals),P=new Uint32Array(this.indices),B=this.positions.length/3,z=this.indices.length/3;return{positions:E,normals:b,indices:P,vertexCount:B,triangleCount:z,bounds:{min:v.x===1/0?{x:0,y:0,z:0}:v,max:_.x===-1/0?{x:1,y:1,z:1}:_}}}processCell(e,i,o,n,u,c,d,f,g,x,v,_,E){const b=[],P=[];for(let M=0;M<8;M++){const[D,Y,F]=yn[M],I=i+D,S=o+Y,O=n+F;b[M]=this.sampleVolume(e,I,S,O),P[M]={x:I,y:S,z:O}}let B=0;for(let M=0;M<8;M++)b[M]<u&&(B|=1<<M);if(B===0||B===255)return;const z=pr[B];if(z===0)return;const A=new Array(12);for(let M=0;M<12;M++)if((z&1<<M)!==0){const[D,Y]=vn[M];A[M]=this.vertexInterp(P[D],P[Y],b[D],b[Y],u)}const k=B*16;for(let M=0;M<15;M+=3){const D=xt[k+M];if(D<0)break;const Y=xt[k+M+1],F=xt[k+M+2],I=A[D],S=A[Y],O=A[F];let N,j,W;if(this.smoothNormals)N=this.calcGradient(e,I.x,I.y,I.z),j=this.calcGradient(e,S.x,S.y,S.z),W=this.calcGradient(e,O.x,O.y,O.z);else{const ie={x:S.x-I.x,y:S.y-I.y,z:S.z-I.z},se={x:O.x-I.x,y:O.y-I.y,z:O.z-I.z},oe=this.cross(ie,se);this.normalize(oe),N=j=W=oe}const Q=this.addVertex(I,N,f,g,x,v,_,E),V=this.addVertex(S,j,f,g,x,v,_,E),Se=this.addVertex(O,W,f,g,x,v,_,E);this.indices.push(Q,V,Se),this.updateBounds(I,f,g,x,v,_,E,c,d),this.updateBounds(S,f,g,x,v,_,E,c,d),this.updateBounds(O,f,g,x,v,_,E,c,d)}}sampleVolume(e,i,o,n){const u=this.resolution,c=this.boundaryMargin;if(i<c||o<c||n<c||i>=u-c||o>=u-c||n>=u-c)return 1;const d=Math.floor(i),f=Math.floor(o),g=Math.floor(n);return e[d+f*u+g*u*u]}sampleVolumeTrilinear(e,i,o,n){const u=this.resolution,c=this.boundaryMargin;if(i<c||o<c||n<c||i>=u-c||o>=u-c||n>=u-c)return 1;const d=Math.floor(i),f=Math.floor(o),g=Math.floor(n),x=Math.min(d+1,u-1),v=Math.min(f+1,u-1),_=Math.min(g+1,u-1),E=i-d,b=o-f,P=n-g,B=(V,Se,ie)=>V+Se*u+ie*u*u,z=e[B(d,f,g)],A=e[B(x,f,g)],k=e[B(d,v,g)],M=e[B(x,v,g)],D=e[B(d,f,_)],Y=e[B(x,f,_)],F=e[B(d,v,_)],I=e[B(x,v,_)],S=z*(1-E)+A*E,O=D*(1-E)+Y*E,N=k*(1-E)+M*E,j=F*(1-E)+I*E,W=S*(1-b)+N*b,Q=O*(1-b)+j*b;return W*(1-P)+Q*P}vertexInterp(e,i,o,n,u){if(Math.abs(u-o)<1e-5)return{...e};if(Math.abs(u-n)<1e-5)return{...i};if(Math.abs(o-n)<1e-5)return{...e};const d=Math.max(0,Math.min(1,(u-o)/(n-o)));return{x:e.x+d*(i.x-e.x),y:e.y+d*(i.y-e.y),z:e.z+d*(i.z-e.z)}}calcGradient(e,i,o,n){const c=this.sampleVolumeTrilinear(e,i+.5,o,n)-this.sampleVolumeTrilinear(e,i-.5,o,n),d=this.sampleVolumeTrilinear(e,i,o+.5,n)-this.sampleVolumeTrilinear(e,i,o-.5,n),f=this.sampleVolumeTrilinear(e,i,o,n+.5)-this.sampleVolumeTrilinear(e,i,o,n-.5),g={x:c,y:d,z:f};return this.normalize(g),g}cross(e,i){return{x:e.y*i.z-e.z*i.y,y:e.z*i.x-e.x*i.z,z:e.x*i.y-e.y*i.x}}normalize(e){const i=Math.sqrt(e.x*e.x+e.y*e.y+e.z*e.z);i>1e-4?(e.x/=i,e.y/=i,e.z/=i):(e.x=0,e.y=1,e.z=0)}updateBounds(e,i,o,n,u,c,d,f,g){const x=e.x*i+u,v=e.y*o+c,_=e.z*n+d;x<f.x&&(f.x=x),v<f.y&&(f.y=v),_<f.z&&(f.z=_),x>g.x&&(g.x=x),v>g.y&&(g.y=v),_>g.z&&(g.z=_)}}function bn(w,e=!1){const i=new Jt;if(i.setAttribute("position",new Fe(w.positions,3)),i.setIndex(new Fe(w.indices,1)),e?i.setAttribute("normal",new Fe(w.normals,3)):i.computeVertexNormals(),w.colors&&w.colors.length>=w.vertexCount*4){const o=new Float32Array(w.vertexCount*3);for(let n=0;n<w.vertexCount;n++)o[n*3+0]=w.colors[n*4+0],o[n*3+1]=w.colors[n*4+1],o[n*3+2]=w.colors[n*4+2];i.setAttribute("color",new Fe(o,3))}return i.computeBoundingBox(),i}function xn(){const w=[];for(let e=0;e<pr.length;e+=8){const i=Array.from(pr.slice(e,e+8)).map(o=>`0x${o.toString(16)}u`).join(", ");w.push(`  ${i},`)}return w.join(`
`)}function _n(){const w=[];for(let e=0;e<xt.length;e+=16){const i=Array.from(xt.slice(e,e+16)).map(o=>o===-1?"255u":`${o}u`).join(", ");w.push(`  ${i},`)}return w.join(`
`)}function wn(){const w=[];for(let i=0;i<256;i++){const o=i*16;let n=0;for(let u=0;u<15&&xt[o+u]!==-1;u+=3)n++;w.push(n)}const e=[];for(let i=0;i<w.length;i+=16){const o=w.slice(i,i+16).map(n=>`${n}u`).join(", ");e.push(`  ${o},`)}return e.join(`
`)}function Sn(){return`
// =============================================================================
// GPU Marching Cubes Compute Shader
// =============================================================================

// Uniforms
struct Uniforms {
  resolution: u32,
  isoValue: f32,
  scale: f32,
  // Per-buffer vertex capacity. JS sets this to PRACTICAL_MAX_VERTICES
  // (default 5,000,000) so the overflow check matches the actually-
  // allocated vertex / normal buffer size. Without it, at res=256 the
  // shader would think it has ~250M slots while the buffer only has 5M,
  // and atomicAdd would return slot indices past the buffer end.
  maxVertices: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> volume: array<f32>;
@group(0) @binding(2) var<storage, read_write> vertices: array<f32>;
@group(0) @binding(3) var<storage, read_write> normals: array<f32>;
@group(0) @binding(4) var<storage, read_write> vertexCount: atomic<u32>;

// Edge table (256 entries)
const edgeTable = array<u32, 256>(
${xn()}
);

// Triangle table (256 * 16 entries, 255 = terminator)
const triTable = array<u32, 4096>(
${_n()}
);

// Triangle count per case (for efficient vertex allocation)
const triCountTable = array<u32, 256>(
${wn()}
);

// Corner offsets (x, y, z) for the 8 corners of a voxel
const cornerOffsets = array<vec3<i32>, 8>(
  vec3<i32>(0, 0, 0), // 0
  vec3<i32>(1, 0, 0), // 1
  vec3<i32>(1, 1, 0), // 2
  vec3<i32>(0, 1, 0), // 3
  vec3<i32>(0, 0, 1), // 4
  vec3<i32>(1, 0, 1), // 5
  vec3<i32>(1, 1, 1), // 6
  vec3<i32>(0, 1, 1), // 7
);

// Edge vertex indices (which corners each edge connects)
const edgeVertices = array<vec2<u32>, 12>(
  vec2<u32>(0u, 1u), // 0
  vec2<u32>(1u, 2u), // 1
  vec2<u32>(2u, 3u), // 2
  vec2<u32>(3u, 0u), // 3
  vec2<u32>(4u, 5u), // 4
  vec2<u32>(5u, 6u), // 5
  vec2<u32>(6u, 7u), // 6
  vec2<u32>(7u, 4u), // 7
  vec2<u32>(0u, 4u), // 8
  vec2<u32>(1u, 5u), // 9
  vec2<u32>(2u, 6u), // 10
  vec2<u32>(3u, 7u), // 11
);

// Get voxel index in 1D array
fn getVoxelIndex(x: i32, y: i32, z: i32, res: u32) -> u32 {
  return u32(x) + u32(y) * res + u32(z) * res * res;
}

// Sample SDF value. Voxels within margin (1 voxel) of any edge are forced to
// "outside" (positive) so the iso-surface always closes INSIDE the grid. Without this, a
// shape that reaches the volume boundary extracts as an open shell (visible
// hollow end). Mirrors the CPU MarchingCubesExtractor's boundaryMargin=1 clamp
// (extract/marching-cubes.ts § sampleVolume) — the GPU port originally only
// handled true out-of-bounds, so it left boundaries open while the CPU path
// closed them (inconsistent topology between resolution tiers + open exports).
fn sampleSDF(x: i32, y: i32, z: i32) -> f32 {
  let res = i32(uniforms.resolution);
  let margin = 1;
  if (x < margin || y < margin || z < margin ||
      x >= res - margin || y >= res - margin || z >= res - margin) {
    return 1.0; // boundary / exterior = outside
  }
  return volume[getVoxelIndex(x, y, z, uniforms.resolution)];
}

// Compute gradient for normal calculation
fn computeGradient(x: i32, y: i32, z: i32) -> vec3<f32> {
  let dx = sampleSDF(x + 1, y, z) - sampleSDF(x - 1, y, z);
  let dy = sampleSDF(x, y + 1, z) - sampleSDF(x, y - 1, z);
  let dz = sampleSDF(x, y, z + 1) - sampleSDF(x, y, z - 1);
  return normalize(vec3<f32>(dx, dy, dz));
}

// Interpolate position along edge
fn interpolateEdge(p1: vec3<f32>, p2: vec3<f32>, v1: f32, v2: f32, iso: f32) -> vec3<f32> {
  if (abs(v2 - v1) < 0.00001) {
    return p1;
  }
  let t = (iso - v1) / (v2 - v1);
  return p1 + t * (p2 - p1);
}

@compute @workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let x = i32(gid.x);
  let y = i32(gid.y);
  let z = i32(gid.z);
  let res = i32(uniforms.resolution);

  // Skip if outside bounds (need room for +1 in each direction)
  if (x >= res - 1 || y >= res - 1 || z >= res - 1) {
    return;
  }

  // Sample 8 corners
  var values: array<f32, 8>;
  for (var i = 0u; i < 8u; i++) {
    let offset = cornerOffsets[i];
    values[i] = sampleSDF(x + offset.x, y + offset.y, z + offset.z);
  }

  // Compute case index (0-255)
  var caseIndex: u32 = 0u;
  let iso = uniforms.isoValue;
  for (var i = 0u; i < 8u; i++) {
    if (values[i] < iso) {
      caseIndex = caseIndex | (1u << i);
    }
  }

  // Skip if no triangles for this case
  let triCount = triCountTable[caseIndex];
  if (triCount == 0u) {
    return;
  }

  // Allocate vertex slots (3 vertices per triangle)
  let vertexSlot = atomicAdd(&vertexCount, triCount * 3u);

  // Bounds-check against the JS-side buffer capacity (uniforms.maxVertices).
  // Per-resolution theoretical max would overflow the buffer at res>=128.
  if (vertexSlot + triCount * 3u > uniforms.maxVertices) {
    return;
  }

  // Compute corner positions in world space
  let scale = uniforms.scale;
  var corners: array<vec3<f32>, 8>;
  for (var i = 0u; i < 8u; i++) {
    let offset = cornerOffsets[i];
    corners[i] = vec3<f32>(
      (f32(x) + f32(offset.x)) * scale,
      (f32(y) + f32(offset.y)) * scale,
      (f32(z) + f32(offset.z)) * scale
    );
  }

  // Compute edge intersections
  var edgePoints: array<vec3<f32>, 12>;
  let edges = edgeTable[caseIndex];
  for (var e = 0u; e < 12u; e++) {
    if ((edges & (1u << e)) != 0u) {
      let ev = edgeVertices[e];
      let v1 = ev.x;
      let v2 = ev.y;
      edgePoints[e] = interpolateEdge(corners[v1], corners[v2], values[v1], values[v2], iso);
    }
  }

  // Generate triangles
  let triBase = caseIndex * 16u;
  var vertexIdx = vertexSlot;

  for (var t = 0u; t < 5u; t++) {
    let e0 = triTable[triBase + t * 3u];
    if (e0 == 255u) { break; } // End of triangles

    let e1 = triTable[triBase + t * 3u + 1u];
    let e2 = triTable[triBase + t * 3u + 2u];

    let p0 = edgePoints[e0];
    let p1 = edgePoints[e1];
    let p2 = edgePoints[e2];

    // Compute face normal
    let edge1 = p1 - p0;
    let edge2 = p2 - p0;
    let faceNormal = normalize(cross(edge1, edge2));

    // Write vertices
    let vi = vertexIdx * 3u;
    vertices[vi] = p0.x;
    vertices[vi + 1u] = p0.y;
    vertices[vi + 2u] = p0.z;
    vertices[vi + 3u] = p1.x;
    vertices[vi + 4u] = p1.y;
    vertices[vi + 5u] = p1.z;
    vertices[vi + 6u] = p2.x;
    vertices[vi + 7u] = p2.y;
    vertices[vi + 8u] = p2.z;

    // Write normals (using face normal for now, could use gradient for smooth)
    let ni = vertexIdx * 3u;
    normals[ni] = faceNormal.x;
    normals[ni + 1u] = faceNormal.y;
    normals[ni + 2u] = faceNormal.z;
    normals[ni + 3u] = faceNormal.x;
    normals[ni + 4u] = faceNormal.y;
    normals[ni + 5u] = faceNormal.z;
    normals[ni + 6u] = faceNormal.x;
    normals[ni + 7u] = faceNormal.y;
    normals[ni + 8u] = faceNormal.z;

    vertexIdx = vertexIdx + 3u;
  }
}
`}const Xt=we("GPUMarchingCubes"),Pn={isoValue:.001};class dr{context;resolution;options;pipeline=null;uniformBuffer=null;vertexBuffer=null;normalBuffer=null;countBuffer=null;readbackBuffer=null;bindGroupLayout=null;maxVertices;vertexBufferSize;normalBufferSize;initialized=!1;constructor(e,i,o={}){this.context=e,this.resolution=i,this.options={...Pn,...o};const n=(i-1)**3,u=5e6;this.maxVertices=Math.min(n*15,u),this.vertexBufferSize=this.maxVertices*3*4,this.normalBufferSize=this.maxVertices*3*4}async initialize(){if(this.initialized)return;const{device:e}=this.context,i=Sn(),o=e.createShaderModule({label:"GPU Marching Cubes Shader",code:i});this.bindGroupLayout=e.createBindGroupLayout({label:"GPU MC Bind Group Layout",entries:[{binding:0,visibility:GPUShaderStage.COMPUTE,buffer:{type:"uniform"}},{binding:1,visibility:GPUShaderStage.COMPUTE,buffer:{type:"read-only-storage"}},{binding:2,visibility:GPUShaderStage.COMPUTE,buffer:{type:"storage"}},{binding:3,visibility:GPUShaderStage.COMPUTE,buffer:{type:"storage"}},{binding:4,visibility:GPUShaderStage.COMPUTE,buffer:{type:"storage"}}]});const n=e.createPipelineLayout({label:"GPU MC Pipeline Layout",bindGroupLayouts:[this.bindGroupLayout]});this.pipeline=e.createComputePipeline({label:"GPU Marching Cubes Pipeline",layout:n,compute:{module:o,entryPoint:"main"}}),this.uniformBuffer=e.createBuffer({label:"GPU MC Uniforms",size:16,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST}),this.vertexBuffer=e.createBuffer({label:"GPU MC Vertices",size:this.vertexBufferSize,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC}),this.normalBuffer=e.createBuffer({label:"GPU MC Normals",size:this.normalBufferSize,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC}),this.countBuffer=e.createBuffer({label:"GPU MC Vertex Count",size:4,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC|GPUBufferUsage.COPY_DST}),this.readbackBuffer=e.createBuffer({label:"GPU MC Readback",size:4,usage:GPUBufferUsage.MAP_READ|GPUBufferUsage.COPY_DST}),this.initialized=!0,Xt.info(`Initialized for ${this.resolution}³ volume`)}async extract(e){this.initialized||await this.initialize();const{device:i}=this.context,o=1/this.resolution,n=new ArrayBuffer(16),u=new DataView(n);u.setUint32(0,this.resolution,!0),u.setFloat32(4,this.options.isoValue,!0),u.setFloat32(8,o,!0),u.setUint32(12,this.maxVertices,!0),i.queue.writeBuffer(this.uniformBuffer,0,n),i.queue.writeBuffer(this.countBuffer,0,new Uint32Array([0]));const c=i.createBindGroup({label:"GPU MC Bind Group",layout:this.bindGroupLayout,entries:[{binding:0,resource:{buffer:this.uniformBuffer}},{binding:1,resource:{buffer:e}},{binding:2,resource:{buffer:this.vertexBuffer}},{binding:3,resource:{buffer:this.normalBuffer}},{binding:4,resource:{buffer:this.countBuffer}}]}),d=i.createCommandEncoder({label:"GPU MC Command Encoder"}),f=d.beginComputePass({label:"GPU MC Compute Pass"});f.setPipeline(this.pipeline),f.setBindGroup(0,c);const g=Math.ceil(this.resolution/4),x=Math.ceil(this.resolution/4),v=Math.ceil(this.resolution/4);f.dispatchWorkgroups(g,x,v),f.end(),d.copyBufferToBuffer(this.countBuffer,0,this.readbackBuffer,0,4),i.queue.submit([d.finish()]),await this.readbackBuffer.mapAsync(GPUMapMode.READ);const E=new Uint32Array(this.readbackBuffer.getMappedRange())[0];if(this.readbackBuffer.unmap(),E===0)return this.createEmptyMesh();const b=Math.min(E,this.maxVertices);E>this.maxVertices&&Xt.warn(`GPU MC vertex cap hit at res=${this.resolution}: ${E} requested, clamped to ${this.maxVertices}. Mesh detail truncated — sculpt is denser than the practical buffer cap.`);const P=await this.readBufferData(this.vertexBuffer,b*3*4),B=await this.readBufferData(this.normalBuffer,b*3*4),z=new Uint32Array(b);for(let M=0;M<b;M++)z[M]=M;const A=this.calculateBounds(P),k={positions:P,normals:B,indices:z,vertexCount:b,triangleCount:b/3,bounds:A};return Xt(`Extracted ${k.triangleCount} triangles (${b} vertices)`),k}async readBufferData(e,i){const{device:o}=this.context,n=o.createBuffer({label:"GPU MC Staging",size:i,usage:GPUBufferUsage.MAP_READ|GPUBufferUsage.COPY_DST}),u=o.createCommandEncoder();u.copyBufferToBuffer(e,0,n,0,i),o.queue.submit([u.finish()]),await n.mapAsync(GPUMapMode.READ);const c=new Float32Array(n.getMappedRange().slice(0));return n.unmap(),n.destroy(),c}calculateBounds(e){const i={x:1/0,y:1/0,z:1/0},o={x:-1/0,y:-1/0,z:-1/0};for(let n=0;n<e.length;n+=3)i.x=Math.min(i.x,e[n]),i.y=Math.min(i.y,e[n+1]),i.z=Math.min(i.z,e[n+2]),o.x=Math.max(o.x,e[n]),o.y=Math.max(o.y,e[n+1]),o.z=Math.max(o.z,e[n+2]);return{min:i,max:o}}createEmptyMesh(){return{positions:new Float32Array(0),normals:new Float32Array(0),indices:new Uint32Array(0),vertexCount:0,triangleCount:0,bounds:{min:{x:0,y:0,z:0},max:{x:0,y:0,z:0}}}}async setResolution(e){if(e===this.resolution)return;this.resolution=e;const i=(e-1)**3,o=5e6;this.maxVertices=Math.min(i*15,o),this.vertexBufferSize=this.maxVertices*3*4,this.normalBufferSize=this.maxVertices*3*4,this.vertexBuffer?.destroy(),this.normalBuffer?.destroy();const{device:n}=this.context;this.vertexBuffer=n.createBuffer({label:"GPU MC Vertices",size:this.vertexBufferSize,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC}),this.normalBuffer=n.createBuffer({label:"GPU MC Normals",size:this.normalBufferSize,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC}),Xt.info(`Resolution updated to ${e}³`)}dispose(){this.uniformBuffer?.destroy(),this.vertexBuffer?.destroy(),this.normalBuffer?.destroy(),this.countBuffer?.destroy(),this.readbackBuffer?.destroy(),this.uniformBuffer=null,this.vertexBuffer=null,this.normalBuffer=null,this.countBuffer=null,this.readbackBuffer=null,this.pipeline=null,this.bindGroupLayout=null,this.initialized=!1}}function Cn(w,e){const i=new Jt;if(i.setAttribute("position",new Fe(w.positions,3)),i.setAttribute("normal",new Fe(w.normals,3)),i.setIndex(new Fe(w.indices,1)),e&&e.length>=w.vertexCount*4){const o=new Float32Array(w.vertexCount*3);for(let n=0;n<w.vertexCount;n++)o[n*3+0]=e[n*4+0],o[n*3+1]=e[n*4+1],o[n*3+2]=e[n*4+2];i.setAttribute("color",new Fe(o,3))}else if(w.colors&&w.colors.length>=w.vertexCount*4){const o=new Float32Array(w.vertexCount*3);for(let n=0;n<w.vertexCount;n++)o[n*3+0]=w.colors[n*4+0],o[n*3+1]=w.colors[n*4+1],o[n*3+2]=w.colors[n*4+2];i.setAttribute("color",new Fe(o,3))}return i.computeBoundingBox(),i.computeBoundingSphere(),i}const En=0,di=En,bt={0:"Smooth",1:"Ridge",2:"Warped",3:"Cellular",4:"Bubbles",5:"Crystal"},$n=[{value:0,label:bt[0]},{value:1,label:bt[1]},{value:2,label:bt[2]},{value:3,label:bt[3]},{value:4,label:bt[4]},{value:5,label:bt[5]}];function fr(w){return w.kind==="brush"}function fi(w){return w.kind==="noise"}const _t=8,mr=64,hi=16+_t*mr,pi=we("CompositePipeline");class Tn{context;pipeline=null;uniformBuffer;bindGroupLayout;pipelineLayout;uniformData;uniformView;constructor(e){this.context=e,this.uniformBuffer=e.device.createBuffer({size:hi,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST}),this.uniformData=new ArrayBuffer(hi),this.uniformView=new DataView(this.uniformData),this.bindGroupLayout=e.device.createBindGroupLayout({entries:[{binding:0,visibility:GPUShaderStage.COMPUTE,buffer:{type:"read-only-storage"}},{binding:1,visibility:GPUShaderStage.COMPUTE,buffer:{type:"storage"}},{binding:2,visibility:GPUShaderStage.COMPUTE,buffer:{type:"uniform"}},{binding:3,visibility:GPUShaderStage.COMPUTE,buffer:{type:"storage"}}]}),this.pipelineLayout=e.device.createPipelineLayout({bindGroupLayouts:[this.bindGroupLayout]})}async initialize(){if(this.pipeline)return;const e=this.context.device.createShaderModule({code:jo});this.pipeline=await this.context.device.createComputePipelineAsync({layout:this.pipelineLayout,compute:{module:e,entryPoint:"main"}}),pi.info("Initialized")}setLayers(e,i,o){if(e.length>_t)throw new Error(`Too many noise layers: ${e.length} > MAX_NOISE_LAYERS (${_t})`);const n=this.uniformView;n.setUint32(0,e.length,!0),n.setFloat32(4,o,!0),n.setUint32(8,i,!0),n.setUint32(12,0,!0);for(let u=0;u<e.length;u++){const c=e[u],d=16+u*mr;n.setFloat32(d+0,c.position.x,!0),n.setFloat32(d+4,c.position.y,!0),n.setFloat32(d+8,c.position.z,!0),n.setFloat32(d+12,c.size,!0),n.setFloat32(d+16,c.scale,!0),n.setFloat32(d+20,c.persistence,!0),n.setFloat32(d+24,c.strength,!0),n.setFloat32(d+28,c.seed,!0),n.setUint32(d+32,Math.max(1,Math.min(6,Math.floor(c.octaves))),!0),n.setUint32(d+36,c.enabled?1:0,!0);const f=Number.isFinite(c.noiseType)?Math.floor(c.noiseType):di;n.setUint32(d+40,f>=0&&f<=5?f:di,!0),n.setUint32(d+44,0,!0),n.setFloat32(d+48,c.color.r,!0),n.setFloat32(d+52,c.color.g,!0),n.setFloat32(d+56,c.color.b,!0),n.setFloat32(d+60,0,!0)}for(let u=e.length;u<_t;u++){const c=16+u*mr;n.setUint32(c+36,0,!0)}this.context.device.queue.writeBuffer(this.uniformBuffer,0,this.uniformData)}async run(e,i,o,n){if(!this.pipeline)throw new Error("CompositePipeline not initialized");const u=this.context.device.createBindGroup({layout:this.bindGroupLayout,entries:[{binding:0,resource:{buffer:e}},{binding:1,resource:{buffer:i}},{binding:2,resource:{buffer:this.uniformBuffer}},{binding:3,resource:{buffer:o}}]}),c=Math.ceil(n/st),d=this.context.device.createCommandEncoder(),f=d.beginComputePass();f.setPipeline(this.pipeline),f.setBindGroup(0,u),f.dispatchWorkgroups(c,c,c),f.end(),this.context.device.queue.submit([d.finish()]),await this.context.device.queue.onSubmittedWorkDone()}dispose(){this.uniformBuffer.destroy(),this.pipeline=null,pi.info("Disposed")}}const mi=we("BrushUnionPipeline"),gi=16,vi=32;class Bn{context;initPipeline=null;initBindGroupLayout;initPipelineLayout;initUniformBuffer;initUniformData;initUniformView;unionPipeline=null;unionBindGroupLayout;unionPipelineLayout;unionUniformBuffer;unionUniformData;unionUniformView;constructor(e){this.context=e,this.initUniformBuffer=e.device.createBuffer({size:gi,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST}),this.initUniformData=new ArrayBuffer(gi),this.initUniformView=new DataView(this.initUniformData),this.initBindGroupLayout=e.device.createBindGroupLayout({entries:[{binding:0,visibility:GPUShaderStage.COMPUTE,buffer:{type:"storage"}},{binding:1,visibility:GPUShaderStage.COMPUTE,buffer:{type:"storage"}},{binding:2,visibility:GPUShaderStage.COMPUTE,buffer:{type:"uniform"}}]}),this.initPipelineLayout=e.device.createPipelineLayout({bindGroupLayouts:[this.initBindGroupLayout]}),this.unionUniformBuffer=e.device.createBuffer({size:vi,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST}),this.unionUniformData=new ArrayBuffer(vi),this.unionUniformView=new DataView(this.unionUniformData),this.unionBindGroupLayout=e.device.createBindGroupLayout({entries:[{binding:0,visibility:GPUShaderStage.COMPUTE,buffer:{type:"storage"}},{binding:1,visibility:GPUShaderStage.COMPUTE,buffer:{type:"storage"}},{binding:2,visibility:GPUShaderStage.COMPUTE,buffer:{type:"read-only-storage"}},{binding:3,visibility:GPUShaderStage.COMPUTE,buffer:{type:"read-only-storage"}},{binding:4,visibility:GPUShaderStage.COMPUTE,buffer:{type:"uniform"}}]}),this.unionPipelineLayout=e.device.createPipelineLayout({bindGroupLayouts:[this.unionBindGroupLayout]})}async initialize(){if(this.initPipeline&&this.unionPipeline)return;const[e,i]=[this.context.device.createShaderModule({code:qo}),this.context.device.createShaderModule({code:Ho})],[o,n]=await Promise.all([this.context.device.createComputePipelineAsync({layout:this.initPipelineLayout,compute:{module:e,entryPoint:"main"}}),this.context.device.createComputePipelineAsync({layout:this.unionPipelineLayout,compute:{module:i,entryPoint:"main"}})]);this.initPipeline=o,this.unionPipeline=n,mi.info("Initialized")}async init(e,i,o){if(!this.initPipeline)throw new Error("BrushUnionPipeline not initialized");this.initUniformView.setUint32(0,o,!0),this.initUniformView.setUint32(4,0,!0),this.initUniformView.setUint32(8,0,!0),this.initUniformView.setUint32(12,0,!0),this.context.device.queue.writeBuffer(this.initUniformBuffer,0,this.initUniformData);const n=this.context.device.createBindGroup({layout:this.initBindGroupLayout,entries:[{binding:0,resource:{buffer:e}},{binding:1,resource:{buffer:i}},{binding:2,resource:{buffer:this.initUniformBuffer}}]}),u=Math.ceil(o/st),c=this.context.device.createCommandEncoder(),d=c.beginComputePass();d.setPipeline(this.initPipeline),d.setBindGroup(0,n),d.dispatchWorkgroups(u,u,u),d.end(),this.context.device.queue.submit([c.finish()]),await this.context.device.queue.onSubmittedWorkDone()}async union(e,i,o,n,u,c={x:0,y:0,z:0}){if(!this.unionPipeline)throw new Error("BrushUnionPipeline not initialized");if(e===o||i===n)throw new Error("BrushUnionPipeline.union: dst and src buffers must be distinct");this.unionUniformView.setUint32(0,u,!0),this.unionUniformView.setInt32(4,Math.round(c.x),!0),this.unionUniformView.setInt32(8,Math.round(c.y),!0),this.unionUniformView.setInt32(12,Math.round(c.z),!0),this.unionUniformView.setUint32(16,0,!0),this.unionUniformView.setUint32(20,0,!0),this.unionUniformView.setUint32(24,0,!0),this.unionUniformView.setUint32(28,0,!0),this.context.device.queue.writeBuffer(this.unionUniformBuffer,0,this.unionUniformData);const d=this.context.device.createBindGroup({layout:this.unionBindGroupLayout,entries:[{binding:0,resource:{buffer:e}},{binding:1,resource:{buffer:i}},{binding:2,resource:{buffer:o}},{binding:3,resource:{buffer:n}},{binding:4,resource:{buffer:this.unionUniformBuffer}}]}),f=Math.ceil(u/st),g=this.context.device.createCommandEncoder(),x=g.beginComputePass();x.setPipeline(this.unionPipeline),x.setBindGroup(0,d),x.dispatchWorkgroups(f,f,f),x.end(),this.context.device.queue.submit([g.finish()]),await this.context.device.queue.onSubmittedWorkDone()}dispose(){this.initUniformBuffer.destroy(),this.unionUniformBuffer.destroy(),this.initPipeline=null,this.unionPipeline=null,mi.info("Disposed")}}const me=we("Sculptor"),ge="default-brush";function hr(w){return w<=.04045?w/12.92:Math.pow((w+.055)/1.055,2.4)}const Fn={resolution:128,maxUndoLevels:20,extractDebounceMs:100,useGPUExtraction:"auto",startEmpty:!0,volumeExtent:1.5},yi=50,Mn={tool:"brush",size:.1,strength:.8,hardness:.5,color:{...ko}};class Nn{context;options;volume;colorVolume;boundsManager;volumeResizer;brushPipeline;colorSmoothPipeline;colorSmoothLocalPipeline;sdfFilterPipeline;paintExpandPipeline;smoothDisplayColor=null;smoothPaintExpandVoxels=4;displayPaintExpand=0;cpuExtractor;gpuExtractor=null;useGPU;compositeSdf;compositePipeline;noiseLayers=[];brushVolumes=new Map;brushColorVolumes=new Map;activeBrushLayerId=ge;brushCompositeSdf;compositeColor;brushUnionPipeline;layerOrder=[];compositeMode="base";inFlightComposite=null;compositeRerunRequested=!1;disposed=!1;brush;isStroking=!1;currentStrokePoints=[];meshDirty=!0;currentGeometry=null;undoStack=[];redoStack=[];extractTimeout=null;lastStrokeExtractAt=0;inFlightStrokeExtraction=null;isTransforming=!1;lastTransformExtractAt=0;inFlightTransformExtraction=null;inFlightExtraction=null;eventListeners=new Set;initialized=!1;constructor(e,i={}){this.context=e,this.options={...Fn,...i},this.brush={...Mn};const o=new Ge(e,{resolution:this.options.resolution,initialValue:1}),n=new qe(e,{resolution:this.options.resolution,initialColor:{r:0,g:0,b:0,a:0}});this.volume=o,this.colorVolume=n,this.brushVolumes.set(ge,o),this.brushColorVolumes.set(ge,n),this.activeBrushLayerId=ge,this.layerOrder=[{id:ge,kind:"brush",enabled:!0}],this.boundsManager=new Go({warningMargin:.1,criticalMargin:.05}),this.boundsManager.on(u=>{me(`Received BoundsManager event: ${u.type}`),u.type==="warning"?this.emitEvent("bounds-warning"):u.type==="critical"?this.emitEvent("bounds-critical"):u.type==="clear"&&this.emitEvent("bounds-clear")}),this.volumeResizer=new Jo(e),this.brushPipeline=new cn(e),this.colorSmoothPipeline=new dn(e),this.colorSmoothLocalPipeline=new fn(e),this.sdfFilterPipeline=new pn(e),this.paintExpandPipeline=new gn(e),this.compositeSdf=new Ge(e,{resolution:this.options.resolution,initialValue:1}),this.compositePipeline=new Tn(e),this.brushCompositeSdf=new Ge(e,{resolution:this.options.resolution,initialValue:1}),this.compositeColor=new qe(e,{resolution:this.options.resolution,initialColor:{r:0,g:0,b:0,a:0}}),this.brushUnionPipeline=new Bn(e),this.options.useGPUExtraction==="auto"?this.useGPU=this.options.resolution>=128&&this.options.resolution<256:this.useGPU=this.options.useGPUExtraction===!0,this.cpuExtractor=new zt(this.options.resolution,{smoothNormals:!0}),this.useGPU&&(this.gpuExtractor=new dr(e,this.options.resolution)),me(`Created with resolution ${this.options.resolution}, GPU extraction: ${this.useGPU}`)}async initialize(){this.initialized||(await this.brushPipeline.initialize(),await this.colorSmoothPipeline.initialize(),await this.colorSmoothLocalPipeline.initialize(),await this.sdfFilterPipeline.initialize(),await this.paintExpandPipeline.initialize(),await this.compositePipeline.initialize(),await this.brushUnionPipeline.initialize(),await this.volumeResizer.initialize(),this.gpuExtractor&&await this.gpuExtractor.initialize(),this.options.startEmpty||await this.volume.initializeSphere(),await this.refreshMesh(),this.initialized=!0,me("Initialized (empty:",this.options.startEmpty,")"))}async loadVolume(e){this.brushPipeline.flush(),await this.context.device.queue.onSubmittedWorkDone(),this.brushPipeline.clearBindGroupCache();for(const i of this.brushVolumes.values())i.dispose();for(const i of this.brushColorVolumes.values())i.dispose();this.brushVolumes.clear(),this.brushColorVolumes.clear(),this.volume=await Ge.deserialize(this.context,e),this.colorVolume=new qe(this.context,{resolution:this.volume.resolution,initialColor:{r:0,g:0,b:0,a:0}}),this.brushVolumes.set(ge,this.volume),this.brushColorVolumes.set(ge,this.colorVolume),this.activeBrushLayerId=ge,this.layerOrder=[{id:ge,kind:"brush",enabled:!0}],this.volume.resolution!==this.options.resolution&&(this.options.resolution=this.volume.resolution,this.cpuExtractor=new zt(this.options.resolution,{smoothNormals:!0}),this.gpuExtractor&&await this.gpuExtractor.setResolution(this.options.resolution)),this.recreateCompositeSdfs(this.options.resolution),this.undoStack=[],this.redoStack=[],this.meshDirty=!0,await this.refreshMesh(),me("Loaded volume")}setBrush(e){this.brush={...this.brush,...e},this.brush.size=Math.max(.01,Math.min(.2,this.brush.size)),this.brush.strength=Math.max(0,Math.min(1,this.brush.strength)),this.brush.hardness=Math.max(0,Math.min(1,this.brush.hardness))}getBrush(){return{...this.brush}}setTool(e){this.brush.tool=e}setSize(e){this.brush.size=Math.max(.01,Math.min(.2,e))}setStrength(e){this.brush.strength=Math.max(0,Math.min(1,e))}setHardness(e){this.brush.hardness=Math.max(0,Math.min(1,e))}setColor(e,i,o,n=1){this.brush.color={x:e,y:i,z:o,w:n}}setSymmetry(e){this.brushPipeline.setSymmetry(e)}getSymmetry(){return this.brushPipeline.getSymmetry()}toggleXSymmetry(){const i=this.brushPipeline.getSymmetry().axis==="x"?"off":"x";return this.brushPipeline.setSymmetry({axis:i}),i==="x"}cycleSymmetryAxis(){return this.brushPipeline.cycleSymmetryAxis()}cycleSymmetryMode(){return this.brushPipeline.cycleSymmetryMode()}setRadialSegments(e){this.brushPipeline.setSymmetry({radialSegments:e})}async beginStroke(){if(this.isStroking)return;this.isStroking=!0,this.currentStrokePoints=[],this.strokePointCount=0,me("beginStroke() called");const e=this.activeBrushLayerId;Promise.all([this.volume.createSnapshot(),this.colorVolume.createSnapshot()]).then(([i,o])=>{for(this.undoStack.push({layerId:e,sdf:i,color:o});this.undoStack.length>this.options.maxUndoLevels;)this.undoStack.shift()}),this.redoStack=[],this.emitEvent("stroke-start")}strokePointCount=0;addStrokePoint(e){if(!this.isStroking)return;this.strokePointCount++,this.currentStrokePoints.push(e);const i=this.brush.size;this.boundsManager.checkPosition(e,i),this.brushPipeline.apply(this.volume,this.brush,e,this.colorVolume),this.brush.tool==="smooth"&&(this.colorSmoothLocalPipeline.apply(this.colorVolume,{brushPos:e,brushRadius:this.brush.size,hardness:this.brush.hardness,strength:this.brush.strength*.5}),this.displayPaintExpand=this.smoothPaintExpandVoxels),this.meshDirty=!0;const o=performance.now();!this.inFlightStrokeExtraction&&o-this.lastStrokeExtractAt>=yi&&(this.lastStrokeExtractAt=o,this.inFlightStrokeExtraction=this.refreshMesh().catch(n=>me.error("Live stroke composite/extraction failed:",n)).finally(()=>{this.inFlightStrokeExtraction=null}))}async endStroke(){if(!this.isStroking)return;this.isStroking=!1,this.volume.incrementEditCount(),this.inFlightStrokeExtraction&&await this.inFlightStrokeExtraction;const e=this.brush.tool;(e==="brush"||e==="pull")&&await this.smoothColors({kernelSize:1,strength:.4,iterations:1},!1),e==="smooth"&&(await this.paintExpandPipeline.dilate(this.colorVolume,{iterations:this.smoothPaintExpandVoxels}),this.displayPaintExpand=0),await this.refreshMesh(),this.emitEvent("stroke-end")}beginTransform(){this.isTransforming=!0}previewLayers(e,i){if(!this.isTransforming||this.disposed)return;const o=performance.now();this.inFlightTransformExtraction||o-this.lastTransformExtractAt<yi||(this.lastTransformExtractAt=o,this.inFlightTransformExtraction=(async()=>{await this.setLayers(e),this.setActiveLayer(i),await this.runComposite(),await this.extractMeshImmediate()})().catch(n=>me.error("Live transform composite/extraction failed:",n)).finally(()=>{this.inFlightTransformExtraction=null}))}async endTransform(){this.isTransforming&&(this.isTransforming=!1,this.inFlightTransformExtraction&&await this.inFlightTransformExtraction)}async undo(){for(;this.undoStack.length>0;){const e=this.undoStack.pop(),i=this.brushVolumes.get(e.layerId);if(!i){me.warn(`undo: snapshot's layer ${e.layerId} no longer exists, skipping`);continue}const[o,n]=await Promise.all([i.createSnapshot(),this.colorVolume.createSnapshot()]);return this.redoStack.push({layerId:e.layerId,sdf:o,color:n}),await Promise.all([i.restoreSnapshot(e.sdf),this.colorVolume.restoreSnapshot(e.color)]),this.setActiveLayer(e.layerId),this.meshDirty=!0,await this.refreshMesh(),this.emitEvent("undo"),!0}return!1}async redo(){for(;this.redoStack.length>0;){const e=this.redoStack.pop(),i=this.brushVolumes.get(e.layerId);if(!i){me.warn(`redo: snapshot's layer ${e.layerId} no longer exists, skipping`);continue}const[o,n]=await Promise.all([i.createSnapshot(),this.colorVolume.createSnapshot()]);return this.undoStack.push({layerId:e.layerId,sdf:o,color:n}),await Promise.all([i.restoreSnapshot(e.sdf),this.colorVolume.restoreSnapshot(e.color)]),this.setActiveLayer(e.layerId),this.meshDirty=!0,await this.refreshMesh(),this.emitEvent("redo"),!0}return!1}get canUndo(){return this.undoStack.length>0}get canRedo(){return this.redoStack.length>0}async filterSDF(e){const i=this.activeBrushLayerId,[o,n]=await Promise.all([this.volume.createSnapshot(),this.colorVolume.createSnapshot()]);for(this.undoStack.push({layerId:i,sdf:o,color:n});this.undoStack.length>this.options.maxUndoLevels;)this.undoStack.shift();this.redoStack=[],this.brushPipeline.flush(),await this.context.device.queue.onSubmittedWorkDone(),await this.sdfFilterPipeline.filter(this.volume,e),this.volume.incrementEditCount();const u=e.iterations??1;await this.paintExpandPipeline.dilate(this.colorVolume,{iterations:u}),this.meshDirty=!0,await this.refreshMesh(),this.emitEvent("mesh-updated")}async clear(){const e=this.activeBrushLayerId,[i,o]=await Promise.all([this.volume.createSnapshot(),this.colorVolume.createSnapshot()]);for(this.undoStack.push({layerId:e,sdf:i,color:o});this.undoStack.length>this.options.maxUndoLevels;)this.undoStack.shift();this.redoStack=[];const n=new Float32Array(this.volume.voxelCount);n.fill(1),await this.volume.upload(n),await this.colorVolume.fill({r:0,g:0,b:0,a:0}),this.meshDirty=!0,await this.refreshMesh(),this.emitEvent("mesh-updated")}async seed(e){if(e.sdf.length!==this.volume.voxelCount)throw new Error(`seed: SDF array length ${e.sdf.length} does not match voxel count ${this.volume.voxelCount}`);if(e.colorAlpha.length!==this.colorVolume.voxelCount*4)throw new Error(`seed: colorAlpha length ${e.colorAlpha.length} does not match expected ${this.colorVolume.voxelCount*4}`);await Promise.all([this.volume.upload(e.sdf),this.colorVolume.upload(e.colorAlpha)]),e.bounds&&(this.volume.bounds=e.bounds),this.undoStack=[],this.redoStack=[],this.meshDirty=!0,await this.refreshMesh(),this.emitEvent("mesh-updated")}async captureSnapshot(){return{sdf:await this.volume.serialize(),color:await this.colorVolume.serialize()}}async restoreSnapshot(e){this.brushPipeline.flush(),await this.context.device.queue.onSubmittedWorkDone(),this.brushPipeline.clearBindGroupCache();for(const i of this.brushVolumes.values())i.dispose();for(const i of this.brushColorVolumes.values())i.dispose();this.brushVolumes.clear(),this.brushColorVolumes.clear(),this.volume=await Ge.deserialize(this.context,e.sdf),this.colorVolume=await qe.deserialize(this.context,e.color),this.brushVolumes.set(ge,this.volume),this.brushColorVolumes.set(ge,this.colorVolume),this.activeBrushLayerId=ge,this.layerOrder=[{id:ge,kind:"brush",enabled:!0}],this.volume.resolution!==this.options.resolution&&(this.options.resolution=this.volume.resolution,this.cpuExtractor=new zt(this.options.resolution,{smoothNormals:!0}),this.gpuExtractor&&await this.gpuExtractor.setResolution(this.options.resolution)),this.recreateCompositeSdfs(this.options.resolution),this.meshDirty=!0,await this.refreshMesh(),this.emitEvent("mesh-updated")}recreateCompositeSdfs(e){this.compositeSdf.dispose(),this.compositeSdf=new Ge(this.context,{resolution:e,initialValue:1}),this.brushCompositeSdf.dispose(),this.brushCompositeSdf=new Ge(this.context,{resolution:e,initialValue:1}),this.compositeColor.dispose(),this.compositeColor=new qe(this.context,{resolution:e,initialColor:{r:0,g:0,b:0,a:0}}),this.smoothDisplayColor?.dispose(),this.smoothDisplayColor=null,this.displayPaintExpand=0,this.compositeMode="base"}ensureSmoothDisplayColor(){const e=this.colorVolume.resolution;return(!this.smoothDisplayColor||this.smoothDisplayColor.resolution!==e)&&(this.smoothDisplayColor?.dispose(),this.smoothDisplayColor=new qe(this.context,{resolution:e,initialColor:{r:0,g:0,b:0,a:0}})),this.smoothDisplayColor}async setLayers(e){const i=e.filter(fi).length;if(i>_t)throw new Error(`setLayers: ${i} noise layers > MAX_NOISE_LAYERS (${_t}) — v1 cap`);const o=new Set;for(const u of e)fr(u)&&o.add(u.id);const n=[];for(const[u,c]of this.brushVolumes)if(!o.has(u)){const d=this.brushColorVolumes.get(u);d&&n.push({sdf:c,color:d})}n.length>0&&(this.brushPipeline.flush(),await this.context.device.queue.onSubmittedWorkDone(),this.brushPipeline.clearBindGroupCache());for(const[u,c]of this.brushVolumes)o.has(u)||(c.dispose(),this.brushVolumes.delete(u));for(const[u,c]of this.brushColorVolumes)o.has(u)||(c.dispose(),this.brushColorVolumes.delete(u));for(const u of e)fr(u)&&!this.brushVolumes.has(u.id)&&(this.brushVolumes.set(u.id,new Ge(this.context,{resolution:this.options.resolution,initialValue:1})),this.brushColorVolumes.set(u.id,new qe(this.context,{resolution:this.options.resolution,initialColor:{r:0,g:0,b:0,a:0}})));if(this.layerOrder=e.map(u=>fr(u)?{id:u.id,kind:u.kind,enabled:u.enabled,offset:{...u.offset}}:{id:u.id,kind:u.kind,enabled:u.enabled}),this.noiseLayers=e.filter(fi),!this.brushVolumes.has(this.activeBrushLayerId)){const u=this.brushVolumes.keys().next().value;if(u!==void 0)this.setActiveLayer(u);else throw new Error("setLayers: no brush layers — at least one is required")}}setActiveLayer(e){const i=this.brushVolumes.get(e),o=this.brushColorVolumes.get(e);!i||!o||(this.volume=i,this.colorVolume=o,this.activeBrushLayerId=e)}getActiveLayerId(){return this.activeBrushLayerId}getNoiseLayers(){return[...this.noiseLayers]}async runComposite(){if(this.compositeRerunRequested=!0,this.inFlightComposite)return this.inFlightComposite;const e=(async()=>{for(;this.compositeRerunRequested;){if(this.disposed)return;this.compositeRerunRequested=!1;const i=this.noiseLayers.filter(n=>n.enabled),o=this.layerOrder.filter(n=>n.kind==="brush"&&n.enabled).map(n=>n.id);if(i.length===0&&o.length===1&&o[0]===this.activeBrushLayerId){const u=this.layerOrder.find(d=>d.id===this.activeBrushLayerId)?.offset;if(!u||u.x===0&&u.y===0&&u.z===0){this.compositeMode="base";continue}}if(this.brushPipeline.flush(),await this.context.device.queue.onSubmittedWorkDone(),this.disposed)return;await this.runBrushUnionAndMaybeNoise(o,i)}})();this.inFlightComposite=e;try{await e}finally{this.inFlightComposite=null}}async runBrushUnionAndMaybeNoise(e,i){await this.brushUnionPipeline.init(this.brushCompositeSdf.buffer,this.compositeColor.buffer,this.options.resolution);const o=this.options.resolution/this.options.volumeExtent;for(const n of e){const u=this.brushVolumes.get(n),c=this.brushColorVolumes.get(n);if(!u||!c)continue;const f=this.layerOrder.find(g=>g.id===n)?.offset??{x:0,y:0,z:0};await this.brushUnionPipeline.union(this.brushCompositeSdf.buffer,this.compositeColor.buffer,u.buffer,c.buffer,this.options.resolution,{x:f.x*o,y:f.y*o,z:f.z*o})}if(i.length===0){this.compositeMode="brush-only";return}this.compositePipeline.setLayers(i,this.options.resolution,this.options.volumeExtent),await this.compositePipeline.run(this.brushCompositeSdf.buffer,this.compositeSdf.buffer,this.compositeColor.buffer,this.options.resolution),this.compositeMode="composite"}async refreshMesh(){return await this.runComposite(),this.extractMeshImmediate()}async getMesh(){return(this.meshDirty||!this.currentGeometry)&&await this.extractMeshImmediate(),this.currentGeometry}getCurrentMesh(){return this.currentGeometry}async extractMeshImmediate(){return this.inFlightExtraction?this.inFlightExtraction:(this.inFlightExtraction=this._extractMeshImmediateInternal().finally(()=>{this.inFlightExtraction=null}),this.inFlightExtraction)}async _extractMeshImmediateInternal(){this.extractTimeout!==null&&(clearTimeout(this.extractTimeout),this.extractTimeout=null),this.brushPipeline.flush();let e;const i=this.compositeMode==="composite"?this.compositeSdf:this.compositeMode==="brush-only"?this.brushCompositeSdf:this.volume,o=this.compositeMode==="base"?this.colorVolume:this.compositeColor;let n=o;if(this.displayPaintExpand>0){const f=this.ensureSmoothDisplayColor(),g=this.context.device.createCommandEncoder();g.copyBufferToBuffer(o.buffer,0,f.buffer,0,o.bufferSize),this.context.device.queue.submit([g.finish()]),await this.paintExpandPipeline.dilate(f,{iterations:this.displayPaintExpand}),n=f}const u=await n.readback(),c=this.volume.bounds,d=this.isStroking||this.isTransforming;if(this.useGPU&&this.gpuExtractor){const f=await this.gpuExtractor.extract(i.buffer),g=d?void 0:f.normals,x=this.sampleColorsAtVertices(f.positions,u,null,g);e=Cn(f,x)}else{const f=await i.readback(),g=this.cpuExtractor.extract(f,c),x=d?void 0:g.normals,v=this.sampleColorsAtVertices(g.positions,u,c,x);g.colors=v,e=bn(g,!0)}return this.currentGeometry&&this.currentGeometry.dispose(),this.currentGeometry=e,this.meshDirty=!1,this.emitEvent("mesh-updated"),e}async smoothColors(e,i=!0){if(i){const o=this.activeBrushLayerId,[n,u]=await Promise.all([this.volume.createSnapshot(),this.colorVolume.createSnapshot()]);for(this.undoStack.push({layerId:o,sdf:n,color:u});this.undoStack.length>this.options.maxUndoLevels;)this.undoStack.shift();this.redoStack=[]}await this.colorSmoothPipeline.smooth(this.colorVolume,e),this.meshDirty=!0,await this.refreshMesh(),this.emitEvent("mesh-updated"),me("Colors smoothed")}async serialize(){return this.volume.serialize(!0)}getMetadata(){return{resolution:this.options.resolution,editCount:this.volume.editCount,lastCommitAt:Date.now(),createdAt:Date.now()}}getState(){return{isStroking:this.isStroking,brush:{...this.brush},editCount:this.volume.editCount,meshDirty:this.meshDirty,undoCount:this.undoStack.length,redoCount:this.redoStack.length}}getResolution(){return this.options.resolution}getEditCount(){return this.volume.editCount}getColorVolumeBuffer(){return this.colorVolume.buffer}getVolumeBuffer(){return this.volume.buffer}getContext(){return this.context}async readColorVolume(){return(this.compositeMode==="base"?this.colorVolume:this.compositeColor).readback()}getVolumeBounds(){return this.volume.bounds}getBoundsWarning(){return this.boundsManager.getLastWarning()}needsExpansion(){const e=this.boundsManager.getLastWarning();return!e||!e.closestEdge?!1:e.closestEdge.distance<.05}canExpand(){return this.options.resolution<256}hasPaintedData(){if(this.noiseLayers.length>0)return!0;for(const e of this.brushVolumes.values())if(e.editCount>0)return!0;return!1}async resetToResolution(e){if(e===this.options.resolution)return;this.brushPipeline.flush(),await this.context.device.queue.onSubmittedWorkDone(),this.brushPipeline.clearBindGroupCache();for(const u of this.brushVolumes.values())u.dispose();for(const u of this.brushColorVolumes.values())u.dispose();this.brushVolumes.clear(),this.brushColorVolumes.clear(),this.options.resolution=e;const i=new Ge(this.context,{resolution:e,initialValue:1}),o=new qe(this.context,{resolution:e,initialColor:{r:0,g:0,b:0,a:0}});this.volume=i,this.colorVolume=o,this.brushVolumes.set(ge,i),this.brushColorVolumes.set(ge,o),this.activeBrushLayerId=ge,this.layerOrder=[{id:ge,kind:"brush",enabled:!0}],this.noiseLayers=[],this.recreateCompositeSdfs(e),this.cpuExtractor=new zt(e,{smoothNormals:!0});const n=this.options.useGPUExtraction==="auto"?e>=128&&e<256:this.options.useGPUExtraction===!0;this.useGPU&&!n?(this.gpuExtractor?.dispose(),this.gpuExtractor=null,this.useGPU=!1):!this.useGPU&&n?(this.gpuExtractor=new dr(this.context,e),await this.gpuExtractor.initialize(),this.useGPU=!0):this.useGPU&&this.gpuExtractor&&await this.gpuExtractor.setResolution(e),this.undoStack=[],this.redoStack=[],this.meshDirty=!0,await this.refreshMesh(),me.info(`Reset to ${e}³ (data cleared)`)}getBoundsManager(){return this.boundsManager}async expandVolume(e){if(!this.canExpand())return me.warn("Cannot expand: already at max resolution (256)"),null;const i=this.options.resolution,o=i*2;o===256&&this.brushVolumes.size>=5&&me.warn(`expandVolume: 256³ with ${this.brushVolumes.size} brush layers ≈ ${(this.brushVolumes.size*64+256).toFixed(0)} MB of GPU memory (SDF + color). Close to most-machine budget — consider deleting unused layers first.`);const n=e?en(i,o,e):Qo(i,o);me(`Expanding volume ${i}³ → ${o}³ with offset (${n.x}, ${n.y}, ${n.z})`);const u=new Map,c=new Map;for(const[f,g]of this.brushVolumes){const x=new Ge(this.context,{resolution:o,initialValue:1});await this.volumeResizer.resizeSDF(g.buffer,x.buffer,{srcResolution:i,dstResolution:o,offset:n,defaultValue:1}),g.bounds&&(x.bounds=g.bounds),u.set(f,x);const v=this.brushColorVolumes.get(f),_=new qe(this.context,{resolution:o,initialColor:{r:0,g:0,b:0,a:0}});v&&await this.volumeResizer.resizeColor(v.buffer,_.buffer,{srcResolution:i,dstResolution:o,offset:n}),c.set(f,_)}this.brushPipeline.flush(),await this.context.device.queue.onSubmittedWorkDone(),this.brushPipeline.clearBindGroupCache();for(const f of this.brushVolumes.values())f.dispose();for(const f of this.brushColorVolumes.values())f.dispose();this.brushVolumes=u,this.brushColorVolumes=c,this.volume=this.brushVolumes.get(this.activeBrushLayerId)??this.brushVolumes.values().next().value,this.colorVolume=this.brushColorVolumes.get(this.activeBrushLayerId)??this.brushColorVolumes.values().next().value,this.options.resolution=o,this.recreateCompositeSdfs(o),this.cpuExtractor=new zt(o,{smoothNormals:!0});const d=this.options.useGPUExtraction==="auto"?o>=128&&o<256:this.options.useGPUExtraction===!0;return this.useGPU&&!d?(this.gpuExtractor?.dispose(),this.gpuExtractor=null,this.useGPU=!1):!this.useGPU&&d?(this.gpuExtractor=new dr(this.context,o),await this.gpuExtractor.initialize(),this.useGPU=!0):this.useGPU&&this.gpuExtractor&&await this.gpuExtractor.setResolution(o),this.undoStack=[],this.redoStack=[],this.boundsManager.clear(),this.meshDirty=!0,await this.refreshMesh(),this.emitEvent("bounds-expanded"),me(`Volume expanded to ${o}³`),o}sampleColorsAtVertices(e,i,o,n){const u=e.length/3,c=new Float32Array(u*4),d=this.options.resolution,f=n!==void 0&&n.length===e.length;for(let g=0;g<u;g++){const x=e[g*3+0],v=e[g*3+1],_=e[g*3+2];let E,b,P;if(o){const z=o.max.x-o.min.x,A=o.max.y-o.min.y,k=o.max.z-o.min.z,M=z>0?(x-o.min.x)/z:.5,D=A>0?(v-o.min.y)/A:.5,Y=k>0?(_-o.min.z)/k:.5;E=M*d,b=D*d,P=Y*d}else E=x*d,b=v*d,P=_*d;let B;if(f){const z=n[g*3+0],A=n[g*3+1],k=n[g*3+2];B=this.sampleColorTriplanar(i,E,b,P,d,z,A,k)}else B=this.sampleColorTrilinear(i,E,b,P,d);c[g*4+0]=hr(B.r),c[g*4+1]=hr(B.g),c[g*4+2]=hr(B.b),c[g*4+3]=B.a}return c}sampleColorTriplanar(e,i,o,n,u,c,d,f){const x=[{ox:0,oy:0,oz:0,w:2},{ox:1,oy:0,oz:0,w:1},{ox:-1,oy:0,oz:0,w:1},{ox:0,oy:1,oz:0,w:1},{ox:0,oy:-1,oz:0,w:1},{ox:0,oy:0,oz:1,w:1},{ox:0,oy:0,oz:-1,w:1},{ox:.7,oy:.7,oz:0,w:.5},{ox:-.7,oy:.7,oz:0,w:.5},{ox:.7,oy:-.7,oz:0,w:.5},{ox:-.7,oy:-.7,oz:0,w:.5},{ox:0,oy:.7,oz:.7,w:.5},{ox:0,oy:-.7,oz:.7,w:.5},{ox:0,oy:.7,oz:-.7,w:.5},{ox:0,oy:-.7,oz:-.7,w:.5}],v=-.3,_=i+c*v,E=o+d*v,b=n+f*v;let P=0,B=0,z=0,A=0,k=0,M=0;for(const{ox:D,oy:Y,oz:F,w:I}of x){const S=_+D*.6,O=E+Y*.6,N=b+F*.6,j=this.sampleColorTrilinear(e,S,O,N,u),W=I*j.a;P+=j.r*W,B+=j.g*W,z+=j.b*W,A+=j.a*I,k+=W,M+=I}return k<.001?{r:.85,g:.85,b:.85,a:0}:{r:P/k,g:B/k,b:z/k,a:A/M}}sampleColorTrilinear(e,i,o,n,u){const c=Math.max(0,Math.min(u-1,i)),d=Math.max(0,Math.min(u-1,o)),f=Math.max(0,Math.min(u-1,n)),g=Math.floor(c),x=Math.floor(d),v=Math.floor(f),_=Math.min(g+1,u-1),E=Math.min(x+1,u-1),b=Math.min(v+1,u-1),P=c-g,B=d-x,z=f-v,A=(ie,se,oe)=>{const Z=(ie+se*u+oe*u*u)*4;return{r:(e[Z+0]??0)/255,g:(e[Z+1]??0)/255,b:(e[Z+2]??0)/255,a:(e[Z+3]??0)/255}},k=A(g,x,v),M=A(_,x,v),D=A(g,E,v),Y=A(_,E,v),F=A(g,x,b),I=A(_,x,b),S=A(g,E,b),O=A(_,E,b),N=(ie,se,oe)=>ie+(se-ie)*oe,j=(ie,se,oe,Z,ee,He,Me,re,Xe,U,ht)=>{const ot=N(ie,se,Xe),pt=N(ee,He,Xe),wt=N(oe,Z,Xe),Pe=N(Me,re,Xe),Ke=N(ot,wt,U),mt=N(pt,Pe,U);return N(Ke,mt,ht)},W=j(k.a,M.a,D.a,Y.a,F.a,I.a,S.a,O.a,P,B,z);if(W<.001)return{r:.85,g:.85,b:.85,a:0};const Q=j(k.r*k.a,M.r*M.a,D.r*D.a,Y.r*Y.a,F.r*F.a,I.r*I.a,S.r*S.a,O.r*O.a,P,B,z),V=j(k.g*k.a,M.g*M.a,D.g*D.a,Y.g*Y.a,F.g*F.a,I.g*I.a,S.g*S.a,O.g*O.a,P,B,z),Se=j(k.b*k.a,M.b*M.a,D.b*D.a,Y.b*Y.a,F.b*F.a,I.b*I.a,S.b*S.a,O.b*O.a,P,B,z);return{r:Q/W,g:V/W,b:Se/W,a:W}}on(e){return this.eventListeners.add(e),()=>{this.eventListeners.delete(e)}}emitEvent(e){const i={type:e,state:this.getState()};for(const o of this.eventListeners)try{o(i)}catch(n){me.error("Event callback error:",n)}}dispose(){this.disposed=!0,this.extractTimeout!==null&&(clearTimeout(this.extractTimeout),this.extractTimeout=null);for(const e of this.brushVolumes.values())e.dispose();for(const e of this.brushColorVolumes.values())e.dispose();this.brushVolumes.clear(),this.brushColorVolumes.clear(),this.compositeSdf.dispose(),this.brushCompositeSdf.dispose(),this.compositeColor.dispose(),this.smoothDisplayColor?.dispose(),this.boundsManager.dispose(),this.volumeResizer.dispose(),this.brushPipeline.dispose(),this.colorSmoothPipeline.dispose(),this.colorSmoothLocalPipeline.dispose(),this.sdfFilterPipeline.dispose(),this.paintExpandPipeline.dispose(),this.compositePipeline.dispose(),this.brushUnionPipeline.dispose(),this.gpuExtractor&&(this.gpuExtractor.dispose(),this.gpuExtractor=null),this.currentGeometry&&(this.currentGeometry.dispose(),this.currentGeometry=null),this.undoStack=[],this.redoStack=[],this.eventListeners.clear(),this.emitEvent("disposed"),me("Disposed")}}we("TriplanarBaker");function zn(w,e,i,o={}){const{iterations:n=4,alphaThreshold:u=1}=o,c=new Uint8Array(e*i);for(let f=0;f<e*i;f++)c[f]=w[f*4+3]>=u?1:0;const d=new Uint8Array(c);for(let f=0;f<n;f++){let g=!1;for(let x=0;x<i;x++)for(let v=0;v<e;v++){const _=x*e+v;if(d[_]===1)continue;let E=0,b=0,P=0,B=0,z=0;const A=[[-1,-1],[0,-1],[1,-1],[-1,0],[1,0],[-1,1],[0,1],[1,1]];for(const[k,M]of A){const D=v+k,Y=x+M;if(D>=0&&D<e&&Y>=0&&Y<i){const F=Y*e+D;if(c[F]===1){const I=F*4;E+=w[I+0],b+=w[I+1],P+=w[I+2],B+=w[I+3],z++}}}if(z>0){const k=_*4;w[k+0]=Math.round(E/z),w[k+1]=Math.round(b/z),w[k+2]=Math.round(P/z),w[k+3]=Math.round(B/z),d[_]=1,g=!0}}if(c.set(d),!g)break}}var Un=(async function(w={}){var e,i=w,o,n,u=new Promise((r,t)=>{o=r,n=t}),c=typeof window=="object",d=typeof WorkerGlobalScope<"u",f=typeof process=="object"&&process.versions?.node&&process.type!="renderer",g=!c&&!f&&!d;if(f){const{createRequire:r}=await Ut(()=>import("./__vite-browser-external-ke9ZaMpT.js").then(t=>t.n),[]);var x=r(import.meta.url)}var v=import.meta.url,_="";function E(r){return _+r}var b,P;if(f){if(!(typeof process=="object"&&process.versions?.node&&process.type!="renderer"))throw new Error("not compiled for this environment (did you build to HTML and try to run it not on the web, or set ENVIRONMENT to something - like node - and run it someplace else - like on the web?)");var B=process.versions.node,z=B.split(".").slice(0,3);if(z=z[0]*1e4+z[1]*100+z[2].split("-")[0]*1,z<16e4)throw new Error("This emscripten-generated code requires node v16.0.0 (detected v"+B+")");var A=x("fs"),k=x("path");v.startsWith("file:")&&(_=k.dirname(x("url").fileURLToPath(v))+"/"),P=t=>{t=ee(t)?new URL(t):t;var s=A.readFileSync(t);return S(Buffer.isBuffer(s)),s},b=async(t,s=!0)=>{t=ee(t)?new URL(t):t;var a=A.readFileSync(t,s?void 0:"utf8");return S(s?Buffer.isBuffer(a):typeof a=="string"),a},process.argv.length>1&&process.argv[1].replace(/\\/g,"/"),process.argv.slice(2)}else if(g){if(typeof process=="object"&&process.versions?.node&&process.type!="renderer"||typeof window=="object"||typeof WorkerGlobalScope<"u")throw new Error("not compiled for this environment (did you build to HTML and try to run it not on the web, or set ENVIRONMENT to something - like node - and run it someplace else - like on the web?)")}else if(c||d){try{_=new URL(".",v).href}catch{}if(!(typeof window=="object"||typeof WorkerGlobalScope<"u"))throw new Error("not compiled for this environment (did you build to HTML and try to run it not on the web, or set ENVIRONMENT to something - like node - and run it someplace else - like on the web?)");d&&(P=r=>{var t=new XMLHttpRequest;return t.open("GET",r,!1),t.responseType="arraybuffer",t.send(null),new Uint8Array(t.response)}),b=async r=>{S(!ee(r),"readAsync does not work with file:// URLs");var t=await fetch(r,{credentials:"same-origin"});if(t.ok)return t.arrayBuffer();throw new Error(t.status+" : "+t.url)}}else throw new Error("environment detection error");var M=console.log.bind(console),D=console.error.bind(console);S(!g,"shell environment detected but not enabled at build time.  Add `shell` to `-sENVIRONMENT` to enable.");var Y;typeof WebAssembly!="object"&&D("no native wasm support detected");var F,I=!1;function S(r,t){r||he("Assertion failed"+(t?": "+t:""))}var O,N,j,W,Q,V,Se,ie,se,oe,Z=!1,ee=r=>r.startsWith("file://");function He(){var r=Gr();S((r&3)==0),r==0&&(r+=4),V[r>>2]=34821223,V[r+4>>2]=2310721022,V[0]=1668509029}function Me(){if(!I){var r=Gr();r==0&&(r+=4);var t=V[r>>2],s=V[r+4>>2];(t!=34821223||s!=2310721022)&&he(`Stack overflow! Stack cookie has been overwritten at ${J(r)}, expected hex dwords 0x89BACDFE and 0x2135467, but received ${J(s)} ${J(t)}`),V[0]!=1668509029&&he("Runtime error: The application has corrupted its heap memory area (address zero)!")}}class re extends Error{}class Xe extends re{constructor(t){super(t),this.excPtr=t;const s=Or(t);this.name=s[0],this.message=s[1]}}(()=>{var r=new Int16Array(1),t=new Int8Array(r.buffer);if(r[0]=25459,t[0]!==115||t[1]!==99)throw"Runtime error: expected the system to be little-endian! (Run with -sSUPPORT_BIG_ENDIAN to bypass)"})();function U(r){Object.getOwnPropertyDescriptor(i,r)&&he(`\`Module.${r}\` was supplied but \`${r}\` not included in INCOMING_MODULE_JS_API`)}function ht(r){return r==="FS_createPath"||r==="FS_createDataFile"||r==="FS_createPreloadedFile"||r==="FS_unlink"||r==="addRunDependency"||r==="FS_createLazyFile"||r==="FS_createDevice"||r==="removeRunDependency"}function ot(r,t){typeof globalThis<"u"&&!Object.getOwnPropertyDescriptor(globalThis,r)&&Object.defineProperty(globalThis,r,{configurable:!0,get(){t()}})}function pt(r,t){ot(r,()=>{Ce(`\`${r}\` is not longer defined by emscripten. ${t}`)})}pt("buffer","Please use HEAP8.buffer or wasmMemory.buffer"),pt("asm","Please use wasmExports instead");function wt(r){ot(r,()=>{var t=`\`${r}\` is a library symbol and not included by default; add it to your library.js __deps or to DEFAULT_LIBRARY_FUNCS_TO_INCLUDE on the command line`,s=r;s.startsWith("_")||(s="$"+r),t+=` (e.g. -sDEFAULT_LIBRARY_FUNCS_TO_INCLUDE='${s}')`,ht(r)&&(t+=". Alternatively, forcing filesystem support (-sFORCE_FILESYSTEM) can export this for you"),Ce(t)}),Pe(r)}function Pe(r){Object.getOwnPropertyDescriptor(i,r)||Object.defineProperty(i,r,{configurable:!0,get(){var t=`'${r}' was not exported. add it to EXPORTED_RUNTIME_METHODS (see the Emscripten FAQ)`;ht(r)&&(t+=". Alternatively, forcing filesystem support (-sFORCE_FILESYSTEM) can export this for you"),he(t)}})}function Ke(){var r=F.buffer;O=new Int8Array(r),j=new Int16Array(r),N=new Uint8Array(r),W=new Uint16Array(r),Q=new Int32Array(r),V=new Uint32Array(r),Se=new Float32Array(r),oe=new Float64Array(r),ie=new BigInt64Array(r),se=new BigUint64Array(r)}S(typeof Int32Array<"u"&&typeof Float64Array<"u"&&Int32Array.prototype.subarray!=null&&Int32Array.prototype.set!=null,"JS engine does not provide full typed array support");function mt(){S(!Z),Z=!0,Me(),ke.__wasm_call_ctors()}function At(){Me()}var et=0,tt=null,$e={},Ae=null;function nt(r){et++,S(!$e[r]),$e[r]=1,Ae===null&&typeof setInterval<"u"&&(Ae=setInterval(()=>{if(I){clearInterval(Ae),Ae=null;return}var t=!1;for(var s in $e)t||(t=!0,D("still waiting on run dependencies:")),D(`dependency: ${s}`);t&&D("(end of list)")},1e4))}function gt(r){if(et--,S($e[r]),delete $e[r],et==0&&(Ae!==null&&(clearInterval(Ae),Ae=null),tt)){var t=tt;tt=null,t()}}function he(r){r="Aborted("+r+")",D(r),I=!0;var t=new WebAssembly.RuntimeError(r);throw n(t),t}var ve={error(){he("Filesystem support (FS) was not included. The problem is that you are using files from JS, but files were not used from C/C++, so filesystem support was not auto-included. You can force-include filesystem support with -sFORCE_FILESYSTEM")},init(){ve.error()},createDataFile(){ve.error()},createPreloadedFile(){ve.error()},createLazyFile(){ve.error()},open(){ve.error()},mkdev(){ve.error()},registerDevice(){ve.error()},analyzePath(){ve.error()},ErrnoError(){ve.error()}};function ye(r,t){return(...s)=>{S(Z,`native function \`${r}\` called before runtime initialization`);var a=ke[r];return S(a,`exported native function \`${r}\` not found`),S(s.length<=t,`native function \`${r}\` called with ${s.length} args but expects ${t}`),a(...s)}}var at;function G(){return i.locateFile?E("watlas.wasm"):new URL("/assets/watlas-2lMxVnno.wasm",import.meta.url).href}function L(r){if(P)return P(r);throw"both async and sync fetching of the wasm failed"}async function ne(r){try{var t=await b(r);return new Uint8Array(t)}catch{}return L(r)}async function ae(r,t){try{var s=await ne(r),a=await WebAssembly.instantiate(s,t);return a}catch(l){D(`failed to asynchronously prepare wasm: ${l}`),ee(at)&&D(`warning: Loading from a file URI (${at}) is not supported in most browsers. See https://emscripten.org/docs/getting_started/FAQ.html#how-do-i-run-a-local-webserver-for-testing-why-does-my-program-stall-in-downloading-or-preparing`),he(l)}}async function ue(r,t,s){if(typeof WebAssembly.instantiateStreaming=="function"&&!f)try{var a=fetch(t,{credentials:"same-origin"}),l=await WebAssembly.instantiateStreaming(a,s);return l}catch(h){D(`wasm streaming compile failed: ${h}`),D("falling back to ArrayBuffer instantiation")}return ae(t,s)}function pe(){return{env:Vr,wasi_snapshot_preview1:Vr}}async function je(){function r(m,p){return ke=m.exports,F=ke.memory,S(F,"memory not found in wasm exports"),Ke(),rr=ke.__indirect_function_table,S(rr,"table not found in wasm exports"),gt("wasm-instantiate"),ke}nt("wasm-instantiate");var t=i;function s(m){return S(i===t,"the Module object should not be replaced during async compilation - perhaps the order of HTML elements is wrong?"),t=null,r(m.instance)}var a=pe();at??=G();try{var l=await ue(Y,at,a),h=s(l);return h}catch(m){return n(m),Promise.reject(m)}}var J=r=>(S(typeof r=="number"),r>>>=0,"0x"+r.toString(16).padStart(8,"0")),te=r=>Js(r),X=()=>eo(),Ce=r=>{Ce.shown||={},Ce.shown[r]||(Ce.shown[r]=1,f&&(r="warning: "+r),D(r))},rt=typeof TextDecoder<"u"?new TextDecoder:void 0,Ie=(r,t=0,s=NaN)=>{for(var a=t+s,l=t;r[l]&&!(l>=a);)++l;if(l-t>16&&r.buffer&&rt)return rt.decode(r.subarray(t,l));for(var h="";t<l;){var m=r[t++];if(!(m&128)){h+=String.fromCharCode(m);continue}var p=r[t++]&63;if((m&224)==192){h+=String.fromCharCode((m&31)<<6|p);continue}var y=r[t++]&63;if((m&240)==224?m=(m&15)<<12|p<<6|y:((m&248)!=240&&Ce("Invalid UTF-8 leading byte "+J(m)+" encountered when deserializing a UTF-8 string in wasm memory to a JS string!"),m=(m&7)<<18|p<<12|y<<6|r[t++]&63),m<65536)h+=String.fromCharCode(m);else{var C=m-65536;h+=String.fromCharCode(55296|C>>10,56320|C&1023)}}return h},ze=(r,t)=>(S(typeof r=="number",`UTF8ToString expects a number (got ${typeof r})`),r?Ie(N,r,t):""),ut=(r,t,s,a)=>he(`Assertion failed: ${ze(r)}, at: `+[t?ze(t):"unknown filename",s,a?ze(a):"unknown function"]),de=r=>{var t=new Ee(r);return t.get_caught()||t.set_caught(!0),t.set_rethrown(!1),$r(r),so(r)},Ne=0;class Ee{constructor(t){this.excPtr=t,this.ptr=t-24}set_type(t){V[this.ptr+4>>2]=t}get_type(){return V[this.ptr+4>>2]}set_destructor(t){V[this.ptr+8>>2]=t}get_destructor(){return V[this.ptr+8>>2]}set_caught(t){t=t?1:0,O[this.ptr+12]=t}get_caught(){return O[this.ptr+12]!=0}set_rethrown(t){t=t?1:0,O[this.ptr+13]=t}get_rethrown(){return O[this.ptr+13]!=0}init(t,s){this.set_adjusted_ptr(0),this.set_type(t),this.set_destructor(s)}set_adjusted_ptr(t){V[this.ptr+16>>2]=t}get_adjusted_ptr(){return V[this.ptr+16>>2]}}var We=r=>Xs(r),Ue=r=>{var t=Ne?.excPtr;if(!t)return We(0),0;var s=new Ee(t);s.set_adjusted_ptr(t);var a=s.get_type();if(!a)return We(0),t;for(var l of r){if(l===0||l===a)break;var h=s.ptr+16;if(io(l,a,h))return We(l),t}return We(a),t},Re=()=>Ue([]),Qt=r=>Ue([r]),St=(r,t,s)=>{var a=new Ee(r);throw a.init(t,s),Ne=new Xe(r),Ne},lt=r=>{throw Ne||(Ne=new Xe(r)),Ne},It=()=>he("native code called abort()"),Rt={},Pt=r=>{for(;r.length;){var t=r.pop(),s=r.pop();s(t)}};function vt(r){return this.fromWireType(V[r>>2])}var yt={},ct={},Dt={},xi=class extends Error{constructor(t){super(t),this.name="InternalError"}},Ot=r=>{throw new xi(r)},Je=(r,t,s)=>{r.forEach(p=>Dt[p]=t);function a(p){var y=s(p);y.length!==r.length&&Ot("Mismatched type converter count");for(var C=0;C<r.length;++C)De(r[C],y[C])}var l=new Array(t.length),h=[],m=0;t.forEach((p,y)=>{ct.hasOwnProperty(p)?l[y]=ct[p]:(h.push(p),yt.hasOwnProperty(p)||(yt[p]=[]),yt[p].push(()=>{l[y]=ct[p],++m,m===h.length&&a(l)}))}),h.length===0&&a(l)},_i=r=>{var t=Rt[r];delete Rt[r];var s=t.elements,a=s.length,l=s.map(p=>p.getterReturnType).concat(s.map(p=>p.setterArgumentType)),h=t.rawConstructor,m=t.rawDestructor;Je([r],l,p=>(s.forEach((y,C)=>{var T=p[C],R=y.getter,q=y.getterContext,H=p[C+a],K=y.setter,ce=y.setterContext;y.read=fe=>T.fromWireType(R(q,fe)),y.write=(fe,_e)=>{var Le=[];K(ce,fe,H.toWireType(Le,_e)),Pt(Le)}}),[{name:t.name,fromWireType:y=>{for(var C=new Array(a),T=0;T<a;++T)C[T]=s[T].read(y);return m(y),C},toWireType:(y,C)=>{if(a!==C.length)throw new TypeError(`Incorrect number of tuple elements for ${t.name}: expected=${a}, actual=${C.length}`);for(var T=h(),R=0;R<a;++R)s[R].write(T,C[R]);return y!==null&&y.push(m,T),T},argPackAdvance:Oe,readValueFromPointer:vt,destructorFunction:m}]))},Vt={},wi=r=>{var t=Vt[r];delete Vt[r];var s=t.rawConstructor,a=t.rawDestructor,l=t.fields,h=l.map(m=>m.getterReturnType).concat(l.map(m=>m.setterArgumentType));Je([r],h,m=>{var p={};return l.forEach((y,C)=>{var T=y.fieldName,R=m[C],q=m[C].optional,H=y.getter,K=y.getterContext,ce=m[C+l.length],fe=y.setter,_e=y.setterContext;p[T]={read:Le=>R.fromWireType(H(K,Le)),write:(Le,Be)=>{var qt=[];fe(_e,Le,ce.toWireType(qt,Be)),Pt(qt)},optional:q}}),[{name:t.name,fromWireType:y=>{var C={};for(var T in p)C[T]=p[T].read(y);return a(y),C},toWireType:(y,C)=>{for(var T in p)if(!(T in C)&&!p[T].optional)throw new TypeError(`Missing field: "${T}"`);var R=s();for(T in p)p[T].write(R,C[T]);return y!==null&&y.push(a,R),R},argPackAdvance:Oe,readValueFromPointer:vt,destructorFunction:a}]})},Si=()=>{for(var r=new Array(256),t=0;t<256;++t)r[t]=String.fromCharCode(t);vr=r},vr,le=r=>{for(var t="",s=r;N[s];)t+=vr[N[s++]];return t},Ct=class extends Error{constructor(t){super(t),this.name="BindingError"}},$=r=>{throw new Ct(r)};function Pi(r,t,s={}){var a=t.name;if(r||$(`type "${a}" must have a positive integer typeid pointer`),ct.hasOwnProperty(r)){if(s.ignoreDuplicateRegistrations)return;$(`Cannot register type '${a}' twice`)}if(ct[r]=t,delete Dt[r],yt.hasOwnProperty(r)){var l=yt[r];delete yt[r],l.forEach(h=>h())}}function De(r,t,s={}){if(t.argPackAdvance===void 0)throw new TypeError("registerType registeredInstance requires argPackAdvance");return Pi(r,t,s)}var yr=(r,t,s)=>{switch(t){case 1:return s?a=>O[a]:a=>N[a];case 2:return s?a=>j[a>>1]:a=>W[a>>1];case 4:return s?a=>Q[a>>2]:a=>V[a>>2];case 8:return s?a=>ie[a>>3]:a=>se[a>>3];default:throw new TypeError(`invalid integer width (${t}): ${r}`)}},dt=r=>{if(r===null)return"null";var t=typeof r;return t==="object"||t==="array"||t==="function"?r.toString():""+r},br=(r,t,s,a)=>{if(t<s||t>a)throw new TypeError(`Passing a number "${dt(t)}" from JS side to C/C++ side to an argument of type "${r}", which is outside the valid range [${s}, ${a}]!`)},Ci=(r,t,s,a,l)=>{t=le(t);const h=a===0n;let m=p=>p;if(h){const p=s*8;m=y=>BigInt.asUintN(p,y),l=m(l)}De(r,{name:t,fromWireType:m,toWireType:(p,y)=>{if(typeof y=="number")y=BigInt(y);else if(typeof y!="bigint")throw new TypeError(`Cannot convert "${dt(y)}" to ${this.name}`);return br(t,y,a,l),y},argPackAdvance:Oe,readValueFromPointer:yr(t,s,!h),destructorFunction:null})},Oe=8,Ei=(r,t,s,a)=>{t=le(t),De(r,{name:t,fromWireType:function(l){return!!l},toWireType:function(l,h){return h?s:a},argPackAdvance:Oe,readValueFromPointer:function(l){return this.fromWireType(N[l])},destructorFunction:null})},Ti=r=>({count:r.count,deleteScheduled:r.deleteScheduled,preservePointerOnDelete:r.preservePointerOnDelete,ptr:r.ptr,ptrType:r.ptrType,smartPtr:r.smartPtr,smartPtrType:r.smartPtrType}),er=r=>{function t(s){return s.$$.ptrType.registeredClass.name}$(t(r)+" instance already deleted")},tr=!1,xr=r=>{},Bi=r=>{r.smartPtr?r.smartPtrType.rawDestructor(r.smartPtr):r.ptrType.registeredClass.rawDestructor(r.ptr)},_r=r=>{r.count.value-=1;var t=r.count.value===0;t&&Bi(r)},wr=(r,t,s)=>{if(t===s)return r;if(s.baseClass===void 0)return null;var a=wr(r,t,s.baseClass);return a===null?null:s.downcast(a)},Sr={},Fi={},Mi=(r,t)=>{for(t===void 0&&$("ptr should not be undefined");r.baseClass;)t=r.upcast(t),r=r.baseClass;return t},zi=(r,t)=>(t=Mi(r,t),Fi[t]),Lt=(r,t)=>{(!t.ptrType||!t.ptr)&&Ot("makeClassHandle requires ptr and ptrType");var s=!!t.smartPtrType,a=!!t.smartPtr;return s!==a&&Ot("Both smartPtrType and smartPtr must be specified"),t.count={value:1},Et(Object.create(r,{$$:{value:t,writable:!0}}))};function Pr(r){var t=this.getPointee(r);if(!t)return this.destructor(r),null;var s=zi(this.registeredClass,t);if(s!==void 0){if(s.$$.count.value===0)return s.$$.ptr=t,s.$$.smartPtr=r,s.clone();var a=s.clone();return this.destructor(r),a}function l(){return this.isSmartPointer?Lt(this.registeredClass.instancePrototype,{ptrType:this.pointeeType,ptr:t,smartPtrType:this,smartPtr:r}):Lt(this.registeredClass.instancePrototype,{ptrType:this,ptr:r})}var h=this.registeredClass.getActualType(t),m=Sr[h];if(!m)return l.call(this);var p;this.isConst?p=m.constPointerType:p=m.pointerType;var y=wr(t,this.registeredClass,p.registeredClass);return y===null?l.call(this):this.isSmartPointer?Lt(p.registeredClass.instancePrototype,{ptrType:p,ptr:y,smartPtrType:this,smartPtr:r}):Lt(p.registeredClass.instancePrototype,{ptrType:p,ptr:y})}var Et=r=>typeof FinalizationRegistry>"u"?(Et=t=>t,r):(tr=new FinalizationRegistry(t=>{console.warn(t.leakWarning),_r(t.$$)}),Et=t=>{var s=t.$$,a=!!s.smartPtr;if(a){var l={$$:s},h=s.ptrType.registeredClass,m=new Error(`Embind found a leaked C++ instance ${h.name} <${J(s.ptr)}>.
We'll free it automatically in this case, but this functionality is not reliable across various environments.
Make sure to invoke .delete() manually once you're done with the instance instead.
Originally allocated`);"captureStackTrace"in Error&&Error.captureStackTrace(m,Pr),l.leakWarning=m.stack.replace(/^Error: /,""),tr.register(t,l,t)}return t},xr=t=>tr.unregister(t),Et(r)),Ui=()=>{let r=Gt.prototype;Object.assign(r,{isAliasOf(s){if(!(this instanceof Gt)||!(s instanceof Gt))return!1;var a=this.$$.ptrType.registeredClass,l=this.$$.ptr;s.$$=s.$$;for(var h=s.$$.ptrType.registeredClass,m=s.$$.ptr;a.baseClass;)l=a.upcast(l),a=a.baseClass;for(;h.baseClass;)m=h.upcast(m),h=h.baseClass;return a===h&&l===m},clone(){if(this.$$.ptr||er(this),this.$$.preservePointerOnDelete)return this.$$.count.value+=1,this;var s=Et(Object.create(Object.getPrototypeOf(this),{$$:{value:Ti(this.$$)}}));return s.$$.count.value+=1,s.$$.deleteScheduled=!1,s},delete(){this.$$.ptr||er(this),this.$$.deleteScheduled&&!this.$$.preservePointerOnDelete&&$("Object already scheduled for deletion"),xr(this),_r(this.$$),this.$$.preservePointerOnDelete||(this.$$.smartPtr=void 0,this.$$.ptr=void 0)},isDeleted(){return!this.$$.ptr},deleteLater(){return this.$$.ptr||er(this),this.$$.deleteScheduled&&!this.$$.preservePointerOnDelete&&$("Object already scheduled for deletion"),this.$$.deleteScheduled=!0,this}});const t=Symbol.dispose;t&&(r[t]=r.delete)};function Gt(){}var $t=(r,t)=>Object.defineProperty(t,"name",{value:r}),Cr=(r,t,s)=>{if(r[t].overloadTable===void 0){var a=r[t];r[t]=function(...l){return r[t].overloadTable.hasOwnProperty(l.length)||$(`Function '${s}' called with an invalid number of arguments (${l.length}) - expects one of (${r[t].overloadTable})!`),r[t].overloadTable[l.length].apply(this,l)},r[t].overloadTable=[],r[t].overloadTable[a.argCount]=a}},Er=(r,t,s)=>{i.hasOwnProperty(r)?($(`Cannot register public name '${r}' twice`),Cr(i,r,r),i[r].overloadTable.hasOwnProperty(s)&&$(`Cannot register multiple overloads of a function with the same number of arguments (${s})!`),i[r].overloadTable[s]=t):(i[r]=t,i[r].argCount=s)},ki=48,Ai=57,Ii=r=>{S(typeof r=="string"),r=r.replace(/[^a-zA-Z0-9_]/g,"$");var t=r.charCodeAt(0);return t>=ki&&t<=Ai?`_${r}`:r};function Ri(r,t,s,a,l,h,m,p){this.name=r,this.constructor=t,this.instancePrototype=s,this.rawDestructor=a,this.baseClass=l,this.getActualType=h,this.upcast=m,this.downcast=p,this.pureVirtualFunctions=[]}var Nt=(r,t,s)=>{for(;t!==s;)t.upcast||$(`Expected null or instance of ${s.name}, got an instance of ${t.name}`),r=t.upcast(r),t=t.baseClass;return r};function Di(r,t){if(t===null)return this.isReference&&$(`null is not a valid ${this.name}`),0;t.$$||$(`Cannot pass "${dt(t)}" as a ${this.name}`),t.$$.ptr||$(`Cannot pass deleted object as a pointer of type ${this.name}`);var s=t.$$.ptrType.registeredClass,a=Nt(t.$$.ptr,s,this.registeredClass);return a}function Oi(r,t){var s;if(t===null)return this.isReference&&$(`null is not a valid ${this.name}`),this.isSmartPointer?(s=this.rawConstructor(),r!==null&&r.push(this.rawDestructor,s),s):0;(!t||!t.$$)&&$(`Cannot pass "${dt(t)}" as a ${this.name}`),t.$$.ptr||$(`Cannot pass deleted object as a pointer of type ${this.name}`),!this.isConst&&t.$$.ptrType.isConst&&$(`Cannot convert argument of type ${t.$$.smartPtrType?t.$$.smartPtrType.name:t.$$.ptrType.name} to parameter type ${this.name}`);var a=t.$$.ptrType.registeredClass;if(s=Nt(t.$$.ptr,a,this.registeredClass),this.isSmartPointer)switch(t.$$.smartPtr===void 0&&$("Passing raw pointer to smart pointer is illegal"),this.sharingPolicy){case 0:t.$$.smartPtrType===this?s=t.$$.smartPtr:$(`Cannot convert argument of type ${t.$$.smartPtrType?t.$$.smartPtrType.name:t.$$.ptrType.name} to parameter type ${this.name}`);break;case 1:s=t.$$.smartPtr;break;case 2:if(t.$$.smartPtrType===this)s=t.$$.smartPtr;else{var l=t.clone();s=this.rawShare(s,Ve.toHandle(()=>l.delete())),r!==null&&r.push(this.rawDestructor,s)}break;default:$("Unsupporting sharing policy")}return s}function Vi(r,t){if(t===null)return this.isReference&&$(`null is not a valid ${this.name}`),0;t.$$||$(`Cannot pass "${dt(t)}" as a ${this.name}`),t.$$.ptr||$(`Cannot pass deleted object as a pointer of type ${this.name}`),t.$$.ptrType.isConst&&$(`Cannot convert argument of type ${t.$$.ptrType.name} to parameter type ${this.name}`);var s=t.$$.ptrType.registeredClass,a=Nt(t.$$.ptr,s,this.registeredClass);return a}var Li=()=>{Object.assign(Wt.prototype,{getPointee(r){return this.rawGetPointee&&(r=this.rawGetPointee(r)),r},destructor(r){this.rawDestructor?.(r)},argPackAdvance:Oe,readValueFromPointer:vt,fromWireType:Pr})};function Wt(r,t,s,a,l,h,m,p,y,C,T){this.name=r,this.registeredClass=t,this.isReference=s,this.isConst=a,this.isSmartPointer=l,this.pointeeType=h,this.sharingPolicy=m,this.rawGetPointee=p,this.rawConstructor=y,this.rawShare=C,this.rawDestructor=T,!l&&t.baseClass===void 0?a?(this.toWireType=Di,this.destructorFunction=null):(this.toWireType=Vi,this.destructorFunction=null):this.toWireType=Oi}var Gi=(r,t,s)=>{i.hasOwnProperty(r)||Ot("Replacing nonexistent public symbol"),i[r].overloadTable!==void 0&&s!==void 0||(i[r]=t,i[r].argCount=s)},rr,be=r=>rr.get(r),xe=(r,t,s=!1)=>{S(!s,"Async bindings are only supported with JSPI."),r=le(r);function a(){var h=be(t);return h}var l=a();return typeof l!="function"&&$(`unknown function pointer with signature ${r}: ${t}`),l};class $i extends Error{}var Tr=r=>{var t=Zs(r),s=le(t);return Qe(t),s},Tt=(r,t)=>{var s=[],a={};function l(h){if(!a[h]&&!ct[h]){if(Dt[h]){Dt[h].forEach(l);return}s.push(h),a[h]=!0}}throw t.forEach(l),new $i(`${r}: `+s.map(Tr).join([", "]))},Ni=(r,t,s,a,l,h,m,p,y,C,T,R,q)=>{T=le(T),h=xe(l,h),p&&=xe(m,p),C&&=xe(y,C),q=xe(R,q);var H=Ii(T);Er(H,function(){Tt(`Cannot construct ${T} due to unbound types`,[a])}),Je([r,t,s],a?[a]:[],K=>{K=K[0];var ce,fe;a?(ce=K.registeredClass,fe=ce.instancePrototype):fe=Gt.prototype;var _e=$t(T,function(...nr){if(Object.getPrototypeOf(this)!==Le)throw new Ct(`Use 'new' to construct ${T}`);if(Be.constructor_body===void 0)throw new Ct(`${T} has no accessible constructor`);var qr=Be.constructor_body[nr.length];if(qr===void 0)throw new Ct(`Tried to invoke ctor of ${T} with invalid number of parameters (${nr.length}) - expected (${Object.keys(Be.constructor_body).toString()}) parameters instead!`);return qr.apply(this,nr)}),Le=Object.create(fe,{constructor:{value:_e}});_e.prototype=Le;var Be=new Ri(T,_e,Le,q,ce,h,p,C);Be.baseClass&&(Be.baseClass.__derivedClasses??=[],Be.baseClass.__derivedClasses.push(Be));var qt=new Wt(T,Be,!0,!1,!1),Wr=new Wt(T+"*",Be,!1,!1,!1),Yr=new Wt(T+" const*",Be,!1,!0,!1);return Sr[r]={pointerType:Wr,constPointerType:Yr},Gi(H,_e),[qt,Wr,Yr]})},Br=(r,t)=>{for(var s=[],a=0;a<r;a++)s.push(V[t+a*4>>2]);return s};function Fr(r){for(var t=1;t<r.length;++t)if(r[t]!==null&&r[t].destructorFunction===void 0)return!0;return!1}function Wi(r,t,s,a,l){if(r<t||r>s){var h=t==s?t:`${t} to ${s}`;l(`function ${a} called with ${r} arguments, expected ${h}`)}}function Yi(r,t,s,a){var l=Fr(r),h=r.length-2,m=[],p=["fn"];t&&p.push("thisWired");for(var y=0;y<h;++y)m.push(`arg${y}`),p.push(`arg${y}Wired`);m=m.join(","),p=p.join(",");var C=`return function (${m}) {
`;C+=`checkArgCount(arguments.length, minArgs, maxArgs, humanName, throwBindingError);
`,l&&(C+=`var destructors = [];
`);var T=l?"destructors":"null",R=["humanName","throwBindingError","invoker","fn","runDestructors","retType","classParam"];t&&(C+=`var thisWired = classParam['toWireType'](${T}, this);
`);for(var y=0;y<h;++y)C+=`var arg${y}Wired = argType${y}['toWireType'](${T}, arg${y});
`,R.push(`argType${y}`);if(C+=(s||a?"var rv = ":"")+`invoker(${p});
`,l)C+=`runDestructors(destructors);
`;else for(var y=t?1:2;y<r.length;++y){var q=y===1?"thisWired":"arg"+(y-2)+"Wired";r[y].destructorFunction!==null&&(C+=`${q}_dtor(${q});
`,R.push(`${q}_dtor`))}return s&&(C+=`var ret = retType['fromWireType'](rv);
return ret;
`),C+=`}
`,R.push("checkArgCount","minArgs","maxArgs"),C=`if (arguments.length !== ${R.length}){ throw new Error(humanName + "Expected ${R.length} closure arguments " + arguments.length + " given."); }
${C}`,[R,C]}function qi(r){for(var t=r.length-2,s=r.length-1;s>=2&&r[s].optional;--s)t--;return t}function Mr(r,t,s,a,l,h){var m=t.length;m<2&&$("argTypes array size mismatch! Must at least get return value and 'this' types!"),S(!h,"Async bindings are only supported with JSPI.");for(var p=t[1]!==null&&s!==null,y=Fr(t),C=t[0].name!=="void",T=m-2,R=qi(t),q=[r,$,a,l,Pt,t[0],t[1]],H=0;H<m-2;++H)q.push(t[H+2]);if(!y)for(var H=p?1:2;H<t.length;++H)t[H].destructorFunction!==null&&q.push(t[H].destructorFunction);q.push(Wi,R,T);let[K,ce]=Yi(t,p,C,h);var fe=new Function(...K,ce)(...q);return $t(r,fe)}var Hi=(r,t,s,a,l,h)=>{S(t>0);var m=Br(t,s);l=xe(a,l),Je([],[r],p=>{p=p[0];var y=`constructor ${p.name}`;if(p.registeredClass.constructor_body===void 0&&(p.registeredClass.constructor_body=[]),p.registeredClass.constructor_body[t-1]!==void 0)throw new Ct(`Cannot register multiple constructors with identical number of parameters (${t-1}) for class '${p.name}'! Overload resolution is currently only performed using the parameter count, not actual type info!`);return p.registeredClass.constructor_body[t-1]=()=>{Tt(`Cannot construct ${p.name} due to unbound types`,m)},Je([],m,C=>(C.splice(1,0,null),p.registeredClass.constructor_body[t-1]=Mr(y,C,null,l,h),[])),[]})},ji=r=>{r=r.trim();const t=r.indexOf("(");return t===-1?r:(S(r.endsWith(")"),"Parentheses for argument names should match."),r.slice(0,t))},Zi=(r,t,s,a,l,h,m,p,y,C)=>{var T=Br(s,a);t=le(t),t=ji(t),h=xe(l,h,y),Je([],[r],R=>{R=R[0];var q=`${R.name}.${t}`;t.startsWith("@@")&&(t=Symbol[t.substring(2)]),p&&R.registeredClass.pureVirtualFunctions.push(t);function H(){Tt(`Cannot call ${q} due to unbound types`,T)}var K=R.registeredClass.instancePrototype,ce=K[t];return ce===void 0||ce.overloadTable===void 0&&ce.className!==R.name&&ce.argCount===s-2?(H.argCount=s-2,H.className=R.name,K[t]=H):(Cr(K,t,q),K[t].overloadTable[s-2]=H),Je([],T,fe=>{var _e=Mr(q,fe,R,h,m,y);return K[t].overloadTable===void 0?(_e.argCount=s-2,K[t]=_e):K[t].overloadTable[s-2]=_e,[]}),[]})},zr=(r,t,s)=>(r instanceof Object||$(`${s} with invalid "this": ${r}`),r instanceof t.registeredClass.constructor||$(`${s} incompatible with "this" of type ${r.constructor.name}`),r.$$.ptr||$(`cannot call emscripten binding method ${s} on deleted object`),Nt(r.$$.ptr,r.$$.ptrType.registeredClass,t.registeredClass)),Xi=(r,t,s,a,l,h,m,p,y,C)=>{t=le(t),l=xe(a,l),Je([],[r],T=>{T=T[0];var R=`${T.name}.${t}`,q={get(){Tt(`Cannot access ${R} due to unbound types`,[s,m])},enumerable:!0,configurable:!0};return y?q.set=()=>Tt(`Cannot access ${R} due to unbound types`,[s,m]):q.set=H=>$(R+" is a read-only property"),Object.defineProperty(T.registeredClass.instancePrototype,t,q),Je([],y?[s,m]:[s],H=>{var K=H[0],ce={get(){var _e=zr(this,T,R+" getter");return K.fromWireType(l(h,_e))},enumerable:!0};if(y){y=xe(p,y);var fe=H[1];ce.set=function(_e){var Le=zr(this,T,R+" setter"),Be=[];y(C,Le,fe.toWireType(Be,_e)),Pt(Be)}}return Object.defineProperty(T.registeredClass.instancePrototype,t,ce),[]}),[]})},Ur=[],Ze=[0,1,,1,null,1,!0,1,!1,1],ir=r=>{r>9&&--Ze[r+1]===0&&(S(Ze[r]!==void 0,"Decref for unallocated handle."),Ze[r]=void 0,Ur.push(r))},Ve={toValue:r=>(r||$(`Cannot use deleted val. handle = ${r}`),S(r===2||Ze[r]!==void 0&&r%2===0,`invalid handle: ${r}`),Ze[r]),toHandle:r=>{switch(r){case void 0:return 2;case null:return 4;case!0:return 6;case!1:return 8;default:{const t=Ur.pop()||Ze.length;return Ze[t]=r,Ze[t+1]=1,t}}}},kr={name:"emscripten::val",fromWireType:r=>{var t=Ve.toValue(r);return ir(r),t},toWireType:(r,t)=>Ve.toHandle(t),argPackAdvance:Oe,readValueFromPointer:vt,destructorFunction:null},Ki=r=>De(r,kr),Ji=(r,t,s)=>{switch(t){case 1:return s?function(a){return this.fromWireType(O[a])}:function(a){return this.fromWireType(N[a])};case 2:return s?function(a){return this.fromWireType(j[a>>1])}:function(a){return this.fromWireType(W[a>>1])};case 4:return s?function(a){return this.fromWireType(Q[a>>2])}:function(a){return this.fromWireType(V[a>>2])};default:throw new TypeError(`invalid integer width (${t}): ${r}`)}},Qi=(r,t,s,a)=>{t=le(t);function l(){}l.values={},De(r,{name:t,constructor:l,fromWireType:function(h){return this.constructor.values[h]},toWireType:(h,m)=>m.value,argPackAdvance:Oe,readValueFromPointer:Ji(t,s,a),destructorFunction:null}),Er(t,l)},Yt=(r,t)=>{var s=ct[r];return s===void 0&&$(`${t} has unknown type ${Tr(r)}`),s},es=(r,t,s)=>{var a=Yt(r,"enum");t=le(t);var l=a.constructor,h=Object.create(a.constructor.prototype,{value:{value:s},constructor:{value:$t(`${a.name}_${t}`,function(){})}});l.values[s]=h,l[t]=h},ts=(r,t)=>{switch(t){case 4:return function(s){return this.fromWireType(Se[s>>2])};case 8:return function(s){return this.fromWireType(oe[s>>3])};default:throw new TypeError(`invalid float width (${t}): ${r}`)}},rs=(r,t,s)=>{t=le(t),De(r,{name:t,fromWireType:a=>a,toWireType:(a,l)=>{if(typeof l!="number"&&typeof l!="boolean")throw new TypeError(`Cannot convert ${dt(l)} to ${this.name}`);return l},argPackAdvance:Oe,readValueFromPointer:ts(t,s),destructorFunction:null})},is=(r,t,s,a,l)=>{t=le(t);const h=a===0;let m=y=>y;if(h){var p=32-8*s;m=y=>y<<p>>>p,l=m(l)}De(r,{name:t,fromWireType:m,toWireType:(y,C)=>{if(typeof C!="number"&&typeof C!="boolean")throw new TypeError(`Cannot convert "${dt(C)}" to ${t}`);return br(t,C,a,l),C},argPackAdvance:Oe,readValueFromPointer:yr(t,s,a!==0),destructorFunction:null})},ss=(r,t,s)=>{var a=[Int8Array,Uint8Array,Int16Array,Uint16Array,Int32Array,Uint32Array,Float32Array,Float64Array,BigInt64Array,BigUint64Array],l=a[t];function h(m){var p=V[m>>2],y=V[m+4>>2];return new l(O.buffer,y,p)}s=le(s),De(r,{name:s,fromWireType:h,argPackAdvance:Oe,readValueFromPointer:h},{ignoreDuplicateRegistrations:!0})},os=Object.assign({optional:!0},kr),ns=(r,t)=>{De(r,os)},as=(r,t,s,a)=>{if(S(typeof r=="string",`stringToUTF8Array expects a string (got ${typeof r})`),!(a>0))return 0;for(var l=s,h=s+a-1,m=0;m<r.length;++m){var p=r.charCodeAt(m);if(p>=55296&&p<=57343){var y=r.charCodeAt(++m);p=65536+((p&1023)<<10)|y&1023}if(p<=127){if(s>=h)break;t[s++]=p}else if(p<=2047){if(s+1>=h)break;t[s++]=192|p>>6,t[s++]=128|p&63}else if(p<=65535){if(s+2>=h)break;t[s++]=224|p>>12,t[s++]=128|p>>6&63,t[s++]=128|p&63}else{if(s+3>=h)break;p>1114111&&Ce("Invalid Unicode code point "+J(p)+" encountered when serializing a JS string to a UTF-8 string in wasm memory! (Valid unicode code points should be in range 0-0x10FFFF)."),t[s++]=240|p>>18,t[s++]=128|p>>12&63,t[s++]=128|p>>6&63,t[s++]=128|p&63}}return t[s]=0,s-l},us=(r,t,s)=>(S(typeof s=="number","stringToUTF8(str, outPtr, maxBytesToWrite) is missing the third parameter that specifies the length of the output buffer!"),as(r,N,t,s)),ls=r=>{for(var t=0,s=0;s<r.length;++s){var a=r.charCodeAt(s);a<=127?t++:a<=2047?t+=2:a>=55296&&a<=57343?(t+=4,++s):t+=3}return t},cs=(r,t)=>{t=le(t),De(r,{name:t,fromWireType(s){for(var a=V[s>>2],l=s+4,h,m,p=l,m=0;m<=a;++m){var y=l+m;if(m==a||N[y]==0){var C=y-p,T=ze(p,C);h===void 0?h=T:(h+="\0",h+=T),p=y+1}}return Qe(s),h},toWireType(s,a){a instanceof ArrayBuffer&&(a=new Uint8Array(a));var l,h=typeof a=="string";h||ArrayBuffer.isView(a)&&a.BYTES_PER_ELEMENT==1||$("Cannot pass non-string to std::string"),h?l=ls(a):l=a.length;var m=Lr(4+l+1),p=m+4;return V[m>>2]=l,h?us(a,p,l+1):N.set(a,p),s!==null&&s.push(Qe,m),m},argPackAdvance:Oe,readValueFromPointer:vt,destructorFunction(s){Qe(s)}})},Ar=typeof TextDecoder<"u"?new TextDecoder("utf-16le"):void 0,ds=(r,t)=>{S(r%2==0,"Pointer passed to UTF16ToString must be aligned to two bytes!");for(var s=r>>1,a=s+t/2,l=s;!(l>=a)&&W[l];)++l;if(l-s>16&&Ar)return Ar.decode(W.subarray(s,l));for(var h="",m=s;!(m>=a);++m){var p=W[m];if(p==0)break;h+=String.fromCharCode(p)}return h},fs=(r,t,s)=>{if(S(t%2==0,"Pointer passed to stringToUTF16 must be aligned to two bytes!"),S(typeof s=="number","stringToUTF16(str, outPtr, maxBytesToWrite) is missing the third parameter that specifies the length of the output buffer!"),s??=2147483647,s<2)return 0;s-=2;for(var a=t,l=s<r.length*2?s/2:r.length,h=0;h<l;++h){var m=r.charCodeAt(h);j[t>>1]=m,t+=2}return j[t>>1]=0,t-a},hs=r=>r.length*2,ps=(r,t)=>{S(r%4==0,"Pointer passed to UTF32ToString must be aligned to four bytes!");for(var s=0,a="";!(s>=t/4);){var l=Q[r+s*4>>2];if(l==0)break;if(++s,l>=65536){var h=l-65536;a+=String.fromCharCode(55296|h>>10,56320|h&1023)}else a+=String.fromCharCode(l)}return a},ms=(r,t,s)=>{if(S(t%4==0,"Pointer passed to stringToUTF32 must be aligned to four bytes!"),S(typeof s=="number","stringToUTF32(str, outPtr, maxBytesToWrite) is missing the third parameter that specifies the length of the output buffer!"),s??=2147483647,s<4)return 0;for(var a=t,l=a+s-4,h=0;h<r.length;++h){var m=r.charCodeAt(h);if(m>=55296&&m<=57343){var p=r.charCodeAt(++h);m=65536+((m&1023)<<10)|p&1023}if(Q[t>>2]=m,t+=4,t+4>l)break}return Q[t>>2]=0,t-a},gs=r=>{for(var t=0,s=0;s<r.length;++s){var a=r.charCodeAt(s);a>=55296&&a<=57343&&++s,t+=4}return t},vs=(r,t,s)=>{s=le(s);var a,l,h,m;t===2?(a=ds,l=fs,m=hs,h=p=>W[p>>1]):t===4&&(a=ps,l=ms,m=gs,h=p=>V[p>>2]),De(r,{name:s,fromWireType:p=>{for(var y=V[p>>2],C,T=p+4,R=0;R<=y;++R){var q=p+4+R*t;if(R==y||h(q)==0){var H=q-T,K=a(T,H);C===void 0?C=K:(C+="\0",C+=K),T=q+t}}return Qe(p),C},toWireType:(p,y)=>{typeof y!="string"&&$(`Cannot pass non-string to C++ string type ${s}`);var C=m(y),T=Lr(4+C+t);return V[T>>2]=C/t,l(y,T+4,C+t),p!==null&&p.push(Qe,T),T},argPackAdvance:Oe,readValueFromPointer:vt,destructorFunction(p){Qe(p)}})},ys=(r,t,s,a,l,h)=>{Rt[r]={name:le(t),rawConstructor:xe(s,a),rawDestructor:xe(l,h),elements:[]}},bs=(r,t,s,a,l,h,m,p,y)=>{Rt[r].elements.push({getterReturnType:t,getter:xe(s,a),getterContext:l,setterArgumentType:h,setter:xe(m,p),setterContext:y})},xs=(r,t,s,a,l,h)=>{Vt[r]={name:le(t),rawConstructor:xe(s,a),rawDestructor:xe(l,h),fields:[]}},_s=(r,t,s,a,l,h,m,p,y,C)=>{Vt[r].fields.push({fieldName:le(t),getterReturnType:s,getter:xe(a,l),getterContext:h,setterArgumentType:m,setter:xe(p,y),setterContext:C})},ws=(r,t)=>{t=le(t),De(r,{isVoid:!0,name:t,argPackAdvance:0,fromWireType:()=>{},toWireType:(s,a)=>{}})},Ir=(r,t,s)=>{var a=[],l=r.toWireType(a,s);return a.length&&(V[t>>2]=Ve.toHandle(a)),l},Ss=(r,t,s)=>(r=Ve.toValue(r),t=Yt(t,"emval::as"),Ir(t,s,r)),Ps={},Rr=r=>{var t=Ps[r];return t===void 0?le(r):t},sr=[],Cs=(r,t,s,a,l)=>(r=sr[r],t=Ve.toValue(t),s=Rr(s),r(t,t[s],a,l)),Es=r=>{var t=sr.length;return sr.push(r),t},Ts=(r,t)=>{for(var s=new Array(r),a=0;a<r;++a)s[a]=Yt(V[t+a*4>>2],`parameter ${a}`);return s},Bs=(r,t,s)=>{var a=Ts(r,t),l=a.shift();r--;var h=`return function (obj, func, destructorsRef, args) {
`,m=0,p=[];s===0&&p.push("obj");for(var y=["retType"],C=[l],T=0;T<r;++T)p.push(`arg${T}`),y.push(`argType${T}`),C.push(a[T]),h+=`  var arg${T} = argType${T}.readValueFromPointer(args${m?"+"+m:""});
`,m+=a[T].argPackAdvance;var R=s===1?"new func":"func.call";h+=`  var rv = ${R}(${p.join(", ")});
`,l.isVoid||(y.push("emval_returnValue"),C.push(Ir),h+=`  return emval_returnValue(retType, destructorsRef, rv);
`),h+=`};
`;var q=new Function(...y,h)(...C),H=`methodCaller<(${a.map(K=>K.name).join(", ")}) => ${l.name}>`;return Es($t(H,q))},Fs=(r,t)=>(r=Ve.toValue(r),t=Ve.toValue(t),Ve.toHandle(r[t])),Ms=r=>{r>9&&(Ze[r+1]+=1)},zs=r=>Ve.toHandle(Rr(r)),Us=r=>{var t=Ve.toValue(r);Pt(t),ir(r)},ks=(r,t)=>{r=Yt(r,"_emval_take_value");var s=r.readValueFromPointer(t);return Ve.toHandle(s)},As=()=>2147483648,Is=(r,t)=>(S(t,"alignment argument is required"),Math.ceil(r/t)*t),Rs=r=>{var t=F.buffer,s=(r-t.byteLength+65535)/65536|0;try{return F.grow(s),Ke(),1}catch(a){D(`growMemory: Attempted to grow heap from ${t.byteLength} bytes to ${r} bytes, but got error: ${a}`)}},Ds=r=>{var t=N.length;r>>>=0,S(r>t);var s=As();if(r>s)return D(`Cannot enlarge memory, requested ${r} bytes, but the limit is ${s} bytes!`),!1;for(var a=1;a<=4;a*=2){var l=t*(1+.2/a);l=Math.min(l,r+100663296);var h=Math.min(s,Is(Math.max(r,l),65536)),m=Rs(h);if(m)return!0}return D(`Failed to grow the heap from ${t} bytes to ${h} bytes, not enough memory!`),!1},Os=r=>{he("fd_close called without SYSCALLS_REQUIRE_FILESYSTEM")};function Vs(r,t,s,a){return 70}var Ls=[null,[],[]],Gs=(r,t)=>{var s=Ls[r];S(s),t===0||t===10?((r===1?M:D)(Ie(s)),s.length=0):s.push(t)},$s=(r,t,s,a)=>{for(var l=0,h=0;h<s;h++){var m=V[t>>2],p=V[t+4>>2];t+=8;for(var y=0;y<p;y++)Gs(r,N[m+y]);l+=p}return V[a>>2]=l,0},Ns=r=>$r(r),Ws=r=>to(r),Dr=r=>Qs(r),Ys=r=>{var t=X(),s=Dr(4),a=Dr(4);ro(r,s,a);var l=V[s>>2],h=V[a>>2],m=ze(l);Qe(l);var p;return h&&(p=ze(h),Qe(h)),te(t),[m,p]},Or=r=>Ys(r);Si(),Ui(),Li(),S(Ze.length===10),i.FS_createDataFile=ve.createDataFile,i.FS_createPreloadedFile=ve.createPreloadedFile,js(),S(typeof i.memoryInitializerPrefixURL>"u","Module.memoryInitializerPrefixURL option was removed, use Module.locateFile instead"),S(typeof i.pthreadMainPrefixURL>"u","Module.pthreadMainPrefixURL option was removed, use Module.locateFile instead"),S(typeof i.cdInitializerPrefixURL>"u","Module.cdInitializerPrefixURL option was removed, use Module.locateFile instead"),S(typeof i.filePackagePrefixURL>"u","Module.filePackagePrefixURL option was removed, use Module.locateFile instead"),S(typeof i.read>"u","Module.read option was removed"),S(typeof i.readAsync>"u","Module.readAsync option was removed (modify readAsync in JS)"),S(typeof i.readBinary>"u","Module.readBinary option was removed (modify readBinary in JS)"),S(typeof i.setWindowTitle>"u","Module.setWindowTitle option was removed (modify emscripten_set_window_title in JS)"),S(typeof i.TOTAL_MEMORY>"u","Module.TOTAL_MEMORY has been renamed Module.INITIAL_MEMORY"),S(typeof i.ENVIRONMENT>"u","Module.ENVIRONMENT has been deprecated. To force the environment, use the ENVIRONMENT compile-time option (for example, -sENVIRONMENT=web or -sENVIRONMENT=node)"),S(typeof i.STACK_SIZE>"u","STACK_SIZE can no longer be set at runtime.  Use -sSTACK_SIZE at link time"),S(typeof i.wasmMemory>"u","Use of `wasmMemory` detected.  Use -sIMPORTED_MEMORY to define wasmMemory externally"),S(typeof i.INITIAL_MEMORY>"u","Detected runtime INITIAL_MEMORY setting.  Use -sIMPORTED_MEMORY to define wasmMemory dynamically");var qs=["writeI53ToI64","writeI53ToI64Clamped","writeI53ToI64Signaling","writeI53ToU64Clamped","writeI53ToU64Signaling","readI53FromI64","readI53FromU64","convertI32PairToI53","convertI32PairToI53Checked","convertU32PairToI53","getTempRet0","zeroMemory","exitJS","strError","inetPton4","inetNtop4","inetPton6","inetNtop6","readSockaddr","writeSockaddr","emscriptenLog","readEmAsmArgs","jstoi_q","getExecutableName","listenOnce","autoResumeAudioContext","getDynCaller","dynCall","setWasmTableEntry","handleException","keepRuntimeAlive","runtimeKeepalivePush","runtimeKeepalivePop","callUserCallback","maybeExit","asmjsMangle","asyncLoad","mmapAlloc","HandleAllocator","getNativeTypeSize","addOnPreRun","addOnInit","addOnPostCtor","addOnPreMain","addOnExit","addOnPostRun","STACK_SIZE","STACK_ALIGN","POINTER_SIZE","ASSERTIONS","ccall","cwrap","uleb128Encode","sigToWasmTypes","generateFuncType","convertJsFunctionToWasm","getEmptyTableSlot","updateTableMap","getFunctionAddress","addFunction","removeFunction","reallyNegative","unSign","strLen","reSign","formatString","intArrayFromString","intArrayToString","AsciiToString","stringToAscii","stringToNewUTF8","stringToUTF8OnStack","writeArrayToMemory","registerKeyEventCallback","maybeCStringToJsString","findEventTarget","getBoundingClientRect","fillMouseEventData","registerMouseEventCallback","registerWheelEventCallback","registerUiEventCallback","registerFocusEventCallback","fillDeviceOrientationEventData","registerDeviceOrientationEventCallback","fillDeviceMotionEventData","registerDeviceMotionEventCallback","screenOrientation","fillOrientationChangeEventData","registerOrientationChangeEventCallback","fillFullscreenChangeEventData","registerFullscreenChangeEventCallback","JSEvents_requestFullscreen","JSEvents_resizeCanvasForFullscreen","registerRestoreOldStyle","hideEverythingExceptGivenElement","restoreHiddenElements","setLetterbox","softFullscreenResizeWebGLRenderTarget","doRequestFullscreen","fillPointerlockChangeEventData","registerPointerlockChangeEventCallback","registerPointerlockErrorEventCallback","requestPointerLock","fillVisibilityChangeEventData","registerVisibilityChangeEventCallback","registerTouchEventCallback","fillGamepadEventData","registerGamepadEventCallback","registerBeforeUnloadEventCallback","fillBatteryEventData","battery","registerBatteryEventCallback","setCanvasElementSize","getCanvasElementSize","jsStackTrace","getCallstack","convertPCtoSourceLocation","getEnvStrings","checkWasiClock","wasiRightsToMuslOFlags","wasiOFlagsToMuslOFlags","initRandomFill","randomFill","safeSetTimeout","setImmediateWrapped","safeRequestAnimationFrame","clearImmediateWrapped","registerPostMainLoop","registerPreMainLoop","getPromise","makePromise","idsToPromises","makePromiseCallback","Browser_asyncPrepareDataCounter","isLeapYear","ydayFromDate","arraySum","addDays","getSocketFromFD","getSocketAddress","FS_createPreloadedFile","FS_modeStringToFlags","FS_getMode","FS_stdin_getChar","FS_mkdirTree","_setNetworkCallback","getFunctionArgsName","createJsInvokerSignature","PureVirtualError","registerInheritedInstance","unregisterInheritedInstance","getInheritedInstanceCount","getLiveInheritedInstances","setDelayFunction","count_emval_handles","emval_get_global"];qs.forEach(wt);var Hs=["run","addRunDependency","removeRunDependency","out","err","callMain","abort","wasmMemory","wasmExports","HEAPF32","HEAPF64","HEAP8","HEAPU8","HEAP16","HEAPU16","HEAP32","HEAPU32","HEAP64","HEAPU64","writeStackCookie","checkStackCookie","INT53_MAX","INT53_MIN","bigintToI53Checked","stackSave","stackRestore","stackAlloc","setTempRet0","ptrToString","getHeapMax","growMemory","ENV","ERRNO_CODES","DNS","Protocols","Sockets","timers","warnOnce","readEmAsmArgsArray","getWasmTableEntry","alignMemory","wasmTable","noExitRuntime","freeTableIndexes","functionsInTableMap","setValue","getValue","PATH","PATH_FS","UTF8Decoder","UTF8ArrayToString","UTF8ToString","stringToUTF8Array","stringToUTF8","lengthBytesUTF8","UTF16Decoder","UTF16ToString","stringToUTF16","lengthBytesUTF16","UTF32ToString","stringToUTF32","lengthBytesUTF32","JSEvents","specialHTMLTargets","findCanvasEventTarget","currentFullscreenStrategy","restoreOldWindowedStyle","UNWIND_CACHE","ExitStatus","flush_NO_FILESYSTEM","emSetImmediate","emClearImmediate_deps","emClearImmediate","promiseMap","uncaughtExceptionCount","exceptionLast","exceptionCaught","ExceptionInfo","findMatchingCatch","getExceptionMessageCommon","Browser","requestFullscreen","requestFullScreen","setCanvasSize","getUserMedia","createContext","getPreloadedImageData__data","wget","MONTH_DAYS_REGULAR","MONTH_DAYS_LEAP","MONTH_DAYS_REGULAR_CUMULATIVE","MONTH_DAYS_LEAP_CUMULATIVE","SYSCALLS","preloadPlugins","FS_stdin_getChar_buffer","FS_unlink","FS_createPath","FS_createDevice","FS_readFile","FS","FS_root","FS_mounts","FS_devices","FS_streams","FS_nextInode","FS_nameTable","FS_currentPath","FS_initialized","FS_ignorePermissions","FS_filesystems","FS_syncFSRequests","FS_lookupPath","FS_getPath","FS_hashName","FS_hashAddNode","FS_hashRemoveNode","FS_lookupNode","FS_createNode","FS_destroyNode","FS_isRoot","FS_isMountpoint","FS_isFile","FS_isDir","FS_isLink","FS_isChrdev","FS_isBlkdev","FS_isFIFO","FS_isSocket","FS_flagsToPermissionString","FS_nodePermissions","FS_mayLookup","FS_mayCreate","FS_mayDelete","FS_mayOpen","FS_checkOpExists","FS_nextfd","FS_getStreamChecked","FS_getStream","FS_createStream","FS_closeStream","FS_dupStream","FS_doSetAttr","FS_chrdev_stream_ops","FS_major","FS_minor","FS_makedev","FS_registerDevice","FS_getDevice","FS_getMounts","FS_syncfs","FS_mount","FS_unmount","FS_lookup","FS_mknod","FS_statfs","FS_statfsStream","FS_statfsNode","FS_create","FS_mkdir","FS_mkdev","FS_symlink","FS_rename","FS_rmdir","FS_readdir","FS_readlink","FS_stat","FS_fstat","FS_lstat","FS_doChmod","FS_chmod","FS_lchmod","FS_fchmod","FS_doChown","FS_chown","FS_lchown","FS_fchown","FS_doTruncate","FS_truncate","FS_ftruncate","FS_utime","FS_open","FS_close","FS_isClosed","FS_llseek","FS_read","FS_write","FS_mmap","FS_msync","FS_ioctl","FS_writeFile","FS_cwd","FS_chdir","FS_createDefaultDirectories","FS_createDefaultDevices","FS_createSpecialDirectories","FS_createStandardStreams","FS_staticInit","FS_init","FS_quit","FS_findObject","FS_analyzePath","FS_createFile","FS_createDataFile","FS_forceLoadFile","FS_createLazyFile","FS_absolutePath","FS_createFolder","FS_createLink","FS_joinPath","FS_mmapAlloc","FS_standardizePath","MEMFS","TTY","PIPEFS","SOCKFS","InternalError","BindingError","throwInternalError","throwBindingError","registeredTypes","awaitingDependencies","typeDependencies","tupleRegistrations","structRegistrations","sharedRegisterType","whenDependentTypesAreResolved","embind_charCodes","embind_init_charCodes","readLatin1String","getTypeName","getFunctionName","heap32VectorToArray","requireRegisteredType","usesDestructorStack","checkArgCount","getRequiredArgCount","createJsInvoker","UnboundTypeError","GenericWireTypeSize","EmValType","EmValOptionalType","throwUnboundTypeError","ensureOverloadTable","exposePublicSymbol","replacePublicSymbol","createNamedFunction","embindRepr","registeredInstances","getBasestPointer","getInheritedInstance","registeredPointers","registerType","integerReadValueFromPointer","enumReadValueFromPointer","floatReadValueFromPointer","assertIntegerRange","readPointer","runDestructors","craftInvokerFunction","embind__requireFunction","genericPointerToWireType","constNoSmartPtrRawPointerToWireType","nonConstNoSmartPtrRawPointerToWireType","init_RegisteredPointer","RegisteredPointer","RegisteredPointer_fromWireType","runDestructor","releaseClassHandle","finalizationRegistry","detachFinalizer_deps","detachFinalizer","attachFinalizer","makeClassHandle","init_ClassHandle","ClassHandle","throwInstanceAlreadyDeleted","deletionQueue","flushPendingDeletes","delayFunction","RegisteredClass","shallowCopyInternalPointer","downcastPointer","upcastPointer","validateThis","char_0","char_9","makeLegalFunctionName","emval_freelist","emval_handles","emval_symbols","getStringOrSymbol","Emval","emval_returnValue","emval_lookupTypes","emval_methodCallers","emval_addMethodCaller","reflectConstruct"];Hs.forEach(Pe),i.incrementExceptionRefcount=Ns,i.decrementExceptionRefcount=Ws,i.getExceptionMessage=Or;function js(){U("ENVIRONMENT"),U("GL_MAX_TEXTURE_IMAGE_UNITS"),U("SDL_canPlayWithWebAudio"),U("SDL_numSimultaneouslyQueuedBuffers"),U("INITIAL_MEMORY"),U("wasmMemory"),U("arguments"),U("buffer"),U("canvas"),U("doNotCaptureKeyboard"),U("dynamicLibraries"),U("elementPointerLock"),U("extraStackTrace"),U("forcedAspectRatio"),U("instantiateWasm"),U("keyboardListeningElement"),U("freePreloadedMediaOnUse"),U("loadSplitModule"),U("locateFile"),U("logReadFiles"),U("mainScriptUrlOrBlob"),U("mem"),U("monitorRunDependencies"),U("noExitRuntime"),U("noInitialRun"),U("onAbort"),U("onCustomMessage"),U("onExit"),U("onFree"),U("onFullScreen"),U("onMalloc"),U("onRealloc"),U("onRuntimeInitialized"),U("postMainLoop"),U("postRun"),U("preInit"),U("preMainLoop"),U("preRun"),U("preinitializedWebGLContext"),U("preloadPlugins"),U("print"),U("printErr"),U("setStatus"),U("statusMessage"),U("stderr"),U("stdin"),U("stdout"),U("thisProgram"),U("wasm"),U("wasmBinary"),U("websocket"),U("fetchSettings")}var Vr={__assert_fail:ut,__cxa_begin_catch:de,__cxa_find_matching_catch_2:Re,__cxa_find_matching_catch_3:Qt,__cxa_throw:St,__resumeException:lt,_abort_js:It,_embind_finalize_value_array:_i,_embind_finalize_value_object:wi,_embind_register_bigint:Ci,_embind_register_bool:Ei,_embind_register_class:Ni,_embind_register_class_constructor:Hi,_embind_register_class_function:Zi,_embind_register_class_property:Xi,_embind_register_emval:Ki,_embind_register_enum:Qi,_embind_register_enum_value:es,_embind_register_float:rs,_embind_register_integer:is,_embind_register_memory_view:ss,_embind_register_optional:ns,_embind_register_std_string:cs,_embind_register_std_wstring:vs,_embind_register_value_array:ys,_embind_register_value_array_element:bs,_embind_register_value_object:xs,_embind_register_value_object_field:_s,_embind_register_void:ws,_emval_as:Ss,_emval_call_method:Cs,_emval_decref:ir,_emval_get_method_caller:Bs,_emval_get_property:Fs,_emval_incref:Ms,_emval_new_cstring:zs,_emval_run_destructors:Us,_emval_take_value:ks,emscripten_resize_heap:Ds,fd_close:Os,fd_seek:Vs,fd_write:$s,invoke_fi:ho,invoke_i:po,invoke_ii:lo,invoke_iifiiii:bo,invoke_iii:ao,invoke_iiii:co,invoke_iiiii:go,invoke_iiiiiiiiiiiiiii:yo,invoke_v:xo,invoke_vi:oo,invoke_vii:no,invoke_viii:uo,invoke_viiii:mo,invoke_viiiiii:vo,invoke_viiiiiiiii:fo},ke=await je(),Zs=ye("__getTypeName",1),Qe=ye("free",1),Lr=ye("malloc",1),Te=ye("setThrew",2),Xs=ye("_emscripten_tempret_set",1),Ks=ke.emscripten_stack_init;ke.emscripten_stack_get_free,ke.emscripten_stack_get_base;var Gr=ke.emscripten_stack_get_end,Js=ke._emscripten_stack_restore,Qs=ke._emscripten_stack_alloc,eo=ke.emscripten_stack_get_current,$r=ye("__cxa_increment_exception_refcount",1),to=ye("__cxa_decrement_exception_refcount",1),ro=ye("__get_exception_message",3),io=ye("__cxa_can_catch",3),so=ye("__cxa_get_exception_ptr",1);function oo(r,t){var s=X();try{be(r)(t)}catch(a){if(te(s),!(a instanceof re))throw a;Te(1,0)}}function no(r,t,s){var a=X();try{be(r)(t,s)}catch(l){if(te(a),!(l instanceof re))throw l;Te(1,0)}}function ao(r,t,s){var a=X();try{return be(r)(t,s)}catch(l){if(te(a),!(l instanceof re))throw l;Te(1,0)}}function uo(r,t,s,a){var l=X();try{be(r)(t,s,a)}catch(h){if(te(l),!(h instanceof re))throw h;Te(1,0)}}function lo(r,t){var s=X();try{return be(r)(t)}catch(a){if(te(s),!(a instanceof re))throw a;Te(1,0)}}function co(r,t,s,a){var l=X();try{return be(r)(t,s,a)}catch(h){if(te(l),!(h instanceof re))throw h;Te(1,0)}}function fo(r,t,s,a,l,h,m,p,y,C){var T=X();try{be(r)(t,s,a,l,h,m,p,y,C)}catch(R){if(te(T),!(R instanceof re))throw R;Te(1,0)}}function ho(r,t){var s=X();try{return be(r)(t)}catch(a){if(te(s),!(a instanceof re))throw a;Te(1,0)}}function po(r){var t=X();try{return be(r)()}catch(s){if(te(t),!(s instanceof re))throw s;Te(1,0)}}function mo(r,t,s,a,l){var h=X();try{be(r)(t,s,a,l)}catch(m){if(te(h),!(m instanceof re))throw m;Te(1,0)}}function go(r,t,s,a,l){var h=X();try{return be(r)(t,s,a,l)}catch(m){if(te(h),!(m instanceof re))throw m;Te(1,0)}}function vo(r,t,s,a,l,h,m){var p=X();try{be(r)(t,s,a,l,h,m)}catch(y){if(te(p),!(y instanceof re))throw y;Te(1,0)}}function yo(r,t,s,a,l,h,m,p,y,C,T,R,q,H,K){var ce=X();try{return be(r)(t,s,a,l,h,m,p,y,C,T,R,q,H,K)}catch(fe){if(te(ce),!(fe instanceof re))throw fe;Te(1,0)}}function bo(r,t,s,a,l,h,m){var p=X();try{return be(r)(t,s,a,l,h,m)}catch(y){if(te(p),!(y instanceof re))throw y;Te(1,0)}}function xo(r){var t=X();try{be(r)()}catch(s){if(te(t),!(s instanceof re))throw s;Te(1,0)}}var Nr;function _o(){Ks(),He()}function or(){if(et>0){tt=or;return}if(_o(),et>0){tt=or;return}function r(){S(!Nr),Nr=!0,i.calledRun=!0,!I&&(mt(),o(i),S(!i._main,'compiled without a main, but one is present. if you added it from JS, use Module["onRuntimeInitialized"]'),At())}r(),Me()}or(),e=u;for(const r of Object.keys(i))r in w||Object.defineProperty(w,r,{configurable:!0,get(){he(`Access to module property ('${r}') is no longer possible via the module constructor argument; Instead, use the result of the module constructor.`)}});return e});function bi(w){switch(w){case 0:return;case 1:throw new Error("Unspecified error occured while adding mesh");case 2:throw new Error("IndexOutOfRange error while adding mesh");case 3:throw new Error("InvalidFaceVertexCount error while adding mesh");case 4:throw new Error("InvalidIndexCount error while adding mesh");default:throw new Error(`Unknown error (${w}) occured while adding mesh`)}}let Kt,gr;async function kn(){Kt?await Kt:(Kt=Un(),gr=await Kt)}class An{#e;constructor(){if(!gr)throw new Error("watlas not initialized! Call `await watlas.Initialize()` before constructing an Atlas instance.");this.#e=new gr.Atlas}delete(){this.#e.delete(),this.#e=null}addMesh(e){bi(this.#e.addMesh(e))}addUvMesh(e){bi(this.#e.addUvMesh(e))}computeCharts(e){this.#e.computeCharts(e)}packCharts(e){this.#e.packCharts(e)}generate(e={},i={}){this.#e.generate(e,i)}getMesh(e){return this.#e.getMesh(e)}getUtilization(e){return this.#e.getUtilization(e)}get width(){return this.#e.width}get height(){return this.#e.height}get atlasCount(){return this.#e.atlasCount}get chartCount(){return this.#e.chartCount}get meshCount(){return this.#e.meshCount}get texelsPerUnit(){return this.#e.texelsPerUnit}}const Ye=we("SculptBake"),In=`
  varying vec3 vVolumePos;
  void main() {
    // Pass volume-space position to the fragment for 3D-texture sampling.
    // Sculpt's marching-cubes output places vertices in [0,1]³ before the
    // local-renderer's geometry.center() call. xatlas may add seam vertices
    // during chart generation, but each new vertex's position is copied from
    // the original input vertex via the xref index — so position.xyz is
    // still the volume-space sample coordinate.
    vVolumePos = position;
    // Rasterize in UV space: UV → clip-space [-1, 1].
    gl_Position = vec4(uv * 2.0 - 1.0, 0.0, 1.0);
  }
`,Rn=`
  precision highp float;
  precision highp sampler3D;
  uniform sampler3D colorVolume;
  varying vec3 vVolumePos;
  out vec4 fragColor;
  void main() {
    // The volume is pre-processed in the quantize step so every voxel holds
    // an opaque RGB value (painted voxels keep their colour, unpainted
    // voxels are filled with the fallback). Hardware trilinear naturally
    // produces smooth gradients between painted and unpainted regions —
    // no threshold, no divide, no oversaturation.
    fragColor = vec4(texture(colorVolume, vVolumePos).rgb, 1.0);
  }
`;async function qn(w,e={}){const{resolution:i=1024,padding:o=2,maxIterations:n=1}=e;if(!w.index)throw new Error("Sculpt bake: geometry must be indexed for xatlas. Sculpt MC output already is — check the caller did not call toNonIndexed().");const u=w.attributes.position,c=u.count,d=w.index,f=d.count;let g=1/0,x=1/0,v=1/0,_=-1/0,E=-1/0,b=-1/0;for(let F=0;F<c;F++){const I=u.getX(F),S=u.getY(F),O=u.getZ(F);I<g&&(g=I),I>_&&(_=I),S<x&&(x=S),S>E&&(E=S),O<v&&(v=O),O>b&&(b=O)}await kn();const P=new Float32Array(c*3);for(let F=0;F<c;F++)P[F*3+0]=u.getX(F),P[F*3+1]=u.getY(F),P[F*3+2]=u.getZ(F);const B=new Uint32Array(f);for(let F=0;F<f;F++)B[F]=d.getX(F);let z;const A=w.attributes.normal;if(A&&A.count===c){z=new Float32Array(c*3);for(let F=0;F<c;F++)z[F*3+0]=A.getX(F),z[F*3+1]=A.getY(F),z[F*3+2]=A.getZ(F)}else{const F=new Jt;F.setAttribute("position",new Fe(P,3)),F.setIndex(new Fe(B,1)),F.computeVertexNormals();const I=F.attributes.normal;z=new Float32Array(I.array),F.dispose()}const k=new An;let M;try{k.addMesh({vertexPositionData:P,vertexCount:c,vertexPositionStride:12,vertexNormalData:z,vertexNormalStride:12,indexData:B,indexCount:f});const F=performance.now();k.generate({maxIterations:n},{padding:o,resolution:i,bilinear:!0,rotateCharts:!0});const I=performance.now()-F,S=k.width,O=k.height,N=k.atlasCount,j=k.meshCount;if(S<=0||O<=0)throw new Error(`Sculpt bake: xatlas returned invalid atlas dims ${S}×${O}`);if(N!==1)throw new Error(`Sculpt bake: xatlas spilled charts into ${N} atlases. Increase resolution (current ${i}) or reduce mesh density.`);if(j!==1)throw new Error(`Sculpt bake: expected meshCount=1, got ${j}`);const W=k.getMesh(0),Q=W.vertexCount,V=W.indexCount,Se=W.chartCount,ie=new Uint32Array(V);W.getIndexArray(ie);const se=new Float32Array(Q*3),oe=new Float32Array(Q*3),Z=new Float32Array(Q*2);for(let ee=0;ee<Q;ee++){const He=W.getVertex(ee),Me=He.xref;se[ee*3+0]=P[Me*3+0],se[ee*3+1]=P[Me*3+1],se[ee*3+2]=P[Me*3+2],oe[ee*3+0]=z[Me*3+0],oe[ee*3+1]=z[Me*3+1],oe[ee*3+2]=z[Me*3+2],Z[ee*2+0]=He.uv[0]/S,Z[ee*2+1]=He.uv[1]/O}Ye.info("UV atlas generated",{inputVertices:c,inputTriangles:f/3,outputVertices:Q,outputTriangles:V/3,chartCount:Se,atlasSize:`${S}×${O}`,unwrapMs:I.toFixed(1),positionBounds:{min:[g.toFixed(3),x.toFixed(3),v.toFixed(3)],max:[_.toFixed(3),E.toFixed(3),b.toFixed(3)]},expectedRange:"[0, 1]³ (pre-center MC output)"}),M={positions:se,normals:oe,uvs:Z,indices:ie,atlasWidth:S,atlasHeight:O,chartCount:Se,outVertexCount:Q}}finally{k.delete()}const D=new Jt;D.setAttribute("position",new Fe(M.positions,3)),D.setAttribute("normal",new Fe(M.normals,3));const Y=new Fe(M.uvs,2);return D.setAttribute("uv",Y),D.setIndex(new Fe(M.indices,1)),{atlasGeometry:D,uvAttribute:Y,atlasWidth:M.atlasWidth,atlasHeight:M.atlasHeight,chartCount:M.chartCount}}async function Hn(w){const{geometry:e,colorVolumeFloat32:i,colorVolumeResolution:o,renderer:n,atlasWidth:u,atlasHeight:c,unpaintedFallback:d={r:.85,g:.85,b:.85},dilateIterations:f=8,blurPasses:g=2,blurMode:x="all",paintExpandIterations:v=0,paintExpandThreshold:_=.5,paintedAlphaThreshold:E=.001}=w,b=new Float32Array(i.length);for(let G=0;G<i.length;G++)b[G]=i[G]/255;const P=performance.now();if(!e.attributes.uv)throw new Error("Sculpt bake: geometry has no UV attribute (call generateAtlasUVs first).");if(u<=0||c<=0)throw new Error(`Sculpt bake: invalid atlas dims ${u}×${c}`);Ye.info("Bake started",{atlasSize:`${u}×${c}`,volumeResolution:o,volumeFloat32Length:b.length,geometryVertCount:e.attributes.position.count,rendererCapabilities:{isWebGL2:n.capabilities.isWebGL2,maxTextureSize:n.capabilities.maxTextureSize}});const B=o**3,z=B*4;if(b.length!==z)throw new Error(`Sculpt bake: colorVolumeFloat32 length ${b.length} != expected ${z}`);if(v>0){const G=performance.now(),L=o,ne=L*L*4,ae=L*4,ue=new Float32Array(b.length);for(let pe=0;pe<v;pe++){ue.set(b);for(let je=0;je<L;je++){const J=je>0?-ne:0,te=je<L-1?ne:0;for(let X=0;X<L;X++){const Ce=X>0?-ae:0,rt=X<L-1?ae:0;for(let Ie=0;Ie<L;Ie++){const ze=Ie>0?-4:0,ut=Ie<L-1?4:0,de=(je*L*L+X*L+Ie)*4,Ne=b[de+3];if(Ne>=_)continue;let Ee=Ne,We=b[de+0],Ue=b[de+1],Re=b[de+2];const Qt=[ze,ut,Ce,rt,J,te];for(let St=0;St<6;St++){const lt=Qt[St];if(lt===0)continue;const It=b[de+lt+3];It>Ee&&(Ee=It,We=b[de+lt+0],Ue=b[de+lt+1],Re=b[de+lt+2])}ue[de+0]=We,ue[de+1]=Ue,ue[de+2]=Re,ue[de+3]=Ee}}}b.set(ue)}Ye.info("Paint expanded",{iterations:v,threshold:_,expandMs:(performance.now()-G).toFixed(1)})}const A=g;if(A>0){const G=performance.now(),L=o,ne=L*L*4,ae=L*4,ue=x==="alpha-only"?3:0,pe=new Float32Array(b.length);for(let je=0;je<A;je++){for(let J=0;J<L;J++){const te=J>0?-ne:0,X=J<L-1?ne:0;for(let Ce=0;Ce<L;Ce++){const rt=Ce>0?-ae:0,Ie=Ce<L-1?ae:0;for(let ze=0;ze<L;ze++){const ut=ze>0?-4:0,de=ze<L-1?4:0,Ne=(J*L*L+Ce*L+ze)*4;let Ee=1;ut&&Ee++,de&&Ee++,rt&&Ee++,Ie&&Ee++,te&&Ee++,X&&Ee++;for(let We=ue;We<4;We++){const Ue=Ne+We;let Re=b[Ue];ut&&(Re+=b[Ue+ut]),de&&(Re+=b[Ue+de]),rt&&(Re+=b[Ue+rt]),Ie&&(Re+=b[Ue+Ie]),te&&(Re+=b[Ue+te]),X&&(Re+=b[Ue+X]),pe[Ue]=Re/Ee}}}}for(let J=ue;J<b.length;J+=4)b[J]=pe[J],ue===0&&(b[J+1]=pe[J+1],b[J+2]=pe[J+2],b[J+3]=pe[J+3])}Ye.info("Volume blurred",{passes:A,mode:x,blurMs:(performance.now()-G).toFixed(1)})}const k=.001,M=Math.max(0,Math.min(1,d.r)),D=Math.max(0,Math.min(1,d.g)),Y=Math.max(0,Math.min(1,d.b)),F=new Uint8Array(z),I=G=>{const L=G>=0?G<=1?G:1:0;return Math.round(L*255)};let S=0,O=0,N=0,j=0,W=1,Q=0,V=1,Se=0,ie=1,se=0;for(let G=0;G<z;G+=4){const L=b[G+3],ne=b[G+0],ae=b[G+1],ue=b[G+2];L>j&&(j=L);const pe=L>=E?1:L<=0?0:L/E;L>=.999?(S++,ne<W&&(W=ne),ne>Q&&(Q=ne),ae<V&&(V=ae),ae>Se&&(Se=ae),ue<ie&&(ie=ue),ue>se&&(se=ue)):L>k?O++:N++,F[G+0]=I(pe*ne+(1-pe)*M),F[G+1]=I(pe*ae+(1-pe)*D),F[G+2]=I(pe*ue+(1-pe)*Y),F[G+3]=255}const oe=S+O;Ye.info("Volume quantized",{fullyPaintedVoxels:S,partiallyPaintedVoxels:O,unpaintedVoxels:N,paintedFraction:(oe/B).toFixed(4),fullyPaintedFraction:(S/B).toFixed(4),maxAlphaInVolume:j.toFixed(4),paintedRgbRange:S>0?{r:[W.toFixed(3),Q.toFixed(3)],g:[V.toFixed(3),Se.toFixed(3)],b:[ie.toFixed(3),se.toFixed(3)]}:"no fully-painted voxels",fallbackRgb:[Math.round(M*255),Math.round(D*255),Math.round(Y*255)],quantizeMode:"continuous-alpha-blend"}),oe===0&&Ye.warn("Sculpt bake: ZERO painted voxels in volume — bake will be uniform fallback colour. Did the brush write to the colour volume?");const Z=new wo(F,o,o,o);Z.format=Hr,Z.type=jr,Z.minFilter=Ht,Z.magFilter=Ht,Z.wrapR=ar,Z.wrapS=ar,Z.wrapT=ar,Z.unpackAlignment=1,Z.needsUpdate=!0;const ee=new So(u,c,{format:Hr,type:jr,minFilter:Ht,magFilter:Ht,generateMipmaps:!1,depthBuffer:!1,stencilBuffer:!1,colorSpace:Po}),He=new Co({glslVersion:To,vertexShader:In,fragmentShader:Rn,uniforms:{colorVolume:{value:Z}},depthTest:!1,depthWrite:!1,side:Eo}),Me=new Bo(e,He),re=new Fo;re.add(Me);const Xe=new Mo,U=n.getRenderTarget(),ht=n.autoClear,ot=new Zr;n.getClearColor(ot);const pt=n.getClearAlpha(),wt=performance.now();try{n.setRenderTarget(ee),n.setClearColor(new Zr(d.r,d.g,d.b),0),n.autoClear=!0,n.clear(!0,!1,!1),n.render(re,Xe)}finally{n.setRenderTarget(U),n.autoClear=ht,n.setClearColor(ot,pt)}Ye.info("Bake render complete",{renderMs:(performance.now()-wt).toFixed(1),rendererInfoCalls:n.info.render.calls,rendererInfoTriangles:n.info.render.triangles});const Pe=new Uint8Array(u*c*4);n.readRenderTargetPixels(ee,0,0,u,c,Pe);let Ke=0,mt=0,At=0,et=0,tt=0,$e=255,Ae=0,nt=255,gt=0,he=255,ve=0;for(let G=0;G<Pe.length;G+=4){const L=Pe[G],ne=Pe[G+1],ae=Pe[G+2];Pe[G+3]>0?Ke++:mt++,At+=L,et+=ne,tt+=ae,L<$e&&($e=L),L>Ae&&(Ae=L),ne<nt&&(nt=ne),ne>gt&&(gt=ne),ae<he&&(he=ae),ae>ve&&(ve=ae)}const ye=u*c;Ye.info("Bake readback",{totalPixels:ye,opaquePixels:Ke,transparentPixels:mt,opaqueFraction:(Ke/ye).toFixed(3),rgbMean:[(At/ye).toFixed(1),(et/ye).toFixed(1),(tt/ye).toFixed(1)],rgbRange:{r:[$e,Ae],g:[nt,gt],b:[he,ve]}}),Ke===0&&Ye.error("Sculpt bake: ZERO opaque pixels — the fragment shader rendered nothing. Check shader compile errors above."),$e===Ae&&nt===gt&&he===ve&&Ye.warn("Sculpt bake: ENTIRE atlas is one solid colour",{uniformRgb:[$e,nt,he],hint:"Either the volume has no painted voxels OR the shader is broken (e.g. compile error)."}),zn(Pe,u,c,{iterations:f});for(let G=3;G<Pe.length;G+=4)Pe[G]=255;const at=await Dn(Pe,u,c);return ee.dispose(),Z.dispose(),He.dispose(),Ye.info("Bake complete",{pngBytes:at.byteLength,totalMs:(performance.now()-P).toFixed(1)}),at}async function Dn(w,e,i){const o=document.createElement("canvas");o.width=e,o.height=i;const n=o.getContext("2d");if(!n)throw new Error("Sculpt bake: failed to acquire 2D context for PNG encoding");const u=n.createImageData(e,i);return u.data.set(w),n.putImageData(u,0,0),await(await new Promise((d,f)=>{o.toBlob(g=>g?d(g):f(new Error("Sculpt bake: canvas.toBlob returned null")),"image/png")})).arrayBuffer()}export{di as D,_t as M,$n as N,Nn as S,Gn as W,fr as a,Hn as b,ge as c,qn as g,fi as i};
