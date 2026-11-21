"use client";
import { useTranslations } from "next-intl";
import { useParams } from "next/navigation";
import { useState } from "react";
import { PageProblem } from "@/components/page-problem";
import { ProblemAlert } from "@/components/problem-alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Textarea } from "@/components/ui/textarea";
import { bff, sendJson } from "@/lib/bff";
import { can } from "@/lib/menu";
import type { Problem } from "@/lib/problem";
import { wallDateTime } from "@/lib/time";
import { useBff } from "@/lib/use-bff";
import type { Branch, FenceTest, GeoFence, IpAllowList, Me } from "@/types/api";

/** A branch: details (ORG-01), time-zone history, geo-fences (ORG-02) and the allowed web IP list (ORG-03). */
export default function BranchPage() {
  const { id } = useParams<{ id: string }>();
  const t = useTranslations("org");
  const c = useTranslations("common");
  const branch = useBff<Branch>(`/branches/${id}`);
  const me = useBff<Me>("/me");
  const g = me.data?.grants ?? [];
  if (branch.problem && !branch.data) return <PageProblem problem={branch.problem} />;
  if (!branch.data) return <Skeleton className="h-24" aria-busy="true" aria-label={c("loading")} />;
  const b = branch.data;
  return (
    <div className="grid max-w-4xl gap-6">
      <h2 className="text-lg font-semibold">{b.name} <span className="text-muted-foreground">({b.code})</span></h2>
      {can(g, "ORG-01") ? <Details branch={b} etag={branch.etag} onSaved={branch.reload} /> : (
        <p>{b.address ?? ""}</p>
      )}
      <div className="grid gap-2">
        <h3 className="font-semibold">{t("tzHistory")}</h3>
        {b.time_zones?.length ? (
          <ul className="text-sm">
            {b.time_zones.map((z) => <li key={z.valid_from}>{z.tz}: {z.valid_from}{z.valid_to ? ` – ${z.valid_to}` : ""}</li>)}
          </ul>
        ) : <p className="text-sm text-muted-foreground">—</p>}
        <p className="text-sm text-muted-foreground">{t("tzReadOnly")}</p>
      </div>
      <Fences branchId={id} canEdit={can(g, "ORG-02")} />
      <IpList branchId={id} etag={branch.etag} canEdit={can(g, "ORG-03")} onSaved={branch.reload} />
    </div>
  );
}

function Details({ branch, etag, onSaved }: { branch: Branch; etag: string | null; onSaved: () => void }) {
  const t = useTranslations("org");
  const [problem, setProblem] = useState<Problem | null>(null);
  return (
    <form
      className="grid gap-3 rounded-lg border p-4 sm:grid-cols-3"
      onSubmit={async (e) => {
        e.preventDefault();
        const f = new FormData(e.currentTarget);
        const text = (k: string) => String(f.get(k) ?? "").trim();
        const r = await bff(`/branches/${branch.id}`, sendJson("PATCH", { name: text("name"), code: text("code"), address: text("address") || null }, { "if-match": etag ?? "" }));
        if (!r.ok) return setProblem(r.problem);
        setProblem(null);
        onSaved();
      }}
    >
      <h3 className="font-semibold sm:col-span-3">{t("details")}</h3>
      <div className="grid gap-1">
        <Label htmlFor="bd-name">{t("name")}</Label>
        <Input id="bd-name" name="name" required maxLength={120} defaultValue={branch.name} />
      </div>
      <div className="grid gap-1">
        <Label htmlFor="bd-code">{t("code")}</Label>
        <Input id="bd-code" name="code" required maxLength={20} defaultValue={branch.code} />
      </div>
      <div className="grid gap-1">
        <Label htmlFor="bd-address">{t("address")}</Label>
        <Input id="bd-address" name="address" maxLength={500} defaultValue={branch.address ?? ""} />
      </div>
      <Button type="submit" className="justify-self-start" disabled={!etag}>{t("save")}</Button>
      <div className="sm:col-span-3"><ProblemAlert problem={problem} /></div>
    </form>
  );
}

const label = (f: GeoFence) => f.name || `${f.latitude}, ${f.longitude}`;

/** The browser's position, or null when it is not shared. */
function position(): Promise<GeolocationPosition | null> {
  return new Promise((resolve) => {
    if (!navigator.geolocation) return resolve(null);
    navigator.geolocation.getCurrentPosition(resolve, () => resolve(null), { enableHighAccuracy: true, timeout: 15_000 });
  });
}

