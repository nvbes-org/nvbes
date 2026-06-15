import { Label } from "../components/ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../components/ui/select";
import { Textarea } from "../components/ui/textarea";
import {
	type AccessReviewFilters,
	AccessReviewFilterControls,
} from "./AccessReviewsPage.filters";

const ChangeTargetRoles = ["owner", "admin", "member", "viewer"] as const;
export type ChangeTargetRole = (typeof ChangeTargetRoles)[number];

export function AccessReviewDecisionControls({
	reviewNote,
	targetRole,
	filters,
	onReviewNoteChange,
	onTargetRoleChange,
	onFiltersChange,
}: {
	reviewNote: string;
	targetRole: ChangeTargetRole;
	filters: AccessReviewFilters;
	onReviewNoteChange: (note: string) => void;
	onTargetRoleChange: (role: ChangeTargetRole) => void;
	onFiltersChange: (filters: AccessReviewFilters) => void;
}) {
	return (
		<>
			<div className="mb-4 grid gap-2">
				<Label htmlFor="access-review-note">Decision note</Label>
				<Textarea
					id="access-review-note"
					value={reviewNote}
					placeholder="Required for revoke or change decisions."
					onChange={(event) => onReviewNoteChange(event.target.value)}
				/>
			</div>
			<div className="mb-4 grid gap-2 sm:max-w-64">
				<Label htmlFor="access-review-target-role">Target role</Label>
				<Select
					value={targetRole}
					onValueChange={(value) =>
						onTargetRoleChange(value as ChangeTargetRole)
					}
				>
					<SelectTrigger id="access-review-target-role">
						<SelectValue />
					</SelectTrigger>
					<SelectContent>
						{ChangeTargetRoles.map((role) => (
							<SelectItem key={role} value={role}>
								{role}
							</SelectItem>
						))}
					</SelectContent>
				</Select>
			</div>
			<AccessReviewFilterControls filters={filters} onChange={onFiltersChange} />
		</>
	);
}
