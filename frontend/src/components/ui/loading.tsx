"use client";

import { cn } from "@/lib/utils";
import { Loader2 } from "lucide-react";
import type { HTMLAttributes } from "react";

export interface LoadingSpinnerProps extends HTMLAttributes<HTMLDivElement> {
  size?: "sm" | "md" | "lg";
}

const SIZES = {
  sm: "h-4 w-4",
  md: "h-6 w-6",
  lg: "h-8 w-8",
};

export function LoadingSpinner({
  size = "md",
  className,
  ...props
}: LoadingSpinnerProps) {
  return (
    <div
      className={cn("flex items-center justify-center", className)}
      {...props}
    >
      <Loader2 className={cn("animate-spin text-muted-foreground", SIZES[size])} />
    </div>
  );
}

export interface LoadingPageProps extends HTMLAttributes<HTMLDivElement> {
  message?: string;
}

export function LoadingPage({ message, className, ...props }: LoadingPageProps) {
  return (
    <div
      className={cn(
        "flex min-h-[50vh] flex-col items-center justify-center gap-4",
        className
      )}
      {...props}
    >
      <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      {message && <p className="text-sm text-muted-foreground">{message}</p>}
    </div>
  );
}