function Fences({ branchId, canEdit }: { branchId: string; canEdit: boolean }) {
  const t = useTranslations("org");
  const list = useBff<{ items: GeoFence[] }>(`/branches/${branchId}/geo-fences`);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [result, setResult] = useState<{ id: string; text: string } | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [key, setKey] = useState(() => crypto.randomUUID());

  async function send(path: string, init: RequestInit) {
    const r = await bff(path, init);
    if (!r.ok) {
      setProblem(r.problem);
      return null;
    }
    setProblem(null);
    list.reload();
    return r;
  }
  async function test(f: GeoFence) {
    setBusy(f.id);
    setResult({ id: f.id, text: t("testing") });
    const pos = await position();
    if (!pos) {
      setBusy(null);
      return setResult({ id: f.id, text: t("noPosition") });
    }
    const r = await bff<FenceTest>(`/geo-fences/${f.id}/test`, sendJson("POST", {
      latitude: pos.coords.latitude,
      longitude: pos.coords.longitude,
      accuracy_m: pos.coords.accuracy,
    }));
    setBusy(null);
    if (!r.ok) {
      setResult(null);
      return setProblem(r.problem);
    }
    const distance = Math.round(r.data.distance_m);
    setResult({ id: f.id, text: r.data.inside ? t("inside", { distance }) : t("outside", { distance, radius: r.data.radius_m ?? f.radius_m }) });
    list.reload();
  }
  const fields = (f?: GeoFence, prefix = "nf") => (
    <>
      <div className="grid gap-1">
        <Label htmlFor={`${prefix}-name`}>{t("fenceName")}</Label>
        <Input id={`${prefix}-name`} name="name" maxLength={120} defaultValue={f?.name ?? ""} />
      </div>
      <div className="grid gap-1">
        <Label htmlFor={`${prefix}-lat`}>{t("latitude")}</Label>
        <Input id={`${prefix}-lat`} name="latitude" type="number" step="0.000001" min={-90} max={90} required defaultValue={f?.latitude ?? ""} />
      </div>
      <div className="grid gap-1">
        <Label htmlFor={`${prefix}-lon`}>{t("longitude")}</Label>
        <Input id={`${prefix}-lon`} name="longitude" type="number" step="0.000001" min={-180} max={180} required defaultValue={f?.longitude ?? ""} />
      </div>
      <div className="grid gap-1">
        <Label htmlFor={`${prefix}-r`}>{t("radius")}</Label>
        <Input id={`${prefix}-r`} name="radius_m" type="number" min={1} step={1} required defaultValue={f?.radius_m ?? ""} />
      </div>
    </>
  );
  const values = (form: HTMLFormElement) => {
    const f = new FormData(form);
    const name = String(f.get("name") ?? "").trim();
    return { name, latitude: Number(f.get("latitude")), longitude: Number(f.get("longitude")), radius_m: Number(f.get("radius_m")) };
  };
  return (
    <div className="grid gap-3">
      <h3 className="font-semibold">{t("fences")}</h3>
      <ProblemAlert problem={list.problem ?? problem} />
      {list.data && list.data.items.length === 0 ? <p className="text-sm text-muted-foreground">{t("noFences")}</p> : null}
      {list.data?.items.map((f) => (
        <div key={f.id} className="grid gap-2 rounded-lg border p-3">
          <div className="flex flex-wrap items-center gap-3 text-sm">
            <span className="font-medium">{label(f)}</span>
            <Badge variant={f.enabled ? "default" : "secondary"}>{f.enabled ? t("on") : t("off")}</Badge>
            <span className="text-muted-foreground">{t("radius")}: {f.radius_m}</span>
            <span className="text-muted-foreground">{t("lastTest")}: {f.tested_at ? wallDateTime(f.tested_at) : t("never")}</span>
          </div>
          {result?.id === f.id ? <p role="status" className="text-sm">{result.text}</p> : null}
          {canEdit ? (
            <>
              <div className="flex flex-wrap gap-2">
                <Button size="sm" variant="outline" disabled={busy === f.id} aria-label={`${t("test")}: ${label(f)}`} onClick={() => test(f)}>{t("test")}</Button>
                <Button
                  size="sm"
                  variant="outline"
                  aria-label={`${f.enabled ? t("switchOff") : t("switchOn")}: ${label(f)}`}
                  onClick={() => send(`/geo-fences/${f.id}/${f.enabled ? "switch-off" : "switch-on"}`, { method: "POST", headers: { "idempotency-key": crypto.randomUUID() } })}
                >
                  {f.enabled ? t("switchOff") : t("switchOn")}
                </Button>
              </div>
              <form
                key={`${f.name}-${f.latitude}-${f.longitude}-${f.radius_m}`}
                className="grid gap-2 sm:grid-cols-5"
                aria-label={`${t("change")}: ${label(f)}`}
                onSubmit={(e) => {
                  e.preventDefault();
                  const v = values(e.currentTarget);
                  send(`/geo-fences/${f.id}`, sendJson("PATCH", { ...v, name: v.name || null }, { "if-match": `W/"${f.updated_at}"` }));
                }}
              >
                {fields(f, `ef-${f.id}`)}
                <Button type="submit" size="sm" variant="outline" className="self-end justify-self-start">{t("change")}</Button>
              </form>
            </>
          ) : null}
        </div>
      ))}
      {canEdit ? (
        <form
          className="grid gap-2 rounded-lg border p-3 sm:grid-cols-5"
          aria-label={t("addFence")}
          onSubmit={async (e) => {
            e.preventDefault();
            const form = e.currentTarget;
            const v = values(form);
            const r = await send(`/branches/${branchId}/geo-fences`, sendJson("POST", { ...v, name: v.name || undefined }, { "idempotency-key": key }));
            if (!r) return;
            setKey(crypto.randomUUID());
            form.reset();
          }}
        >
          {fields()}
          <Button type="submit" size="sm" className="self-end justify-self-start">{t("addFence")}</Button>
        </form>
      ) : null}
    </div>
  );
}

