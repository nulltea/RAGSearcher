"use client";

import { Textarea } from "../ui/textarea";

export interface TextInputProps {
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
  error?: string;
}

export function TextInput({
  value,
  onChange,
  disabled,
  error,
}: TextInputProps) {
  return (
    <div>
      <Textarea
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder="Paste your research content here...

You can paste:
- Full research papers
- Blog posts
- Technical articles
- Notes and excerpts

The system will extract structured patterns (Claim/Evidence/Context) from your text."
        disabled={disabled}
        error={error}
        className="min-h-[300px] resize-y font-mono text-sm"
      />
      <p className="mt-2 text-xs text-muted-foreground">
        {value.length > 0
          ? `${value.length.toLocaleString()} characters`
          : "Paste or type your content above"}
      </p>
    </div>
  );
}
