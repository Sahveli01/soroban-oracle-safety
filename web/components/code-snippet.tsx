"use client";

import { useState } from "react";

/**
 * Inline copy-to-clipboard code snippet for the hero CTA row.
 * Renders as a pill with a `$` prefix and a copy icon that flips to
 * "✓ copied" for 2 seconds on click.
 */
export function CodeSnippet({ code }: { code: string }) {
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Browsers without clipboard API permission silently no-op.
    }
  };

  return (
    <div className="code-block flex items-center gap-3 px-5 py-3">
      <span className="text-[var(--color-text-dim)]">$</span>
      <span className="text-[var(--color-text)]">{code}</span>
      <button
        onClick={copy}
        className="ml-2 text-[var(--color-text-dim)] transition-colors hover:text-[var(--color-text)]"
        aria-label="Copy command"
      >
        {copied ? (
          <span className="text-sm text-[var(--color-accent)]">✓ copied</span>
        ) : (
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <rect x="9" y="9" width="13" height="13" rx="2" />
            <path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1" />
          </svg>
        )}
      </button>
    </div>
  );
}
