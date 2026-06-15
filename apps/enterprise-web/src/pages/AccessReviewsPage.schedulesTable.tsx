import type { AccessReviewSchedule } from "@nvbes/identity-client";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "../components/ui/table";

export function AccessReviewSchedulesTable({
	mutatingScheduleId,
	schedules,
	onDisable,
	onEnable,
	onRunNow,
}: {
	mutatingScheduleId: string | null;
	schedules: AccessReviewSchedule[];
	onDisable: (scheduleId: string) => void;
	onEnable: (scheduleId: string) => void;
	onRunNow: (scheduleId: string) => void;
}) {
	if (schedules.length === 0) {
		return (
			<div className="flex min-h-44 items-center justify-center rounded-lg border border-dashed text-sm text-muted-foreground">
				No recurring access review schedules yet.
			</div>
		);
	}

	return (
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead>Name</TableHead>
					<TableHead>Cadence</TableHead>
					<TableHead>Next run</TableHead>
					<TableHead>Status</TableHead>
					<TableHead className="text-right">Actions</TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{schedules.map((schedule) => {
					const busy = mutatingScheduleId === schedule.id;
					return (
						<TableRow key={schedule.id}>
							<TableCell className="font-medium">{schedule.name}</TableCell>
							<TableCell>
								Every {schedule.recurrence_days}d, due +{schedule.due_after_days}d
							</TableCell>
							<TableCell>{formatDate(schedule.next_run_at)}</TableCell>
							<TableCell>
								<Badge variant={schedule.disabled_at ? "outline" : "default"}>
									{schedule.disabled_at ? "disabled" : "active"}
								</Badge>
							</TableCell>
							<TableCell>
								<div className="flex justify-end gap-2">
									<Button
										type="button"
										variant="outline"
										size="sm"
										disabled={busy || Boolean(schedule.disabled_at)}
										onClick={() => onRunNow(schedule.id)}
									>
										Run now
									</Button>
									{schedule.disabled_at ? (
										<Button
											type="button"
											variant="outline"
											size="sm"
											disabled={busy}
											onClick={() => onEnable(schedule.id)}
										>
											Enable
										</Button>
									) : (
										<Button
											type="button"
											variant="outline"
											size="sm"
											disabled={busy}
											onClick={() => onDisable(schedule.id)}
										>
											Disable
										</Button>
									)}
								</div>
							</TableCell>
						</TableRow>
					);
				})}
			</TableBody>
		</Table>
	);
}

function formatDate(value: string) {
	return new Intl.DateTimeFormat(undefined, {
		day: "2-digit",
		month: "short",
		year: "numeric",
	}).format(new Date(value));
}
