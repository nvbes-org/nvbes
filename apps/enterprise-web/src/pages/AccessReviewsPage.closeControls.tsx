import type {
	CloseAccessReviewCampaignInput,
	AccessReviewCampaignSummary,
} from "@nvbes/identity-client";
import { AlertTriangle, LockKeyhole } from "lucide-react";
import { useState } from "react";
import { Button } from "../components/ui/button";
import {
	Dialog,
	DialogClose,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
} from "../components/ui/dialog";
import { Textarea } from "../components/ui/textarea";

export function AccessReviewCloseControls({
	campaign,
	pending,
	onClose,
}: {
	campaign: AccessReviewCampaignSummary | null;
	pending: boolean;
	onClose: (campaignId: string, input: CloseAccessReviewCampaignInput) => void;
}) {
	const [open, setOpen] = useState(false);
	const [note, setNote] = useState("");
	const disabled = !campaign || campaign.status !== "active" || pending;
	const pendingItems = campaign?.pending_items ?? 0;
	const trimmedNote = note.trim();
	const noteRequired = pendingItems > 0;
	const noteTooLong = trimmedNote.length > 1000;
	const noteMissing = noteRequired && trimmedNote.length === 0;
	const closeDisabled = !campaign || pending || noteMissing || noteTooLong;
	const closeLabel =
		pendingItems > 0
			? `Close with ${pendingItems} pending`
			: "Close completed review";

	return (
		<Dialog
			open={open}
			onOpenChange={(nextOpen) => {
				setOpen(nextOpen);
				if (!nextOpen) {
					setNote("");
				}
			}}
		>
			<DialogTrigger asChild>
				<Button type="button" variant="outline" size="sm" disabled={disabled}>
					<LockKeyhole className="size-3.5" />
					Close campaign
				</Button>
			</DialogTrigger>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>Close access review</DialogTitle>
					<DialogDescription>
						Closing a campaign freezes the review snapshot and prevents further
						item decisions.
					</DialogDescription>
				</DialogHeader>
				{campaign ? (
					<div className="rounded-md border bg-muted/30 p-3 text-sm">
						<div className="font-medium">{campaign.name}</div>
						<div className="mt-1 text-muted-foreground">
							{pendingItems} pending, {campaign.approved_items} approved,{" "}
							{campaign.revoked_items} revoked, {campaign.changed_items} changed
						</div>
					</div>
				) : null}
				{pendingItems > 0 ? (
					<div className="flex gap-2 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
						<AlertTriangle className="mt-0.5 size-4 shrink-0" />
						<span>
							This campaign still has pending items. They will remain undecided
							in the exported evidence.
						</span>
					</div>
				) : null}
				<div className="grid gap-2">
					<label className="text-sm font-medium" htmlFor="access-review-close-note">
						Close note{noteRequired ? "" : " (optional)"}
					</label>
					<Textarea
						id="access-review-close-note"
						value={note}
						maxLength={1000}
						aria-invalid={noteMissing || noteTooLong}
						placeholder={
							noteRequired
								? "Explain why this review is being closed with pending items."
								: "Add context for the audit trail."
						}
						onChange={(event) => setNote(event.target.value)}
					/>
					<div className="flex justify-between gap-3 text-xs text-muted-foreground">
						<span>
							{noteMissing
								? "Required while pending items remain."
								: "Saved in the campaign close audit event."}
						</span>
						<span>{trimmedNote.length}/1000</span>
					</div>
				</div>
				<DialogFooter>
					<DialogClose asChild>
						<Button type="button" variant="outline">
							Cancel
						</Button>
					</DialogClose>
					<Button
						type="button"
						variant="destructive"
						disabled={closeDisabled}
						onClick={() => {
							if (!campaign) {
								return;
							}
							setOpen(false);
							onClose(campaign.id, {
								note: trimmedNote.length > 0 ? trimmedNote : undefined,
							});
						}}
					>
						<LockKeyhole className="size-3.5" />
						{closeLabel}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
