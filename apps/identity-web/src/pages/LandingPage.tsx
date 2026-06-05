import { Link } from "@tanstack/react-router";
import { ArrowRight, Globe2, LockKeyhole } from "lucide-react";
import { Button } from "@/components/ui/button";
import { capabilities, workflow } from "./LandingPage.content";
import { ProductMockup } from "./LandingPage.product";
import { CheckList } from "./LandingPage.ui";

export default function LandingPage() {
	return (
		<main className="min-h-screen bg-white text-zinc-950">
			<header className="mx-auto flex max-w-7xl items-center justify-between px-6 py-6 lg:px-10">
				<Link className="text-3xl font-semibold tracking-normal" to="/">
					nvbes
				</Link>
				<nav className="hidden items-center gap-12 text-sm text-zinc-700 md:flex">
					{["Platform", "Security", "Pricing", "Docs"].map((item) => (
						<a href={`#${item.toLowerCase()}`} key={item}>
							{item}
						</a>
					))}
				</nav>
				<Button
					asChild
					className="border-zinc-300 bg-white text-zinc-950 hover:bg-zinc-50"
					variant="outline"
				>
					<Link to="/login">Sign in</Link>
				</Button>
			</header>

			<section className="mx-auto grid max-w-[88rem] gap-12 px-6 pb-12 pt-14 min-[1320px]:grid-cols-[0.65fr_1fr] lg:px-10">
				<div className="self-center">
					<h1 className="max-w-xl text-5xl font-semibold leading-[1.08] tracking-normal text-zinc-950 md:text-6xl">
						Identity and files for teams that ship securely
					</h1>
					<p className="mt-7 max-w-md text-lg leading-8 text-zinc-600">
						nvbes gives growing teams one control plane for access, audit
						trails, billing, and private workspace storage.
					</p>
					<div className="mt-10 flex flex-wrap gap-4">
						<Button
							asChild
							className="h-12 bg-teal-700 px-8 text-base hover:bg-teal-800"
						>
							<Link to="/register">Start workspace</Link>
						</Button>
						<Button
							asChild
							className="h-12 border-teal-700 px-8 text-base text-teal-800"
							variant="outline"
						>
							<a href="#docs">View docs</a>
						</Button>
					</div>
				</div>
				<ProductMockup />
			</section>

			<section
				className="mx-auto max-w-6xl border-y border-zinc-200 py-3"
				id="platform"
			>
				{capabilities.map(({ title, body, icon: IconComponent, points }) => (
					<div
						className="grid gap-8 border-b border-zinc-200 px-6 py-8 last:border-b-0 md:grid-cols-[130px_1fr_1fr] md:items-center"
						key={title}
					>
						<div className="flex size-20 items-center justify-center rounded-md bg-teal-50 text-teal-800">
							<IconComponent className="size-10" />
						</div>
						<div>
							<h2 className="text-2xl font-semibold tracking-normal">
								{title}
							</h2>
							<p className="mt-2 max-w-md leading-7 text-zinc-600">{body}</p>
						</div>
						<CheckList items={points} />
					</div>
				))}
			</section>

			<section className="border-b border-zinc-200 bg-zinc-50 py-16">
				<div className="mx-auto grid max-w-7xl gap-12 px-6 lg:grid-cols-[280px_1fr] lg:px-10">
					<div>
						<h2 className="text-3xl font-semibold tracking-normal">
							From access to audit, all in one flow
						</h2>
						<p className="mt-4 leading-7 text-zinc-600">
							A single control plane for your team&apos;s most sensitive
							workflows.
						</p>
						<a
							className="mt-6 inline-flex items-center gap-2 text-sm font-medium text-teal-800"
							href="#docs"
						>
							View docs <ArrowRight className="size-4" />
						</a>
					</div>
					<div className="grid gap-6 md:grid-cols-5">
						{workflow.map(({ title, body, icon: IconComponent }, index) => (
							<div className="relative" key={title}>
								{index < workflow.length - 1 ? (
									<div className="absolute left-14 top-9 hidden h-px w-full border-t border-dashed border-zinc-400 md:block" />
								) : null}
								<div className="relative flex size-18 items-center justify-center rounded-full bg-teal-100 text-teal-900">
									<IconComponent className="size-8" />
								</div>
								<h3 className="mt-5 font-semibold">
									{index + 1}. {title}
								</h3>
								<p className="mt-2 text-sm leading-6 text-zinc-600">{body}</p>
							</div>
						))}
					</div>
				</div>
			</section>

			<section
				className="mx-auto grid max-w-7xl gap-12 px-6 py-16 lg:grid-cols-[0.8fr_1fr_1fr] lg:px-10"
				id="security"
			>
				<div>
					<h2 className="text-3xl font-semibold tracking-normal">
						Security by design. Compliance by default.
					</h2>
					<p className="mt-5 leading-7 text-zinc-600">
						nvbes is built on a security-first architecture with encryption at
						every layer and privacy controls you can rely on.
					</p>
					<a
						className="mt-7 inline-flex items-center gap-2 text-sm font-medium text-teal-800"
						href="#security"
					>
						View security overview <ArrowRight className="size-4" />
					</a>
				</div>
				<div className="border-l border-zinc-200 pl-8">
					<LockKeyhole className="size-6 text-zinc-950" />
					<h3 className="mt-5 font-semibold">Audit you can trust</h3>
					<p className="mt-3 text-sm leading-6 text-zinc-600">
						Every action is recorded, immutable, and exportable.
					</p>
					<CheckList
						items={[
							"Real-time audit stream",
							"Tamper-evident logs",
							"Export to SIEM via API",
							"Retention policies & holds",
						]}
					/>
				</div>
				<div className="border-l border-zinc-200 pl-8">
					<Globe2 className="size-6 text-zinc-950" />
					<h3 className="mt-5 font-semibold">Data residency & compliance</h3>
					<p className="mt-3 text-sm leading-6 text-zinc-600">
						Your data stays in the region you choose with industry-leading
						compliance.
					</p>
					<CheckList
						items={[
							"Region selection (US, EU, APAC)",
							"SOC 2 Type II compliant",
							"GDPR & CCPA aligned",
							"DPA available",
						]}
					/>
				</div>
			</section>

			<section className="mx-auto max-w-7xl px-6 pb-14 lg:px-10" id="pricing">
				<div className="grid gap-8 rounded-lg bg-teal-800 px-8 py-9 text-white md:grid-cols-[1.4fr_0.6fr_1fr_0.8fr] md:items-center">
					<div>
						<h2 className="text-3xl font-semibold tracking-normal">
							Simple pricing that scales with your team
						</h2>
						<p className="mt-3 text-sm text-teal-50">
							Start free, upgrade when you&apos;re ready.
						</p>
					</div>
					<div>
						<div className="text-5xl font-medium">$20</div>
						<div className="mt-2 text-sm text-teal-50">
							Per user / month
							<br />
							Billed monthly
						</div>
					</div>
					<CheckList
						tone="dark"
						items={[
							"All platform features",
							"10 GB storage per user",
							"SSO, MFA, and audit logs",
							"Community support",
						]}
					/>
					<div className="space-y-4">
						<Button
							asChild
							className="h-12 w-full bg-white text-zinc-950 hover:bg-zinc-100"
						>
							<Link to="/register">Start workspace</Link>
						</Button>
						<a
							className="flex items-center justify-center gap-2 text-sm font-medium text-white"
							href="mailto:sales@nvbes.com"
						>
							Contact sales <ArrowRight className="size-4" />
						</a>
					</div>
				</div>
			</section>

			<footer
				className="border-t border-zinc-200 px-6 py-9 text-sm text-zinc-600 lg:px-10"
				id="docs"
			>
				<div className="mx-auto grid max-w-7xl gap-10 md:grid-cols-[1.2fr_1fr_1fr_1fr_1fr]">
					<div>
						<div className="text-2xl font-semibold text-zinc-950">nvbes</div>
						<p className="mt-3 max-w-40 leading-6">
							Identity and files for teams that ship securely.
						</p>
					</div>
					{["Platform", "Security", "Docs", "Company"].map((group) => (
						<div key={group}>
							<h3 className="font-semibold text-zinc-950">{group}</h3>
							<div className="mt-3 space-y-2">
								{["Overview", "Guides", "API reference", "Status"].map(
									(item) => (
										<a
											className="block"
											href="#platform"
											key={`${group}-${item}`}
										>
											{item}
										</a>
									),
								)}
							</div>
						</div>
					))}
				</div>
			</footer>
		</main>
	);
}
