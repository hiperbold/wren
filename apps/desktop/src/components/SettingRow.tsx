import * as React from "react";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";

/**
 * SettingRow: a `[label + help on the left] —— [control on the right]` row, for
 * toggles/switches inside a grouping Card. Use with <Switch/> as `control`. For
 * a stack of rows, wrap them in a Card and separate with <Separator/> or use
 * `divide-y`.
 */
export function SettingRow({
  label,
  hint,
  htmlFor,
  control,
  disabled,
  className,
}: {
  label: React.ReactNode;
  hint?: React.ReactNode;
  htmlFor?: string;
  control: React.ReactNode;
  disabled?: boolean;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex items-start justify-between gap-4 py-3",
        disabled && "opacity-50",
        className,
      )}
    >
      <div className="min-w-0 space-y-1">
        <Label htmlFor={htmlFor} className="block">
          {label}
        </Label>
        {hint && (
          <p className="text-sm text-muted-foreground leading-relaxed">
            {hint}
          </p>
        )}
      </div>
      <div className="shrink-0 pt-0.5">{control}</div>
    </div>
  );
}
