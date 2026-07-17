import * as React from "react";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";

/**
 * Field: standard `Label + Control + Help/Error` wrapper with fixed rhythm
 * (label→control 8px, control→help 6px). EVERY labeled setting row should be a
 * Field — it removes gap inconsistency. Group several Fields in a `space-y-4`
 * container (field→field 16px).
 */
export function Field({
  label,
  htmlFor,
  hint,
  error,
  children,
  className,
  labelAddon,
}: {
  label?: React.ReactNode;
  htmlFor?: string;
  hint?: React.ReactNode;
  error?: React.ReactNode;
  children: React.ReactNode;
  className?: string;
  /** Element to the right of the label (e.g. an egress badge). */
  labelAddon?: React.ReactNode;
}) {
  return (
    <div className={cn(className)}>
      {label && (
        <div className="mb-2 flex items-center justify-between gap-2">
          <Label htmlFor={htmlFor}>{label}</Label>
          {labelAddon}
        </div>
      )}
      {children}
      {hint && (
        <p className="mt-1.5 text-sm text-muted-foreground leading-relaxed">
          {hint}
        </p>
      )}
      {error && <p className="mt-1.5 text-sm text-danger">{error}</p>}
    </div>
  );
}
