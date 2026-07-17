import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

/** Combines conditional classes and resolves Tailwind conflicts. shadcn standard. */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
