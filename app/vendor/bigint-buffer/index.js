'use strict';
// Changed by units: the pure JavaScript paths of bigint-buffer 1.1.5, without its native addon.

function toBigIntLE(buf) {
  const reversed = Buffer.from(buf);
  reversed.reverse();
  const hex = reversed.toString('hex');
  return hex.length === 0 ? BigInt(0) : BigInt(`0x${hex}`);
}

function toBigIntBE(buf) {
  const hex = Buffer.from(buf).toString('hex');
  return hex.length === 0 ? BigInt(0) : BigInt(`0x${hex}`);
}

function toBufferBE(num, width) {
  const hex = num.toString(16);
  return Buffer.from(hex.padStart(width * 2, '0').slice(0, width * 2), 'hex');
}

function toBufferLE(num, width) {
  const buffer = toBufferBE(num, width);
  buffer.reverse();
  return buffer;
}

module.exports = { toBigIntLE, toBigIntBE, toBufferLE, toBufferBE };
