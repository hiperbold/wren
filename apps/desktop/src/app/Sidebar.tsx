import { useTranslation } from "react-i18next";
import wrenMark from "@/assets/wren-mark.png";
import { NAV } from "./nav";
import { cn } from "@/lib/utils";

/** Vertical navigation (~200px): brand on top + one icon+label item per
 * section. The active item gets a surface + an accent bar on the left. */
export function Sidebar({
  active,
  onSelect,
}: {
  active: string;
  onSelect: (id: string) => void;
}) {
  const { t } = useTranslation(["nav", "common"]);
  return (
    <nav
      aria-label={t("common:sidebar.sections")}
      className="flex w-[200px] shrink-0 flex-col gap-1 border-r border-border bg-surface p-3"
    >
      <div className="mb-3 flex items-center gap-2.5 px-2 py-1">
        <img
          src={wrenMark}
          alt=""
          width={28}
          height={28}
          className="rounded-md"
        />
        <span className="text-lg font-semibold tracking-tight">Wren</span>
      </div>

      {NAV.map((item) => {
        const Icon = item.icon;
        const isActive = item.id === active;
        return (
          <button
            key={item.id}
            type="button"
            aria-current={isActive ? "page" : undefined}
            onClick={() => onSelect(item.id)}
            className={cn(
              "relative flex items-center gap-2.5 rounded-sm px-2.5 py-2 text-sm font-medium transition-colors cursor-pointer",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
              isActive
                ? "bg-surface-2 text-foreground"
                : "text-muted-foreground hover:bg-surface-2 hover:text-foreground",
            )}
          >
            {isActive && (
              <span className="absolute left-0 top-1.5 bottom-1.5 w-0.5 rounded-full bg-primary" />
            )}
            <Icon
              className={cn(
                "size-4 shrink-0",
                isActive ? "text-primary" : "text-subtle-foreground",
              )}
            />
            {t(`nav:${item.id}`)}
          </button>
        );
      })}
    </nav>
  );
}
