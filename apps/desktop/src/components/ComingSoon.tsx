import { useTranslation } from "react-i18next";
import { Hammer } from "lucide-react";
import { Card } from "@/components/ui/card";

/** "Under construction" notice for views not yet redesigned. The view agents
 * replace the whole view body; this just keeps the app compiling and navigable
 * with the full sidebar. */
export function ComingSoon({ note }: { note?: string }) {
  const { t } = useTranslation("common");
  return (
    <Card className="border-dashed">
      <div className="flex items-center gap-3 p-6 text-sm text-muted-foreground">
        <Hammer className="size-5 shrink-0 text-subtle-foreground" />
        <span>{note ?? t("comingSoon.default")}</span>
      </div>
    </Card>
  );
}
