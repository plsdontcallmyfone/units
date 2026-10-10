import{a7 as we,a1 as E,aF as _,a4 as We,J as M,U as je,b5 as ia,a6 as ot,a5 as oa,f as A,ay as Ye,aG as kt,bB as sa,bC as zt,Y as Be,al as N,p as de,j as V,aw as na,d as P,aB as me,l as la,M as _e,h as Le,ad as Ie,Z as fe,aK as pe,e as oe,P as Ke,K as W,H as B,X as H,aq as $,$ as Q,bD as ca,Q as ua,aH as fa,D as Ht,br as ha,a$ as da,aE as ma,k as j,N as pa,bA as Bt,af as Ze,x as ga,aa as va,c as xa,S as q,bE as ba,bF as ya,E as Nt,ar as Ta,aM as wa,aQ as _a,r as Sa,aj as Ia,W as Ra,ab as Lt,A as Ma,aO as Aa,as as Fa,a3 as Ca,a2 as Da}from"./three-CbhGz-nM.js";import{x as Wt,ul as De,mt as st,s as Pa,tX as Ut}from"./main-DamfXklH.js";import{d as Gt}from"./brush-textures-BLmKcWao.js";import{s as Ea}from"./geometry-utils-DnucbU7f.js";import{g as Oa,a as ka,B as nt,I as za,C as Ha,O as Ba,S as Na,b as La,L as Wa,c as Ua,M as Ga}from"./MeshBVH-PQsxEzdn.js";function qa(r){switch(r){case 1:return"R";case 2:return"RG";case 3:return"RGBA";case 4:return"RGBA"}throw new Error}function Va(r){switch(r){case 1:return Be;case 2:return zt;case 3:return A;case 4:return A}}function lt(r){switch(r){case 1:return sa;case 2:return kt;case 3:return Ye;case 4:return Ye}}class qt extends E{constructor(){super(),this.minFilter=_,this.magFilter=_,this.generateMipmaps=!1,this.overrideItemSize=null,this._forcedType=null}updateFrom(e){const t=this.overrideItemSize,a=e.itemSize,o=e.count;if(t!==null){if(a*o%t!==0)throw new Error("VertexAttributeTexture: overrideItemSize must divide evenly into buffer length.");e.itemSize=t,e.count=o*a/t}const s=e.itemSize,i=e.count,l=e.normalized,c=e.array.constructor,d=c.BYTES_PER_ELEMENT;let h=this._forcedType,f=s;if(h===null)switch(c){case Float32Array:h=M;break;case Uint8Array:case Uint16Array:case Uint32Array:h=we;break;case Int8Array:case Int16Array:case Int32Array:h=We;break}let n,g,m,p,u=qa(s);switch(h){case M:m=1,g=Va(s),l&&d===1?(p=c,u+="8",c===Uint8Array?n=je:(n=ot,u+="_SNORM")):(p=Float32Array,u+="32F",n=M);break;case We:u+=d*8+"I",m=l?Math.pow(2,c.BYTES_PER_ELEMENT*8-1):1,g=lt(s),d===1?(p=Int8Array,n=ot):d===2?(p=Int16Array,n=oa):(p=Int32Array,n=We);break;case we:u+=d*8+"UI",m=l?Math.pow(2,c.BYTES_PER_ELEMENT*8-1):1,g=lt(s),d===1?(p=Uint8Array,n=je):d===2?(p=Uint16Array,n=ia):(p=Uint32Array,n=we);break}f===3&&(g===A||g===Ye)&&(f=4);const v=Math.ceil(Math.sqrt(i))||1,x=f*v*v,b=new p(x),T=e.normalized;e.normalized=!1;for(let y=0;y<i;y++){const I=f*y;b[I]=e.getX(y)/m,s>=2&&(b[I+1]=e.getY(y)/m),s>=3&&(b[I+2]=e.getZ(y)/m,f===4&&(b[I+3]=1)),s>=4&&(b[I+3]=e.getW(y)/m)}e.normalized=T,this.internalFormat=u,this.format=g,this.type=n,this.image.width=v,this.image.height=v,this.image.data=b,this.needsUpdate=!0,this.dispose(),e.itemSize=a,e.count=o}}class Vt extends qt{constructor(){super(),this._forcedType=we}}class $t extends qt{constructor(){super(),this._forcedType=M}}class $a{constructor(){this.index=new Vt,this.position=new $t,this.bvhBounds=new E,this.bvhContents=new E,this._cachedIndexAttr=null,this.index.overrideItemSize=3}updateFrom(e){const{geometry:t}=e;if(Ya(e,this.bvhBounds,this.bvhContents),this.position.updateFrom(t.attributes.position),e.indirect){const a=e._indirectBuffer;if(this._cachedIndexAttr===null||this._cachedIndexAttr.count!==a.length)if(t.index)this._cachedIndexAttr=t.index.clone();else{const o=Oa(ka(t));this._cachedIndexAttr=new N(o,1,!1)}ja(t,a,this._cachedIndexAttr),this.index.updateFrom(this._cachedIndexAttr)}else this.index.updateFrom(t.index)}dispose(){const{index:e,position:t,bvhBounds:a,bvhContents:o}=this;e&&e.dispose(),t&&t.dispose(),a&&a.dispose(),o&&o.dispose()}}function ja(r,e,t){const a=t.array,o=r.index?r.index.array:null;for(let s=0,i=e.length;s<i;s++){const l=3*s,c=3*e[s];for(let d=0;d<3;d++)a[l+d]=o?o[c+d]:c+d}}function Ya(r,e,t){const a=r._roots;if(a.length!==1)throw new Error("MeshBVHUniformStruct: Multi-root BVHs not supported.");const o=a[0],s=new Uint16Array(o),i=new Uint32Array(o),l=new Float32Array(o),c=o.byteLength/nt,d=2*Math.ceil(Math.sqrt(c/2)),h=new Float32Array(4*d*d),f=Math.ceil(Math.sqrt(c)),n=new Uint32Array(2*f*f);for(let g=0;g<c;g++){const m=g*nt/4,p=m*2,u=La(m);for(let v=0;v<3;v++)h[8*g+0+v]=l[u+0+v],h[8*g+4+v]=l[u+3+v];if(za(p,s)){const v=Ha(p,s),x=Ba(m,i),b=Wa|v;n[g*2+0]=b,n[g*2+1]=x}else{const v=i[m+6],x=Na(m,i);n[g*2+0]=x,n[g*2+1]=v}}e.image.data=h,e.image.width=d,e.image.height=d,e.format=A,e.type=M,e.internalFormat="RGBA32F",e.minFilter=_,e.magFilter=_,e.generateMipmaps=!1,e.needsUpdate=!0,e.dispose(),t.image.data=n,t.image.width=f,t.image.height=f,t.format=kt,t.type=we,t.internalFormat="RG32UI",t.minFilter=_,t.magFilter=_,t.generateMipmaps=!1,t.needsUpdate=!0,t.dispose()}const Qa=`

// A stack of uint32 indices can can store the indices for
// a perfectly balanced tree with a depth up to 31. Lower stack
// depth gets higher performance.
//
// However not all trees are balanced. Best value to set this to
// is the trees max depth.
#ifndef BVH_STACK_DEPTH
#define BVH_STACK_DEPTH 60
#endif

#ifndef INFINITY
#define INFINITY 1e20
#endif

// Utilities
uvec4 uTexelFetch1D( usampler2D tex, uint index ) {

	uint width = uint( textureSize( tex, 0 ).x );
	uvec2 uv;
	uv.x = index % width;
	uv.y = index / width;

	return texelFetch( tex, ivec2( uv ), 0 );

}

ivec4 iTexelFetch1D( isampler2D tex, uint index ) {

	uint width = uint( textureSize( tex, 0 ).x );
	uvec2 uv;
	uv.x = index % width;
	uv.y = index / width;

	return texelFetch( tex, ivec2( uv ), 0 );

}

vec4 texelFetch1D( sampler2D tex, uint index ) {

	uint width = uint( textureSize( tex, 0 ).x );
	uvec2 uv;
	uv.x = index % width;
	uv.y = index / width;

	return texelFetch( tex, ivec2( uv ), 0 );

}

vec4 textureSampleBarycoord( sampler2D tex, vec3 barycoord, uvec3 faceIndices ) {

	return
		barycoord.x * texelFetch1D( tex, faceIndices.x ) +
		barycoord.y * texelFetch1D( tex, faceIndices.y ) +
		barycoord.z * texelFetch1D( tex, faceIndices.z );

}

void ndcToCameraRay(
	vec2 coord, mat4 cameraWorld, mat4 invProjectionMatrix,
	out vec3 rayOrigin, out vec3 rayDirection
) {

	// get camera look direction and near plane for camera clipping
	vec4 lookDirection = cameraWorld * vec4( 0.0, 0.0, - 1.0, 0.0 );
	vec4 nearVector = invProjectionMatrix * vec4( 0.0, 0.0, - 1.0, 1.0 );
	float near = abs( nearVector.z / nearVector.w );

	// get the camera direction and position from camera matrices
	vec4 origin = cameraWorld * vec4( 0.0, 0.0, 0.0, 1.0 );
	vec4 direction = invProjectionMatrix * vec4( coord, 0.5, 1.0 );
	direction /= direction.w;
	direction = cameraWorld * direction - origin;

	// slide the origin along the ray until it sits at the near clip plane position
	origin.xyz += direction.xyz * near / dot( direction, lookDirection );

	rayOrigin = origin.xyz;
	rayDirection = direction.xyz;

}
`,Xa=`

#ifndef TRI_INTERSECT_EPSILON
#define TRI_INTERSECT_EPSILON 1e-5
#endif

// Raycasting
bool intersectsBounds( vec3 rayOrigin, vec3 rayDirection, vec3 boundsMin, vec3 boundsMax, out float dist ) {

	// https://www.reddit.com/r/opengl/comments/8ntzz5/fast_glsl_ray_box_intersection/
	// https://tavianator.com/2011/ray_box.html
	vec3 invDir = 1.0 / rayDirection;

	// find intersection distances for each plane
	vec3 tMinPlane = invDir * ( boundsMin - rayOrigin );
	vec3 tMaxPlane = invDir * ( boundsMax - rayOrigin );

	// get the min and max distances from each intersection
	vec3 tMinHit = min( tMaxPlane, tMinPlane );
	vec3 tMaxHit = max( tMaxPlane, tMinPlane );

	// get the furthest hit distance
	vec2 t = max( tMinHit.xx, tMinHit.yz );
	float t0 = max( t.x, t.y );

	// get the minimum hit distance
	t = min( tMaxHit.xx, tMaxHit.yz );
	float t1 = min( t.x, t.y );

	// set distance to 0.0 if the ray starts inside the box
	dist = max( t0, 0.0 );

	return t1 >= dist;

}

bool intersectsTriangle(
	vec3 rayOrigin, vec3 rayDirection, vec3 a, vec3 b, vec3 c,
	out vec3 barycoord, out vec3 norm, out float dist, out float side
) {

	// https://stackoverflow.com/questions/42740765/intersection-between-line-and-triangle-in-3d
	vec3 edge1 = b - a;
	vec3 edge2 = c - a;
	norm = cross( edge1, edge2 );

	float det = - dot( rayDirection, norm );
	float invdet = 1.0 / det;

	vec3 AO = rayOrigin - a;
	vec3 DAO = cross( AO, rayDirection );

	vec4 uvt;
	uvt.x = dot( edge2, DAO ) * invdet;
	uvt.y = - dot( edge1, DAO ) * invdet;
	uvt.z = dot( AO, norm ) * invdet;
	uvt.w = 1.0 - uvt.x - uvt.y;

	// set the hit information
	barycoord = uvt.wxy; // arranged in A, B, C order
	dist = uvt.z;
	side = sign( det );
	norm = side * normalize( norm );

	// add an epsilon to avoid misses between triangles
	uvt += vec4( TRI_INTERSECT_EPSILON );

	return all( greaterThanEqual( uvt, vec4( 0.0 ) ) );

}

bool intersectTriangles(
	// geometry info and triangle range
	sampler2D positionAttr, usampler2D indexAttr, uint offset, uint count,

	// ray
	vec3 rayOrigin, vec3 rayDirection,

	// outputs
	inout float minDistance, inout uvec4 faceIndices, inout vec3 faceNormal, inout vec3 barycoord,
	inout float side, inout float dist
) {

	bool found = false;
	vec3 localBarycoord, localNormal;
	float localDist, localSide;
	for ( uint i = offset, l = offset + count; i < l; i ++ ) {

		uvec3 indices = uTexelFetch1D( indexAttr, i ).xyz;
		vec3 a = texelFetch1D( positionAttr, indices.x ).rgb;
		vec3 b = texelFetch1D( positionAttr, indices.y ).rgb;
		vec3 c = texelFetch1D( positionAttr, indices.z ).rgb;

		if (
			intersectsTriangle( rayOrigin, rayDirection, a, b, c, localBarycoord, localNormal, localDist, localSide )
			&& localDist < minDistance
		) {

			found = true;
			minDistance = localDist;

			faceIndices = uvec4( indices.xyz, i );
			faceNormal = localNormal;

			side = localSide;
			barycoord = localBarycoord;
			dist = localDist;

		}

	}

	return found;

}

bool intersectsBVHNodeBounds( vec3 rayOrigin, vec3 rayDirection, sampler2D bvhBounds, uint currNodeIndex, out float dist ) {

	uint cni2 = currNodeIndex * 2u;
	vec3 boundsMin = texelFetch1D( bvhBounds, cni2 ).xyz;
	vec3 boundsMax = texelFetch1D( bvhBounds, cni2 + 1u ).xyz;
	return intersectsBounds( rayOrigin, rayDirection, boundsMin, boundsMax, dist );

}

// use a macro to hide the fact that we need to expand the struct into separate fields
#define	bvhIntersectFirstHit(		bvh,		rayOrigin, rayDirection, faceIndices, faceNormal, barycoord, side, dist	)	_bvhIntersectFirstHit(		bvh.position, bvh.index, bvh.bvhBounds, bvh.bvhContents,		rayOrigin, rayDirection, faceIndices, faceNormal, barycoord, side, dist	)

bool _bvhIntersectFirstHit(
	// bvh info
	sampler2D bvh_position, usampler2D bvh_index, sampler2D bvh_bvhBounds, usampler2D bvh_bvhContents,

	// ray
	vec3 rayOrigin, vec3 rayDirection,

	// output variables split into separate variables due to output precision
	inout uvec4 faceIndices, inout vec3 faceNormal, inout vec3 barycoord,
	inout float side, inout float dist
) {

	// stack needs to be twice as long as the deepest tree we expect because
	// we push both the left and right child onto the stack every traversal
	int pointer = 0;
	uint stack[ BVH_STACK_DEPTH ];
	stack[ 0 ] = 0u;

	float triangleDistance = INFINITY;
	bool found = false;
	while ( pointer > - 1 && pointer < BVH_STACK_DEPTH ) {

		uint currNodeIndex = stack[ pointer ];
		pointer --;

		// check if we intersect the current bounds
		float boundsHitDistance;
		if (
			! intersectsBVHNodeBounds( rayOrigin, rayDirection, bvh_bvhBounds, currNodeIndex, boundsHitDistance )
			|| boundsHitDistance > triangleDistance
		) {

			continue;

		}

		uvec2 boundsInfo = uTexelFetch1D( bvh_bvhContents, currNodeIndex ).xy;
		bool isLeaf = bool( boundsInfo.x & 0xffff0000u );

		if ( isLeaf ) {

			uint count = boundsInfo.x & 0x0000ffffu;
			uint offset = boundsInfo.y;

			found = intersectTriangles(
				bvh_position, bvh_index, offset, count,
				rayOrigin, rayDirection, triangleDistance,
				faceIndices, faceNormal, barycoord, side, dist
			) || found;

		} else {

			uint leftIndex = currNodeIndex + 1u;
			uint splitAxis = boundsInfo.x & 0x0000ffffu;
			uint rightIndex = currNodeIndex + boundsInfo.y;

			bool leftToRight = rayDirection[ splitAxis ] >= 0.0;
			uint c1 = leftToRight ? leftIndex : rightIndex;
			uint c2 = leftToRight ? rightIndex : leftIndex;

			// set c2 in the stack so we traverse it later. We need to keep track of a pointer in
			// the stack while we traverse. The second pointer added is the one that will be
			// traversed first
			pointer ++;
			stack[ pointer ] = c2;

			pointer ++;
			stack[ pointer ] = c1;

		}

	}

	return found;

}
`,Ka=`
struct BVH {

	usampler2D index;
	sampler2D position;

	sampler2D bvhBounds;
	usampler2D bvhContents;

};
`;async function Za(r){return new Promise((e,t)=>{const a=URL.createObjectURL(r),o=new Image;o.onload=()=>{URL.revokeObjectURL(a),e(o)},o.onerror=()=>{URL.revokeObjectURL(a),t(new Error("Failed to load image"))},o.src=a})}async function Ja(r,e={}){const{quality:t=.88,maxDimension:a=4096}=e,o=Math.min(1,a/Math.max(r.width,r.height)),s=Math.round(r.width*o),i=Math.round(r.height*o),l=document.createElement("canvas");return l.width=s,l.height=i,l.getContext("2d").drawImage(r,0,0,s,i),new Promise((d,h)=>{l.toBlob(f=>f?d(f):h(new Error("Failed to encode WebP")),"image/webp",t)})}async function po(r){const e=await Za(r);return Ja(e,{quality:.88,maxDimension:4096})}async function go(r,e=1024,t=.85){try{let a;if(r.startsWith("data:"))a=await(await fetch(r)).blob();else{const h=await fetch(r);if(!h.ok)return null;a=await h.blob()}if(!a.type.startsWith("image/"))return null;const o=await createImageBitmap(a),s=Math.min(1,e/Math.max(o.width,o.height)),i=Math.round(o.width*s),l=Math.round(o.height*s),c=document.createElement("canvas");return c.width=i,c.height=l,c.getContext("2d").drawImage(o,0,0,i,l),o.close(),c.toDataURL("image/jpeg",t)}catch{return null}}function er(r){const e=r.split(","),t=e[0].match(/:(.*?);/)?.[1]||"image/webp",a=atob(e[1]),o=new Uint8Array(a.length);for(let s=0;s<a.length;s++)o[s]=a.charCodeAt(s);return new Blob([o],{type:t})}function jt(r,e,t=0){if(r.isInterleavedBufferAttribute){const a=r.itemSize;for(let o=0,s=r.count;o<s;o++){const i=o+t;e.setX(i,r.getX(o)),a>=2&&e.setY(i,r.getY(o)),a>=3&&e.setZ(i,r.getZ(o)),a>=4&&e.setW(i,r.getW(o))}}else{const a=e.array,o=a.constructor,s=a.BYTES_PER_ELEMENT*r.itemSize*t;new o(a.buffer,s,r.array.length).set(r.array)}}function Te(r,e=null){const t=r.array.constructor,a=r.normalized,o=r.itemSize,s=e===null?r.count:e;return new N(new t(o*s),o,a)}function ue(r,e){if(!r&&!e)return!0;if(!!r!=!!e)return!1;const t=r.count===e.count,a=r.normalized===e.normalized,o=r.array.constructor===e.array.constructor,s=r.itemSize===e.itemSize;return!(!t||!a||!o||!s)}function tr(r){const e=r[0].index!==null,t=new Set(Object.keys(r[0].attributes));if(!r[0].getAttribute("position"))throw new Error("StaticGeometryGenerator: position attribute is required.");for(let a=0;a<r.length;++a){const o=r[a];let s=0;if(e!==(o.index!==null))throw new Error("StaticGeometryGenerator: All geometries must have compatible attributes; make sure index attribute exists among all geometries, or in none of them.");for(const i in o.attributes){if(!t.has(i))throw new Error('StaticGeometryGenerator: All geometries must have compatible attributes; make sure "'+i+'" attribute exists among all geometries, or in none of them.');s++}if(s!==t.size)throw new Error("StaticGeometryGenerator: All geometries must have the same number of attributes.")}}function ar(r){let e=0;for(let t=0,a=r.length;t<a;t++)e+=r[t].getIndex().count;return e}function rr(r){let e=0;for(let t=0,a=r.length;t<a;t++)e+=r[t].getAttribute("position").count;return e}function ir(r,e,t){r.index&&r.index.count!==e&&r.setIndex(null);const a=r.attributes;for(const o in a)a[o].count!==t&&r.deleteAttribute(o)}function or(r,e={},t=new de){const{useGroups:a=!1,forceUpdate:o=!1,skipAssigningAttributes:s=[],overwriteIndex:i=!0}=e;tr(r);const l=r[0].index!==null,c=l?ar(r):-1,d=rr(r);if(ir(t,c,d),a){let f=0;for(let n=0,g=r.length;n<g;n++){const m=r[n];let p;l?p=m.getIndex().count:p=m.getAttribute("position").count,t.addGroup(f,p,n),f+=p}}if(l){let f=!1;if(t.index||(t.setIndex(new N(new Uint32Array(c),1,!1)),f=!0),f||i){let n=0,g=0;const m=t.getIndex();for(let p=0,u=r.length;p<u;p++){const v=r[p],x=v.getIndex();if(!(!o&&!f&&s[p]))for(let T=0;T<x.count;++T)m.setX(n+T,x.getX(T)+g);n+=x.count,g+=v.getAttribute("position").count}}}const h=Object.keys(r[0].attributes);for(let f=0,n=h.length;f<n;f++){let g=!1;const m=h[f];if(!t.getAttribute(m)){const v=r[0].getAttribute(m);t.setAttribute(m,Te(v,d)),g=!0}let p=0;const u=t.getAttribute(m);for(let v=0,x=r.length;v<x;v++){const b=r[v],T=!o&&!g&&s[v],y=b.getAttribute(m);if(!T)if(m==="color"&&u.itemSize!==y.itemSize)for(let I=p,O=y.count;I<O;I++)y.setXYZW(I,u.getX(I),u.getY(I),u.getZ(I),1);else jt(y,u,p);p+=y.count}}}function sr(r,e,t){const a=r.index,s=r.attributes.position.count,i=a?a.count:s;let l=r.groups;l.length===0&&(l=[{count:i,start:0,materialIndex:0}]);let c=r.getAttribute("materialIndex");if(!c||c.count!==s){let h;t.length<=255?h=new Uint8Array(s):h=new Uint16Array(s),c=new N(h,1,!1),r.deleteAttribute("materialIndex"),r.setAttribute("materialIndex",c)}const d=c.array;for(let h=0;h<l.length;h++){const f=l[h],n=f.start,g=f.count,m=Math.min(g,i-n),p=Array.isArray(e)?e[f.materialIndex]:e,u=t.indexOf(p);for(let v=0;v<m;v++){let x=n+v;a&&(x=a.getX(x)),d[x]=u}}}function nr(r,e){if(!r.index){const t=r.attributes.position.count,a=new Array(t);for(let o=0;o<t;o++)a[o]=o;r.setIndex(a)}if(!r.attributes.normal&&e&&e.includes("normal")&&r.computeVertexNormals(),!r.attributes.uv&&e&&e.includes("uv")){const t=r.attributes.position.count;r.setAttribute("uv",new N(new Float32Array(t*2),2,!1))}if(!r.attributes.uv2&&e&&e.includes("uv2")){const t=r.attributes.position.count;r.setAttribute("uv2",new N(new Float32Array(t*2),2,!1))}if(!r.attributes.tangent&&e&&e.includes("tangent"))if(r.attributes.uv&&r.attributes.normal)r.computeTangents();else{const t=r.attributes.position.count;r.setAttribute("tangent",new N(new Float32Array(t*4),4,!1))}if(!r.attributes.color&&e&&e.includes("color")){const t=r.attributes.position.count,a=new Float32Array(t*4);a.fill(1),r.setAttribute("color",new N(a,4))}}function Je(r){let e=0;if(r.byteLength!==0){const t=new Uint8Array(r);for(let a=0;a<r.byteLength;a++){const o=t[a];e=(e<<5)-e+o,e|=0}}return e}function ct(r){let e=r.uuid;const t=Object.values(r.attributes);r.index&&(t.push(r.index),e+=`index|${r.index.version}`);const a=Object.keys(t).sort();for(const o of a){const s=t[o];e+=`${o}_${s.version}|`}return e}function ut(r){const e=r.skeleton;return e?(e.boneTexture||e.computeBoneTexture(),`${Je(e.boneTexture.image.data.buffer)}_${e.boneTexture.uuid}`):null}class lr{constructor(e=null){this.matrixWorld=new V,this.geometryHash=null,this.skeletonHash=null,this.primitiveCount=-1,e!==null&&this.updateFrom(e)}updateFrom(e){const t=e.geometry,a=(t.index?t.index.count:t.attributes.position.count)/3;this.matrixWorld.copy(e.matrixWorld),this.geometryHash=ct(t),this.primitiveCount=a,this.skeletonHash=ut(e)}didChange(e){const t=e.geometry,a=(t.index?t.index.count:t.attributes.position.count)/3;return!(this.matrixWorld.equals(e.matrixWorld)&&this.geometryHash===ct(t)&&this.skeletonHash===ut(e)&&this.primitiveCount===a)}}const J=new P,ee=new P,te=new P,ft=new me,Pe=new P,Ue=new P,ht=new me,dt=new me,Ee=new V,mt=new V;function pt(r,e,t){const a=r.skeleton,o=r.geometry,s=a.bones,i=a.boneInverses;ht.fromBufferAttribute(o.attributes.skinIndex,e),dt.fromBufferAttribute(o.attributes.skinWeight,e),Ee.elements.fill(0);for(let l=0;l<4;l++){const c=dt.getComponent(l);if(c!==0){const d=ht.getComponent(l);mt.multiplyMatrices(s[d].matrixWorld,i[d]),cr(Ee,mt,c)}}return Ee.multiply(r.bindMatrix).premultiply(r.bindMatrixInverse),t.transformDirection(Ee),t}function Ge(r,e,t,a,o){Pe.set(0,0,0);for(let s=0,i=r.length;s<i;s++){const l=e[s],c=r[s];l!==0&&(Ue.fromBufferAttribute(c,a),t?Pe.addScaledVector(Ue,l):Pe.addScaledVector(Ue.sub(o),l))}o.add(Pe)}function cr(r,e,t){const a=r.elements,o=e.elements;for(let s=0,i=o.length;s<i;s++)a[s]+=o[s]*t}function ur(r){const{index:e,attributes:t}=r;if(e)for(let a=0,o=e.count;a<o;a+=3){const s=e.getX(a),i=e.getX(a+2);e.setX(a,i),e.setX(a+2,s)}else for(const a in t){const o=t[a],s=o.itemSize;for(let i=0,l=o.count;i<l;i+=3)for(let c=0;c<s;c++){const d=o.getComponent(i,c),h=o.getComponent(i+2,c);o.setComponent(i,c,h),o.setComponent(i+2,c,d)}}return r}function fr(r,e={},t=new de){e={applyWorldTransforms:!0,attributes:[],...e};const a=r.geometry,o=e.applyWorldTransforms,s=e.attributes.includes("normal"),i=e.attributes.includes("tangent"),l=a.attributes,c=t.attributes;for(const x in t.attributes)(!e.attributes.includes(x)||!(x in a.attributes))&&t.deleteAttribute(x);!t.index&&a.index&&(t.index=a.index.clone()),c.position||t.setAttribute("position",Te(l.position)),s&&!c.normal&&l.normal&&t.setAttribute("normal",Te(l.normal)),i&&!c.tangent&&l.tangent&&t.setAttribute("tangent",Te(l.tangent)),ue(a.index,t.index),ue(l.position,c.position),s&&ue(l.normal,c.normal),i&&ue(l.tangent,c.tangent);const d=l.position,h=s?l.normal:null,f=i?l.tangent:null,n=a.morphAttributes.position,g=a.morphAttributes.normal,m=a.morphAttributes.tangent,p=a.morphTargetsRelative,u=r.morphTargetInfluences,v=new na;v.getNormalMatrix(r.matrixWorld),a.index&&t.index.array.set(a.index.array);for(let x=0,b=l.position.count;x<b;x++)J.fromBufferAttribute(d,x),h&&ee.fromBufferAttribute(h,x),f&&(ft.fromBufferAttribute(f,x),te.fromBufferAttribute(f,x)),u&&(n&&Ge(n,u,p,x,J),g&&Ge(g,u,p,x,ee),m&&Ge(m,u,p,x,te)),r.isSkinnedMesh&&(r.applyBoneTransform(x,J),h&&pt(r,x,ee),f&&pt(r,x,te)),o&&J.applyMatrix4(r.matrixWorld),c.position.setXYZ(x,J.x,J.y,J.z),h&&(o&&ee.applyNormalMatrix(v),c.normal.setXYZ(x,ee.x,ee.y,ee.z)),f&&(o&&te.transformDirection(r.matrixWorld),c.tangent.setXYZW(x,te.x,te.y,te.z,ft.w));for(const x in e.attributes){const b=e.attributes[x];b==="position"||b==="tangent"||b==="normal"||!(b in l)||(c[b]||t.setAttribute(b,Te(l[b])),ue(l[b],c[b]),jt(l[b],c[b]))}return r.matrixWorld.determinant()<0&&ur(t),t}class hr extends de{constructor(){super(),this.version=0,this.hash=null,this._diff=new lr}isCompatible(e,t){const a=e.geometry;for(let o=0;o<t.length;o++){const s=t[o],i=a.attributes[s],l=this.attributes[s];if(i&&!ue(i,l))return!1}return!0}updateFrom(e,t){const a=this._diff;return a.didChange(e)?(fr(e,t,this),a.updateFrom(e),this.version++,this.hash=`${this.uuid}_${this.version}`,!0):!1}}const Qe=0,Yt=1,Qt=2;function dr(r,e){for(let t=0,a=r.length;t<a;t++)r[t].traverseVisible(s=>{s.isMesh&&e(s)})}function mr(r){const e=[];for(let t=0,a=r.length;t<a;t++){const o=r[t];Array.isArray(o.material)?e.push(...o.material):e.push(o.material)}return e}function pr(r,e,t){if(r.length===0){e.setIndex(null);const a=e.attributes;for(const o in a)e.deleteAttribute(o);for(const o in t.attributes)e.setAttribute(t.attributes[o],new N(new Float32Array(0),4,!1))}else or(r,t,e);for(const a in e.attributes)e.attributes[a].needsUpdate=!0}class gr{constructor(e){this.objects=null,this.useGroups=!0,this.applyWorldTransforms=!0,this.generateMissingAttributes=!0,this.overwriteIndex=!0,this.attributes=["position","normal","color","tangent","uv","uv2"],this._intermediateGeometry=new Map,this._geometryMergeSets=new WeakMap,this._mergeOrder=[],this._dummyMesh=null,this.setObjects(e||[])}_getDummyMesh(){if(!this._dummyMesh){const e=new la,t=new de;t.setAttribute("position",new N(new Float32Array(9),3)),this._dummyMesh=new _e(t,e)}return this._dummyMesh}_getMeshes(){const e=[];return dr(this.objects,t=>{e.push(t)}),e.sort((t,a)=>t.uuid>a.uuid?1:t.uuid<a.uuid?-1:0),e.length===0&&e.push(this._getDummyMesh()),e}_updateIntermediateGeometries(){const{_intermediateGeometry:e}=this,t=this._getMeshes(),a=new Set(e.keys()),o={attributes:this.attributes,applyWorldTransforms:this.applyWorldTransforms};for(let s=0,i=t.length;s<i;s++){const l=t[s],c=l.uuid;a.delete(c);let d=e.get(c);(!d||!d.isCompatible(l,this.attributes))&&(d&&d.dispose(),d=new hr,e.set(c,d)),d.updateFrom(l,o)&&this.generateMissingAttributes&&nr(d,this.attributes)}a.forEach(s=>{e.delete(s)})}setObjects(e){Array.isArray(e)?this.objects=[...e]:this.objects=[e]}generate(e=new de){const{useGroups:t,overwriteIndex:a,_intermediateGeometry:o,_geometryMergeSets:s}=this,i=this._getMeshes(),l=[],c=[],d=s.get(e)||[];this._updateIntermediateGeometries();let h=!1;i.length!==d.length&&(h=!0);for(let n=0,g=i.length;n<g;n++){const m=i[n],p=o.get(m.uuid);c.push(p);const u=d[n];!u||u.uuid!==p.uuid?(l.push(!1),h=!0):u.version!==p.version?l.push(!1):l.push(!0)}pr(c,e,{useGroups:t,forceUpdate:h,skipAssigningAttributes:l,overwriteIndex:a}),h&&e.dispose(),s.set(e,c.map(n=>({version:n.version,uuid:n.uuid})));let f=Qe;return h?f=Qt:l.includes(!1)&&(f=Yt),{changeType:f,materials:mr(i),geometry:e}}}function vr(r){const e=new Set;for(let t=0,a=r.length;t<a;t++){const o=r[t];for(const s in o){const i=o[s];i&&i.isTexture&&e.add(i)}}return Array.from(e)}function xr(r){const e=[],t=new Set;for(let o=0,s=r.length;o<s;o++)r[o].traverse(i=>{i.visible&&(i.isRectAreaLight||i.isSpotLight||i.isPointLight||i.isDirectionalLight)&&(e.push(i),i.iesMap&&t.add(i.iesMap))});const a=Array.from(t).sort((o,s)=>o.uuid<s.uuid?1:o.uuid>s.uuid?-1:0);return{lights:e,iesTextures:a}}class br{get initialized(){return!!this.bvh}constructor(e){this.bvhOptions={},this.attributes=["position","normal","tangent","color","uv","uv2"],this.generateBVH=!0,this.bvh=null,this.geometry=new de,this.staticGeometryGenerator=new gr(e),this._bvhWorker=null,this._pendingGenerate=null,this._buildAsync=!1,this._materialUuids=null}setObjects(e){this.staticGeometryGenerator.setObjects(e)}setBVHWorker(e){this._bvhWorker=e}async generateAsync(e=null){if(!this._bvhWorker)throw new Error('PathTracingSceneGenerator: "setBVHWorker" must be called before "generateAsync" can be called.');if(this.bvh instanceof Promise)return this._pendingGenerate||(this._pendingGenerate=new Promise(async()=>(await this.bvh,this._pendingGenerate=null,this.generateAsync(e)))),this._pendingGenerate;{this._buildAsync=!0;const t=this.generate(e);return this._buildAsync=!1,t.bvh=this.bvh=await t.bvh,t}}generate(e=null){const{staticGeometryGenerator:t,geometry:a,attributes:o}=this,s=t.objects;t.attributes=o,s.forEach(n=>{n.traverse(g=>{g.isSkinnedMesh&&g.skeleton&&g.skeleton.update()})});const i=t.generate(a),l=i.materials;let c=i.changeType!==Qe||this._materialUuids===null||this._materialUuids.length!==length;if(!c){for(let n=0,g=l.length;n<g;n++)if(l[n].uuid!==this._materialUuids[n]){c=!0;break}}const d=vr(l),{lights:h,iesTextures:f}=xr(s);if(c&&(sr(a,l,l),this._materialUuids=l.map(n=>n.uuid)),this.generateBVH){if(this.bvh instanceof Promise)throw new Error("PathTracingSceneGenerator: BVH is already building asynchronously.");if(i.changeType===Qt){const n={strategy:Ua,maxLeafTris:1,indirect:!0,onProgress:e,...this.bvhOptions};this._buildAsync?this.bvh=this._bvhWorker.generate(a,n):this.bvh=new Ga(a,n)}else i.changeType===Yt&&this.bvh.refit()}return{bvhChanged:i.changeType!==Qe,bvh:this.bvh,needsMaterialIndexUpdate:c,lights:h,iesTextures:f,geometry:a,materials:l,textures:d,objects:s}}}class et extends Le{set needsUpdate(e){super.needsUpdate=!0,this.dispatchEvent({type:"recompilation"})}constructor(e){super(e);for(const t in this.uniforms)Object.defineProperty(this,t,{get(){return this.uniforms[t].value},set(a){this.uniforms[t].value=a}})}setDefine(e,t=void 0){if(t==null){if(e in this.defines)return delete this.defines[e],this.needsUpdate=!0,!0}else if(this.defines[e]!==t)return this.defines[e]=t,this.needsUpdate=!0,!0;return!1}}class yr extends et{constructor(e){super({blending:Ie,uniforms:{target1:{value:null},target2:{value:null},opacity:{value:1}},vertexShader:`

				varying vec2 vUv;

				void main() {

					vUv = uv;
					gl_Position = projectionMatrix * modelViewMatrix * vec4( position, 1.0 );

				}`,fragmentShader:`

				uniform float opacity;

				uniform sampler2D target1;
				uniform sampler2D target2;

				varying vec2 vUv;

				void main() {

					vec4 color1 = texture2D( target1, vUv );
					vec4 color2 = texture2D( target2, vUv );

					float invOpacity = 1.0 - opacity;
					float totalAlpha = color1.a * invOpacity + color2.a * opacity;

					if ( color1.a != 0.0 || color2.a != 0.0 ) {

						gl_FragColor.rgb = color1.rgb * ( invOpacity * color1.a / totalAlpha ) + color2.rgb * ( opacity * color2.a / totalAlpha );
						gl_FragColor.a = totalAlpha;

					} else {

						gl_FragColor = vec4( 0.0 );

					}

				}`}),this.setValues(e)}}function Oe(r=1){let e="uint";return r>1&&(e="uvec"+r),`
		${e} sobolReverseBits( ${e} x ) {

			x = ( ( ( x & 0xaaaaaaaau ) >> 1 ) | ( ( x & 0x55555555u ) << 1 ) );
			x = ( ( ( x & 0xccccccccu ) >> 2 ) | ( ( x & 0x33333333u ) << 2 ) );
			x = ( ( ( x & 0xf0f0f0f0u ) >> 4 ) | ( ( x & 0x0f0f0f0fu ) << 4 ) );
			x = ( ( ( x & 0xff00ff00u ) >> 8 ) | ( ( x & 0x00ff00ffu ) << 8 ) );
			return ( ( x >> 16 ) | ( x << 16 ) );

		}

		${e} sobolHashCombine( uint seed, ${e} v ) {

			return seed ^ ( v + ${e}( ( seed << 6 ) + ( seed >> 2 ) ) );

		}

		${e} sobolLaineKarrasPermutation( ${e} x, ${e} seed ) {

			x += seed;
			x ^= x * 0x6c50b47cu;
			x ^= x * 0xb82f1e52u;
			x ^= x * 0xc7afe638u;
			x ^= x * 0x8d22f6e6u;
			return x;

		}

		${e} nestedUniformScrambleBase2( ${e} x, ${e} seed ) {

			x = sobolLaineKarrasPermutation( x, seed );
			x = sobolReverseBits( x );
			return x;

		}
	`}function ke(r=1){let e="uint",t="float",a="",o=".r",s="1u";return r>1&&(e="uvec"+r,t="vec"+r,a=r+"",r===2?(o=".rg",s="uvec2( 1u, 2u )"):r===3?(o=".rgb",s="uvec3( 1u, 2u, 3u )"):(o="",s="uvec4( 1u, 2u, 3u, 4u )")),`

		${t} sobol${a}( int effect ) {

			uint seed = sobolGetSeed( sobolBounceIndex, uint( effect ) );
			uint index = sobolPathIndex;

			uint shuffle_seed = sobolHashCombine( seed, 0u );
			uint shuffled_index = nestedUniformScrambleBase2( sobolReverseBits( index ), shuffle_seed );
			${t} sobol_pt = sobolGetTexturePoint( shuffled_index )${o};
			${e} result = ${e}( sobol_pt * 16777216.0 );

			${e} seed2 = sobolHashCombine( seed, ${s} );
			result = nestedUniformScrambleBase2( result, seed2 );

			return SOBOL_FACTOR * ${t}( result >> 8 );

		}
	`}const Xt=`

	// Utils
	const float SOBOL_FACTOR = 1.0 / 16777216.0;
	const uint SOBOL_MAX_POINTS = 256u * 256u;

	${Oe(1)}
	${Oe(2)}
	${Oe(3)}
	${Oe(4)}

	uint sobolHash( uint x ) {

		// finalizer from murmurhash3
		x ^= x >> 16;
		x *= 0x85ebca6bu;
		x ^= x >> 13;
		x *= 0xc2b2ae35u;
		x ^= x >> 16;
		return x;

	}

`,Tr=`

	const uint SOBOL_DIRECTIONS_1[ 32 ] = uint[ 32 ](
		0x80000000u, 0xc0000000u, 0xa0000000u, 0xf0000000u,
		0x88000000u, 0xcc000000u, 0xaa000000u, 0xff000000u,
		0x80800000u, 0xc0c00000u, 0xa0a00000u, 0xf0f00000u,
		0x88880000u, 0xcccc0000u, 0xaaaa0000u, 0xffff0000u,
		0x80008000u, 0xc000c000u, 0xa000a000u, 0xf000f000u,
		0x88008800u, 0xcc00cc00u, 0xaa00aa00u, 0xff00ff00u,
		0x80808080u, 0xc0c0c0c0u, 0xa0a0a0a0u, 0xf0f0f0f0u,
		0x88888888u, 0xccccccccu, 0xaaaaaaaau, 0xffffffffu
	);

	const uint SOBOL_DIRECTIONS_2[ 32 ] = uint[ 32 ](
		0x80000000u, 0xc0000000u, 0x60000000u, 0x90000000u,
		0xe8000000u, 0x5c000000u, 0x8e000000u, 0xc5000000u,
		0x68800000u, 0x9cc00000u, 0xee600000u, 0x55900000u,
		0x80680000u, 0xc09c0000u, 0x60ee0000u, 0x90550000u,
		0xe8808000u, 0x5cc0c000u, 0x8e606000u, 0xc5909000u,
		0x6868e800u, 0x9c9c5c00u, 0xeeee8e00u, 0x5555c500u,
		0x8000e880u, 0xc0005cc0u, 0x60008e60u, 0x9000c590u,
		0xe8006868u, 0x5c009c9cu, 0x8e00eeeeu, 0xc5005555u
	);

	const uint SOBOL_DIRECTIONS_3[ 32 ] = uint[ 32 ](
		0x80000000u, 0xc0000000u, 0x20000000u, 0x50000000u,
		0xf8000000u, 0x74000000u, 0xa2000000u, 0x93000000u,
		0xd8800000u, 0x25400000u, 0x59e00000u, 0xe6d00000u,
		0x78080000u, 0xb40c0000u, 0x82020000u, 0xc3050000u,
		0x208f8000u, 0x51474000u, 0xfbea2000u, 0x75d93000u,
		0xa0858800u, 0x914e5400u, 0xdbe79e00u, 0x25db6d00u,
		0x58800080u, 0xe54000c0u, 0x79e00020u, 0xb6d00050u,
		0x800800f8u, 0xc00c0074u, 0x200200a2u, 0x50050093u
	);

	const uint SOBOL_DIRECTIONS_4[ 32 ] = uint[ 32 ](
		0x80000000u, 0x40000000u, 0x20000000u, 0xb0000000u,
		0xf8000000u, 0xdc000000u, 0x7a000000u, 0x9d000000u,
		0x5a800000u, 0x2fc00000u, 0xa1600000u, 0xf0b00000u,
		0xda880000u, 0x6fc40000u, 0x81620000u, 0x40bb0000u,
		0x22878000u, 0xb3c9c000u, 0xfb65a000u, 0xddb2d000u,
		0x78022800u, 0x9c0b3c00u, 0x5a0fb600u, 0x2d0ddb00u,
		0xa2878080u, 0xf3c9c040u, 0xdb65a020u, 0x6db2d0b0u,
		0x800228f8u, 0x400b3cdcu, 0x200fb67au, 0xb00ddb9du
	);

	uint getMaskedSobol( uint index, uint directions[ 32 ] ) {

		uint X = 0u;
		for ( int bit = 0; bit < 32; bit ++ ) {

			uint mask = ( index >> bit ) & 1u;
			X ^= mask * directions[ bit ];

		}
		return X;

	}

	vec4 generateSobolPoint( uint index ) {

		if ( index >= SOBOL_MAX_POINTS ) {

			return vec4( 0.0 );

		}

		// NOTE: this sobol "direction" is also available but we can't write out 5 components
		// uint x = index & 0x00ffffffu;
		uint x = sobolReverseBits( getMaskedSobol( index, SOBOL_DIRECTIONS_1 ) ) & 0x00ffffffu;
		uint y = sobolReverseBits( getMaskedSobol( index, SOBOL_DIRECTIONS_2 ) ) & 0x00ffffffu;
		uint z = sobolReverseBits( getMaskedSobol( index, SOBOL_DIRECTIONS_3 ) ) & 0x00ffffffu;
		uint w = sobolReverseBits( getMaskedSobol( index, SOBOL_DIRECTIONS_4 ) ) & 0x00ffffffu;

		return vec4( x, y, z, w ) * SOBOL_FACTOR;

	}

`,wr=`

	// Seeds
	uniform sampler2D sobolTexture;
	uint sobolPixelIndex = 0u;
	uint sobolPathIndex = 0u;
	uint sobolBounceIndex = 0u;

	uint sobolGetSeed( uint bounce, uint effect ) {

		return sobolHash(
			sobolHashCombine(
				sobolHashCombine(
					sobolHash( bounce ),
					sobolPixelIndex
				),
				effect
			)
		);

	}

	vec4 sobolGetTexturePoint( uint index ) {

		if ( index >= SOBOL_MAX_POINTS ) {

			index = index % SOBOL_MAX_POINTS;

		}

		uvec2 dim = uvec2( textureSize( sobolTexture, 0 ).xy );
		uint y = index / dim.x;
		uint x = index - y * dim.x;
		vec2 uv = vec2( x, y ) / vec2( dim );
		return texture( sobolTexture, uv );

	}

	${ke(1)}
	${ke(2)}
	${ke(3)}
	${ke(4)}

`;class _r extends et{constructor(){super({blending:Ie,uniforms:{resolution:{value:new oe}},vertexShader:`

				varying vec2 vUv;
				void main() {

					vUv = uv;
					gl_Position = projectionMatrix * modelViewMatrix * vec4( position, 1.0 );

				}
			`,fragmentShader:`

				${Xt}
				${Tr}

				varying vec2 vUv;
				uniform vec2 resolution;
				void main() {

					uint index = uint( gl_FragCoord.y ) * uint( resolution.x ) + uint( gl_FragCoord.x );
					gl_FragColor = generateSobolPoint( index );

				}
			`})}}class Sr{generate(e,t=256){const a=new fe(t,t,{type:M,format:A,minFilter:_,magFilter:_,generateMipmaps:!1}),o=e.getRenderTarget();e.setRenderTarget(a);const s=new pe(new _r);return s.material.resolution.set(t,t),s.render(e),e.setRenderTarget(o),s.dispose(),a}}class Ir extends Ke{set bokehSize(e){this.fStop=this.getFocalLength()/e}get bokehSize(){return this.getFocalLength()/this.fStop}constructor(...e){super(...e),this.fStop=1.4,this.apertureBlades=0,this.apertureRotation=0,this.focusDistance=25,this.anamorphicRatio=1}copy(e,t){return super.copy(e,t),this.fStop=e.fStop,this.apertureBlades=e.apertureBlades,this.apertureRotation=e.apertureRotation,this.focusDistance=e.focusDistance,this.anamorphicRatio=e.anamorphicRatio,this}}class Rr{constructor(){this.bokehSize=0,this.apertureBlades=0,this.apertureRotation=0,this.focusDistance=10,this.anamorphicRatio=1}updateFrom(e){e instanceof Ir?(this.bokehSize=e.bokehSize,this.apertureBlades=e.apertureBlades,this.apertureRotation=e.apertureRotation,this.focusDistance=e.focusDistance,this.anamorphicRatio=e.anamorphicRatio):(this.bokehSize=0,this.apertureRotation=0,this.apertureBlades=0,this.focusDistance=10,this.anamorphicRatio=1)}}function qe(r){const e=new Uint16Array(r.length);for(let t=0,a=r.length;t<a;++t)e[t]=W.toHalfFloat(r[t]);return e}function gt(r,e,t=0,a=r.length){let o=t,s=t+a-1;for(;o<s;){const i=o+s>>1;r[i]<e?o=i+1:s=i}return o-t}function Mr(r,e,t){return .2126*r+.7152*e+.0722*t}function Ar(r,e=B){const t=r.clone();t.source=new ca({...t.image});const{width:a,height:o,data:s}=t.image;let i=s;if(t.type!==e){e===B?i=new Uint16Array(s.length):i=new Float32Array(s.length);let l;s instanceof Int8Array||s instanceof Int16Array||s instanceof Int32Array?l=2**(8*s.BYTES_PER_ELEMENT-1)-1:l=2**(8*s.BYTES_PER_ELEMENT)-1;for(let c=0,d=s.length;c<d;c++){let h=s[c];t.type===B&&(h=W.fromHalfFloat(s[c])),t.type!==M&&t.type!==B&&(h/=l),e===B&&(i[c]=W.toHalfFloat(h))}t.image.data=i,t.type=e}if(t.flipY){const l=i;i=i.slice();for(let c=0;c<o;c++)for(let d=0;d<a;d++){const h=o-c-1,f=4*(c*a+d),n=4*(h*a+d);i[n+0]=l[f+0],i[n+1]=l[f+1],i[n+2]=l[f+2],i[n+3]=l[f+3]}t.flipY=!1,t.image.data=i}return t}class Fr{constructor(){const e=new E(qe(new Float32Array([0,0,0,0])),1,1);e.type=B,e.format=A,e.minFilter=H,e.magFilter=H,e.wrapS=$,e.wrapT=$,e.generateMipmaps=!1,e.needsUpdate=!0;const t=new E(qe(new Float32Array([0,1])),1,2);t.type=B,t.format=Be,t.minFilter=H,t.magFilter=H,t.generateMipmaps=!1,t.needsUpdate=!0;const a=new E(qe(new Float32Array([0,0,1,1])),2,2);a.type=B,a.format=Be,a.minFilter=H,a.magFilter=H,a.generateMipmaps=!1,a.needsUpdate=!0,this.map=e,this.marginalWeights=t,this.conditionalWeights=a,this.totalSum=0}dispose(){this.marginalWeights.dispose(),this.conditionalWeights.dispose(),this.map.dispose()}updateFrom(e){const t=Ar(e);t.wrapS=$,t.wrapT=Q;const{width:a,height:o,data:s}=t.image,i=new Float32Array(a*o),l=new Float32Array(a*o),c=new Float32Array(o),d=new Float32Array(o);let h=0,f=0;for(let u=0;u<o;u++){let v=0;for(let x=0;x<a;x++){const b=u*a+x,T=W.fromHalfFloat(s[4*b+0]),y=W.fromHalfFloat(s[4*b+1]),I=W.fromHalfFloat(s[4*b+2]),O=Mr(T,y,I);v+=O,h+=O,i[b]=O,l[b]=v}if(v!==0)for(let x=u*a,b=u*a+a;x<b;x++)i[x]/=v,l[x]/=v;f+=v,c[u]=v,d[u]=f}if(f!==0)for(let u=0,v=c.length;u<v;u++)c[u]/=f,d[u]/=f;const n=new Uint16Array(o),g=new Uint16Array(a*o);for(let u=0;u<o;u++){const v=(u+1)/o,x=gt(d,v);n[u]=W.toHalfFloat((x+.5)/o)}for(let u=0;u<o;u++)for(let v=0;v<a;v++){const x=u*a+v,b=(v+1)/a,T=gt(l,b,u*a,a);g[x]=W.toHalfFloat((T+.5)/a)}this.dispose();const{marginalWeights:m,conditionalWeights:p}=this;m.image={width:o,height:1,data:n},m.needsUpdate=!0,p.image={width:a,height:o,data:g},p.needsUpdate=!0,this.totalSum=h,this.map=t}}const Ve=6,Cr=0,Dr=1,Pr=2,Er=3,Or=4,z=new P,C=new P,vt=new V,ne=new ua,xt=new P,le=new P,kr=new P(0,1,0);class zr{constructor(){const e=new E(new Float32Array(4),1,1);e.format=A,e.type=M,e.wrapS=Q,e.wrapT=Q,e.generateMipmaps=!1,e.minFilter=_,e.magFilter=_,this.tex=e,this.count=0}updateFrom(e,t=[]){const a=this.tex,o=Math.max(e.length*Ve,1),s=Math.ceil(Math.sqrt(o));a.image.width!==s&&(a.dispose(),a.image.data=new Float32Array(s*s*4),a.image.width=s,a.image.height=s);const i=a.image.data;for(let c=0,d=e.length;c<d;c++){const h=e[c],f=c*Ve*4;let n=0;for(let m=0;m<Ve*4;m++)i[f+m]=0;h.getWorldPosition(C),i[f+n++]=C.x,i[f+n++]=C.y,i[f+n++]=C.z;let g=Cr;if(h.isRectAreaLight&&h.isCircular?g=Dr:h.isSpotLight?g=Pr:h.isDirectionalLight?g=Er:h.isPointLight&&(g=Or),i[f+n++]=g,i[f+n++]=h.color.r,i[f+n++]=h.color.g,i[f+n++]=h.color.b,i[f+n++]=h.intensity,h.getWorldQuaternion(ne),h.isRectAreaLight)z.set(h.width,0,0).applyQuaternion(ne),i[f+n++]=z.x,i[f+n++]=z.y,i[f+n++]=z.z,n++,C.set(0,h.height,0).applyQuaternion(ne),i[f+n++]=C.x,i[f+n++]=C.y,i[f+n++]=C.z,i[f+n++]=z.cross(C).length()*(h.isCircular?Math.PI/4:1);else if(h.isSpotLight){const m=h.radius||0;xt.setFromMatrixPosition(h.matrixWorld),le.setFromMatrixPosition(h.target.matrixWorld),vt.lookAt(xt,le,kr),ne.setFromRotationMatrix(vt),z.set(1,0,0).applyQuaternion(ne),i[f+n++]=z.x,i[f+n++]=z.y,i[f+n++]=z.z,n++,C.set(0,1,0).applyQuaternion(ne),i[f+n++]=C.x,i[f+n++]=C.y,i[f+n++]=C.z,i[f+n++]=Math.PI*m*m,i[f+n++]=m,i[f+n++]=h.decay,i[f+n++]=h.distance,i[f+n++]=Math.cos(h.angle),i[f+n++]=Math.cos(h.angle*(1-h.penumbra)),i[f+n++]=h.iesMap?t.indexOf(h.iesMap):-1}else if(h.isPointLight){const m=z.setFromMatrixPosition(h.matrixWorld);i[f+n++]=m.x,i[f+n++]=m.y,i[f+n++]=m.z,n++,n+=4,n+=1,i[f+n++]=h.decay,i[f+n++]=h.distance}else if(h.isDirectionalLight){const m=z.setFromMatrixPosition(h.matrixWorld),p=C.setFromMatrixPosition(h.target.matrixWorld);le.subVectors(m,p).normalize(),i[f+n++]=le.x,i[f+n++]=le.y,i[f+n++]=le.z}}this.count=e.length;const l=Je(i.buffer);return this.hash!==l?(this.hash=l,a.needsUpdate=!0,!0):!1}}function bt(r,e,t,a,o){if(e>a)throw new Error;const s=r.length/e,i=r.constructor.BYTES_PER_ELEMENT*8;let l=1;switch(r.constructor){case Uint8Array:case Uint16Array:case Uint32Array:l=2**i-1;break;case Int8Array:case Int16Array:case Int32Array:l=2**(i-1)-1;break}for(let c=0;c<s;c++){const d=4*c,h=e*c;for(let f=0;f<a;f++)t[o+d+f]=e>=f+1?r[h+f]/l:0}}class Hr extends fa{constructor(){super(),this._textures=[],this.type=M,this.format=A,this.internalFormat="RGBA32F"}updateAttribute(e,t){const a=this._textures[e];a.updateFrom(t);const o=a.image,s=this.image;if(o.width!==s.width||o.height!==s.height)throw new Error("FloatAttributeTextureArray: Attribute must be the same dimensions when updating single layer.");const{width:i,height:l,data:c}=s,h=i*l*4*e;let f=t.itemSize;f===3&&(f=4),bt(a.image.data,f,c,4,h),this.dispose(),this.needsUpdate=!0}setAttributes(e){const t=e[0].count,a=e.length;for(let f=0,n=a;f<n;f++)if(e[f].count!==t)throw new Error("FloatAttributeTextureArray: All attributes must have the same item count.");const o=this._textures;for(;o.length<a;){const f=new $t;o.push(f)}for(;o.length>a;)o.pop();for(let f=0,n=a;f<n;f++)o[f].updateFrom(e[f]);const i=o[0].image,l=this.image;(i.width!==l.width||i.height!==l.height||i.depth!==a)&&(l.width=i.width,l.height=i.height,l.depth=a,l.data=new Float32Array(l.width*l.height*l.depth*4));const{data:c,width:d,height:h}=l;for(let f=0,n=a;f<n;f++){const g=o[f],p=d*h*4*f;let u=e[f].itemSize;u===3&&(u=4),bt(g.image.data,u,c,4,p)}this.dispose(),this.needsUpdate=!0}}class Br extends Hr{updateNormalAttribute(e){this.updateAttribute(0,e)}updateTangentAttribute(e){this.updateAttribute(1,e)}updateUvAttribute(e){this.updateAttribute(2,e)}updateColorAttribute(e){this.updateAttribute(3,e)}updateFrom(e,t,a,o){this.setAttributes([e,t,a,o])}}function tt(r,e){return r.uuid<e.uuid?1:r.uuid>e.uuid?-1:0}function Xe(r){return`${r.source.uuid}:${r.colorSpace}`}function Nr(r){const e=new Set,t=[];for(let a=0,o=r.length;a<o;a++){const s=r[a],i=Xe(s);e.has(i)||(e.add(i),t.push(s))}return t}function Lr(r){const e=r.map(a=>a.iesMap||null).filter(a=>a),t=new Set(e);return Array.from(t).sort(tt)}function Wr(r){const e=new Set;for(let a=0,o=r.length;a<o;a++){const s=r[a];for(const i in s){const l=s[i];l&&l.isTexture&&e.add(l)}}const t=Array.from(e);return Nr(t).sort(tt)}function Ur(r){const e=[];return r.traverse(t=>{t.visible&&(t.isRectAreaLight||t.isSpotLight||t.isPointLight||t.isDirectionalLight)&&e.push(t)}),e.sort(tt)}const at=47,yt=at*4;class Gr{constructor(){this._features={}}isUsed(e){return e in this._features}setUsed(e,t=!0){t===!1?delete this._features[e]:this._features[e]=!0}reset(){this._features={}}}class qr extends E{constructor(){super(new Float32Array(4),1,1),this.format=A,this.type=M,this.wrapS=Q,this.wrapT=Q,this.minFilter=_,this.magFilter=_,this.generateMipmaps=!1,this.features=new Gr}updateFrom(e,t){function a(m,p,u=-1){if(p in m&&m[p]){const v=Xe(m[p]);return f[v]}else return u}function o(m,p,u){return p in m?m[p]:u}function s(m,p,u,v){const x=m[p]&&m[p].isTexture?m[p]:null;if(x){x.matrixAutoUpdate&&x.updateMatrix();const b=x.matrix.elements;let T=0;u[v+T++]=b[0],u[v+T++]=b[3],u[v+T++]=b[6],T++,u[v+T++]=b[1],u[v+T++]=b[4],u[v+T++]=b[7],T++}return 8}let i=0;const l=e.length*at,c=Math.ceil(Math.sqrt(l))||1,{image:d,features:h}=this,f={};for(let m=0,p=t.length;m<p;m++)f[Xe(t[m])]=m;d.width!==c&&(this.dispose(),d.data=new Float32Array(c*c*4),d.width=c,d.height=c);const n=d.data;h.reset();for(let m=0,p=e.length;m<p;m++){const u=e[m];if(u.isFogVolumeMaterial){h.setUsed("FOG");for(let b=0;b<yt;b++)n[i+b]=0;n[i+0+0]=u.color.r,n[i+0+1]=u.color.g,n[i+0+2]=u.color.b,n[i+8+3]=o(u,"emissiveIntensity",0),n[i+12+0]=u.emissive.r,n[i+12+1]=u.emissive.g,n[i+12+2]=u.emissive.b,n[i+52+1]=u.density,n[i+52+3]=0,n[i+56+2]=4,i+=yt;continue}n[i++]=u.color.r,n[i++]=u.color.g,n[i++]=u.color.b,n[i++]=a(u,"map"),n[i++]=o(u,"metalness",0),n[i++]=a(u,"metalnessMap"),n[i++]=o(u,"roughness",0),n[i++]=a(u,"roughnessMap"),n[i++]=o(u,"ior",1.5),n[i++]=o(u,"transmission",0),n[i++]=a(u,"transmissionMap"),n[i++]=o(u,"emissiveIntensity",0),"emissive"in u?(n[i++]=u.emissive.r,n[i++]=u.emissive.g,n[i++]=u.emissive.b):(n[i++]=0,n[i++]=0,n[i++]=0),n[i++]=a(u,"emissiveMap"),n[i++]=a(u,"normalMap"),"normalScale"in u?(n[i++]=u.normalScale.x,n[i++]=u.normalScale.y):(n[i++]=1,n[i++]=1),n[i++]=o(u,"clearcoat",0),n[i++]=a(u,"clearcoatMap"),n[i++]=o(u,"clearcoatRoughness",0),n[i++]=a(u,"clearcoatRoughnessMap"),n[i++]=a(u,"clearcoatNormalMap"),"clearcoatNormalScale"in u?(n[i++]=u.clearcoatNormalScale.x,n[i++]=u.clearcoatNormalScale.y):(n[i++]=1,n[i++]=1),i++,n[i++]=o(u,"sheen",0),"sheenColor"in u?(n[i++]=u.sheenColor.r,n[i++]=u.sheenColor.g,n[i++]=u.sheenColor.b):(n[i++]=0,n[i++]=0,n[i++]=0),n[i++]=a(u,"sheenColorMap"),n[i++]=o(u,"sheenRoughness",0),n[i++]=a(u,"sheenRoughnessMap"),n[i++]=a(u,"iridescenceMap"),n[i++]=a(u,"iridescenceThicknessMap"),n[i++]=o(u,"iridescence",0),n[i++]=o(u,"iridescenceIOR",1.3);const v=o(u,"iridescenceThicknessRange",[100,400]);n[i++]=v[0],n[i++]=v[1],"specularColor"in u?(n[i++]=u.specularColor.r,n[i++]=u.specularColor.g,n[i++]=u.specularColor.b):(n[i++]=1,n[i++]=1,n[i++]=1),n[i++]=a(u,"specularColorMap"),n[i++]=o(u,"specularIntensity",1),n[i++]=a(u,"specularIntensityMap");const x=o(u,"thickness",0)===0&&o(u,"attenuationDistance",1/0)===1/0;if(n[i++]=Number(x),i++,"attenuationColor"in u?(n[i++]=u.attenuationColor.r,n[i++]=u.attenuationColor.g,n[i++]=u.attenuationColor.b):(n[i++]=1,n[i++]=1,n[i++]=1),n[i++]=o(u,"attenuationDistance",1/0),n[i++]=a(u,"alphaMap"),n[i++]=u.opacity,n[i++]=u.alphaTest,!x&&u.transmission>0)n[i++]=0;else switch(u.side){case da:n[i++]=1;break;case ha:n[i++]=-1;break;case Ht:n[i++]=0;break}n[i++]=Number(o(u,"matte",!1)),n[i++]=Number(o(u,"castShadow",!0)),n[i++]=Number(u.vertexColors)|Number(u.flatShading)<<1,n[i++]=Number(u.transparent),i+=s(u,"map",n,i),i+=s(u,"metalnessMap",n,i),i+=s(u,"roughnessMap",n,i),i+=s(u,"transmissionMap",n,i),i+=s(u,"emissiveMap",n,i),i+=s(u,"normalMap",n,i),i+=s(u,"clearcoatMap",n,i),i+=s(u,"clearcoatNormalMap",n,i),i+=s(u,"clearcoatRoughnessMap",n,i),i+=s(u,"sheenColorMap",n,i),i+=s(u,"sheenRoughnessMap",n,i),i+=s(u,"iridescenceMap",n,i),i+=s(u,"iridescenceThicknessMap",n,i),i+=s(u,"specularColorMap",n,i),i+=s(u,"specularIntensityMap",n,i),i+=s(u,"alphaMap",n,i)}const g=Je(n.buffer);return this.hash!==g?(this.hash=g,this.needsUpdate=!0,!0):!1}}const Tt=new j;function Vr(r){return r?`${r.uuid}:${r.version}`:null}function $r(r,e){for(const t in e)t in r&&(r[t]=e[t])}class wt extends ma{constructor(e,t,a){const o={format:A,type:je,minFilter:H,magFilter:H,wrapS:$,wrapT:$,generateMipmaps:!1,...a};super(e,t,1,o),$r(this.texture,o),this.texture.setTextures=(...i)=>{this.setTextures(...i)},this.hashes=[null];const s=new pe(new jr);this.fsQuad=s}setTextures(e,t,a=this.width,o=this.height){const s=e.getRenderTarget(),i=e.toneMapping,l=e.getClearAlpha();e.getClearColor(Tt);const c=t.length||1;(a!==this.width||o!==this.height||this.depth!==c)&&(this.setSize(a,o,c),this.hashes=new Array(c).fill(null)),e.setClearColor(0,0),e.toneMapping=pa;const d=this.fsQuad,h=this.hashes;let f=!1;for(let n=0,g=c;n<g;n++){const m=t[n],p=Vr(m);m&&(h[n]!==p||m.isWebGLRenderTarget)&&(m.matrixAutoUpdate=!1,m.matrix.identity(),d.material.map=m,e.setRenderTarget(this,n),d.render(e),m.updateMatrix(),m.matrixAutoUpdate=!0,h[n]=p,f=!0)}return d.material.map=null,e.setClearColor(Tt,l),e.setRenderTarget(s),e.toneMapping=i,f}dispose(){super.dispose(),this.fsQuad.dispose()}}class jr extends Le{get map(){return this.uniforms.map.value}set map(e){this.uniforms.map.value=e}constructor(){super({uniforms:{map:{value:null}},vertexShader:`
				varying vec2 vUv;
				void main() {

					vUv = uv;
					gl_Position = projectionMatrix * modelViewMatrix * vec4( position, 1.0 );

				}
			`,fragmentShader:`
				uniform sampler2D map;
				varying vec2 vUv;
				void main() {

					gl_FragColor = texture2D( map, vUv );

				}
			`})}}function Yr(r,e=Math.random()){for(let t=r.length-1;t>0;t--){const a=Math.floor(e()*(t+1)),o=r[t];r[t]=r[a],r[a]=o}return r}class Qr{constructor(e,t,a=Math.random){const o=e**t,s=new Uint16Array(o);let i=o;for(let l=0;l<o;l++)s[l]=l;this.samples=new Float32Array(t),this.strataCount=e,this.reset=function(){for(let l=0;l<o;l++)s[l]=l;i=0},this.reshuffle=function(){i=0},this.next=function(){const{samples:l}=this;i>=s.length&&(Yr(s,a),this.reshuffle());let c=s[i++];for(let d=0;d<t;d++)l[d]=(c%e+a())/e,c=Math.floor(c/e);return l}}}class Xr{constructor(e,t,a=Math.random){let o=0;for(const c of t)o+=c;const s=new Float32Array(o),i=[];let l=0;for(const c of t){const d=new Qr(e,c,a);d.samples=new Float32Array(s.buffer,l,d.samples.length),l+=d.samples.length*4,i.push(d)}this.samples=s,this.strataCount=e,this.next=function(){for(const c of i)c.next();return s},this.reshuffle=function(){for(const c of i)c.reshuffle()},this.reset=function(){for(const c of i)c.reset()}}}class Kr{constructor(e=0){this.m=2147483648,this.a=1103515245,this.c=12345,this.seed=e}nextInt(){return this.seed=(this.a*this.seed+this.c)%this.m,this.seed}nextFloat(){return this.nextInt()/(this.m-1)}}class Zr extends E{constructor(e=1,t=1,a=8){super(new Float32Array(1),1,1,A,M),this.minFilter=_,this.magFilter=_,this.strata=a,this.sampler=null,this.generator=new Kr,this.stableNoise=!1,this.random=()=>this.stableNoise?this.generator.nextFloat():Math.random(),this.init(e,t,a)}init(e=this.image.height,t=this.image.width,a=this.strata){const{image:o}=this;if(o.width===t&&o.height===e&&this.sampler!==null)return;const s=new Array(e*t).fill(4),i=new Xr(a,s,this.random);o.width=t,o.height=e,o.data=i.samples,this.sampler=i,this.dispose(),this.next()}next(){this.sampler.next(),this.needsUpdate=!0}reset(){this.sampler.reset(),this.generator.seed=0}}function Jr(r,e=Math.random){for(let t=r.length-1;t>0;t--){const a=~~((e()-1e-6)*t),o=r[t];r[t]=r[a],r[a]=o}}function ei(r,e){r.fill(0);for(let t=0;t<e;t++)r[t]=1}class _t{constructor(e){this.count=0,this.size=-1,this.sigma=-1,this.radius=-1,this.lookupTable=null,this.score=null,this.binaryPattern=null,this.resize(e),this.setSigma(1.5)}findVoid(){const{score:e,binaryPattern:t}=this;let a=1/0,o=-1;for(let s=0,i=t.length;s<i;s++){if(t[s]!==0)continue;const l=e[s];l<a&&(a=l,o=s)}return o}findCluster(){const{score:e,binaryPattern:t}=this;let a=-1/0,o=-1;for(let s=0,i=t.length;s<i;s++){if(t[s]!==1)continue;const l=e[s];l>a&&(a=l,o=s)}return o}setSigma(e){if(e===this.sigma)return;const t=~~(Math.sqrt(20*e**2)+1),a=2*t+1,o=new Float32Array(a*a),s=e*e;for(let i=-t;i<=t;i++)for(let l=-t;l<=t;l++){const c=(t+l)*a+i+t,d=i*i+l*l;o[c]=Math.E**(-d/(2*s))}this.lookupTable=o,this.sigma=e,this.radius=t}resize(e){this.size!==e&&(this.size=e,this.score=new Float32Array(e*e),this.binaryPattern=new Uint8Array(e*e))}invert(){const{binaryPattern:e,score:t,size:a}=this;t.fill(0);for(let o=0,s=e.length;o<s;o++)if(e[o]===0){const i=~~(o/a),l=o-i*a;this.updateScore(l,i,1),e[o]=1}else e[o]=0}updateScore(e,t,a){const{size:o,score:s,lookupTable:i}=this,l=this.radius,c=2*l+1;for(let d=-l;d<=l;d++)for(let h=-l;h<=l;h++){const f=(l+h)*c+d+l,n=i[f];let g=e+d;g=g<0?o+g:g%o;let m=t+h;m=m<0?o+m:m%o;const p=m*o+g;s[p]+=a*n}}addPointIndex(e){this.binaryPattern[e]=1;const t=this.size,a=~~(e/t),o=e-a*t;this.updateScore(o,a,1),this.count++}removePointIndex(e){this.binaryPattern[e]=0;const t=this.size,a=~~(e/t),o=e-a*t;this.updateScore(o,a,-1),this.count--}copy(e){this.resize(e.size),this.score.set(e.score),this.binaryPattern.set(e.binaryPattern),this.setSigma(e.sigma),this.count=e.count}}class ti{constructor(){this.random=Math.random,this.sigma=1.5,this.size=64,this.majorityPointsRatio=.1,this.samples=new _t(1),this.savedSamples=new _t(1)}generate(){const{samples:e,savedSamples:t,sigma:a,majorityPointsRatio:o,size:s}=this;e.resize(s),e.setSigma(a);const i=Math.floor(s*s*o),l=e.binaryPattern;ei(l,i),Jr(l,this.random);for(let f=0,n=l.length;f<n;f++)l[f]===1&&e.addPointIndex(f);for(;;){const f=e.findCluster();e.removePointIndex(f);const n=e.findVoid();if(f===n){e.addPointIndex(f);break}e.addPointIndex(n)}const c=new Uint32Array(s*s);t.copy(e);let d;for(d=e.count-1;d>=0;){const f=e.findCluster();e.removePointIndex(f),c[f]=d,d--}const h=s*s;for(d=t.count;d<h/2;){const f=t.findVoid();t.addPointIndex(f),c[f]=d,d++}for(t.invert();d<h;){const f=t.findCluster();t.removePointIndex(f),c[f]=d,d++}return{data:c,maxValue:h}}}function ai(r){return r>=3?4:r}function ri(r){switch(r){case 1:return Be;case 2:return zt;default:return A}}class ii extends E{constructor(e=64,t=1){super(new Float32Array(4),1,1,A,M),this.minFilter=_,this.magFilter=_,this.size=e,this.channels=t,this.update()}update(){const e=this.channels,t=this.size,a=new ti;a.channels=e,a.size=t;const o=ai(e),s=ri(o);(this.image.width!==t||s!==this.format)&&(this.image.width=t,this.image.height=t,this.image.data=new Float32Array(t**2*o),this.format=s,this.dispose());const i=this.image.data;for(let l=0,c=e;l<c;l++){const d=a.generate(),h=d.data,f=d.maxValue;for(let n=0,g=h.length;n<g;n++){const m=h[n]/f;i[n*o+l]=m}}this.needsUpdate=!0}}const oi=`

	struct PhysicalCamera {

		float focusDistance;
		float anamorphicRatio;
		float bokehSize;
		int apertureBlades;
		float apertureRotation;

	};

`,si=`

	struct EquirectHdrInfo {

		sampler2D marginalWeights;
		sampler2D conditionalWeights;
		sampler2D map;

		float totalSum;

	};

`,ni=`

	#define RECT_AREA_LIGHT_TYPE 0
	#define CIRC_AREA_LIGHT_TYPE 1
	#define SPOT_LIGHT_TYPE 2
	#define DIR_LIGHT_TYPE 3
	#define POINT_LIGHT_TYPE 4

	struct LightsInfo {

		sampler2D tex;
		uint count;

	};

	struct Light {

		vec3 position;
		int type;

		vec3 color;
		float intensity;

		vec3 u;
		vec3 v;
		float area;

		// spot light fields
		float radius;
		float near;
		float decay;
		float distance;
		float coneCos;
		float penumbraCos;
		int iesProfile;

	};

	Light readLightInfo( sampler2D tex, uint index ) {

		uint i = index * 6u;

		vec4 s0 = texelFetch1D( tex, i + 0u );
		vec4 s1 = texelFetch1D( tex, i + 1u );
		vec4 s2 = texelFetch1D( tex, i + 2u );
		vec4 s3 = texelFetch1D( tex, i + 3u );

		Light l;
		l.position = s0.rgb;
		l.type = int( round( s0.a ) );

		l.color = s1.rgb;
		l.intensity = s1.a;

		l.u = s2.rgb;
		l.v = s3.rgb;
		l.area = s3.a;

		if ( l.type == SPOT_LIGHT_TYPE || l.type == POINT_LIGHT_TYPE ) {

			vec4 s4 = texelFetch1D( tex, i + 4u );
			vec4 s5 = texelFetch1D( tex, i + 5u );
			l.radius = s4.r;
			l.decay = s4.g;
			l.distance = s4.b;
			l.coneCos = s4.a;

			l.penumbraCos = s5.r;
			l.iesProfile = int( round( s5.g ) );

		} else {

			l.radius = 0.0;
			l.decay = 0.0;
			l.distance = 0.0;

			l.coneCos = 0.0;
			l.penumbraCos = 0.0;
			l.iesProfile = - 1;

		}

		return l;

	}

`,li=`

	struct Material {

		vec3 color;
		int map;

		float metalness;
		int metalnessMap;

		float roughness;
		int roughnessMap;

		float ior;
		float transmission;
		int transmissionMap;

		float emissiveIntensity;
		vec3 emissive;
		int emissiveMap;

		int normalMap;
		vec2 normalScale;

		float clearcoat;
		int clearcoatMap;
		int clearcoatNormalMap;
		vec2 clearcoatNormalScale;
		float clearcoatRoughness;
		int clearcoatRoughnessMap;

		int iridescenceMap;
		int iridescenceThicknessMap;
		float iridescence;
		float iridescenceIor;
		float iridescenceThicknessMinimum;
		float iridescenceThicknessMaximum;

		vec3 specularColor;
		int specularColorMap;

		float specularIntensity;
		int specularIntensityMap;
		bool thinFilm;

		vec3 attenuationColor;
		float attenuationDistance;

		int alphaMap;

		bool castShadow;
		float opacity;
		float alphaTest;

		float side;
		bool matte;

		float sheen;
		vec3 sheenColor;
		int sheenColorMap;
		float sheenRoughness;
		int sheenRoughnessMap;

		bool vertexColors;
		bool flatShading;
		bool transparent;
		bool fogVolume;

		mat3 mapTransform;
		mat3 metalnessMapTransform;
		mat3 roughnessMapTransform;
		mat3 transmissionMapTransform;
		mat3 emissiveMapTransform;
		mat3 normalMapTransform;
		mat3 clearcoatMapTransform;
		mat3 clearcoatNormalMapTransform;
		mat3 clearcoatRoughnessMapTransform;
		mat3 sheenColorMapTransform;
		mat3 sheenRoughnessMapTransform;
		mat3 iridescenceMapTransform;
		mat3 iridescenceThicknessMapTransform;
		mat3 specularColorMapTransform;
		mat3 specularIntensityMapTransform;
		mat3 alphaMapTransform;

	};

	mat3 readTextureTransform( sampler2D tex, uint index ) {

		mat3 textureTransform;

		vec4 row1 = texelFetch1D( tex, index );
		vec4 row2 = texelFetch1D( tex, index + 1u );

		textureTransform[0] = vec3(row1.r, row2.r, 0.0);
		textureTransform[1] = vec3(row1.g, row2.g, 0.0);
		textureTransform[2] = vec3(row1.b, row2.b, 1.0);

		return textureTransform;

	}

	Material readMaterialInfo( sampler2D tex, uint index ) {

		uint i = index * uint( MATERIAL_PIXELS );

		vec4 s0 = texelFetch1D( tex, i + 0u );
		vec4 s1 = texelFetch1D( tex, i + 1u );
		vec4 s2 = texelFetch1D( tex, i + 2u );
		vec4 s3 = texelFetch1D( tex, i + 3u );
		vec4 s4 = texelFetch1D( tex, i + 4u );
		vec4 s5 = texelFetch1D( tex, i + 5u );
		vec4 s6 = texelFetch1D( tex, i + 6u );
		vec4 s7 = texelFetch1D( tex, i + 7u );
		vec4 s8 = texelFetch1D( tex, i + 8u );
		vec4 s9 = texelFetch1D( tex, i + 9u );
		vec4 s10 = texelFetch1D( tex, i + 10u );
		vec4 s11 = texelFetch1D( tex, i + 11u );
		vec4 s12 = texelFetch1D( tex, i + 12u );
		vec4 s13 = texelFetch1D( tex, i + 13u );
		vec4 s14 = texelFetch1D( tex, i + 14u );

		Material m;
		m.color = s0.rgb;
		m.map = int( round( s0.a ) );

		m.metalness = s1.r;
		m.metalnessMap = int( round( s1.g ) );
		m.roughness = s1.b;
		m.roughnessMap = int( round( s1.a ) );

		m.ior = s2.r;
		m.transmission = s2.g;
		m.transmissionMap = int( round( s2.b ) );
		m.emissiveIntensity = s2.a;

		m.emissive = s3.rgb;
		m.emissiveMap = int( round( s3.a ) );

		m.normalMap = int( round( s4.r ) );
		m.normalScale = s4.gb;

		m.clearcoat = s4.a;
		m.clearcoatMap = int( round( s5.r ) );
		m.clearcoatRoughness = s5.g;
		m.clearcoatRoughnessMap = int( round( s5.b ) );
		m.clearcoatNormalMap = int( round( s5.a ) );
		m.clearcoatNormalScale = s6.rg;

		m.sheen = s6.a;
		m.sheenColor = s7.rgb;
		m.sheenColorMap = int( round( s7.a ) );
		m.sheenRoughness = s8.r;
		m.sheenRoughnessMap = int( round( s8.g ) );

		m.iridescenceMap = int( round( s8.b ) );
		m.iridescenceThicknessMap = int( round( s8.a ) );
		m.iridescence = s9.r;
		m.iridescenceIor = s9.g;
		m.iridescenceThicknessMinimum = s9.b;
		m.iridescenceThicknessMaximum = s9.a;

		m.specularColor = s10.rgb;
		m.specularColorMap = int( round( s10.a ) );

		m.specularIntensity = s11.r;
		m.specularIntensityMap = int( round( s11.g ) );
		m.thinFilm = bool( s11.b );

		m.attenuationColor = s12.rgb;
		m.attenuationDistance = s12.a;

		m.alphaMap = int( round( s13.r ) );

		m.opacity = s13.g;
		m.alphaTest = s13.b;
		m.side = s13.a;

		m.matte = bool( s14.r );
		m.castShadow = bool( s14.g );
		m.vertexColors = bool( int( s14.b ) & 1 );
		m.flatShading = bool( int( s14.b ) & 2 );
		m.fogVolume = bool( int( s14.b ) & 4 );
		m.transparent = bool( s14.a );

		uint firstTextureTransformIdx = i + 15u;

		// mat3( 1.0 ) is an identity matrix
		m.mapTransform = m.map == - 1 ? mat3( 1.0 ) : readTextureTransform( tex, firstTextureTransformIdx );
		m.metalnessMapTransform = m.metalnessMap == - 1 ? mat3( 1.0 ) : readTextureTransform( tex, firstTextureTransformIdx + 2u );
		m.roughnessMapTransform = m.roughnessMap == - 1 ? mat3( 1.0 ) : readTextureTransform( tex, firstTextureTransformIdx + 4u );
		m.transmissionMapTransform = m.transmissionMap == - 1 ? mat3( 1.0 ) : readTextureTransform( tex, firstTextureTransformIdx + 6u );
		m.emissiveMapTransform = m.emissiveMap == - 1 ? mat3( 1.0 ) : readTextureTransform( tex, firstTextureTransformIdx + 8u );
		m.normalMapTransform = m.normalMap == - 1 ? mat3( 1.0 ) : readTextureTransform( tex, firstTextureTransformIdx + 10u );
		m.clearcoatMapTransform = m.clearcoatMap == - 1 ? mat3( 1.0 ) : readTextureTransform( tex, firstTextureTransformIdx + 12u );
		m.clearcoatNormalMapTransform = m.clearcoatNormalMap == - 1 ? mat3( 1.0 ) : readTextureTransform( tex, firstTextureTransformIdx + 14u );
		m.clearcoatRoughnessMapTransform = m.clearcoatRoughnessMap == - 1 ? mat3( 1.0 ) : readTextureTransform( tex, firstTextureTransformIdx + 16u );
		m.sheenColorMapTransform = m.sheenColorMap == - 1 ? mat3( 1.0 ) : readTextureTransform( tex, firstTextureTransformIdx + 18u );
		m.sheenRoughnessMapTransform = m.sheenRoughnessMap == - 1 ? mat3( 1.0 ) : readTextureTransform( tex, firstTextureTransformIdx + 20u );
		m.iridescenceMapTransform = m.iridescenceMap == - 1 ? mat3( 1.0 ) : readTextureTransform( tex, firstTextureTransformIdx + 22u );
		m.iridescenceThicknessMapTransform = m.iridescenceThicknessMap == - 1 ? mat3( 1.0 ) : readTextureTransform( tex, firstTextureTransformIdx + 24u );
		m.specularColorMapTransform = m.specularColorMap == - 1 ? mat3( 1.0 ) : readTextureTransform( tex, firstTextureTransformIdx + 26u );
		m.specularIntensityMapTransform = m.specularIntensityMap == - 1 ? mat3( 1.0 ) : readTextureTransform( tex, firstTextureTransformIdx + 28u );
		m.alphaMapTransform = m.alphaMap == - 1 ? mat3( 1.0 ) : readTextureTransform( tex, firstTextureTransformIdx + 30u );

		return m;

	}

`,ci=`

	struct SurfaceRecord {

		// surface type
		bool volumeParticle;

		// geometry
		vec3 faceNormal;
		bool frontFace;
		vec3 normal;
		mat3 normalBasis;
		mat3 normalInvBasis;

		// cached properties
		float eta;
		float f0;

		// material
		float roughness;
		float filteredRoughness;
		float metalness;
		vec3 color;
		vec3 emission;

		// transmission
		float ior;
		float transmission;
		bool thinFilm;
		vec3 attenuationColor;
		float attenuationDistance;

		// clearcoat
		vec3 clearcoatNormal;
		mat3 clearcoatBasis;
		mat3 clearcoatInvBasis;
		float clearcoat;
		float clearcoatRoughness;
		float filteredClearcoatRoughness;

		// sheen
		float sheen;
		vec3 sheenColor;
		float sheenRoughness;

		// iridescence
		float iridescence;
		float iridescenceIor;
		float iridescenceThickness;

		// specular
		vec3 specularColor;
		float specularIntensity;
	};

	struct ScatterRecord {
		float specularPdf;
		float pdf;
		vec3 direction;
		vec3 color;
	};

`,ui=`

	// samples the the given environment map in the given direction
	vec3 sampleEquirectColor( sampler2D envMap, vec3 direction ) {

		return texture2D( envMap, equirectDirectionToUv( direction ) ).rgb;

	}

	// gets the pdf of the given direction to sample
	float equirectDirectionPdf( vec3 direction ) {

		vec2 uv = equirectDirectionToUv( direction );
		float theta = uv.y * PI;
		float sinTheta = sin( theta );
		if ( sinTheta == 0.0 ) {

			return 0.0;

		}

		return 1.0 / ( 2.0 * PI * PI * sinTheta );

	}

	// samples the color given env map with CDF and returns the pdf of the direction
	float sampleEquirect( vec3 direction, inout vec3 color ) {

		float totalSum = envMapInfo.totalSum;
		if ( totalSum == 0.0 ) {

			color = vec3( 0.0 );
			return 1.0;

		}

		vec2 uv = equirectDirectionToUv( direction );
		color = texture2D( envMapInfo.map, uv ).rgb;

		float lum = luminance( color );
		ivec2 resolution = textureSize( envMapInfo.map, 0 );
		float pdf = lum / totalSum;

		return float( resolution.x * resolution.y ) * pdf * equirectDirectionPdf( direction );

	}

	// samples a direction of the envmap with color and retrieves pdf
	float sampleEquirectProbability( vec2 r, inout vec3 color, inout vec3 direction ) {

		// sample env map cdf
		float v = texture2D( envMapInfo.marginalWeights, vec2( r.x, 0.0 ) ).x;
		float u = texture2D( envMapInfo.conditionalWeights, vec2( r.y, v ) ).x;
		vec2 uv = vec2( u, v );

		vec3 derivedDirection = equirectUvToDirection( uv );
		direction = derivedDirection;
		color = texture2D( envMapInfo.map, uv ).rgb;

		float totalSum = envMapInfo.totalSum;
		float lum = luminance( color );
		ivec2 resolution = textureSize( envMapInfo.map, 0 );
		float pdf = lum / totalSum;

		return float( resolution.x * resolution.y ) * pdf * equirectDirectionPdf( direction );

	}
`,fi=`

	float getSpotAttenuation( const in float coneCosine, const in float penumbraCosine, const in float angleCosine ) {

		return smoothstep( coneCosine, penumbraCosine, angleCosine );

	}

	float getDistanceAttenuation( const in float lightDistance, const in float cutoffDistance, const in float decayExponent ) {

		// based upon Frostbite 3 Moving to Physically-based Rendering
		// page 32, equation 26: E[window1]
		// https://seblagarde.files.wordpress.com/2015/07/course_notes_moving_frostbite_to_pbr_v32.pdf
		float distanceFalloff = 1.0 / max( pow( lightDistance, decayExponent ), EPSILON );

		if ( cutoffDistance > 0.0 ) {

			distanceFalloff *= pow2( saturate( 1.0 - pow4( lightDistance / cutoffDistance ) ) );

		}

		return distanceFalloff;

	}

	float getPhotometricAttenuation( sampler2DArray iesProfiles, int iesProfile, vec3 posToLight, vec3 lightDir, vec3 u, vec3 v ) {

		float cosTheta = dot( posToLight, lightDir );
		float angle = acos( cosTheta ) / PI;

		return texture2D( iesProfiles, vec3( angle, 0.0, iesProfile ) ).r;

	}

	struct LightRecord {

		float dist;
		vec3 direction;
		float pdf;
		vec3 emission;
		int type;

	};

	bool intersectLightAtIndex( sampler2D lights, vec3 rayOrigin, vec3 rayDirection, uint l, inout LightRecord lightRec ) {

		bool didHit = false;
		Light light = readLightInfo( lights, l );

		vec3 u = light.u;
		vec3 v = light.v;

		// check for backface
		vec3 normal = normalize( cross( u, v ) );
		if ( dot( normal, rayDirection ) > 0.0 ) {

			u *= 1.0 / dot( u, u );
			v *= 1.0 / dot( v, v );

			float dist;

			// MIS / light intersection is not supported for punctual lights.
			if(
				( light.type == RECT_AREA_LIGHT_TYPE && intersectsRectangle( light.position, normal, u, v, rayOrigin, rayDirection, dist ) ) ||
				( light.type == CIRC_AREA_LIGHT_TYPE && intersectsCircle( light.position, normal, u, v, rayOrigin, rayDirection, dist ) )
			) {

				float cosTheta = dot( rayDirection, normal );
				didHit = true;
				lightRec.dist = dist;
				lightRec.pdf = ( dist * dist ) / ( light.area * cosTheta );
				lightRec.emission = light.color * light.intensity;
				lightRec.direction = rayDirection;
				lightRec.type = light.type;

			}

		}

		return didHit;

	}

	LightRecord randomAreaLightSample( Light light, vec3 rayOrigin, vec2 ruv ) {

		vec3 randomPos;
		if( light.type == RECT_AREA_LIGHT_TYPE ) {

			// rectangular area light
			randomPos = light.position + light.u * ( ruv.x - 0.5 ) + light.v * ( ruv.y - 0.5 );

		} else if( light.type == CIRC_AREA_LIGHT_TYPE ) {

			// circular area light
			float r = 0.5 * sqrt( ruv.x );
			float theta = ruv.y * 2.0 * PI;
			float x = r * cos( theta );
			float y = r * sin( theta );

			randomPos = light.position + light.u * x + light.v * y;

		}

		vec3 toLight = randomPos - rayOrigin;
		float lightDistSq = dot( toLight, toLight );
		float dist = sqrt( lightDistSq );
		vec3 direction = toLight / dist;
		vec3 lightNormal = normalize( cross( light.u, light.v ) );

		LightRecord lightRec;
		lightRec.type = light.type;
		lightRec.emission = light.color * light.intensity;
		lightRec.dist = dist;
		lightRec.direction = direction;

		// TODO: the denominator is potentially zero
		lightRec.pdf = lightDistSq / ( light.area * dot( direction, lightNormal ) );

		return lightRec;

	}

	LightRecord randomSpotLightSample( Light light, sampler2DArray iesProfiles, vec3 rayOrigin, vec2 ruv ) {

		float radius = light.radius * sqrt( ruv.x );
		float theta = ruv.y * 2.0 * PI;
		float x = radius * cos( theta );
		float y = radius * sin( theta );

		vec3 u = light.u;
		vec3 v = light.v;
		vec3 normal = normalize( cross( u, v ) );

		float angle = acos( light.coneCos );
		float angleTan = tan( angle );
		float startDistance = light.radius / max( angleTan, EPSILON );

		vec3 randomPos = light.position - normal * startDistance + u * x + v * y;
		vec3 toLight = randomPos - rayOrigin;
		float lightDistSq = dot( toLight, toLight );
		float dist = sqrt( lightDistSq );

		vec3 direction = toLight / max( dist, EPSILON );
		float cosTheta = dot( direction, normal );

		float spotAttenuation = light.iesProfile != - 1 ?
			getPhotometricAttenuation( iesProfiles, light.iesProfile, direction, normal, u, v ) :
			getSpotAttenuation( light.coneCos, light.penumbraCos, cosTheta );

		float distanceAttenuation = getDistanceAttenuation( dist, light.distance, light.decay );
		LightRecord lightRec;
		lightRec.type = light.type;
		lightRec.dist = dist;
		lightRec.direction = direction;
		lightRec.emission = light.color * light.intensity * distanceAttenuation * spotAttenuation;
		lightRec.pdf = 1.0;

		return lightRec;

	}

	LightRecord randomLightSample( sampler2D lights, sampler2DArray iesProfiles, uint lightCount, vec3 rayOrigin, vec3 ruv ) {

		LightRecord result;

		// pick a random light
		uint l = uint( ruv.x * float( lightCount ) );
		Light light = readLightInfo( lights, l );

		if ( light.type == SPOT_LIGHT_TYPE ) {

			result = randomSpotLightSample( light, iesProfiles, rayOrigin, ruv.yz );

		} else if ( light.type == POINT_LIGHT_TYPE ) {

			vec3 lightRay = light.u - rayOrigin;
			float lightDist = length( lightRay );
			float cutoffDistance = light.distance;
			float distanceFalloff = 1.0 / max( pow( lightDist, light.decay ), 0.01 );
			if ( cutoffDistance > 0.0 ) {

				distanceFalloff *= pow2( saturate( 1.0 - pow4( lightDist / cutoffDistance ) ) );

			}

			LightRecord rec;
			rec.direction = normalize( lightRay );
			rec.dist = length( lightRay );
			rec.pdf = 1.0;
			rec.emission = light.color * light.intensity * distanceFalloff;
			rec.type = light.type;
			result = rec;

		} else if ( light.type == DIR_LIGHT_TYPE ) {

			LightRecord rec;
			rec.dist = 1e10;
			rec.direction = light.u;
			rec.pdf = 1.0;
			rec.emission = light.color * light.intensity;
			rec.type = light.type;

			result = rec;

		} else {

			// sample the light
			result = randomAreaLightSample( light, rayOrigin, ruv.yz );

		}

		return result;

	}

`,hi=`

	vec3 sampleHemisphere( vec3 n, vec2 uv ) {

		// https://www.rorydriscoll.com/2009/01/07/better-sampling/
		// https://graphics.pixar.com/library/OrthonormalB/paper.pdf
		float sign = n.z == 0.0 ? 1.0 : sign( n.z );
		float a = - 1.0 / ( sign + n.z );
		float b = n.x * n.y * a;
		vec3 b1 = vec3( 1.0 + sign * n.x * n.x * a, sign * b, - sign * n.x );
		vec3 b2 = vec3( b, sign + n.y * n.y * a, - n.y );

		float r = sqrt( uv.x );
		float theta = 2.0 * PI * uv.y;
		float x = r * cos( theta );
		float y = r * sin( theta );
		return x * b1 + y * b2 + sqrt( 1.0 - uv.x ) * n;

	}

	vec2 sampleTriangle( vec2 a, vec2 b, vec2 c, vec2 r ) {

		// get the edges of the triangle and the diagonal across the
		// center of the parallelogram
		vec2 e1 = a - b;
		vec2 e2 = c - b;
		vec2 diag = normalize( e1 + e2 );

		// pick the point in the parallelogram
		if ( r.x + r.y > 1.0 ) {

			r = vec2( 1.0 ) - r;

		}

		return e1 * r.x + e2 * r.y;

	}

	vec2 sampleCircle( vec2 uv ) {

		float angle = 2.0 * PI * uv.x;
		float radius = sqrt( uv.y );
		return vec2( cos( angle ), sin( angle ) ) * radius;

	}

	vec3 sampleSphere( vec2 uv ) {

		float u = ( uv.x - 0.5 ) * 2.0;
		float t = uv.y * PI * 2.0;
		float f = sqrt( 1.0 - u * u );

		return vec3( f * cos( t ), f * sin( t ), u );

	}

	vec2 sampleRegularPolygon( int sides, vec3 uvw ) {

		sides = max( sides, 3 );

		vec3 r = uvw;
		float anglePerSegment = 2.0 * PI / float( sides );
		float segment = floor( float( sides ) * r.x );

		float angle1 = anglePerSegment * segment;
		float angle2 = angle1 + anglePerSegment;
		vec2 a = vec2( sin( angle1 ), cos( angle1 ) );
		vec2 b = vec2( 0.0, 0.0 );
		vec2 c = vec2( sin( angle2 ), cos( angle2 ) );

		return sampleTriangle( a, b, c, r.yz );

	}

	// samples an aperture shape with the given number of sides. 0 means circle
	vec2 sampleAperture( int blades, vec3 uvw ) {

		return blades == 0 ?
			sampleCircle( uvw.xy ) :
			sampleRegularPolygon( blades, uvw );

	}


`,di=`

	bool totalInternalReflection( float cosTheta, float eta ) {

		float sinTheta = sqrt( 1.0 - cosTheta * cosTheta );
		return eta * sinTheta > 1.0;

	}

	// https://google.github.io/filament/Filament.md.html#materialsystem/diffusebrdf
	float schlickFresnel( float cosine, float f0 ) {

		return f0 + ( 1.0 - f0 ) * pow( 1.0 - cosine, 5.0 );

	}

	vec3 schlickFresnel( float cosine, vec3 f0 ) {

		return f0 + ( 1.0 - f0 ) * pow( 1.0 - cosine, 5.0 );

	}

	vec3 schlickFresnel( float cosine, vec3 f0, vec3 f90 ) {

		return f0 + ( f90 - f0 ) * pow( 1.0 - cosine, 5.0 );

	}

	float dielectricFresnel( float cosThetaI, float eta ) {

		// https://schuttejoe.github.io/post/disneybsdf/
		float ni = eta;
		float nt = 1.0;

		// Check for total internal reflection
		float sinThetaISq = 1.0f - cosThetaI * cosThetaI;
		float sinThetaTSq = eta * eta * sinThetaISq;
		if( sinThetaTSq >= 1.0 ) {

			return 1.0;

		}

		float sinThetaT = sqrt( sinThetaTSq );

		float cosThetaT = sqrt( max( 0.0, 1.0f - sinThetaT * sinThetaT ) );
		float rParallel = ( ( nt * cosThetaI ) - ( ni * cosThetaT ) ) / ( ( nt * cosThetaI ) + ( ni * cosThetaT ) );
		float rPerpendicular = ( ( ni * cosThetaI ) - ( nt * cosThetaT ) ) / ( ( ni * cosThetaI ) + ( nt * cosThetaT ) );
		return ( rParallel * rParallel + rPerpendicular * rPerpendicular ) / 2.0;

	}

	// https://raytracing.github.io/books/RayTracingInOneWeekend.html#dielectrics/schlickapproximation
	float iorRatioToF0( float eta ) {

		return pow( ( 1.0 - eta ) / ( 1.0 + eta ), 2.0 );

	}

	vec3 evaluateFresnel( float cosTheta, float eta, vec3 f0, vec3 f90 ) {

		if ( totalInternalReflection( cosTheta, eta ) ) {

			return f90;

		}

		return schlickFresnel( cosTheta, f0, f90 );

	}

	// TODO: disney fresnel was removed and replaced with this fresnel function to better align with
	// the glTF but is causing blown out pixels. Should be revisited
	// float evaluateFresnelWeight( float cosTheta, float eta, float f0 ) {

	// 	if ( totalInternalReflection( cosTheta, eta ) ) {

	// 		return 1.0;

	// 	}

	// 	return schlickFresnel( cosTheta, f0 );

	// }

	// https://schuttejoe.github.io/post/disneybsdf/
	float disneyFresnel( vec3 wo, vec3 wi, vec3 wh, float f0, float eta, float metalness ) {

		float dotHV = dot( wo, wh );
		if ( totalInternalReflection( dotHV, eta ) ) {

			return 1.0;

		}

		float dotHL = dot( wi, wh );
		float dielectricFresnel = dielectricFresnel( abs( dotHV ), eta );
		float metallicFresnel = schlickFresnel( dotHL, f0 );

		return mix( dielectricFresnel, metallicFresnel, metalness );

	}

`,mi=`

	// Fast arccos approximation used to remove banding artifacts caused by numerical errors in acos.
	// This is a cubic Lagrange interpolating polynomial for x = [-1, -1/2, 0, 1/2, 1].
	// For more information see: https://github.com/gkjohnson/three-gpu-pathtracer/pull/171#issuecomment-1152275248
	float acosApprox( float x ) {

		x = clamp( x, -1.0, 1.0 );
		return ( - 0.69813170079773212 * x * x - 0.87266462599716477 ) * x + 1.5707963267948966;

	}

	// An acos with input values bound to the range [-1, 1].
	float acosSafe( float x ) {

		return acos( clamp( x, -1.0, 1.0 ) );

	}

	float saturateCos( float val ) {

		return clamp( val, 0.001, 1.0 );

	}

	float square( float t ) {

		return t * t;

	}

	vec2 square( vec2 t ) {

		return t * t;

	}

	vec3 square( vec3 t ) {

		return t * t;

	}

	vec4 square( vec4 t ) {

		return t * t;

	}

	vec2 rotateVector( vec2 v, float t ) {

		float ac = cos( t );
		float as = sin( t );
		return vec2(
			v.x * ac - v.y * as,
			v.x * as + v.y * ac
		);

	}

	// forms a basis with the normal vector as Z
	mat3 getBasisFromNormal( vec3 normal ) {

		vec3 other;
		if ( abs( normal.x ) > 0.5 ) {

			other = vec3( 0.0, 1.0, 0.0 );

		} else {

			other = vec3( 1.0, 0.0, 0.0 );

		}

		vec3 ortho = normalize( cross( normal, other ) );
		vec3 ortho2 = normalize( cross( normal, ortho ) );
		return mat3( ortho2, ortho, normal );

	}

`,pi=`

	// Finds the point where the ray intersects the plane defined by u and v and checks if this point
	// falls in the bounds of the rectangle on that same plane.
	// Plane intersection: https://lousodrome.net/blog/light/2020/07/03/intersection-of-a-ray-and-a-plane/
	bool intersectsRectangle( vec3 center, vec3 normal, vec3 u, vec3 v, vec3 rayOrigin, vec3 rayDirection, inout float dist ) {

		float t = dot( center - rayOrigin, normal ) / dot( rayDirection, normal );

		if ( t > EPSILON ) {

			vec3 p = rayOrigin + rayDirection * t;
			vec3 vi = p - center;

			// check if p falls inside the rectangle
			float a1 = dot( u, vi );
			if ( abs( a1 ) <= 0.5 ) {

				float a2 = dot( v, vi );
				if ( abs( a2 ) <= 0.5 ) {

					dist = t;
					return true;

				}

			}

		}

		return false;

	}

	// Finds the point where the ray intersects the plane defined by u and v and checks if this point
	// falls in the bounds of the circle on that same plane. See above URL for a description of the plane intersection algorithm.
	bool intersectsCircle( vec3 position, vec3 normal, vec3 u, vec3 v, vec3 rayOrigin, vec3 rayDirection, inout float dist ) {

		float t = dot( position - rayOrigin, normal ) / dot( rayDirection, normal );

		if ( t > EPSILON ) {

			vec3 hit = rayOrigin + rayDirection * t;
			vec3 vi = hit - position;

			float a1 = dot( u, vi );
			float a2 = dot( v, vi );

			if( length( vec2( a1, a2 ) ) <= 0.5 ) {

				dist = t;
				return true;

			}

		}

		return false;

	}

`,gi=`

	// add texel fetch functions for texture arrays
	vec4 texelFetch1D( sampler2DArray tex, int layer, uint index ) {

		uint width = uint( textureSize( tex, 0 ).x );
		uvec2 uv;
		uv.x = index % width;
		uv.y = index / width;

		return texelFetch( tex, ivec3( uv, layer ), 0 );

	}

	vec4 textureSampleBarycoord( sampler2DArray tex, int layer, vec3 barycoord, uvec3 faceIndices ) {

		return
			barycoord.x * texelFetch1D( tex, layer, faceIndices.x ) +
			barycoord.y * texelFetch1D( tex, layer, faceIndices.y ) +
			barycoord.z * texelFetch1D( tex, layer, faceIndices.z );

	}

`,Kt=`

	// TODO: possibly this should be renamed something related to material or path tracing logic

	#ifndef RAY_OFFSET
	#define RAY_OFFSET 1e-4
	#endif

	// adjust the hit point by the surface normal by a factor of some offset and the
	// maximum component-wise value of the current point to accommodate floating point
	// error as values increase.
	vec3 stepRayOrigin( vec3 rayOrigin, vec3 rayDirection, vec3 offset, float dist ) {

		vec3 point = rayOrigin + rayDirection * dist;
		vec3 absPoint = abs( point );
		float maxPoint = max( absPoint.x, max( absPoint.y, absPoint.z ) );
		return point + offset * ( maxPoint + 1.0 ) * RAY_OFFSET;

	}

	// https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_volume/README.md#attenuation
	vec3 transmissionAttenuation( float dist, vec3 attColor, float attDist ) {

		vec3 ot = - log( attColor ) / attDist;
		return exp( - ot * dist );

	}

	vec3 getHalfVector( vec3 wi, vec3 wo, float eta ) {

		// get the half vector - assuming if the light incident vector is on the other side
		// of the that it's transmissive.
		vec3 h;
		if ( wi.z > 0.0 ) {

			h = normalize( wi + wo );

		} else {

			// Scale by the ior ratio to retrieve the appropriate half vector
			// From Section 2.2 on computing the transmission half vector:
			// https://blog.selfshadow.com/publications/s2015-shading-course/burley/s2015_pbs_disney_bsdf_notes.pdf
			h = normalize( wi + wo * eta );

		}

		h *= sign( h.z );
		return h;

	}

	vec3 getHalfVector( vec3 a, vec3 b ) {

		return normalize( a + b );

	}

	// The discrepancy between interpolated surface normal and geometry normal can cause issues when a ray
	// is cast that is on the top side of the geometry normal plane but below the surface normal plane. If
	// we find a ray like that we ignore it to avoid artifacts.
	// This function returns if the direction is on the same side of both planes.
	bool isDirectionValid( vec3 direction, vec3 surfaceNormal, vec3 geometryNormal ) {

		bool aboveSurfaceNormal = dot( direction, surfaceNormal ) > 0.0;
		bool aboveGeometryNormal = dot( direction, geometryNormal ) > 0.0;
		return aboveSurfaceNormal == aboveGeometryNormal;

	}

	// ray sampling x and z are swapped to align with expected background view
	vec2 equirectDirectionToUv( vec3 direction ) {

		// from Spherical.setFromCartesianCoords
		vec2 uv = vec2( atan( direction.z, direction.x ), acos( direction.y ) );
		uv /= vec2( 2.0 * PI, PI );

		// apply adjustments to get values in range [0, 1] and y right side up
		uv.x += 0.5;
		uv.y = 1.0 - uv.y;
		return uv;

	}

	vec3 equirectUvToDirection( vec2 uv ) {

		// undo above adjustments
		uv.x -= 0.5;
		uv.y = 1.0 - uv.y;

		// from Vector3.setFromSphericalCoords
		float theta = uv.x * 2.0 * PI;
		float phi = uv.y * PI;

		float sinPhi = sin( phi );

		return vec3( sinPhi * cos( theta ), cos( phi ), sinPhi * sin( theta ) );

	}

	// power heuristic for multiple importance sampling
	float misHeuristic( float a, float b ) {

		float aa = a * a;
		float bb = b * b;
		return aa / ( aa + bb );

	}

	// tentFilter from Peter Shirley's 'Realistic Ray Tracing (2nd Edition)' book, pg. 60
	// erichlof/THREE.js-PathTracing-Renderer/
	float tentFilter( float x ) {

		return x < 0.5 ? sqrt( 2.0 * x ) - 1.0 : 1.0 - sqrt( 2.0 - ( 2.0 * x ) );

	}
`,St=`

	// https://www.shadertoy.com/view/wltcRS
	uvec4 WHITE_NOISE_SEED;

	void rng_initialize( vec2 p, int frame ) {

		// white noise seed
		WHITE_NOISE_SEED = uvec4( p, uint( frame ), uint( p.x ) + uint( p.y ) );

	}

	// https://www.pcg-random.org/
	void pcg4d( inout uvec4 v ) {

		v = v * 1664525u + 1013904223u;
		v.x += v.y * v.w;
		v.y += v.z * v.x;
		v.z += v.x * v.y;
		v.w += v.y * v.z;
		v = v ^ ( v >> 16u );
		v.x += v.y*v.w;
		v.y += v.z*v.x;
		v.z += v.x*v.y;
		v.w += v.y*v.z;

	}

	// returns [ 0, 1 ]
	float pcgRand() {

		pcg4d( WHITE_NOISE_SEED );
		return float( WHITE_NOISE_SEED.x ) / float( 0xffffffffu );

	}

	vec2 pcgRand2() {

		pcg4d( WHITE_NOISE_SEED );
		return vec2( WHITE_NOISE_SEED.xy ) / float(0xffffffffu);

	}

	vec3 pcgRand3() {

		pcg4d( WHITE_NOISE_SEED );
		return vec3( WHITE_NOISE_SEED.xyz ) / float( 0xffffffffu );

	}

	vec4 pcgRand4() {

		pcg4d( WHITE_NOISE_SEED );
		return vec4( WHITE_NOISE_SEED ) / float( 0xffffffffu );

	}
`,vi=`

	uniform sampler2D stratifiedTexture;
	uniform sampler2D stratifiedOffsetTexture;

	uint sobolPixelIndex = 0u;
	uint sobolPathIndex = 0u;
	uint sobolBounceIndex = 0u;
	vec4 pixelSeed = vec4( 0 );

	vec4 rand4( int v ) {

		ivec2 uv = ivec2( v, sobolBounceIndex );
		vec4 stratifiedSample = texelFetch( stratifiedTexture, uv, 0 );
		return fract( stratifiedSample + pixelSeed.r ); // blue noise + stratified samples

	}

	vec3 rand3( int v ) {

		return rand4( v ).xyz;

	}

	vec2 rand2( int v ) {

		return rand4( v ).xy;

	}

	float rand( int v ) {

		return rand4( v ).x;

	}

	void rng_initialize( vec2 screenCoord, int frame ) {

		// tile the small noise texture across the entire screen
		ivec2 noiseSize = ivec2( textureSize( stratifiedOffsetTexture, 0 ) );
		ivec2 pixel = ivec2( screenCoord.xy ) % noiseSize;
		vec2 pixelWidth = 1.0 / vec2( noiseSize );
		vec2 uv = vec2( pixel ) * pixelWidth + pixelWidth * 0.5;

		// note that using "texelFetch" here seems to break Android for some reason
		pixelSeed = texture( stratifiedOffsetTexture, uv );

	}

`,xi=`

	// diffuse
	float diffuseEval( vec3 wo, vec3 wi, vec3 wh, SurfaceRecord surf, inout vec3 color ) {

		// https://schuttejoe.github.io/post/disneybsdf/
		float fl = schlickFresnel( wi.z, 0.0 );
		float fv = schlickFresnel( wo.z, 0.0 );

		float metalFactor = ( 1.0 - surf.metalness );
		float transFactor = ( 1.0 - surf.transmission );
		float rr = 0.5 + 2.0 * surf.roughness * fl * fl;
		float retro = rr * ( fl + fv + fl * fv * ( rr - 1.0f ) );
		float lambert = ( 1.0f - 0.5f * fl ) * ( 1.0f - 0.5f * fv );

		// TODO: subsurface approx?

		// float F = evaluateFresnelWeight( dot( wo, wh ), surf.eta, surf.f0 );
		float F = disneyFresnel( wo, wi, wh, surf.f0, surf.eta, surf.metalness );
		color = ( 1.0 - F ) * transFactor * metalFactor * wi.z * surf.color * ( retro + lambert ) / PI;

		return wi.z / PI;

	}

	vec3 diffuseDirection( vec3 wo, SurfaceRecord surf ) {

		vec3 lightDirection = sampleSphere( rand2( 11 ) );
		lightDirection.z += 1.0;
		lightDirection = normalize( lightDirection );

		return lightDirection;

	}

	// specular
	float specularEval( vec3 wo, vec3 wi, vec3 wh, SurfaceRecord surf, inout vec3 color ) {

		// if roughness is set to 0 then D === NaN which results in black pixels
		float metalness = surf.metalness;
		float roughness = surf.filteredRoughness;

		float eta = surf.eta;
		float f0 = surf.f0;

		vec3 f0Color = mix( f0 * surf.specularColor * surf.specularIntensity, surf.color, surf.metalness );
		vec3 f90Color = vec3( mix( surf.specularIntensity, 1.0, surf.metalness ) );
		vec3 F = evaluateFresnel( dot( wo, wh ), eta, f0Color, f90Color );

		vec3 iridescenceF = evalIridescence( 1.0, surf.iridescenceIor, dot( wi, wh ), surf.iridescenceThickness, f0Color );
		F = mix( F, iridescenceF,  surf.iridescence );

		// PDF
		// See 14.1.1 Microfacet BxDFs in https://www.pbr-book.org/
		float incidentTheta = acos( wo.z );
		float G = ggxShadowMaskG2( wi, wo, roughness );
		float D = ggxDistribution( wh, roughness );
		float G1 = ggxShadowMaskG1( incidentTheta, roughness );
		float ggxPdf = D * G1 * max( 0.0, abs( dot( wo, wh ) ) ) / abs ( wo.z );

		color = wi.z * F * G * D / ( 4.0 * abs( wi.z * wo.z ) );
		return ggxPdf / ( 4.0 * dot( wo, wh ) );

	}

	vec3 specularDirection( vec3 wo, SurfaceRecord surf ) {

		// sample ggx vndf distribution which gives a new normal
		float roughness = surf.filteredRoughness;
		vec3 halfVector = ggxDirection(
			wo,
			vec2( roughness ),
			rand2( 12 )
		);

		// apply to new ray by reflecting off the new normal
		return - reflect( wo, halfVector );

	}


	// transmission
	/*
	float transmissionEval( vec3 wo, vec3 wi, vec3 wh, SurfaceRecord surf, inout vec3 color ) {

		// See section 4.2 in https://www.cs.cornell.edu/~srm/publications/EGSR07-btdf.pdf

		float filteredRoughness = surf.filteredRoughness;
		float eta = surf.eta;
		bool frontFace = surf.frontFace;
		bool thinFilm = surf.thinFilm;

		color = surf.transmission * surf.color;

		float denom = pow( eta * dot( wi, wh ) + dot( wo, wh ), 2.0 );
		return ggxPDF( wo, wh, filteredRoughness ) / denom;

	}

	vec3 transmissionDirection( vec3 wo, SurfaceRecord surf ) {

		float filteredRoughness = surf.filteredRoughness;
		float eta = surf.eta;
		bool frontFace = surf.frontFace;

		// sample ggx vndf distribution which gives a new normal
		vec3 halfVector = ggxDirection(
			wo,
			vec2( filteredRoughness ),
			rand2( 13 )
		);

		vec3 lightDirection = refract( normalize( - wo ), halfVector, eta );
		if ( surf.thinFilm ) {

			lightDirection = - refract( normalize( - lightDirection ), - vec3( 0.0, 0.0, 1.0 ), 1.0 / eta );

		}

		return normalize( lightDirection );

	}
	*/

	// TODO: This is just using a basic cosine-weighted specular distribution with an
	// incorrect PDF value at the moment. Update it to correctly use a GGX distribution
	float transmissionEval( vec3 wo, vec3 wi, vec3 wh, SurfaceRecord surf, inout vec3 color ) {

		color = surf.transmission * surf.color;

		// PDF
		// float F = evaluateFresnelWeight( dot( wo, wh ), surf.eta, surf.f0 );
		// float F = disneyFresnel( wo, wi, wh, surf.f0, surf.eta, surf.metalness );
		// if ( F >= 1.0 ) {

		// 	return 0.0;

		// }

		// return 1.0 / ( 1.0 - F );

		// reverted to previous to transmission. The above was causing black pixels
		float eta = surf.eta;
		float f0 = surf.f0;
		float cosTheta = min( wo.z, 1.0 );
		float sinTheta = sqrt( 1.0 - cosTheta * cosTheta );
		float reflectance = schlickFresnel( cosTheta, f0 );
		bool cannotRefract = eta * sinTheta > 1.0;
		if ( cannotRefract ) {

			return 0.0;

		}

		return 1.0 / ( 1.0 - reflectance );

	}

	vec3 transmissionDirection( vec3 wo, SurfaceRecord surf ) {

		float roughness = surf.filteredRoughness;
		float eta = surf.eta;
		vec3 halfVector = normalize( vec3( 0.0, 0.0, 1.0 ) + sampleSphere( rand2( 13 ) ) * roughness );
		vec3 lightDirection = refract( normalize( - wo ), halfVector, eta );

		if ( surf.thinFilm ) {

			lightDirection = - refract( normalize( - lightDirection ), - vec3( 0.0, 0.0, 1.0 ), 1.0 / eta );

		}
		return normalize( lightDirection );

	}

	// clearcoat
	float clearcoatEval( vec3 wo, vec3 wi, vec3 wh, SurfaceRecord surf, inout vec3 color ) {

		float ior = 1.5;
		float f0 = iorRatioToF0( ior );
		bool frontFace = surf.frontFace;
		float roughness = surf.filteredClearcoatRoughness;

		float eta = frontFace ? 1.0 / ior : ior;
		float G = ggxShadowMaskG2( wi, wo, roughness );
		float D = ggxDistribution( wh, roughness );
		float F = schlickFresnel( dot( wi, wh ), f0 );

		float fClearcoat = F * D * G / ( 4.0 * abs( wi.z * wo.z ) );
		color = color * ( 1.0 - surf.clearcoat * F ) + fClearcoat * surf.clearcoat * wi.z;

		// PDF
		// See equation (27) in http://jcgt.org/published/0003/02/03/
		return ggxPDF( wo, wh, roughness ) / ( 4.0 * dot( wi, wh ) );

	}

	vec3 clearcoatDirection( vec3 wo, SurfaceRecord surf ) {

		// sample ggx vndf distribution which gives a new normal
		float roughness = surf.filteredClearcoatRoughness;
		vec3 halfVector = ggxDirection(
			wo,
			vec2( roughness ),
			rand2( 14 )
		);

		// apply to new ray by reflecting off the new normal
		return - reflect( wo, halfVector );

	}

	// sheen
	vec3 sheenColor( vec3 wo, vec3 wi, vec3 wh, SurfaceRecord surf ) {

		float cosThetaO = saturateCos( wo.z );
		float cosThetaI = saturateCos( wi.z );
		float cosThetaH = wh.z;

		float D = velvetD( cosThetaH, surf.sheenRoughness );
		float G = velvetG( cosThetaO, cosThetaI, surf.sheenRoughness );

		// See equation (1) in http://www.aconty.com/pdf/s2017_pbs_imageworks_sheen.pdf
		vec3 color = surf.sheenColor;
		color *= D * G / ( 4.0 * abs( cosThetaO * cosThetaI ) );
		color *= wi.z;

		return color;

	}

	// bsdf
	void getLobeWeights(
		vec3 wo, vec3 wi, vec3 wh, vec3 clearcoatWo, SurfaceRecord surf,
		inout float diffuseWeight, inout float specularWeight, inout float transmissionWeight, inout float clearcoatWeight
	) {

		float metalness = surf.metalness;
		float transmission = surf.transmission;
		// float fEstimate = evaluateFresnelWeight( dot( wo, wh ), surf.eta, surf.f0 );
		float fEstimate = disneyFresnel( wo, wi, wh, surf.f0, surf.eta, surf.metalness );

		float transSpecularProb = mix( max( 0.25, fEstimate ), 1.0, metalness );
		float diffSpecularProb = 0.5 + 0.5 * metalness;

		diffuseWeight = ( 1.0 - transmission ) * ( 1.0 - diffSpecularProb );
		specularWeight = transmission * transSpecularProb + ( 1.0 - transmission ) * diffSpecularProb;
		transmissionWeight = transmission * ( 1.0 - transSpecularProb );
		clearcoatWeight = surf.clearcoat * schlickFresnel( clearcoatWo.z, 0.04 );

		float totalWeight = diffuseWeight + specularWeight + transmissionWeight + clearcoatWeight;
		diffuseWeight /= totalWeight;
		specularWeight /= totalWeight;
		transmissionWeight /= totalWeight;
		clearcoatWeight /= totalWeight;
	}

	float bsdfEval(
		vec3 wo, vec3 clearcoatWo, vec3 wi, vec3 clearcoatWi, SurfaceRecord surf,
		float diffuseWeight, float specularWeight, float transmissionWeight, float clearcoatWeight, inout float specularPdf, inout vec3 color
	) {

		float metalness = surf.metalness;
		float transmission = surf.transmission;

		float spdf = 0.0;
		float dpdf = 0.0;
		float tpdf = 0.0;
		float cpdf = 0.0;
		color = vec3( 0.0 );

		vec3 halfVector = getHalfVector( wi, wo, surf.eta );

		// diffuse
		if ( diffuseWeight > 0.0 && wi.z > 0.0 ) {

			dpdf = diffuseEval( wo, wi, halfVector, surf, color );
			color *= 1.0 - surf.transmission;

		}

		// ggx specular
		if ( specularWeight > 0.0 && wi.z > 0.0 ) {

			vec3 outColor;
			spdf = specularEval( wo, wi, getHalfVector( wi, wo ), surf, outColor );
			color += outColor;

		}

		// transmission
		if ( transmissionWeight > 0.0 && wi.z < 0.0 ) {

			tpdf = transmissionEval( wo, wi, halfVector, surf, color );

		}

		// sheen
		color *= mix( 1.0, sheenAlbedoScaling( wo, wi, surf ), surf.sheen );
		color += sheenColor( wo, wi, halfVector, surf ) * surf.sheen;

		// clearcoat
		if ( clearcoatWi.z >= 0.0 && clearcoatWeight > 0.0 ) {

			vec3 clearcoatHalfVector = getHalfVector( clearcoatWo, clearcoatWi );
			cpdf = clearcoatEval( clearcoatWo, clearcoatWi, clearcoatHalfVector, surf, color );

		}

		float pdf =
			dpdf * diffuseWeight
			+ spdf * specularWeight
			+ tpdf * transmissionWeight
			+ cpdf * clearcoatWeight;

		// retrieve specular rays for the shadows flag
		specularPdf = spdf * specularWeight + cpdf * clearcoatWeight;

		return pdf;

	}

	float bsdfResult( vec3 worldWo, vec3 worldWi, SurfaceRecord surf, inout vec3 color ) {

		if ( surf.volumeParticle ) {

			color = surf.color / ( 4.0 * PI );
			return 1.0 / ( 4.0 * PI );

		}

		vec3 wo = normalize( surf.normalInvBasis * worldWo );
		vec3 wi = normalize( surf.normalInvBasis * worldWi );

		vec3 clearcoatWo = normalize( surf.clearcoatInvBasis * worldWo );
		vec3 clearcoatWi = normalize( surf.clearcoatInvBasis * worldWi );

		vec3 wh = getHalfVector( wo, wi, surf.eta );
		float diffuseWeight;
		float specularWeight;
		float transmissionWeight;
		float clearcoatWeight;
		getLobeWeights( wo, wi, wh, clearcoatWo, surf, diffuseWeight, specularWeight, transmissionWeight, clearcoatWeight );

		float specularPdf;
		return bsdfEval( wo, clearcoatWo, wi, clearcoatWi, surf, diffuseWeight, specularWeight, transmissionWeight, clearcoatWeight, specularPdf, color );

	}

	ScatterRecord bsdfSample( vec3 worldWo, SurfaceRecord surf ) {

		if ( surf.volumeParticle ) {

			ScatterRecord sampleRec;
			sampleRec.specularPdf = 0.0;
			sampleRec.pdf = 1.0 / ( 4.0 * PI );
			sampleRec.direction = sampleSphere( rand2( 16 ) );
			sampleRec.color = surf.color / ( 4.0 * PI );
			return sampleRec;

		}

		vec3 wo = normalize( surf.normalInvBasis * worldWo );
		vec3 clearcoatWo = normalize( surf.clearcoatInvBasis * worldWo );
		mat3 normalBasis = surf.normalBasis;
		mat3 invBasis = surf.normalInvBasis;
		mat3 clearcoatNormalBasis = surf.clearcoatBasis;
		mat3 clearcoatInvBasis = surf.clearcoatInvBasis;

		float diffuseWeight;
		float specularWeight;
		float transmissionWeight;
		float clearcoatWeight;
		// using normal and basically-reflected ray since we don't have proper half vector here
		getLobeWeights( wo, wo, vec3( 0, 0, 1 ), clearcoatWo, surf, diffuseWeight, specularWeight, transmissionWeight, clearcoatWeight );

		float pdf[4];
		pdf[0] = diffuseWeight;
		pdf[1] = specularWeight;
		pdf[2] = transmissionWeight;
		pdf[3] = clearcoatWeight;

		float cdf[4];
		cdf[0] = pdf[0];
		cdf[1] = pdf[1] + cdf[0];
		cdf[2] = pdf[2] + cdf[1];
		cdf[3] = pdf[3] + cdf[2];

		if( cdf[3] != 0.0 ) {

			float invMaxCdf = 1.0 / cdf[3];
			cdf[0] *= invMaxCdf;
			cdf[1] *= invMaxCdf;
			cdf[2] *= invMaxCdf;
			cdf[3] *= invMaxCdf;

		} else {

			cdf[0] = 1.0;
			cdf[1] = 0.0;
			cdf[2] = 0.0;
			cdf[3] = 0.0;

		}

		vec3 wi;
		vec3 clearcoatWi;

		float r = rand( 15 );
		if ( r <= cdf[0] ) { // diffuse

			wi = diffuseDirection( wo, surf );
			clearcoatWi = normalize( clearcoatInvBasis * normalize( normalBasis * wi ) );

		} else if ( r <= cdf[1] ) { // specular

			wi = specularDirection( wo, surf );
			clearcoatWi = normalize( clearcoatInvBasis * normalize( normalBasis * wi ) );

		} else if ( r <= cdf[2] ) { // transmission / refraction

			wi = transmissionDirection( wo, surf );
			clearcoatWi = normalize( clearcoatInvBasis * normalize( normalBasis * wi ) );

		} else if ( r <= cdf[3] ) { // clearcoat

			clearcoatWi = clearcoatDirection( clearcoatWo, surf );
			wi = normalize( invBasis * normalize( clearcoatNormalBasis * clearcoatWi ) );

		}

		ScatterRecord result;
		result.pdf = bsdfEval( wo, clearcoatWo, wi, clearcoatWi, surf, diffuseWeight, specularWeight, transmissionWeight, clearcoatWeight, result.specularPdf, result.color );
		result.direction = normalize( surf.normalBasis * wi );

		return result;

	}

`,bi=`

	// returns the hit distance given the material density
	float intersectFogVolume( Material material, float u ) {

		// https://raytracing.github.io/books/RayTracingTheNextWeek.html#volumes/constantdensitymediums
		return material.opacity == 0.0 ? INFINITY : ( - 1.0 / material.opacity ) * log( u );

	}

	ScatterRecord sampleFogVolume( SurfaceRecord surf, vec2 uv ) {

		ScatterRecord sampleRec;
		sampleRec.specularPdf = 0.0;
		sampleRec.pdf = 1.0 / ( 2.0 * PI );
		sampleRec.direction = sampleSphere( uv );
		sampleRec.color = surf.color;
		return sampleRec;

	}

`,yi=`

	// The GGX functions provide sampling and distribution information for normals as output so
	// in order to get probability of scatter direction the half vector must be computed and provided.
	// [0] https://www.cs.cornell.edu/~srm/publications/EGSR07-btdf.pdf
	// [1] https://hal.archives-ouvertes.fr/hal-01509746/document
	// [2] http://jcgt.org/published/0007/04/01/
	// [4] http://jcgt.org/published/0003/02/03/

	// trowbridge-reitz === GGX === GTR

	vec3 ggxDirection( vec3 incidentDir, vec2 roughness, vec2 uv ) {

		// TODO: try GGXVNDF implementation from reference [2], here. Needs to update ggxDistribution
		// function below, as well

		// Implementation from reference [1]
		// stretch view
		vec3 V = normalize( vec3( roughness * incidentDir.xy, incidentDir.z ) );

		// orthonormal basis
		vec3 T1 = ( V.z < 0.9999 ) ? normalize( cross( V, vec3( 0.0, 0.0, 1.0 ) ) ) : vec3( 1.0, 0.0, 0.0 );
		vec3 T2 = cross( T1, V );

		// sample point with polar coordinates (r, phi)
		float a = 1.0 / ( 1.0 + V.z );
		float r = sqrt( uv.x );
		float phi = ( uv.y < a ) ? uv.y / a * PI : PI + ( uv.y - a ) / ( 1.0 - a ) * PI;
		float P1 = r * cos( phi );
		float P2 = r * sin( phi ) * ( ( uv.y < a ) ? 1.0 : V.z );

		// compute normal
		vec3 N = P1 * T1 + P2 * T2 + V * sqrt( max( 0.0, 1.0 - P1 * P1 - P2 * P2 ) );

		// unstretch
		N = normalize( vec3( roughness * N.xy, max( 0.0, N.z ) ) );

		return N;

	}

	// Below are PDF and related functions for use in a Monte Carlo path tracer
	// as specified in Appendix B of the following paper
	// See equation (34) from reference [0]
	float ggxLamda( float theta, float roughness ) {

		float tanTheta = tan( theta );
		float tanTheta2 = tanTheta * tanTheta;
		float alpha2 = roughness * roughness;

		float numerator = - 1.0 + sqrt( 1.0 + alpha2 * tanTheta2 );
		return numerator / 2.0;

	}

	// See equation (34) from reference [0]
	float ggxShadowMaskG1( float theta, float roughness ) {

		return 1.0 / ( 1.0 + ggxLamda( theta, roughness ) );

	}

	// See equation (125) from reference [4]
	float ggxShadowMaskG2( vec3 wi, vec3 wo, float roughness ) {

		float incidentTheta = acos( wi.z );
		float scatterTheta = acos( wo.z );
		return 1.0 / ( 1.0 + ggxLamda( incidentTheta, roughness ) + ggxLamda( scatterTheta, roughness ) );

	}

	// See equation (33) from reference [0]
	float ggxDistribution( vec3 halfVector, float roughness ) {

		float a2 = roughness * roughness;
		a2 = max( EPSILON, a2 );
		float cosTheta = halfVector.z;
		float cosTheta4 = pow( cosTheta, 4.0 );

		if ( cosTheta == 0.0 ) return 0.0;

		float theta = acosSafe( halfVector.z );
		float tanTheta = tan( theta );
		float tanTheta2 = pow( tanTheta, 2.0 );

		float denom = PI * cosTheta4 * pow( a2 + tanTheta2, 2.0 );
		return ( a2 / denom );

	}

	// See equation (3) from reference [2]
	float ggxPDF( vec3 wi, vec3 halfVector, float roughness ) {

		float incidentTheta = acos( wi.z );
		float D = ggxDistribution( halfVector, roughness );
		float G1 = ggxShadowMaskG1( incidentTheta, roughness );

		return D * G1 * max( 0.0, dot( wi, halfVector ) ) / wi.z;

	}

`,Ti=`

	// XYZ to sRGB color space
	const mat3 XYZ_TO_REC709 = mat3(
		3.2404542, -0.9692660,  0.0556434,
		-1.5371385,  1.8760108, -0.2040259,
		-0.4985314,  0.0415560,  1.0572252
	);

	vec3 fresnel0ToIor( vec3 fresnel0 ) {

		vec3 sqrtF0 = sqrt( fresnel0 );
		return ( vec3( 1.0 ) + sqrtF0 ) / ( vec3( 1.0 ) - sqrtF0 );

	}

	// Conversion FO/IOR
	vec3 iorToFresnel0( vec3 transmittedIor, float incidentIor ) {

		return square( ( transmittedIor - vec3( incidentIor ) ) / ( transmittedIor + vec3( incidentIor ) ) );

	}

	// ior is a value between 1.0 and 3.0. 1.0 is air interface
	float iorToFresnel0( float transmittedIor, float incidentIor ) {

		return square( ( transmittedIor - incidentIor ) / ( transmittedIor + incidentIor ) );

	}

	// Fresnel equations for dielectric/dielectric interfaces. See https://belcour.github.io/blog/research/2017/05/01/brdf-thin-film.html
	vec3 evalSensitivity( float OPD, vec3 shift ) {

		float phase = 2.0 * PI * OPD * 1.0e-9;

		vec3 val = vec3( 5.4856e-13, 4.4201e-13, 5.2481e-13 );
		vec3 pos = vec3( 1.6810e+06, 1.7953e+06, 2.2084e+06 );
		vec3 var = vec3( 4.3278e+09, 9.3046e+09, 6.6121e+09 );

		vec3 xyz = val * sqrt( 2.0 * PI * var ) * cos( pos * phase + shift ) * exp( - square( phase ) * var );
		xyz.x += 9.7470e-14 * sqrt( 2.0 * PI * 4.5282e+09 ) * cos( 2.2399e+06 * phase + shift[ 0 ] ) * exp( - 4.5282e+09 * square( phase ) );
		xyz /= 1.0685e-7;

		vec3 srgb = XYZ_TO_REC709 * xyz;
		return srgb;

	}

	// See Section 4. Analytic Spectral Integration, A Practical Extension to Microfacet Theory for the Modeling of Varying Iridescence, https://hal.archives-ouvertes.fr/hal-01518344/document
	vec3 evalIridescence( float outsideIOR, float eta2, float cosTheta1, float thinFilmThickness, vec3 baseF0 ) {

		vec3 I;

		// Force iridescenceIor -> outsideIOR when thinFilmThickness -> 0.0
		float iridescenceIor = mix( outsideIOR, eta2, smoothstep( 0.0, 0.03, thinFilmThickness ) );

		// Evaluate the cosTheta on the base layer (Snell law)
		float sinTheta2Sq = square( outsideIOR / iridescenceIor ) * ( 1.0 - square( cosTheta1 ) );

		// Handle TIR:
		float cosTheta2Sq = 1.0 - sinTheta2Sq;
		if ( cosTheta2Sq < 0.0 ) {

			return vec3( 1.0 );

		}

		float cosTheta2 = sqrt( cosTheta2Sq );

		// First interface
		float R0 = iorToFresnel0( iridescenceIor, outsideIOR );
		float R12 = schlickFresnel( cosTheta1, R0 );
		float R21 = R12;
		float T121 = 1.0 - R12;
		float phi12 = 0.0;
		if ( iridescenceIor < outsideIOR ) {

			phi12 = PI;

		}

		float phi21 = PI - phi12;

		// Second interface
		vec3 baseIOR = fresnel0ToIor( clamp( baseF0, 0.0, 0.9999 ) ); // guard against 1.0
		vec3 R1 = iorToFresnel0( baseIOR, iridescenceIor );
		vec3 R23 = schlickFresnel( cosTheta2, R1 );
		vec3 phi23 = vec3( 0.0 );
		if ( baseIOR[0] < iridescenceIor ) {

			phi23[ 0 ] = PI;

		}

		if ( baseIOR[1] < iridescenceIor ) {

			phi23[ 1 ] = PI;

		}

		if ( baseIOR[2] < iridescenceIor ) {

			phi23[ 2 ] = PI;

		}

		// Phase shift
		float OPD = 2.0 * iridescenceIor * thinFilmThickness * cosTheta2;
		vec3 phi = vec3( phi21 ) + phi23;

		// Compound terms
		vec3 R123 = clamp( R12 * R23, 1e-5, 0.9999 );
		vec3 r123 = sqrt( R123 );
		vec3 Rs = square( T121 ) * R23 / ( vec3( 1.0 ) - R123 );

		// Reflectance term for m = 0 (DC term amplitude)
		vec3 C0 = R12 + Rs;
		I = C0;

		// Reflectance term for m > 0 (pairs of diracs)
		vec3 Cm = Rs - T121;
		for ( int m = 1; m <= 2; ++ m ) {

			Cm *= r123;
			vec3 Sm = 2.0 * evalSensitivity( float( m ) * OPD, float( m ) * phi );
			I += Cm * Sm;

		}

		// Since out of gamut colors might be produced, negative color values are clamped to 0.
		return max( I, vec3( 0.0 ) );

	}

`,wi=`

	// See equation (2) in http://www.aconty.com/pdf/s2017_pbs_imageworks_sheen.pdf
	float velvetD( float cosThetaH, float roughness ) {

		float alpha = max( roughness, 0.07 );
		alpha = alpha * alpha;

		float invAlpha = 1.0 / alpha;

		float sqrCosThetaH = cosThetaH * cosThetaH;
		float sinThetaH = max( 1.0 - sqrCosThetaH, 0.001 );

		return ( 2.0 + invAlpha ) * pow( sinThetaH, 0.5 * invAlpha ) / ( 2.0 * PI );

	}

	float velvetParamsInterpolate( int i, float oneMinusAlphaSquared ) {

		const float p0[5] = float[5]( 25.3245, 3.32435, 0.16801, -1.27393, -4.85967 );
		const float p1[5] = float[5]( 21.5473, 3.82987, 0.19823, -1.97760, -4.32054 );

		return mix( p1[i], p0[i], oneMinusAlphaSquared );

	}

	float velvetL( float x, float alpha ) {

		float oneMinusAlpha = 1.0 - alpha;
		float oneMinusAlphaSquared = oneMinusAlpha * oneMinusAlpha;

		float a = velvetParamsInterpolate( 0, oneMinusAlphaSquared );
		float b = velvetParamsInterpolate( 1, oneMinusAlphaSquared );
		float c = velvetParamsInterpolate( 2, oneMinusAlphaSquared );
		float d = velvetParamsInterpolate( 3, oneMinusAlphaSquared );
		float e = velvetParamsInterpolate( 4, oneMinusAlphaSquared );

		return a / ( 1.0 + b * pow( abs( x ), c ) ) + d * x + e;

	}

	// See equation (3) in http://www.aconty.com/pdf/s2017_pbs_imageworks_sheen.pdf
	float velvetLambda( float cosTheta, float alpha ) {

		return abs( cosTheta ) < 0.5 ? exp( velvetL( cosTheta, alpha ) ) : exp( 2.0 * velvetL( 0.5, alpha ) - velvetL( 1.0 - cosTheta, alpha ) );

	}

	// See Section 3, Shadowing Term, in http://www.aconty.com/pdf/s2017_pbs_imageworks_sheen.pdf
	float velvetG( float cosThetaO, float cosThetaI, float roughness ) {

		float alpha = max( roughness, 0.07 );
		alpha = alpha * alpha;

		return 1.0 / ( 1.0 + velvetLambda( cosThetaO, alpha ) + velvetLambda( cosThetaI, alpha ) );

	}

	float directionalAlbedoSheen( float cosTheta, float alpha ) {

		cosTheta = saturate( cosTheta );

		float c = 1.0 - cosTheta;
		float c3 = c * c * c;

		return 0.65584461 * c3 + 1.0 / ( 4.16526551 + exp( -7.97291361 * sqrt( alpha ) + 6.33516894 ) );

	}

	float sheenAlbedoScaling( vec3 wo, vec3 wi, SurfaceRecord surf ) {

		float alpha = max( surf.sheenRoughness, 0.07 );
		alpha = alpha * alpha;

		float maxSheenColor = max( max( surf.sheenColor.r, surf.sheenColor.g ), surf.sheenColor.b );

		float eWo = directionalAlbedoSheen( saturateCos( wo.z ), alpha );
		float eWi = directionalAlbedoSheen( saturateCos( wi.z ), alpha );

		return min( 1.0 - maxSheenColor * eWo, 1.0 - maxSheenColor * eWi );

	}

	// See Section 5, Layering, in http://www.aconty.com/pdf/s2017_pbs_imageworks_sheen.pdf
	float sheenAlbedoScaling( vec3 wo, SurfaceRecord surf ) {

		float alpha = max( surf.sheenRoughness, 0.07 );
		alpha = alpha * alpha;

		float maxSheenColor = max( max( surf.sheenColor.r, surf.sheenColor.g ), surf.sheenColor.b );

		float eWo = directionalAlbedoSheen( saturateCos( wo.z ), alpha );

		return 1.0 - maxSheenColor * eWo;

	}

`,_i=`

#ifndef FOG_CHECK_ITERATIONS
#define FOG_CHECK_ITERATIONS 30
#endif

// returns whether the given material is a fog material or not
bool isMaterialFogVolume( sampler2D materials, uint materialIndex ) {

	uint i = materialIndex * uint( MATERIAL_PIXELS );
	vec4 s14 = texelFetch1D( materials, i + 14u );
	return bool( int( s14.b ) & 4 );

}

// returns true if we're within the first fog volume we hit
bool bvhIntersectFogVolumeHit(
	vec3 rayOrigin, vec3 rayDirection,
	usampler2D materialIndexAttribute, sampler2D materials,
	inout Material material
) {

	material.fogVolume = false;

	for ( int i = 0; i < FOG_CHECK_ITERATIONS; i ++ ) {

		// find nearest hit
		uvec4 faceIndices = uvec4( 0u );
		vec3 faceNormal = vec3( 0.0, 0.0, 1.0 );
		vec3 barycoord = vec3( 0.0 );
		float side = 1.0;
		float dist = 0.0;
		bool hit = bvhIntersectFirstHit( bvh, rayOrigin, rayDirection, faceIndices, faceNormal, barycoord, side, dist );
		if ( hit ) {

			// if it's a fog volume return whether we hit the front or back face
			uint materialIndex = uTexelFetch1D( materialIndexAttribute, faceIndices.x ).r;
			if ( isMaterialFogVolume( materials, materialIndex ) ) {

				material = readMaterialInfo( materials, materialIndex );
				return side == - 1.0;

			} else {

				// move the ray forward
				rayOrigin = stepRayOrigin( rayOrigin, rayDirection, - faceNormal, dist );

			}

		} else {

			return false;

		}

	}

	return false;

}

`,Si=`

	// step through multiple surface hits and accumulate color attenuation based on transmissive surfaces
	// returns true if a solid surface was hit
	bool attenuateHit(
		RenderState state,
		Ray ray, float rayDist,
		out vec3 color
	) {

		// store the original bounce index so we can reset it after
		uint originalBounceIndex = sobolBounceIndex;

		int traversals = state.traversals;
		int transmissiveTraversals = state.transmissiveTraversals;
		bool isShadowRay = state.isShadowRay;
		Material fogMaterial = state.fogMaterial;

		vec3 startPoint = ray.origin;

		// hit results
		SurfaceHit surfaceHit;

		color = vec3( 1.0 );

		bool result = true;
		for ( int i = 0; i < traversals; i ++ ) {

			sobolBounceIndex ++;

			int hitType = traceScene( ray, fogMaterial, surfaceHit );

			if ( hitType == FOG_HIT ) {

				result = true;
				break;

			} else if ( hitType == SURFACE_HIT ) {

				float totalDist = distance( startPoint, ray.origin + ray.direction * surfaceHit.dist );
				if ( totalDist > rayDist ) {

					result = false;
					break;

				}

				// TODO: attenuate the contribution based on the PDF of the resulting ray including refraction values
				// Should be able to work using the material BSDF functions which will take into account specularity, etc.
				// TODO: should we account for emissive surfaces here?

				uint materialIndex = uTexelFetch1D( materialIndexAttribute, surfaceHit.faceIndices.x ).r;
				Material material = readMaterialInfo( materials, materialIndex );

				// adjust the ray to the new surface
				bool isEntering = surfaceHit.side == 1.0;
				ray.origin = stepRayOrigin( ray.origin, ray.direction, - surfaceHit.faceNormal, surfaceHit.dist );

				#if FEATURE_FOG

				if ( material.fogVolume ) {

					fogMaterial = material;
					fogMaterial.fogVolume = surfaceHit.side == 1.0;
					i -= sign( transmissiveTraversals );
					transmissiveTraversals --;
					continue;

				}

				#endif

				if ( ! material.castShadow && isShadowRay ) {

					continue;

				}

				vec2 uv = textureSampleBarycoord( attributesArray, ATTR_UV, surfaceHit.barycoord, surfaceHit.faceIndices.xyz ).xy;
				vec4 vertexColor = textureSampleBarycoord( attributesArray, ATTR_COLOR, surfaceHit.barycoord, surfaceHit.faceIndices.xyz );

				// albedo
				vec4 albedo = vec4( material.color, material.opacity );
				if ( material.map != - 1 ) {

					vec3 uvPrime = material.mapTransform * vec3( uv, 1 );
					albedo *= texture2D( textures, vec3( uvPrime.xy, material.map ) );

				}

				if ( material.vertexColors ) {

					albedo *= vertexColor;

				}

				// alphaMap
				if ( material.alphaMap != - 1 ) {

					vec3 uvPrime = material.alphaMapTransform * vec3( uv, 1 );
					albedo.a *= texture2D( textures, vec3( uvPrime.xy, material.alphaMap ) ).x;

				}

				// transmission
				float transmission = material.transmission;
				if ( material.transmissionMap != - 1 ) {

					vec3 uvPrime = material.transmissionMapTransform * vec3( uv, 1 );
					transmission *= texture2D( textures, vec3( uvPrime.xy, material.transmissionMap ) ).r;

				}

				// metalness
				float metalness = material.metalness;
				if ( material.metalnessMap != - 1 ) {

					vec3 uvPrime = material.metalnessMapTransform * vec3( uv, 1 );
					metalness *= texture2D( textures, vec3( uvPrime.xy, material.metalnessMap ) ).b;

				}

				float alphaTest = material.alphaTest;
				bool useAlphaTest = alphaTest != 0.0;
				float transmissionFactor = ( 1.0 - metalness ) * transmission;
				if (
					transmissionFactor < rand( 9 ) && ! (
						// material sidedness
						material.side != 0.0 && surfaceHit.side == material.side

						// alpha test
						|| useAlphaTest && albedo.a < alphaTest

						// opacity
						|| material.transparent && ! useAlphaTest && albedo.a < rand( 10 )
					)
				) {

					result = true;
					break;

				}

				if ( surfaceHit.side == 1.0 && isEntering ) {

					// only attenuate by surface color on the way in
					color *= mix( vec3( 1.0 ), albedo.rgb, transmissionFactor );

				} else if ( surfaceHit.side == - 1.0 ) {

					// attenuate by medium once we hit the opposite side of the model
					color *= transmissionAttenuation( surfaceHit.dist, material.attenuationColor, material.attenuationDistance );

				}

				bool isTransmissiveRay = dot( ray.direction, surfaceHit.faceNormal * surfaceHit.side ) < 0.0;
				if ( ( isTransmissiveRay || isEntering ) && transmissiveTraversals > 0 ) {

					i -= sign( transmissiveTraversals );
					transmissiveTraversals --;

				}

			} else {

				result = false;
				break;

			}

		}

		// reset the bounce index
		sobolBounceIndex = originalBounceIndex;
		return result;

	}

`,Ii=`

	vec3 ndcToRayOrigin( vec2 coord ) {

		vec4 rayOrigin4 = cameraWorldMatrix * invProjectionMatrix * vec4( coord, - 1.0, 1.0 );
		return rayOrigin4.xyz / rayOrigin4.w;
	}

	Ray getCameraRay() {

		vec2 ssd = vec2( 1.0 ) / resolution;

		// Jitter the camera ray by finding a uv coordinate at a random sample
		// around this pixel's UV coordinate for AA
		vec2 ruv = rand2( 0 );
		vec2 jitteredUv = vUv + vec2( tentFilter( ruv.x ) * ssd.x, tentFilter( ruv.y ) * ssd.y );
		Ray ray;

		#if CAMERA_TYPE == 2

			// Equirectangular projection
			vec4 rayDirection4 = vec4( equirectUvToDirection( jitteredUv ), 0.0 );
			vec4 rayOrigin4 = vec4( 0.0, 0.0, 0.0, 1.0 );

			rayDirection4 = cameraWorldMatrix * rayDirection4;
			rayOrigin4 = cameraWorldMatrix * rayOrigin4;

			ray.direction = normalize( rayDirection4.xyz );
			ray.origin = rayOrigin4.xyz / rayOrigin4.w;

		#else

			// get [- 1, 1] normalized device coordinates
			vec2 ndc = 2.0 * jitteredUv - vec2( 1.0 );
			ray.origin = ndcToRayOrigin( ndc );

			#if CAMERA_TYPE == 1

				// Orthographic projection
				ray.direction = ( cameraWorldMatrix * vec4( 0.0, 0.0, - 1.0, 0.0 ) ).xyz;
				ray.direction = normalize( ray.direction );

			#else

				// Perspective projection
				ray.direction = normalize( mat3( cameraWorldMatrix ) * ( invProjectionMatrix * vec4( ndc, 0.0, 1.0 ) ).xyz );

			#endif

		#endif

		#if FEATURE_DOF
		{

			// depth of field
			vec3 focalPoint = ray.origin + normalize( ray.direction ) * physicalCamera.focusDistance;

			// get the aperture sample
			// if blades === 0 then we assume a circle
			vec3 shapeUVW= rand3( 1 );
			int blades = physicalCamera.apertureBlades;
			float anamorphicRatio = physicalCamera.anamorphicRatio;
			vec2 apertureSample = sampleAperture( blades, shapeUVW );
			apertureSample *= physicalCamera.bokehSize * 0.5 * 1e-3;

			// rotate the aperture shape
			apertureSample =
				rotateVector( apertureSample, physicalCamera.apertureRotation ) *
				saturate( vec2( anamorphicRatio, 1.0 / anamorphicRatio ) );

			// create the new ray
			ray.origin += ( cameraWorldMatrix * vec4( apertureSample, 0.0, 0.0 ) ).xyz;
			ray.direction = focalPoint - ray.origin;

		}
		#endif

		ray.direction = normalize( ray.direction );

		return ray;

	}

`,Ri=`

	vec3 directLightContribution( vec3 worldWo, SurfaceRecord surf, RenderState state, vec3 rayOrigin ) {

		vec3 result = vec3( 0.0 );

		// uniformly pick a light or environment map
		if( lightsDenom != 0.0 && rand( 5 ) < float( lights.count ) / lightsDenom ) {

			// sample a light or environment
			LightRecord lightRec = randomLightSample( lights.tex, iesProfiles, lights.count, rayOrigin, rand3( 6 ) );

			bool isSampleBelowSurface = ! surf.volumeParticle && dot( surf.faceNormal, lightRec.direction ) < 0.0;
			if ( isSampleBelowSurface ) {

				lightRec.pdf = 0.0;

			}

			// check if a ray could even reach the light area
			Ray lightRay;
			lightRay.origin = rayOrigin;
			lightRay.direction = lightRec.direction;
			vec3 attenuatedColor;
			if (
				lightRec.pdf > 0.0 &&
				isDirectionValid( lightRec.direction, surf.normal, surf.faceNormal ) &&
				! attenuateHit( state, lightRay, lightRec.dist, attenuatedColor )
			) {

				// get the material pdf
				vec3 sampleColor;
				float lightMaterialPdf = bsdfResult( worldWo, lightRec.direction, surf, sampleColor );
				bool isValidSampleColor = all( greaterThanEqual( sampleColor, vec3( 0.0 ) ) );
				if ( lightMaterialPdf > 0.0 && isValidSampleColor ) {

					// weight the direct light contribution
					float lightPdf = lightRec.pdf / lightsDenom;
					float misWeight = lightRec.type == SPOT_LIGHT_TYPE || lightRec.type == DIR_LIGHT_TYPE || lightRec.type == POINT_LIGHT_TYPE ? 1.0 : misHeuristic( lightPdf, lightMaterialPdf );
					result = attenuatedColor * lightRec.emission * state.throughputColor * sampleColor * misWeight / lightPdf;

				}

			}

		} else if ( envMapInfo.totalSum != 0.0 && environmentIntensity != 0.0 ) {

			// find a sample in the environment map to include in the contribution
			vec3 envColor, envDirection;
			float envPdf = sampleEquirectProbability( rand2( 7 ), envColor, envDirection );
			envDirection = invEnvRotation3x3 * envDirection;

			// this env sampling is not set up for transmissive sampling and yields overly bright
			// results so we ignore the sample in this case.
			// TODO: this should be improved but how? The env samples could traverse a few layers?
			bool isSampleBelowSurface = ! surf.volumeParticle && dot( surf.faceNormal, envDirection ) < 0.0;
			if ( isSampleBelowSurface ) {

				envPdf = 0.0;

			}

			// check if a ray could even reach the surface
			Ray envRay;
			envRay.origin = rayOrigin;
			envRay.direction = envDirection;
			vec3 attenuatedColor;
			if (
				envPdf > 0.0 &&
				isDirectionValid( envDirection, surf.normal, surf.faceNormal ) &&
				! attenuateHit( state, envRay, INFINITY, attenuatedColor )
			) {

				// get the material pdf
				vec3 sampleColor;
				float envMaterialPdf = bsdfResult( worldWo, envDirection, surf, sampleColor );
				bool isValidSampleColor = all( greaterThanEqual( sampleColor, vec3( 0.0 ) ) );
				if ( envMaterialPdf > 0.0 && isValidSampleColor ) {

					// weight the direct light contribution
					envPdf /= lightsDenom;
					float misWeight = misHeuristic( envPdf, envMaterialPdf );
					result = attenuatedColor * environmentIntensity * envColor * state.throughputColor * sampleColor * misWeight / envPdf;

				}

			}

		}

		// Function changed to have a single return statement to potentially help with crashes on Mac OS.
		// See issue #470
		return result;

	}

`,Mi=`

	#define SKIP_SURFACE 0
	#define HIT_SURFACE 1
	int getSurfaceRecord(
		Material material, SurfaceHit surfaceHit, sampler2DArray attributesArray,
		float accumulatedRoughness,
		inout SurfaceRecord surf
	) {

		if ( material.fogVolume ) {

			vec3 normal = vec3( 0, 0, 1 );

			SurfaceRecord fogSurface;
			fogSurface.volumeParticle = true;
			fogSurface.color = material.color;
			fogSurface.emission = material.emissiveIntensity * material.emissive;
			fogSurface.normal = normal;
			fogSurface.faceNormal = normal;
			fogSurface.clearcoatNormal = normal;

			surf = fogSurface;
			return HIT_SURFACE;

		}

		// uv coord for textures
		vec2 uv = textureSampleBarycoord( attributesArray, ATTR_UV, surfaceHit.barycoord, surfaceHit.faceIndices.xyz ).xy;
		vec4 vertexColor = textureSampleBarycoord( attributesArray, ATTR_COLOR, surfaceHit.barycoord, surfaceHit.faceIndices.xyz );

		// albedo
		vec4 albedo = vec4( material.color, material.opacity );
		if ( material.map != - 1 ) {

			vec3 uvPrime = material.mapTransform * vec3( uv, 1 );
			albedo *= texture2D( textures, vec3( uvPrime.xy, material.map ) );

		}

		if ( material.vertexColors ) {

			albedo *= vertexColor;

		}

		// alphaMap
		if ( material.alphaMap != - 1 ) {

			vec3 uvPrime = material.alphaMapTransform * vec3( uv, 1 );
			albedo.a *= texture2D( textures, vec3( uvPrime.xy, material.alphaMap ) ).x;

		}

		// possibly skip this sample if it's transparent, alpha test is enabled, or we hit the wrong material side
		// and it's single sided.
		// - alpha test is disabled when it === 0
		// - the material sidedness test is complicated because we want light to pass through the back side but still
		// be able to see the front side. This boolean checks if the side we hit is the front side on the first ray
		// and we're rendering the other then we skip it. Do the opposite on subsequent bounces to get incoming light.
		float alphaTest = material.alphaTest;
		bool useAlphaTest = alphaTest != 0.0;
		if (
			// material sidedness
			material.side != 0.0 && surfaceHit.side != material.side

			// alpha test
			|| useAlphaTest && albedo.a < alphaTest

			// opacity
			|| material.transparent && ! useAlphaTest && albedo.a < rand( 3 )
		) {

			return SKIP_SURFACE;

		}

		// fetch the interpolated smooth normal
		vec3 normal = normalize( textureSampleBarycoord(
			attributesArray,
			ATTR_NORMAL,
			surfaceHit.barycoord,
			surfaceHit.faceIndices.xyz
		).xyz );

		// roughness
		float roughness = material.roughness;
		if ( material.roughnessMap != - 1 ) {

			vec3 uvPrime = material.roughnessMapTransform * vec3( uv, 1 );
			roughness *= texture2D( textures, vec3( uvPrime.xy, material.roughnessMap ) ).g;

		}

		// metalness
		float metalness = material.metalness;
		if ( material.metalnessMap != - 1 ) {

			vec3 uvPrime = material.metalnessMapTransform * vec3( uv, 1 );
			metalness *= texture2D( textures, vec3( uvPrime.xy, material.metalnessMap ) ).b;

		}

		// emission
		vec3 emission = material.emissiveIntensity * material.emissive;
		if ( material.emissiveMap != - 1 ) {

			vec3 uvPrime = material.emissiveMapTransform * vec3( uv, 1 );
			emission *= texture2D( textures, vec3( uvPrime.xy, material.emissiveMap ) ).xyz;

		}

		// transmission
		float transmission = material.transmission;
		if ( material.transmissionMap != - 1 ) {

			vec3 uvPrime = material.transmissionMapTransform * vec3( uv, 1 );
			transmission *= texture2D( textures, vec3( uvPrime.xy, material.transmissionMap ) ).r;

		}

		// normal
		if ( material.flatShading ) {

			// if we're rendering a flat shaded object then use the face normals - the face normal
			// is provided based on the side the ray hits the mesh so flip it to align with the
			// interpolated vertex normals.
			normal = surfaceHit.faceNormal * surfaceHit.side;

		}

		vec3 baseNormal = normal;
		if ( material.normalMap != - 1 ) {

			vec4 tangentSample = textureSampleBarycoord(
				attributesArray,
				ATTR_TANGENT,
				surfaceHit.barycoord,
				surfaceHit.faceIndices.xyz
			);

			// some provided tangents can be malformed (0, 0, 0) causing the normal to be degenerate
			// resulting in NaNs and slow path tracing.
			if ( length( tangentSample.xyz ) > 0.0 ) {

				vec3 tangent = normalize( tangentSample.xyz );
				vec3 bitangent = normalize( cross( normal, tangent ) * tangentSample.w );
				mat3 vTBN = mat3( tangent, bitangent, normal );

				vec3 uvPrime = material.normalMapTransform * vec3( uv, 1 );
				vec3 texNormal = texture2D( textures, vec3( uvPrime.xy, material.normalMap ) ).xyz * 2.0 - 1.0;
				texNormal.xy *= material.normalScale;
				normal = vTBN * texNormal;

			}

		}

		normal *= surfaceHit.side;

		// clearcoat
		float clearcoat = material.clearcoat;
		if ( material.clearcoatMap != - 1 ) {

			vec3 uvPrime = material.clearcoatMapTransform * vec3( uv, 1 );
			clearcoat *= texture2D( textures, vec3( uvPrime.xy, material.clearcoatMap ) ).r;

		}

		// clearcoatRoughness
		float clearcoatRoughness = material.clearcoatRoughness;
		if ( material.clearcoatRoughnessMap != - 1 ) {

			vec3 uvPrime = material.clearcoatRoughnessMapTransform * vec3( uv, 1 );
			clearcoatRoughness *= texture2D( textures, vec3( uvPrime.xy, material.clearcoatRoughnessMap ) ).g;

		}

		// clearcoatNormal
		vec3 clearcoatNormal = baseNormal;
		if ( material.clearcoatNormalMap != - 1 ) {

			vec4 tangentSample = textureSampleBarycoord(
				attributesArray,
				ATTR_TANGENT,
				surfaceHit.barycoord,
				surfaceHit.faceIndices.xyz
			);

			// some provided tangents can be malformed (0, 0, 0) causing the normal to be degenerate
			// resulting in NaNs and slow path tracing.
			if ( length( tangentSample.xyz ) > 0.0 ) {

				vec3 tangent = normalize( tangentSample.xyz );
				vec3 bitangent = normalize( cross( clearcoatNormal, tangent ) * tangentSample.w );
				mat3 vTBN = mat3( tangent, bitangent, clearcoatNormal );

				vec3 uvPrime = material.clearcoatNormalMapTransform * vec3( uv, 1 );
				vec3 texNormal = texture2D( textures, vec3( uvPrime.xy, material.clearcoatNormalMap ) ).xyz * 2.0 - 1.0;
				texNormal.xy *= material.clearcoatNormalScale;
				clearcoatNormal = vTBN * texNormal;

			}

		}

		clearcoatNormal *= surfaceHit.side;

		// sheenColor
		vec3 sheenColor = material.sheenColor;
		if ( material.sheenColorMap != - 1 ) {

			vec3 uvPrime = material.sheenColorMapTransform * vec3( uv, 1 );
			sheenColor *= texture2D( textures, vec3( uvPrime.xy, material.sheenColorMap ) ).rgb;

		}

		// sheenRoughness
		float sheenRoughness = material.sheenRoughness;
		if ( material.sheenRoughnessMap != - 1 ) {

			vec3 uvPrime = material.sheenRoughnessMapTransform * vec3( uv, 1 );
			sheenRoughness *= texture2D( textures, vec3( uvPrime.xy, material.sheenRoughnessMap ) ).a;

		}

		// iridescence
		float iridescence = material.iridescence;
		if ( material.iridescenceMap != - 1 ) {

			vec3 uvPrime = material.iridescenceMapTransform * vec3( uv, 1 );
			iridescence *= texture2D( textures, vec3( uvPrime.xy, material.iridescenceMap ) ).r;

		}

		// iridescence thickness
		float iridescenceThickness = material.iridescenceThicknessMaximum;
		if ( material.iridescenceThicknessMap != - 1 ) {

			vec3 uvPrime = material.iridescenceThicknessMapTransform * vec3( uv, 1 );
			float iridescenceThicknessSampled = texture2D( textures, vec3( uvPrime.xy, material.iridescenceThicknessMap ) ).g;
			iridescenceThickness = mix( material.iridescenceThicknessMinimum, material.iridescenceThicknessMaximum, iridescenceThicknessSampled );

		}

		iridescence = iridescenceThickness == 0.0 ? 0.0 : iridescence;

		// specular color
		vec3 specularColor = material.specularColor;
		if ( material.specularColorMap != - 1 ) {

			vec3 uvPrime = material.specularColorMapTransform * vec3( uv, 1 );
			specularColor *= texture2D( textures, vec3( uvPrime.xy, material.specularColorMap ) ).rgb;

		}

		// specular intensity
		float specularIntensity = material.specularIntensity;
		if ( material.specularIntensityMap != - 1 ) {

			vec3 uvPrime = material.specularIntensityMapTransform * vec3( uv, 1 );
			specularIntensity *= texture2D( textures, vec3( uvPrime.xy, material.specularIntensityMap ) ).a;

		}

		surf.volumeParticle = false;

		surf.faceNormal = surfaceHit.faceNormal;
		surf.normal = normal;

		surf.metalness = metalness;
		surf.color = albedo.rgb;
		surf.emission = emission;

		surf.ior = material.ior;
		surf.transmission = transmission;
		surf.thinFilm = material.thinFilm;
		surf.attenuationColor = material.attenuationColor;
		surf.attenuationDistance = material.attenuationDistance;

		surf.clearcoatNormal = clearcoatNormal;
		surf.clearcoat = clearcoat;

		surf.sheen = material.sheen;
		surf.sheenColor = sheenColor;

		surf.iridescence = iridescence;
		surf.iridescenceIor = material.iridescenceIor;
		surf.iridescenceThickness = iridescenceThickness;

		surf.specularColor = specularColor;
		surf.specularIntensity = specularIntensity;

		// apply perceptual roughness factor from gltf. sheen perceptual roughness is
		// applied by its brdf function
		// https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#microfacet-surfaces
		surf.roughness = roughness * roughness;
		surf.clearcoatRoughness = clearcoatRoughness * clearcoatRoughness;
		surf.sheenRoughness = sheenRoughness;

		// frontFace is used to determine transmissive properties and PDF. If no transmission is used
		// then we can just always assume this is a front face.
		surf.frontFace = surfaceHit.side == 1.0 || transmission == 0.0;
		surf.eta = material.thinFilm || surf.frontFace ? 1.0 / material.ior : material.ior;
		surf.f0 = iorRatioToF0( surf.eta );

		// Compute the filtered roughness value to use during specular reflection computations.
		// The accumulated roughness value is scaled by a user setting and a "magic value" of 5.0.
		// If we're exiting something transmissive then scale the factor down significantly so we can retain
		// sharp internal reflections
		surf.filteredRoughness = applyFilteredGlossy( surf.roughness, accumulatedRoughness );
		surf.filteredClearcoatRoughness = applyFilteredGlossy( surf.clearcoatRoughness, accumulatedRoughness );

		// get the normal frames
		surf.normalBasis = getBasisFromNormal( surf.normal );
		surf.normalInvBasis = inverse( surf.normalBasis );

		surf.clearcoatBasis = getBasisFromNormal( surf.clearcoatNormal );
		surf.clearcoatInvBasis = inverse( surf.clearcoatBasis );

		return HIT_SURFACE;

	}
`,Ai=`

	struct Ray {

		vec3 origin;
		vec3 direction;

	};

	struct SurfaceHit {

		uvec4 faceIndices;
		vec3 barycoord;
		vec3 faceNormal;
		float side;
		float dist;

	};

	struct RenderState {

		bool firstRay;
		bool transmissiveRay;
		bool isShadowRay;
		float accumulatedRoughness;
		int transmissiveTraversals;
		int traversals;
		uint depth;
		vec3 throughputColor;
		Material fogMaterial;

	};

	RenderState initRenderState() {

		RenderState result;
		result.firstRay = true;
		result.transmissiveRay = true;
		result.isShadowRay = false;
		result.accumulatedRoughness = 0.0;
		result.transmissiveTraversals = 0;
		result.traversals = 0;
		result.throughputColor = vec3( 1.0 );
		result.depth = 0u;
		result.fogMaterial.fogVolume = false;
		return result;

	}

`,Fi=`

	#define NO_HIT 0
	#define SURFACE_HIT 1
	#define LIGHT_HIT 2
	#define FOG_HIT 3

	// Passing the global variable 'lights' into this function caused shader program errors.
	// So global variables like 'lights' and 'bvh' were moved out of the function parameters.
	// For more information, refer to: https://github.com/gkjohnson/three-gpu-pathtracer/pull/457
	int traceScene(
		Ray ray, Material fogMaterial, inout SurfaceHit surfaceHit
	) {

		int result = NO_HIT;
		bool hit = bvhIntersectFirstHit( bvh, ray.origin, ray.direction, surfaceHit.faceIndices, surfaceHit.faceNormal, surfaceHit.barycoord, surfaceHit.side, surfaceHit.dist );

		#if FEATURE_FOG

		if ( fogMaterial.fogVolume ) {

			// offset the distance so we don't run into issues with particles on the same surface
			// as other objects
			float particleDist = intersectFogVolume( fogMaterial, rand( 1 ) );
			if ( particleDist + RAY_OFFSET < surfaceHit.dist ) {

				surfaceHit.side = 1.0;
				surfaceHit.faceNormal = normalize( - ray.direction );
				surfaceHit.dist = particleDist;
				return FOG_HIT;

			}

		}

		#endif

		if ( hit ) {

			result = SURFACE_HIT;

		}

		return result;

	}

`;class Ci extends et{onBeforeRender(){this.setDefine("FEATURE_DOF",this.physicalCamera.bokehSize===0?0:1),this.setDefine("FEATURE_BACKGROUND_MAP",this.backgroundMap?1:0),this.setDefine("FEATURE_FOG",this.materials.features.isUsed("FOG")?1:0)}constructor(e){super({transparent:!0,depthWrite:!1,defines:{FEATURE_MIS:1,FEATURE_RUSSIAN_ROULETTE:1,FEATURE_DOF:1,FEATURE_BACKGROUND_MAP:0,FEATURE_FOG:1,RANDOM_TYPE:2,CAMERA_TYPE:0,DEBUG_MODE:0,ATTR_NORMAL:0,ATTR_TANGENT:1,ATTR_UV:2,ATTR_COLOR:3,MATERIAL_PIXELS:at},uniforms:{resolution:{value:new oe},opacity:{value:1},bounces:{value:10},transmissiveBounces:{value:10},filterGlossyFactor:{value:0},physicalCamera:{value:new Rr},cameraWorldMatrix:{value:new V},invProjectionMatrix:{value:new V},bvh:{value:new $a},attributesArray:{value:new Br},materialIndexAttribute:{value:new Vt},materials:{value:new qr},textures:{value:new wt().texture},lights:{value:new zr},iesProfiles:{value:new wt(360,180,{type:B,wrapS:Q,wrapT:Q}).texture},environmentIntensity:{value:1},environmentRotation:{value:new V},envMapInfo:{value:new Fr},backgroundBlur:{value:0},backgroundMap:{value:null},backgroundAlpha:{value:1},backgroundIntensity:{value:1},backgroundRotation:{value:new V},seed:{value:0},sobolTexture:{value:null},stratifiedTexture:{value:new Zr},stratifiedOffsetTexture:{value:new ii(64,1)}},vertexShader:`

				varying vec2 vUv;
				void main() {

					vec4 mvPosition = vec4( position, 1.0 );
					mvPosition = modelViewMatrix * mvPosition;
					gl_Position = projectionMatrix * mvPosition;

					vUv = uv;

				}

			`,fragmentShader:`
				#define RAY_OFFSET 1e-4
				#define INFINITY 1e20

				precision highp isampler2D;
				precision highp usampler2D;
				precision highp sampler2DArray;
				vec4 envMapTexelToLinear( vec4 a ) { return a; }
				#include <common>

				// bvh intersection
				${Qa}
				${Ka}
				${Xa}

				// uniform structs
				${oi}
				${ni}
				${si}
				${li}
				${ci}

				// random
				#if RANDOM_TYPE == 2 	// Stratified List

					${vi}

				#elif RANDOM_TYPE == 1 	// Sobol

					${St}
					${Xt}
					${wr}

					#define rand(v) sobol(v)
					#define rand2(v) sobol2(v)
					#define rand3(v) sobol3(v)
					#define rand4(v) sobol4(v)

				#else 					// PCG

				${St}

					// Using the sobol functions seems to break the the compiler on MacOS
					// - specifically the "sobolReverseBits" function.
					uint sobolPixelIndex = 0u;
					uint sobolPathIndex = 0u;
					uint sobolBounceIndex = 0u;

					#define rand(v) pcgRand()
					#define rand2(v) pcgRand2()
					#define rand3(v) pcgRand3()
					#define rand4(v) pcgRand4()

				#endif

				// common
				${gi}
				${di}
				${Kt}
				${mi}
				${pi}

				// environment
				uniform EquirectHdrInfo envMapInfo;
				uniform mat4 environmentRotation;
				uniform float environmentIntensity;

				// lighting
				uniform sampler2DArray iesProfiles;
				uniform LightsInfo lights;

				// background
				uniform float backgroundBlur;
				uniform float backgroundAlpha;
				#if FEATURE_BACKGROUND_MAP

				uniform sampler2D backgroundMap;
				uniform mat4 backgroundRotation;
				uniform float backgroundIntensity;

				#endif

				// camera
				uniform mat4 cameraWorldMatrix;
				uniform mat4 invProjectionMatrix;
				#if FEATURE_DOF

				uniform PhysicalCamera physicalCamera;

				#endif

				// geometry
				uniform sampler2DArray attributesArray;
				uniform usampler2D materialIndexAttribute;
				uniform sampler2D materials;
				uniform sampler2DArray textures;
				uniform BVH bvh;

				// path tracer
				uniform int bounces;
				uniform int transmissiveBounces;
				uniform float filterGlossyFactor;
				uniform int seed;

				// image
				uniform vec2 resolution;
				uniform float opacity;

				varying vec2 vUv;

				// globals
				mat3 envRotation3x3;
				mat3 invEnvRotation3x3;
				float lightsDenom;

				// sampling
				${hi}
				${ui}
				${fi}

				${_i}
				${yi}
				${wi}
				${Ti}
				${bi}
				${xi}

				float applyFilteredGlossy( float roughness, float accumulatedRoughness ) {

					return clamp(
						max(
							roughness,
							accumulatedRoughness * filterGlossyFactor * 5.0 ),
						0.0,
						1.0
					);

				}

				vec3 sampleBackground( vec3 direction, vec2 uv ) {

					vec3 sampleDir = sampleHemisphere( direction, uv ) * 0.5 * backgroundBlur;

					#if FEATURE_BACKGROUND_MAP

					sampleDir = normalize( mat3( backgroundRotation ) * direction + sampleDir );
					return backgroundIntensity * sampleEquirectColor( backgroundMap, sampleDir );

					#else

					sampleDir = normalize( envRotation3x3 * direction + sampleDir );
					return environmentIntensity * sampleEquirectColor( envMapInfo.map, sampleDir );

					#endif

				}

				${Ai}
				${Ii}
				${Fi}
				${Si}
				${Ri}
				${Mi}

				void main() {

					// init
					rng_initialize( gl_FragCoord.xy, seed );
					sobolPixelIndex = ( uint( gl_FragCoord.x ) << 16 ) | uint( gl_FragCoord.y );
					sobolPathIndex = uint( seed );

					// get camera ray
					Ray ray = getCameraRay();

					// inverse environment rotation
					envRotation3x3 = mat3( environmentRotation );
					invEnvRotation3x3 = inverse( envRotation3x3 );
					lightsDenom =
						( environmentIntensity == 0.0 || envMapInfo.totalSum == 0.0 ) && lights.count != 0u ?
							float( lights.count ) :
							float( lights.count + 1u );

					// final color
					gl_FragColor = vec4( 0, 0, 0, 1 );

					// surface results
					SurfaceHit surfaceHit;
					ScatterRecord scatterRec;

					// path tracing state
					RenderState state = initRenderState();
					state.transmissiveTraversals = transmissiveBounces;
					#if FEATURE_FOG

					state.fogMaterial.fogVolume = bvhIntersectFogVolumeHit(
						ray.origin, - ray.direction,
						materialIndexAttribute, materials,
						state.fogMaterial
					);

					#endif

					for ( int i = 0; i < bounces; i ++ ) {

						sobolBounceIndex ++;

						state.depth ++;
						state.traversals = bounces - i;
						state.firstRay = i == 0 && state.transmissiveTraversals == transmissiveBounces;

						int hitType = traceScene( ray, state.fogMaterial, surfaceHit );

						// check if we intersect any lights and accumulate the light contribution
						// TODO: we can add support for light surface rendering in the else condition if we
						// add the ability to toggle visibility of the the light
						if ( ! state.firstRay && ! state.transmissiveRay ) {

							LightRecord lightRec;
							float lightDist = hitType == NO_HIT ? INFINITY : surfaceHit.dist;
							for ( uint i = 0u; i < lights.count; i ++ ) {

								if (
									intersectLightAtIndex( lights.tex, ray.origin, ray.direction, i, lightRec ) &&
									lightRec.dist < lightDist
								) {

									#if FEATURE_MIS

									// weight the contribution
									// NOTE: Only area lights are supported for forward sampling and can be hit
									float misWeight = misHeuristic( scatterRec.pdf, lightRec.pdf / lightsDenom );
									gl_FragColor.rgb += lightRec.emission * state.throughputColor * misWeight;

									#else

									gl_FragColor.rgb += lightRec.emission * state.throughputColor;

									#endif

								}

							}

						}

						if ( hitType == NO_HIT ) {

							if ( state.firstRay || state.transmissiveRay ) {

								gl_FragColor.rgb += sampleBackground( ray.direction, rand2( 2 ) ) * state.throughputColor;
								gl_FragColor.a = backgroundAlpha;

							} else {

								#if FEATURE_MIS

								// get the PDF of the hit envmap point
								vec3 envColor;
								float envPdf = sampleEquirect( envRotation3x3 * ray.direction, envColor );
								envPdf /= lightsDenom;

								// and weight the contribution
								float misWeight = misHeuristic( scatterRec.pdf, envPdf );
								gl_FragColor.rgb += environmentIntensity * envColor * state.throughputColor * misWeight;

								#else

								gl_FragColor.rgb +=
									environmentIntensity *
									sampleEquirectColor( envMapInfo.map, envRotation3x3 * ray.direction ) *
									state.throughputColor;

								#endif

							}
							break;

						}

						uint materialIndex = uTexelFetch1D( materialIndexAttribute, surfaceHit.faceIndices.x ).r;
						Material material = readMaterialInfo( materials, materialIndex );

						#if FEATURE_FOG

						if ( hitType == FOG_HIT ) {

							material = state.fogMaterial;
							state.accumulatedRoughness += 0.2;

						} else if ( material.fogVolume ) {

							state.fogMaterial = material;
							state.fogMaterial.fogVolume = surfaceHit.side == 1.0;

							ray.origin = stepRayOrigin( ray.origin, ray.direction, - surfaceHit.faceNormal, surfaceHit.dist );

							i -= sign( state.transmissiveTraversals );
							state.transmissiveTraversals -= sign( state.transmissiveTraversals );
							continue;

						}

						#endif

						// early out if this is a matte material
						if ( material.matte && state.firstRay ) {

							gl_FragColor = vec4( 0.0 );
							break;

						}

						// if we've determined that this is a shadow ray and we've hit an item with no shadow casting
						// then skip it
						if ( ! material.castShadow && state.isShadowRay ) {

							ray.origin = stepRayOrigin( ray.origin, ray.direction, - surfaceHit.faceNormal, surfaceHit.dist );
							continue;

						}

						SurfaceRecord surf;
						if (
							getSurfaceRecord(
								material, surfaceHit, attributesArray, state.accumulatedRoughness,
								surf
							) == SKIP_SURFACE
						) {

							// only allow a limited number of transparency discards otherwise we could
							// crash the context with too long a loop.
							i -= sign( state.transmissiveTraversals );
							state.transmissiveTraversals -= sign( state.transmissiveTraversals );

							ray.origin = stepRayOrigin( ray.origin, ray.direction, - surfaceHit.faceNormal, surfaceHit.dist );
							continue;

						}

						scatterRec = bsdfSample( - ray.direction, surf );
						state.isShadowRay = scatterRec.specularPdf < rand( 4 );

						bool isBelowSurface = ! surf.volumeParticle && dot( scatterRec.direction, surf.faceNormal ) < 0.0;
						vec3 hitPoint = stepRayOrigin( ray.origin, ray.direction, isBelowSurface ? - surf.faceNormal : surf.faceNormal, surfaceHit.dist );

						// next event estimation
						#if FEATURE_MIS

						gl_FragColor.rgb += directLightContribution( - ray.direction, surf, state, hitPoint );

						#endif

						// accumulate a roughness value to offset diffuse, specular, diffuse rays that have high contribution
						// to a single pixel resulting in fireflies
						// TODO: handle transmissive surfaces
						if ( ! surf.volumeParticle && ! isBelowSurface ) {

							// determine if this is a rough normal or not by checking how far off straight up it is
							vec3 halfVector = normalize( - ray.direction + scatterRec.direction );
							state.accumulatedRoughness += max(
								sin( acosApprox( dot( halfVector, surf.normal ) ) ),
								sin( acosApprox( dot( halfVector, surf.clearcoatNormal ) ) )
							);

							state.transmissiveRay = false;

						}

						// accumulate emissive color
						gl_FragColor.rgb += ( surf.emission * state.throughputColor );

						// skip the sample if our PDF or ray is impossible
						if ( scatterRec.pdf <= 0.0 || ! isDirectionValid( scatterRec.direction, surf.normal, surf.faceNormal ) ) {

							break;

						}

						// if we're bouncing around the inside a transmissive material then decrement
						// perform this separate from a bounce
						bool isTransmissiveRay = ! surf.volumeParticle && dot( scatterRec.direction, surf.faceNormal * surfaceHit.side ) < 0.0;
						if ( ( isTransmissiveRay || isBelowSurface ) && state.transmissiveTraversals > 0 ) {

							state.transmissiveTraversals --;
							i --;

						}

						//

						// handle throughput color transformation
						// attenuate the throughput color by the medium color
						if ( ! surf.frontFace ) {

							state.throughputColor *= transmissionAttenuation( surfaceHit.dist, surf.attenuationColor, surf.attenuationDistance );

						}

						#if FEATURE_RUSSIAN_ROULETTE

						// russian roulette path termination
						// https://www.arnoldrenderer.com/research/physically_based_shader_design_in_arnold.pdf
						uint minBounces = 3u;
						float depthProb = float( state.depth < minBounces );

						float rrProb = luminance( state.throughputColor * scatterRec.color / scatterRec.pdf );
						rrProb /= luminance( state.throughputColor );
						rrProb = sqrt( rrProb );
						rrProb = max( rrProb, depthProb );
						rrProb = min( rrProb, 1.0 );
						if ( rand( 8 ) > rrProb ) {

							break;

						}

						// perform sample clamping here to avoid bright pixels
						state.throughputColor *= min( 1.0 / rrProb, 20.0 );

						#endif

						// adjust the throughput and discard and exit if we find discard the sample if there are any NaNs
						state.throughputColor *= scatterRec.color / scatterRec.pdf;
						if ( any( isnan( state.throughputColor ) ) || any( isinf( state.throughputColor ) ) ) {

							break;

						}

						//

						// prepare for next ray
						ray.direction = scatterRec.direction;
						ray.origin = hitPoint;

					}

					gl_FragColor.a *= opacity;

					#if DEBUG_MODE == 1

					// output the number of rays checked in the path and number of
					// transmissive rays encountered.
					gl_FragColor.rgb = vec3(
						float( state.depth ),
						transmissiveBounces - state.transmissiveTraversals,
						0.0
					);
					gl_FragColor.a = 1.0;

					#endif

				}

			`}),this.setValues(e)}}function*Di(){const{_renderer:r,_fsQuad:e,_blendQuad:t,_primaryTarget:a,_blendTargets:o,_sobolTarget:s,_subframe:i,alpha:l,material:c}=this,d=new me,h=new me,f=t.material;let[n,g]=o;for(;;){l?(f.opacity=this._opacityFactor/(this.samples+1),c.blending=Ie,c.opacity=1):(c.opacity=this._opacityFactor/(this.samples+1),c.blending=Bt);const[m,p,u,v]=i,x=a.width,b=a.height;c.resolution.set(x*u,b*v),c.sobolTexture=s.texture,c.stratifiedTexture.init(20,c.bounces+c.transmissiveBounces+5),c.stratifiedTexture.next(),c.seed++;const T=this.tiles.x||1,y=this.tiles.y||1,I=T*y,O=Math.ceil(x*u),X=Math.ceil(b*v),ge=Math.floor(m*x),se=Math.floor(p*b),K=Math.ceil(O/T),Me=Math.ceil(X/y);for(let U=0;U<y;U++)for(let G=0;G<T;G++){const ve=r.getRenderTarget(),xe=r.autoClear,Ae=r.getScissorTest();r.getScissor(d),r.getViewport(h);let be=G,Fe=U;if(!this.stableTiles){const Z=this._currentTile%(T*y);be=Z%T,Fe=~~(Z/T),this._currentTile=Z+1}const Ce=y-Fe-1;a.scissor.set(ge+be*K,se+Ce*Me,Math.min(K,O-be*K),Math.min(Me,X-Ce*Me)),a.viewport.set(ge,se,O,X),r.setRenderTarget(a),r.setScissorTest(!0),r.autoClear=!1,e.render(r),r.setViewport(h),r.setScissor(d),r.setScissorTest(Ae),r.setRenderTarget(ve),r.autoClear=xe,l&&(f.target1=n.texture,f.target2=a.texture,r.setRenderTarget(g),t.render(r),r.setRenderTarget(ve)),this.samples+=1/I,G===T-1&&U===y-1&&(this.samples=Math.round(this.samples)),yield}[n,g]=[g,n]}}const It=new j;class Rt{get material(){return this._fsQuad.material}set material(e){this._fsQuad.material.removeEventListener("recompilation",this._compileFunction),e.addEventListener("recompilation",this._compileFunction),this._fsQuad.material=e}get target(){return this._alpha?this._blendTargets[1]:this._primaryTarget}set alpha(e){this._alpha!==e&&(e||(this._blendTargets[0].dispose(),this._blendTargets[1].dispose()),this._alpha=e,this.reset())}get alpha(){return this._alpha}get isCompiling(){return!!this._compilePromise}constructor(e){this.camera=null,this.tiles=new oe(3,3),this.stableNoise=!1,this.stableTiles=!0,this.samples=0,this._subframe=new me(0,0,1,1),this._opacityFactor=1,this._renderer=e,this._alpha=!1,this._fsQuad=new pe(new Ci),this._blendQuad=new pe(new yr),this._task=null,this._currentTile=0,this._compilePromise=null,this._sobolTarget=new Sr().generate(e),this._primaryTarget=new fe(1,1,{format:A,type:M,magFilter:_,minFilter:_}),this._blendTargets=[new fe(1,1,{format:A,type:M,magFilter:_,minFilter:_}),new fe(1,1,{format:A,type:M,magFilter:_,minFilter:_})],this._compileFunction=()=>{const t=this.compileMaterial(this._fsQuad._mesh);t.then(()=>{this._compilePromise===t&&(this._compilePromise=null)}),this._compilePromise=t},this.material.addEventListener("recompilation",this._compileFunction)}compileMaterial(){return this._renderer.compileAsync(this._fsQuad._mesh)}setCamera(e){const{material:t}=this;t.cameraWorldMatrix.copy(e.matrixWorld),t.invProjectionMatrix.copy(e.projectionMatrixInverse),t.physicalCamera.updateFrom(e);let a=0;e.projectionMatrix.elements[15]>0&&(a=1),e.isEquirectCamera&&(a=2),t.setDefine("CAMERA_TYPE",a),this.camera=e}setSize(e,t){e=Math.ceil(e),t=Math.ceil(t),!(this._primaryTarget.width===e&&this._primaryTarget.height===t)&&(this._primaryTarget.setSize(e,t),this._blendTargets[0].setSize(e,t),this._blendTargets[1].setSize(e,t),this.reset())}getSize(e){e.x=this._primaryTarget.width,e.y=this._primaryTarget.height}dispose(){this._primaryTarget.dispose(),this._blendTargets[0].dispose(),this._blendTargets[1].dispose(),this._sobolTarget.dispose(),this._fsQuad.dispose(),this._blendQuad.dispose(),this._task=null}reset(){const{_renderer:e,_primaryTarget:t,_blendTargets:a}=this,o=e.getRenderTarget(),s=e.getClearAlpha();e.getClearColor(It),e.setRenderTarget(t),e.setClearColor(0,0),e.clearColor(),e.setRenderTarget(a[0]),e.setClearColor(0,0),e.clearColor(),e.setRenderTarget(a[1]),e.setClearColor(0,0),e.clearColor(),e.setClearColor(It,s),e.setRenderTarget(o),this.samples=0,this._task=null,this.material.stratifiedTexture.stableNoise=this.stableNoise,this.stableNoise&&(this.material.seed=0,this.material.stratifiedTexture.reset())}update(){this.material.onBeforeRender(),!this.isCompiling&&(this._task||(this._task=Di.call(this)),this._task.next())}}const ae=new oe,Mt=new oe,ze=new ga,He=new j;class Pi extends E{constructor(e=512,t=512){super(new Float32Array(e*t*4),e,t,A,M,Ze,$,Q,H,H),this.generationCallback=null}update(){this.dispose(),this.needsUpdate=!0;const{data:e,width:t,height:a}=this.image;for(let o=0;o<t;o++)for(let s=0;s<a;s++){Mt.set(t,a),ae.set(o/t,s/a),ae.x-=.5,ae.y=1-ae.y,ze.theta=ae.x*2*Math.PI,ze.phi=ae.y*Math.PI,ze.radius=1,this.generationCallback(ze,ae,Mt,He);const l=4*(s*t+o);e[l+0]=He.r,e[l+1]=He.g,e[l+2]=He.b,e[l+3]=1}}copy(e){return super.copy(e),this.generationCallback=e.generationCallback,this}}const At=new P;class Ei extends Pi{constructor(e=512){super(e,e),this.topColor=new j().set(16777215),this.bottomColor=new j().set(0),this.exponent=2,this.generationCallback=(t,a,o,s)=>{At.setFromSpherical(t);const i=At.y*.5+.5;s.lerpColors(this.bottomColor,this.topColor,i**this.exponent)}}copy(e){return super.copy(e),this.topColor.copy(e.topColor),this.bottomColor.copy(e.bottomColor),this}}class Oi extends Le{get map(){return this.uniforms.map.value}set map(e){this.uniforms.map.value=e}get opacity(){return this.uniforms.opacity.value}set opacity(e){this.uniforms&&(this.uniforms.opacity.value=e)}constructor(e){super({uniforms:{map:{value:null},opacity:{value:1}},vertexShader:`
				varying vec2 vUv;
				void main() {

					vUv = uv;
					gl_Position = projectionMatrix * modelViewMatrix * vec4( position, 1.0 );

				}
			`,fragmentShader:`
				uniform sampler2D map;
				uniform float opacity;
				varying vec2 vUv;

				vec4 clampedTexelFatch( sampler2D map, ivec2 px, int lod ) {

					vec4 res = texelFetch( map, ivec2( px.x, px.y ), 0 );

					#if defined( TONE_MAPPING )

					res.xyz = toneMapping( res.xyz );

					#endif

			  		return linearToOutputTexel( res );

				}

				void main() {

					vec2 size = vec2( textureSize( map, 0 ) );
					vec2 pxUv = vUv * size;
					vec2 pxCurr = floor( pxUv );
					vec2 pxFrac = fract( pxUv ) - 0.5;
					vec2 pxOffset;
					pxOffset.x = pxFrac.x > 0.0 ? 1.0 : - 1.0;
					pxOffset.y = pxFrac.y > 0.0 ? 1.0 : - 1.0;

					vec2 pxNext = clamp( pxOffset + pxCurr, vec2( 0.0 ), size - 1.0 );
					vec2 alpha = abs( pxFrac );

					vec4 p1 = mix(
						clampedTexelFatch( map, ivec2( pxCurr.x, pxCurr.y ), 0 ),
						clampedTexelFatch( map, ivec2( pxNext.x, pxCurr.y ), 0 ),
						alpha.x
					);

					vec4 p2 = mix(
						clampedTexelFatch( map, ivec2( pxCurr.x, pxNext.y ), 0 ),
						clampedTexelFatch( map, ivec2( pxNext.x, pxNext.y ), 0 ),
						alpha.x
					);

					gl_FragColor = mix( p1, p2, alpha.y );
					gl_FragColor.a *= opacity;
					#include <premultiplied_alpha_fragment>

				}
			`}),this.setValues(e)}}class ki extends Le{constructor(){super({uniforms:{envMap:{value:null},flipEnvMap:{value:-1}},vertexShader:`
				varying vec2 vUv;
				void main() {

					vUv = uv;
					gl_Position = projectionMatrix * modelViewMatrix * vec4( position, 1.0 );

				}`,fragmentShader:`
				#define ENVMAP_TYPE_CUBE_UV

				uniform samplerCube envMap;
				uniform float flipEnvMap;
				varying vec2 vUv;

				#include <common>
				#include <cube_uv_reflection_fragment>

				${Kt}

				void main() {

					vec3 rayDirection = equirectUvToDirection( vUv );
					rayDirection.x *= flipEnvMap;
					gl_FragColor = textureCube( envMap, rayDirection );

				}`}),this.depthWrite=!1,this.depthTest=!1}}class Ft{constructor(e){this._renderer=e,this._quad=new pe(new ki)}generate(e,t=null,a=null){if(!e.isCubeTexture)throw new Error("CubeToEquirectMaterial: Source can only be cube textures.");const o=e.images[0],s=this._renderer,i=this._quad;t===null&&(t=4*o.height),a===null&&(a=2*o.height);const l=new fe(t,a,{type:M,colorSpace:o.colorSpace}),c=o.height,d=Math.log2(c)-2,h=1/c,f=1/(3*Math.max(Math.pow(2,d),112));i.material.defines.CUBEUV_MAX_MIP=`${d}.0`,i.material.defines.CUBEUV_TEXEL_WIDTH=f,i.material.defines.CUBEUV_TEXEL_HEIGHT=h,i.material.uniforms.envMap.value=e,i.material.uniforms.flipEnvMap.value=e.isRenderTargetTexture?1:-1,i.material.needsUpdate=!0;const n=s.getRenderTarget(),g=s.autoClear;s.autoClear=!0,s.setRenderTarget(l),i.render(s),s.setRenderTarget(n),s.autoClear=g;const m=new Uint16Array(t*a*4),p=new Float32Array(t*a*4);s.readRenderTargetPixels(l,0,0,t,a,p),l.dispose();for(let v=0,x=p.length;v<x;v++)m[v]=W.toHalfFloat(p[v]);const u=new E(m,t,a,A,B);return u.minFilter=va,u.magFilter=H,u.wrapS=$,u.wrapT=$,u.mapping=Ze,u.needsUpdate=!0,u}dispose(){this._quad.dispose()}}function zi(r){return r.extensions.get("EXT_float_blend")}const ce=new oe;class Hi{get multipleImportanceSampling(){return!!this._pathTracer.material.defines.FEATURE_MIS}set multipleImportanceSampling(e){this._pathTracer.material.setDefine("FEATURE_MIS",e?1:0)}get transmissiveBounces(){return this._pathTracer.material.transmissiveBounces}set transmissiveBounces(e){this._pathTracer.material.transmissiveBounces=e}get bounces(){return this._pathTracer.material.bounces}set bounces(e){this._pathTracer.material.bounces=e}get filterGlossyFactor(){return this._pathTracer.material.filterGlossyFactor}set filterGlossyFactor(e){this._pathTracer.material.filterGlossyFactor=e}get samples(){return this._pathTracer.samples}get target(){return this._pathTracer.target}get tiles(){return this._pathTracer.tiles}get stableNoise(){return this._pathTracer.stableNoise}set stableNoise(e){this._pathTracer.stableNoise=e}get isCompiling(){return!!this._pathTracer.isCompiling}constructor(e){this._renderer=e,this._generator=new br,this._pathTracer=new Rt(e),this._queueReset=!1,this._clock=new xa,this._compilePromise=null,this._lowResPathTracer=new Rt(e),this._lowResPathTracer.tiles.set(1,1),this._quad=new pe(new Oi({map:null,transparent:!0,blending:Ie,premultipliedAlpha:e.getContextAttributes().premultipliedAlpha})),this._materials=null,this._previousEnvironment=null,this._previousBackground=null,this._internalBackground=null,this.renderDelay=100,this.minSamples=5,this.fadeDuration=500,this.enablePathTracing=!0,this.pausePathTracing=!1,this.dynamicLowRes=!1,this.lowResScale=.25,this.renderScale=1,this.synchronizeRenderSize=!0,this.rasterizeScene=!0,this.renderToCanvas=!0,this.textureSize=new oe(1024,1024),this.rasterizeSceneCallback=(t,a)=>{this._renderer.render(t,a)},this.renderToCanvasCallback=(t,a,o)=>{const s=a.autoClear;a.autoClear=!1,o.render(a),a.autoClear=s},this.setScene(new q,new Ke)}setBVHWorker(e){this._generator.setBVHWorker(e)}setScene(e,t,a={}){e.updateMatrixWorld(!0),t.updateMatrixWorld();const o=this._generator;if(o.setObjects(e),this._buildAsync)return o.generateAsync(a.onProgress).then(s=>this._updateFromResults(e,t,s));{const s=o.generate();return this._updateFromResults(e,t,s)}}setSceneAsync(...e){this._buildAsync=!0;const t=this.setScene(...e);return this._buildAsync=!1,t}setCamera(e){this.camera=e,this.updateCamera()}updateCamera(){const e=this.camera;e.updateMatrixWorld(),this._pathTracer.setCamera(e),this._lowResPathTracer.setCamera(e),this.reset()}updateMaterials(){const e=this._pathTracer.material,t=this._renderer,a=this._materials,o=this.textureSize,s=Wr(a);e.textures.setTextures(t,s,o.x,o.y),e.materials.updateFrom(a,s),this.reset()}updateLights(){const e=this.scene,t=this._renderer,a=this._pathTracer.material,o=Ur(e),s=Lr(o);a.lights.updateFrom(o,s),a.iesProfiles.setTextures(t,s),this.reset()}updateEnvironment(){const e=this.scene,t=this._pathTracer.material;if(this._internalBackground&&(this._internalBackground.dispose(),this._internalBackground=null),t.backgroundBlur=e.backgroundBlurriness,t.backgroundIntensity=e.backgroundIntensity??1,t.backgroundRotation.makeRotationFromEuler(e.backgroundRotation).invert(),e.background===null)t.backgroundMap=null,t.backgroundAlpha=0;else if(e.background.isColor){this._colorBackground=this._colorBackground||new Ei(16);const a=this._colorBackground;a.topColor.equals(e.background)||(a.topColor.set(e.background),a.bottomColor.set(e.background),a.update()),t.backgroundMap=a,t.backgroundAlpha=1}else if(e.background.isCubeTexture){if(e.background!==this._previousBackground){const a=new Ft(this._renderer).generate(e.background);this._internalBackground=a,t.backgroundMap=a,t.backgroundAlpha=1}}else t.backgroundMap=e.background,t.backgroundAlpha=1;if(t.environmentIntensity=e.environment!==null?e.environmentIntensity??1:0,t.environmentRotation.makeRotationFromEuler(e.environmentRotation).invert(),this._previousEnvironment!==e.environment&&e.environment!==null)if(e.environment.isCubeTexture){const a=new Ft(this._renderer).generate(e.environment);t.envMapInfo.updateFrom(a)}else t.envMapInfo.updateFrom(e.environment);this._previousEnvironment=e.environment,this._previousBackground=e.background,this.reset()}_updateFromResults(e,t,a){const{materials:o,geometry:s,bvh:i,bvhChanged:l,needsMaterialIndexUpdate:c}=a;this._materials=o;const h=this._pathTracer.material;return l&&(h.bvh.updateFrom(i),h.attributesArray.updateFrom(s.attributes.normal,s.attributes.tangent,s.attributes.uv,s.attributes.color)),c&&h.materialIndexAttribute.updateFrom(s.attributes.materialIndex),this._previousScene=e,this.scene=e,this.camera=t,this.updateCamera(),this.updateMaterials(),this.updateEnvironment(),this.updateLights(),a}renderSample(){const e=this._lowResPathTracer,t=this._pathTracer,a=this._renderer,o=this._clock,s=this._quad;this._updateScale(),this._queueReset&&(t.reset(),e.reset(),this._queueReset=!1,s.material.opacity=0,o.start());const i=o.getDelta()*1e3,l=o.getElapsedTime()*1e3;if(!this.pausePathTracing&&this.enablePathTracing&&this.renderDelay<=l&&!this.isCompiling&&t.update(),t.alpha=t.material.backgroundAlpha!==1||!zi(a),e.alpha=t.alpha,this.renderToCanvas){const c=this._renderer,d=this.minSamples;if(l>=this.renderDelay&&this.samples>=this.minSamples&&(this.fadeDuration!==0?s.material.opacity=Math.min(s.material.opacity+i/this.fadeDuration,1):s.material.opacity=1),!this.enablePathTracing||this.samples<d||s.material.opacity<1){if(this.dynamicLowRes&&!this.isCompiling){e.samples<1&&(e.material=t.material,e.update());const h=s.material.opacity;s.material.opacity=1-s.material.opacity,s.material.map=e.target.texture,s.render(c),s.material.opacity=h}(!this.dynamicLowRes&&this.rasterizeScene||this.dynamicLowRes&&this.isCompiling)&&this.rasterizeSceneCallback(this.scene,this.camera)}this.enablePathTracing&&s.material.opacity>0&&(s.material.opacity<1&&(s.material.blending=this.dynamicLowRes?ba:Bt),s.material.map=t.target.texture,this.renderToCanvasCallback(t.target,c,s),s.material.blending=Ie)}}reset(){this._queueReset=!0,this._pathTracer.samples=0}dispose(){this._quad.dispose(),this._quad.material.dispose(),this._pathTracer.dispose()}_updateScale(){if(this.synchronizeRenderSize){this._renderer.getDrawingBufferSize(ce);const e=Math.floor(this.renderScale*ce.x),t=Math.floor(this.renderScale*ce.y);if(this._pathTracer.getSize(ce),ce.x!==e||ce.y!==t){const a=this.lowResScale;this._pathTracer.setSize(e,t),this._lowResPathTracer.setSize(Math.floor(e*a),Math.floor(t*a))}}}}class Bi extends ya{constructor(...e){super(...e),this.iesMap=null,this.radius=0}copy(e,t){return super.copy(e,t),this.iesMap=e.iesMap,this.radius=e.radius,this}}let Ct=!1;function Ni(){if(Ct)return;const r=new Nt(0,0,0);Object.getOwnPropertyDescriptor(q.prototype,"environmentRotation")||Object.defineProperty(q.prototype,"environmentRotation",{get(){return this._environmentRotation??r},set(e){this._environmentRotation=e},configurable:!0}),Object.getOwnPropertyDescriptor(q.prototype,"backgroundRotation")||Object.defineProperty(q.prototype,"backgroundRotation",{get(){return this._backgroundRotation??r},set(e){this._backgroundRotation=e},configurable:!0}),Object.getOwnPropertyDescriptor(q.prototype,"environmentIntensity")||Object.defineProperty(q.prototype,"environmentIntensity",{get(){return this._environmentIntensity??1},set(e){this._environmentIntensity=e},configurable:!0}),Ct=!0}const Re=Wt("HQThumbnailScheduler"),Li=2e3,Wi=1500,Ui=500,Dt=300*1e3,Y=new Map;let re=null,$e=!1,Se=!1,Zt=0,Ne=!1,Pt=!1,he=null;function Gi(){if(Pt||typeof window>"u")return;Pt=!0;const r=()=>{Zt=performance.now()};window.addEventListener("pointerdown",()=>{Se=!0,r()},{capture:!0,passive:!0}),window.addEventListener("pointerup",()=>{Se=!1,r()},{capture:!0,passive:!0}),window.addEventListener("pointercancel",()=>{Se=!1,r()},{capture:!0,passive:!0}),window.addEventListener("wheel",r,{capture:!0,passive:!0}),window.addEventListener("pagehide",()=>$i("pagehide"))}function Et(){return Ne||Se||performance.now()-Zt<Wi}async function qi(){if(!Et())return;Re.info(`HQ queue paused (${Ne?"render window open":"user interacting"}), ${Y.size+1} job(s) waiting`);const r=performance.now()+Dt;for(;Et();){if(performance.now()>r){console.error(`[SAFEGUARD] hq-thumbnail-scheduler: waitForIdle exceeded ${Dt/1e3}s (renderWindowOpen=${Ne}, pointerDown=${Se}) — proceeding anyway`);break}await new Promise(e=>setTimeout(e,Ui))}Re.info("HQ queue resumed (idle)")}function vo(r){Ne=r,r&&Re.info("Render Window open — HQ thumbnail queue paused")}function Vi(){return he?.signal??null}function $i(r){Y.size===0&&!he||(Re.info(`Cancelling HQ thumbnails (${r}): ${Y.size} queued, ${he?1:0} in flight`),Y.clear(),re&&(clearTimeout(re),re=null),he?.abort(r))}async function ji(){if(!$e){$e=!0;try{for(;Y.size>0;){await qi();const r=Y.entries().next();if(r.done)break;const[e,t]=r.value;Y.delete(e),he=new AbortController;try{await t()}catch(a){Re.warn(`HQ thumbnail job failed for ${e}:`,a)}finally{he=null}}}finally{$e=!1}}}function xo(r,e){Gi(),Y.set(r,e),re&&clearTimeout(re),re=setTimeout(()=>{re=null,ji()},Li)}const D=Wt("HQThumbnail"),Yi={width:1024,height:1024,samples:128,bounces:6,backgroundColor:"#1a1a1a",hdriEnabled:!0,hdriPath:"/hdri/outdoor.hdr",environmentIntensity:1,hdriRotation:192,sunLight:{enabled:!0,azimuth:127,elevation:-50,color:"#fffaf0",intensity:2,radius:2.6},floor:{enabled:!1,color:"#808080",yOffset:0,scale:5},exposure:1,format:"image/webp",quality:.8},ye=new Map,Qi=3;async function Xi(r){let e=ye.get(r);if(!e)try{const a=await fetch(r);if(!a.ok)return D.warn(`HDR fetch failed for ${r}: HTTP ${a.status}`),null;for(e=await a.arrayBuffer(),ye.set(r,e);ye.size>Qi;){const o=ye.keys().next().value;if(o===void 0)break;ye.delete(o)}}catch(a){return D.warn(`HDR fetch failed for ${r}:`,a),null}const t=URL.createObjectURL(new Blob([e]));try{return await new Promise(a=>{new Aa().load(t,o=>{o.mapping=Ze,a(o)},void 0,o=>{D.warn(`HDR parse failed for ${r}:`,o),a(null)})})}finally{URL.revokeObjectURL(t)}}function Ki(r){return new Promise((e,t)=>{new Fa().parse(r,"",o=>e(o),o=>t(o))})}function Zi(r){return r[0]===255&&r[1]===216?"image/jpeg":r[0]===137&&r[1]===80&&r[2]===78&&r[3]===71?"image/png":r.length>11&&r[8]===87&&r[9]===69&&r[10]===66&&r[11]===80?"image/webp":"image/png"}function Jt(r,e){if(!e)return;const t=e[r];if(t)return t;for(const a of Object.keys(e))if(Ut(r,a))return e[a]}function Ji(r,e){if(!e)return;const t=e.get(r);if(t)return t;for(const[a,o]of e)if(Ut(r,a))return o}async function Ot(r){const e=new Map;for(const[t,a]of r)try{const o=new Uint8Array(a),s=Zi(o),i=new Blob([a],{type:s}),l=await createImageBitmap(i,{colorSpaceConversion:"none"}),c=new Ca(l);c.colorSpace=t==="base_color"?Lt:Da,c.flipY=!1,c.needsUpdate=!0,e.set(t,c)}catch(o){D.warn(`Failed to create texture "${t}":`,o)}return e}function eo(r,e){const t=e.get("base_color");t&&(r.map=t,r.color.set(16777215));const a=e.get("normal");a&&(r.normalMap=a);const o=e.get("metallic_roughness")??e.get("metallic")??e.get("roughness");o&&(r.metalnessMap=o,r.roughnessMap=o,r.metalness=1,r.roughness=1);const s=e.get("ao")??e.get("occlusion");s&&(r.aoMap=s);const i=e.get("emissive");i&&(r.emissiveMap=i,r.emissive.set(16777215))}async function to(r,e,t,a,o){const s=[],i=e&&e.size>0?await Ot(e):new Map;for(const d of i.values())s.push(d);const l=new Map,c=[];r.traverse(d=>{d instanceof _e&&c.push(d)});for(const d of c){const h=d.material;if(!h?.isMeshStandardMaterial)continue;const f=Ji(h.name,o);let n;if(f){let m=l.get(h.name);if(!m){m=await Ot(f),l.set(h.name,m);for(const p of m.values())s.push(p)}n=m}else n=i;eo(h,n);const g=Jt(h.name,a)??t;g&&Gt(h,g),h.needsUpdate=!0}return s}const ao=3e4;let F=null,ie=null;function ea(){if(F){D.info("Disposing shared HQ render context (queue idle)");try{F.pathTracer.dispose()}catch{}F.renderer.dispose(),F.renderer.forceContextLoss(),F=null}}function ro(r,e){if(ie&&(clearTimeout(ie),ie=null),F&&F.renderer.getContext().isContextLost()&&(D.warn("Shared HQ context was lost — recreating"),ea()),!F){const t=document.createElement("canvas");t.width=r,t.height=e;const a=new Ra({canvas:t,antialias:!1,alpha:!1,preserveDrawingBuffer:!0,powerPreference:"high-performance"});a.outputColorSpace=Lt,a.toneMapping=Ma;const o=new Hi(a);F={canvas:t,renderer:a,pathTracer:o},D.info("Created shared HQ render context")}return(F.canvas.width!==r||F.canvas.height!==e)&&(F.canvas.width=r,F.canvas.height=e),F.renderer.setSize(r,e),F}function io(){ie&&clearTimeout(ie),ie=setTimeout(()=>{ie=null,ea()},ao)}async function ta(r,e){const t={...Yi,...e},a=performance.now(),o=Vi();D.info(`Starting HQ render: ${t.width}x${t.height}, ${t.samples} samples, ${t.bounces} bounces`),Ni();const{canvas:s,renderer:i,pathTracer:l}=ro(t.width,t.height);i.toneMappingExposure=t.exposure;const c=new q;c.background=new j(t.backgroundColor);const d=t.hdriEnabled!==!1;let h=null;d&&(h=await Xi(t.hdriPath),h&&(c.environment=h,c.environmentIntensity=t.environmentIntensity,t.hdriRotation&&(c.environmentRotation=new Nt(0,t.hdriRotation*Math.PI/180,0))));const f=[],n=[],g=[],m=[],p=t.width/t.height,x=2*Math.atan(24/(2*200)),b=x*180/Math.PI,T=new Ke(b,p,.1,1e4);try{const y=await Ki(r);if(c.add(y.scene),m.push(...y.animations??[]),t.orientationCorrection){const{x:w,y:S,z:R}=t.orientationCorrection;if(w||S||R){const L=Math.PI/180;y.scene.rotation.set(w*L,S*L,R*L),y.scene.updateMatrixWorld(!0)}}Ea(y.scene,{logPrefix:"HQThumbnail"});const I=!!t.perMaterialTextureBuffers&&t.perMaterialTextureBuffers.size>0,O=!!t.textureBuffers&&t.textureBuffers.size>0;if(I||O){const w=await to(y.scene,t.textureBuffers,t.materialFactors,t.perMaterialFactors,t.perMaterialTextureBuffers);f.push(...w)}else(t.materialFactors||t.perMaterialFactors)&&y.scene.traverse(w=>{if(!(w instanceof _e))return;const S=w.material;if(!S?.isMeshStandardMaterial)return;const R=Jt(S.name,t.perMaterialFactors)??t.materialFactors;R&&(Gt(S,R),S.needsUpdate=!0)});const X=new Ta().setFromObject(y.scene),ge=X.getCenter(new P),se=X.getSize(new P);y.scene.position.sub(ge);const K=Math.max(se.x,se.y,se.z),U=K*.8/Math.tan(x/2),G=Math.PI/6,ve=Math.PI/4;if(T.position.set(Math.cos(ve)*Math.cos(G)*U,Math.sin(G)*U,Math.sin(ve)*Math.cos(G)*U),T.lookAt(0,0,0),T.updateProjectionMatrix(),!h){c.add(new wa(16777215,.5));const w=new _a(16777215,1.5);w.position.set(5,10,7),c.add(w)}if(t.sunLight?.enabled){const w=t.sunLight.azimuth*Math.PI/180,S=t.sunLight.elevation*Math.PI/180,R=K*10,L=new P(R*Math.cos(S)*Math.sin(w),R*Math.sin(S),R*Math.cos(S)*Math.cos(w)),k=new Bi(new j(t.sunLight.color));k.position.copy(L),k.intensity=t.sunLight.intensity,k.radius=t.sunLight.radius,k.decay=0,k.angle=Math.PI/4,k.penumbra=1,k.target.position.set(0,0,0),c.add(k),c.add(k.target)}if(t.floor?.enabled){const w=K*t.floor.scale,S=new Sa(w,w),R=new Ia({color:new j(t.floor.color),roughness:.9,metalness:0,side:Ht}),L=new _e(S,R);L.rotation.x=-Math.PI/2;const k=X.min.y-ge.y;L.position.y=k+t.floor.yOffset,c.add(L),g.push(S),n.push(R)}if(await i.compileAsync(c,T),i.render(c,T),await new Promise(w=>requestAnimationFrame(w)),o?.aborted)throw new DOMException(`HQ render aborted: ${String(o.reason??"")}`,"AbortError");l.bounces=t.bounces,l.filterGlossyFactor=.3,l.renderDelay=0,l.tiles.set(2,2);const xe=l._pathTracer,Ae=new fe(1,1,{type:M}),be=i.getRenderTarget();i.setRenderTarget(Ae);try{l.setScene(c,T),l.updateEnvironment(),xe?.material?.onBeforeRender?.()}finally{i.setRenderTarget(be),Ae.dispose()}xe?._compilePromise&&(D.info("Waiting for shader compilation..."),await xe._compilePromise);const Fe=5e3,Ce=performance.now();for(;l.isCompiling&&performance.now()-Ce<Fe;)await new Promise(w=>setTimeout(w,50));l.isCompiling&&D.warn("Shader still compiling after timeout — rendering may produce black");let Z=0;const aa=16;for(;Z<t.samples;){if(await new Promise(S=>setTimeout(S,10)),o?.aborted)throw new DOMException(`HQ render aborted: ${String(o.reason??"")}`,"AbortError");if(l.isCompiling)continue;const w=Math.min(aa,t.samples-Z);for(let S=0;S<w;S++)l.renderSample();Z+=w}const rt=await new Promise(w=>s.toBlob(w,t.format,t.quality));if(!rt)throw new Error("HQ render: the canvas produced no image");const ra=await new Promise((w,S)=>{const R=new FileReader;R.onload=()=>w(R.result),R.onerror=()=>S(R.error),R.readAsDataURL(rt)}),it=performance.now()-a;return D.info(`HQ render complete: ${t.samples} samples in ${it.toFixed(0)}ms`),{dataUrl:ra,renderTimeMs:it}}finally{h&&h.dispose(),c.environment=null;for(const y of f)y.dispose();for(const y of g)y.dispose();for(const y of n)y.dispose();c.traverse(y=>{y instanceof _e&&(y.geometry?.dispose(),Array.isArray(y.material)?y.material.forEach(I=>I.dispose()):y.material?.dispose())});for(const y of m)y.tracks.length=0;io()}}const oo="thumbnail_hq.webp",so="thumbnail_hq.png";async function no(r){try{const e=await createImageBitmap(r),t=document.createElement("canvas");t.width=e.width,t.height=e.height;const a=t.getContext("2d");return a?(a.drawImage(e,0,0),e.close(),await new Promise(o=>{t.toBlob(s=>o(s),"image/png")})):(e.close(),null)}catch{return null}}async function lo(r,e,t){const a=await r.getRecord(e);if(!a){D.warn(`HQ thumbnail: no record for ${De(e)}`);return}const o=await r.getAssetBuffer(e);if(!o){D.warn(`HQ thumbnail: no GLB for ${De(e)}`);return}const s=new Map,i=new Map,l=a.textures?.type==="external",c=m=>m.startsWith(`${e}/`)?m.slice(`${e}/`.length):m;if(l){if(a.textures.files){for(const[m,p]of Object.entries(a.textures.files))if(p)try{const u=await r.getAssetFileBuffer(e,c(p));u&&s.set(m,u)}catch{}}if(a.textures.materialFiles)for(const[m,p]of Object.entries(a.textures.materialFiles)){if(!p)continue;const u=new Map;for(const[v,x]of Object.entries(p))if(x)try{const b=await r.getAssetFileBuffer(e,c(x));b&&u.set(v,b)}catch{}u.size>0&&i.set(m,u)}}const{dataUrl:d}=await ta(o,{textureBuffers:s.size>0?s:void 0,perMaterialTextureBuffers:i.size>0?i:void 0,orientationCorrection:a.metadata?.orientationCorrection,materialFactors:a.textures?.factors,perMaterialFactors:a.textures?.materialFactors}),h=er(d),f=e,{data:n,error:g}=await st(t,f,h,oo);if(g||!n?.path){console.warn(`[SAFEGUARD] HQ thumbnail R2 upload failed for ${De(e)} after retries:`,g);return}no(h).then(m=>{m&&st(t,f,m,so).catch(()=>{})}),await Pa.from("assets").update({thumbnail_hq_url:n.path}).eq("id",f),D.info(`HQ thumbnail stored for ${De(e)}: ${n.path}`)}const bo=Object.freeze(Object.defineProperty({__proto__:null,generateAndStoreHQThumbnail:lo,generateHQThumbnail:ta},Symbol.toStringTag,{value:"Module"}));export{Bi as P,Hi as W,Ni as a,Ir as b,xo as c,er as d,lo as e,go as f,ta as g,bo as h,po as p,vo as s};
