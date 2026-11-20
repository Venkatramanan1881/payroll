"use client";
import { useRouter } from "next/navigation";
import { Decide } from "@/components/decide";
import { Card, CardContent } from "@/components/ui/card";

/** Approve / reject on the request page. Whether the caller may decide is the API's answer, shown if refused. */
export function DecideHere({ id }: { id: string }) {
  const router = useRouter();
  return (
    <Card>
      <CardContent className="pt-6">
        <Decide base={`/attendance/regularisation-requests/${id}`} yes="approve" onDone={() => router.refresh()} />
      </CardContent>
    </Card>
  );
}
