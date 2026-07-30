import { Link } from "@tanstack/react-router";

export function AuthFooterLink({
	prompt,
	to,
	label,
}: {
	prompt?: string;
	to: string;
	label: string;
}) {
	return (
		<p className="text-center text-sm text-muted-foreground">
			{prompt ? `${prompt} ` : null}
			<Link to={to} className="font-medium text-primary hover:underline">
				{label}
			</Link>
		</p>
	);
}
