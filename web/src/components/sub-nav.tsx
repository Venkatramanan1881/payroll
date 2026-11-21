"use client";
import Link from "next/link";
import { usePathname } from "next/navigation";

/** A section's sub-navigation; the current page is marked for screen readers (aria-current). */
export function SubNav({ label, links }: { label: string; links: { href: string; text: string }[] }) {
  const path = usePathname();
  return (
    <nav aria-label={label} className="flex flex-wrap gap-4 border-b pb-2 text-sm">
      {links.map((l) => (
        <Link key={l.href} href={l.href} aria-current={path === l.href ? "page" : undefined} className="hover:underline aria-[current=page]:font-semibold">
          {l.text}
        </Link>
      ))}
    </nav>
  );
}
