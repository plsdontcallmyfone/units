const A=16e3;const g="plnty-vision-stream-pcm16k",M=`
class PcmCapture extends AudioWorkletProcessor {
  constructor() {
    super();
    this.ratio = sampleRate / 16000;
    this.acc = 0;
    this.sum = 0;
    this.count = 0;
    this.out = new Float32Array(1600);
    this.n = 0;
  }
  process(inputs) {
    const input = inputs[0];
    if (!input || input.length === 0) return true;
    const a = input[0];
    const b = input.length > 1 ? input[1] : null;
    for (let i = 0; i < a.length; i++) {
      this.sum += b ? (a[i] + b[i]) * 0.5 : a[i];
      this.count++;
      this.acc += 1;
      if (this.acc >= this.ratio) {
        this.acc -= this.ratio;
        this.out[this.n++] = this.sum / this.count;
        this.sum = 0;
        this.count = 0;
        if (this.n === this.out.length) {
          this.port.postMessage(this.out, [this.out.buffer]);
          this.out = new Float32Array(1600);
          this.n = 0;
        }
      }
    }
    return true;
  }
}
registerProcessor('${g}', PcmCapture);
`;class h extends Error{constructor(t,e){super(e),this.reason=t,this.name="MicCaptureError"}}async function T(r){if(typeof navigator>"u"||!navigator.mediaDevices?.getUserMedia||typeof AudioWorkletNode>"u")throw new h("unsupported","This browser cannot listen to a microphone here.");let t;try{t=await navigator.mediaDevices.getUserMedia({audio:{echoCancellation:!0,noiseSuppression:!0,autoGainControl:!0,channelCount:1}})}catch(n){const c=n instanceof DOMException?n.name:"";throw c==="NotAllowedError"||c==="SecurityError"?new h("denied","The microphone is blocked for this site."):c==="NotFoundError"||c==="OverconstrainedError"?new h("no-device","No microphone was found."):new h("failed",n instanceof Error?n.message:"The microphone did not start.")}const e=new AudioContext,i=URL.createObjectURL(new Blob([M],{type:"application/javascript"}));try{await e.audioWorklet.addModule(i)}catch(n){throw t.getTracks().forEach(c=>c.stop()),e.close(),URL.revokeObjectURL(i),new h("failed",n instanceof Error?n.message:"The microphone did not start.")}e.state==="suspended"&&await e.resume().catch(()=>{});const l=e.createMediaStreamSource(t),a=new AudioWorkletNode(e,g,{numberOfInputs:1,numberOfOutputs:1});a.port.onmessage=n=>r(n.data);const u=e.createAnalyser();u.fftSize=512;const d=e.createGain();d.gain.value=0,l.connect(a),l.connect(u),a.connect(d),u.connect(d),d.connect(e.destination);const o=new Float32Array(u.fftSize);let s=!1;return{level:()=>{if(s)return 0;u.getFloatTimeDomainData(o);let n=0;for(let p=0;p<o.length;p++)n+=o[p]*o[p];const c=Math.sqrt(n/o.length);if(c<=0)return 0;const f=20*Math.log10(c);return Math.min(1,Math.max(0,(f+60)/48))},setMuted:n=>{for(const c of t.getAudioTracks())c.enabled=!n},stop:()=>{if(!s){s=!0,a.port.onmessage=null;try{l.disconnect(),a.disconnect(),u.disconnect(),d.disconnect()}catch{}t.getTracks().forEach(n=>n.stop()),e.close(),URL.revokeObjectURL(i)}}}}const E=[-1,-1,-1,-1,2,4,6,8,-1,-1,-1,-1,2,4,6,8],w=[7,8,9,10,11,12,13,14,16,17,19,21,23,25,28,31,34,37,41,45,50,55,60,66,73,80,88,97,107,118,130,143,157,173,190,209,230,253,279,307,337,371,408,449,494,544,598,658,724,796,876,963,1060,1166,1282,1411,1552,1707,1878,2066,2272,2499,2749,3024,3327,3660,4026,4428,4871,5358,5894,6484,7132,7845,8630,9493,10442,11487,12635,13899,15289,16818,18500,20350,22385,24623,27086,29794,32767];function S(){return{predicted:0,index:0}}const m=r=>r>32767?32767:r<-32768?-32768:r;function b(r,t){const e=r.length&-2,i=new Uint8Array(3+e/2);i[0]=t.predicted&255,i[1]=t.predicted>>8&255,i[2]=t.index;let l=t.predicted,a=t.index;for(let u=0;u<e;u++){const d=m(Math.round(r[u]*32767));let o=w[a],s=d-l,n=0;s<0&&(n=8,s=-s);let c=o>>3;s>=o&&(n|=4,s-=o,c+=o),o>>=1,s>=o&&(n|=2,s-=o,c+=o),o>>=1,s>=o&&(n|=1,c+=o),l=m(n&8?l-c:l+c),a=Math.min(88,Math.max(0,a+E[n]));const f=3+(u>>1);i[f]=u&1?i[f]|n<<4:n}return t.predicted=l,t.index=a,i}function v(r){if(r.length<3)return new Float32Array(0);let t=(r[0]|r[1]<<8)<<16>>16,e=Math.min(88,r[2]);const i=(r.length-3)*2,l=new Float32Array(i);for(let a=0;a<i;a++){const u=r[3+(a>>1)],d=a&1?u>>4:u&15,o=w[e];let s=o>>3;d&4&&(s+=o),d&2&&(s+=o>>1),d&1&&(s+=o>>2),t=m(d&8?t-s:t+s),e=Math.min(88,Math.max(0,e+E[d])),l[a]=t/32768}return l}function L(r){let t="";for(let e=0;e<r.length;e++)t+=String.fromCharCode(r[e]);return btoa(t)}function x(r){const t=atob(r),e=new Uint8Array(t.length);for(let i=0;i<t.length;i++)e[i]=t.charCodeAt(i);return e}export{h as M,A as S,x as a,v as d,b as e,S as n,T as s,L as v};
