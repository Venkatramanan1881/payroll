import { LeaveDetail } from "@/components/leave-detail";

export default async function MyLeaveRequestPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  return <LeaveDetail id={id} back="/me/leave" />;
}
