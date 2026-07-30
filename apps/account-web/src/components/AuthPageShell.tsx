import type { ReactNode } from "react";
import { Card, CardContent } from "@/components/ui/card";

export function AuthPageShell({
	brand,
	children,
	footerLinks,
}: {
	brand: ReactNode;
	children: ReactNode;
	footerLinks: ReactNode;
}) {
	return (
		<main className="flex min-h-dvh w-full items-center bg-background px-4 py-8 sm:px-8 lg:px-12">
			<div className="mx-auto w-full max-w-6xl animate-fade-slide-up [animation-delay:100ms]">
				<Card className="grid w-full gap-0 overflow-hidden rounded-4xl py-0 shadow-none lg:grid-cols-[minmax(0,0.78fr)_minmax(30rem,1.22fr)]">
					<section className="border-b border-border p-8 sm:p-10 lg:border-r lg:border-b-0 lg:p-12 xl:p-14">
						{brand}
					</section>
					<CardContent className="p-8 sm:p-10 lg:p-12 xl:p-14">
						{children}
					</CardContent>
				</Card>

				<footer className="mt-5 flex flex-col gap-3 px-4 text-xs text-muted-foreground sm:flex-row sm:items-start sm:justify-between">
					<p className="shrink-0">&copy; {new Date().getFullYear()} nvbes</p>
					<div className="[&>nav]:mt-0 sm:[&>nav]:justify-end">
						{footerLinks}
					</div>
				</footer>
			</div>
		</main>
	);
}