// The list's ETag is the branch's (organisation.yaml): it changes whenever the branch details are saved, too.
function IpList({ branchId, etag, canEdit, onSaved }: { branchId: string; etag: string | null; canEdit: boolean; onSaved: () => void }) {
  const t = useTranslations("org");
  const ip = useBff<IpAllowList>(`/branches/${branchId}/ip-allow-list`);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [saved, setSaved] = useState(false);
  const [key, setKey] = useState(() => crypto.randomUUID());
  if (!ip.data) return <ProblemAlert problem={ip.problem} />;
  const networks = ip.data.networks;
  return (
    <div className="grid gap-2">
      <h3 className="font-semibold">{t("ip")}</h3>
      <p className="text-sm text-muted-foreground">{t("ipHelp")}</p>
      {!canEdit ? (
        networks.length ? <ul className="text-sm">{networks.map((n) => <li key={n}>{n}</li>)}</ul> : <p className="text-sm text-muted-foreground">{t("noNetworks")}</p>
      ) : (
        <form
          key={networks.join(" ")}
          className="grid gap-3 rounded-lg border p-4"
          onSubmit={async (e) => {
            e.preventDefault();
            const f = new FormData(e.currentTarget);
            const list = String(f.get("networks") ?? "").split(/\s+/).map((x) => x.trim()).filter(Boolean);
            setSaved(false);
            const r = await bff(`/branches/${branchId}/ip-allow-list/change`, sendJson("POST", {
              networks: [...new Set(list)],
              reason: String(f.get("reason") ?? "").trim(),
            }, { "idempotency-key": key, "if-match": etag ?? "" }));
            if (!r.ok) return setProblem(r.problem);
            setProblem(null);
            setSaved(true);
            setKey(crypto.randomUUID());
            ip.reload();
            onSaved(); // a new branch ETag
          }}
        >
          <div className="grid gap-1">
            <Label htmlFor="ip-networks">{t("networks")}</Label>
            <Textarea id="ip-networks" name="networks" rows={4} defaultValue={networks.join("\n")} />
          </div>
          <div className="grid gap-1">
            <Label htmlFor="ip-reason">{t("reason")}</Label>
            <Input id="ip-reason" name="reason" required maxLength={1000} />
          </div>
          <Button type="submit" className="justify-self-start" disabled={!etag}>{t("saveIp")}</Button>
          <ProblemAlert problem={problem} />
          {saved ? <p role="status" className="text-sm text-green-700 dark:text-green-400">{t("saved")}</p> : null}
        </form>
      )}
    </div>
  );
}
