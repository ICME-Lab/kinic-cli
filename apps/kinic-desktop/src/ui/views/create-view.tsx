// Where: Create memory workflow.
// What: shows account cost state and submits create requests.
// Why: desktop users need balance context before deploying a memory canister.

import { Button, Card, FieldLabel, Input, Metric, Textarea } from "../primitives";
import { valueOrUnavailable } from "@/desktop/format";
import { useDesktopStore } from "@/store/useDesktopStore";

export function CreateView() {
  const session = useDesktopStore((state) => state.session);
  const name = useDesktopStore((state) => state.createName);
  const description = useDesktopStore((state) => state.createDescription);
  const loading = useDesktopStore((state) => state.loading);
  const setName = useDesktopStore((state) => state.setCreateName);
  const setDescription = useDesktopStore((state) => state.setCreateDescription);
  const submitCreate = useDesktopStore((state) => state.submitCreate);
  const insufficient = session?.sufficient_balance === false;

  return (
    <div className="grid max-w-5xl gap-5 xl:grid-cols-[minmax(0,1fr)_360px]">
      <Card className="p-6">
        <h2 className="text-xl font-semibold">Create Memory</h2>
        <p className="mt-1 text-sm text-muted-foreground">Deploy a new memory canister with the active identity.</p>
        <div className="mt-6 grid gap-5">
          <div className="grid gap-2">
            <FieldLabel htmlFor="create-name">Name</FieldLabel>
            <Input id="create-name" value={name} onChange={(event) => setName(event.target.value)} />
          </div>
          <div className="grid gap-2">
            <FieldLabel htmlFor="create-description">Description</FieldLabel>
            <Textarea
              id="create-description"
              value={description}
              onChange={(event) => setDescription(event.target.value)}
            />
          </div>
          {insufficient ? (
            <p className="rounded-2xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">
              KINIC balance is below the required create cost.
            </p>
          ) : null}
          <Button disabled={loading || insufficient || !name.trim()} onClick={() => void submitCreate()}>
            {loading ? "Creating..." : "Create Memory"}
          </Button>
        </div>
      </Card>

      <Card className="p-5">
        <h3 className="text-lg font-semibold">Account</h3>
        <div className="mt-4 grid gap-3">
          <Metric label="Principal" value={valueOrUnavailable(session?.principal_id)} />
          <Metric label="Balance" value={valueOrUnavailable(session?.balance_kinic)} />
          <Metric label="Fee" value={valueOrUnavailable(session?.fee_base_units)} />
          <Metric label="Create Price" value={valueOrUnavailable(session?.price_base_units)} />
          <Metric label="Required Total" value={valueOrUnavailable(session?.required_total_kinic)} />
        </div>
      </Card>
    </div>
  );
}
