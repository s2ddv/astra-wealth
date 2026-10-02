import { AccountsView } from "@/components/dashboard/AccountsView";
export default async function AccountPage({
  params,
}: {
  params: Promise<{ accountId: string }>;
}) {
  const { accountId } = await params;
  return <AccountsView selectedId={accountId} />;
}
