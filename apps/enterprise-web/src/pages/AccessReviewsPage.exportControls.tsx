import { Download } from "lucide-react";
import { Button } from "../components/ui/button";

export function AccessReviewExportControls({
	disabled,
	onExport,
}: {
	disabled: boolean;
	onExport: (format: "csv" | "json") => void;
}) {
	return (
		<div className="flex gap-2">
			<Button
				type="button"
				variant="outline"
				size="sm"
				disabled={disabled}
				onClick={() => onExport("csv")}
			>
				<Download className="size-3.5" />
				CSV
			</Button>
			<Button
				type="button"
				variant="outline"
				size="sm"
				disabled={disabled}
				onClick={() => onExport("json")}
			>
				<Download className="size-3.5" />
				JSON
			</Button>
		</div>
	);
}
