'use client';
import { useState } from 'react';

export function CopyMint({ mint }: { mint: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button className="cx-mint" title="Copy the mint address" type="button" onClick={async () => { await navigator.clipboard.writeText(mint); setCopied(true); setTimeout(() => setCopied(false), 2000); }}>
      <span>{mint.slice(0, 7)}...{mint.slice(-6)}</span><em>{copied ? 'copied' : 'copy'}</em>
    </button>
  );
}
