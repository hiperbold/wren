import * as React from "react";
import { cn } from "@/lib/utils";

/**
 * Header for each view: title (text-xl) + one muted description line. Sits at
 * the top of the content area, before the cards. `actions` goes on the right
 * (e.g. a "Refresh" button on data screens).
 */
export function SectionHeader({
  title,
  description,
  actions,
  className,
}: {
  title: React.ReactNode;
  description?: React.ReactNode;
  actions?: React.ReactNode;
  className?: string;
}) {
  return (
    <header
      className={cn(
        "mb-6 flex items-start justify-between gap-4 border-b border-border pb-4",
        className,
      )}
    >
      <div className="space-y-1">
        <h1 className="text-xl font-semibold tracking-tight">{title}</h1>
        {description && (
          <p className="text-sm text-muted-foreground leading-relaxed">
            {description}
          </p>
        )}
      </div>
      {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
    </header>
  );
}
