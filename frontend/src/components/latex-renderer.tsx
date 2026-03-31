"use client";

import katex from "katex";
import { useMemo } from "react";

interface LatexRendererProps {
  text: string;
  className?: string;
  block?: boolean;
}

/**
 * Renders text containing LaTeX delimiters ($...$ for inline, $$...$$ for display).
 * Non-LaTeX text is rendered as-is.
 */
export function LatexRenderer({ text, className, block }: LatexRendererProps) {
  const html = useMemo(() => renderLatex(text), [text]);

  if (block) {
    return (
      <div
        className={className}
        dangerouslySetInnerHTML={{ __html: html }}
      />
    );
  }

  return (
    <span
      className={className}
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}

/**
 * Renders a full block of LaTeX (no delimiter detection — entire string is LaTeX).
 */
export function LatexBlock({ text, className }: { text: string; className?: string }) {
  const html = useMemo(() => {
    try {
      return katex.renderToString(text, {
        displayMode: true,
        throwOnError: false,
        trust: true,
      });
    } catch {
      return escapeHtml(text);
    }
  }, [text]);

  return (
    <div
      className={className}
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}

const LATEX_PATTERN = /(\$\$[\s\S]+?\$\$|\$(?!\s)(?:[^$\\]|\\.)+?\$)/g;

function renderLatex(text: string): string {
  const parts: string[] = [];
  let lastIndex = 0;

  for (const match of text.matchAll(LATEX_PATTERN)) {
    const matchStart = match.index!;
    if (matchStart > lastIndex) {
      parts.push(escapeHtml(text.slice(lastIndex, matchStart)));
    }

    const raw = match[0];
    const isDisplay = raw.startsWith("$$");
    const inner = isDisplay ? raw.slice(2, -2) : raw.slice(1, -1);

    try {
      parts.push(
        katex.renderToString(inner.trim(), {
          displayMode: isDisplay,
          throwOnError: false,
          trust: true,
        })
      );
    } catch {
      parts.push(`<code>${escapeHtml(raw)}</code>`);
    }

    lastIndex = matchStart + raw.length;
  }

  if (lastIndex < text.length) {
    parts.push(escapeHtml(text.slice(lastIndex)));
  }

  return parts.join("");
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}
