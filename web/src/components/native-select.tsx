import { cn } from "@/lib/utils";

// A plain <select> styled like the Input: works with FormData and keyboards, and allows an empty "none" choice.
export function NativeSelect({ className, ...props }: React.ComponentProps<"select">) {
  return (
    <select
      className={cn(
        "h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-xs outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 aria-invalid:border-destructive disabled:opacity-50",
        className,
      )}
      {...props}
    />
  );
}
