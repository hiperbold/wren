import { lazy, type ComponentType, type LazyExoticComponent } from "react";
import {
  Activity,
  Boxes,
  Cloud,
  History,
  Keyboard,
  Mic,
  SlidersHorizontal,
  type LucideIcon,
} from "lucide-react";

/**
 * SINGLE REGISTRY of the 7 sections. Each view is lazy-loaded (React.lazy +
 * Suspense in App). To add/edit a view, touch ONLY the view file and, if
 * needed, this list — the view agents touch nothing else.
 *
 * Contract: each view is a DEFAULT export of a props-less component. `id`
 * doubles as the i18n key in the `nav` namespace for the sidebar label.
 */
export interface NavItem {
  id: string;
  icon: LucideIcon;
  Component: LazyExoticComponent<ComponentType>;
}

export const NAV: NavItem[] = [
  {
    id: "shortcuts",
    icon: Keyboard,
    Component: lazy(() => import("@/views/ShortcutsView")),
  },
  {
    id: "provider",
    icon: Cloud,
    Component: lazy(() => import("@/views/ProviderView")),
  },
  {
    id: "models",
    icon: Boxes,
    Component: lazy(() => import("@/views/ModelsView")),
  },
  {
    id: "audio",
    icon: Mic,
    Component: lazy(() => import("@/views/AudioView")),
  },
  {
    id: "system",
    icon: SlidersHorizontal,
    Component: lazy(() => import("@/views/SystemView")),
  },
  {
    id: "history",
    icon: History,
    Component: lazy(() => import("@/views/HistoryView")),
  },
  {
    id: "diagnostics",
    icon: Activity,
    Component: lazy(() => import("@/views/DiagnosticsView")),
  },
];
