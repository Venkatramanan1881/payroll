import { notFound } from "next/navigation";
import { RequestDetail } from "@/components/request-detail";
import { isKind } from "@/lib/requests";

export default async function MyRequestPage({ params }: { params: Promise<{ kind: string; id: string }> }) {
  const { kind, id } = await params;
  if (!isKind(kind)) notFound();
  return <RequestDetail kind={kind} id={id} back="/me/requests" />;
}
