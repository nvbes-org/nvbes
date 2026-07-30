import { Link } from "@tanstack/react-router";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
	Card,
	CardAction,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { CookieConsentSettings } from "../components/CookieConsentSettings";
import { DEFAULT_CONSENT, getTrackingConsent } from "../tracking-consent";
import { AccountPrivacyConsentList } from "./AccountPrivacyConsentList";
import {
	GpcStatusAlert,
	PrivacyDeleteCard,
	PrivacyExportCard,
} from "./AccountPrivacyPage.cards";
import { DeleteAccountDialog } from "./AccountPrivacyPage.dialog";
import { PrivacyOverviewCard } from "./AccountPrivacyPage.overview";
import { createPrivacyOverview } from "./AccountPrivacyPage.presentation";
import type { useAccountPrivacyPage } from "./useAccountPrivacyPage";

export { deleteAccount, exportAccountData } from "./AccountPrivacyPage.api";
export {
	GpcStatusAlert,
	PrivacyDeleteCard,
	PrivacyExportCard,
	PrivacySkeleton,
} from "./AccountPrivacyPage.cards";
export {
	ConsentEmptyState,
	ConsentRow,
	consentLabels,
	isVisibleConsentType,
} from "./AccountPrivacyPage.consents";
export { DeleteAccountDialog } from "./AccountPrivacyPage.dialog";

type AccountPrivacyPageContentProps = ReturnType<
	typeof useAccountPrivacyPage
> & {
	onExport: () => void;
	onDelete: () => void;
};

export function AccountPrivacyPageContent({
	consents,
	gpc,
	cookieConsentRevision,
	exporting,
	exportSuccess,
	exportError,
	showDeleteDialog,
	deleting,
	deleteConfirmText,
	deleteError,
	handleRevoke,
	onExport,
	handleOpenDelete,
	onDelete,
	setShowDeleteDialog,
	setDeleteConfirmText,
	handleCookieConsentChange,
}: AccountPrivacyPageContentProps) {
	const overview = createPrivacyOverview({
		consents,
		trackingConsent: getTrackingConsent() ?? DEFAULT_CONSENT,
		gpcEnabled: Boolean(gpc?.gpc_enabled),
	});

	return (
		<div className="flex animate-fade-slide-up flex-col gap-8 [animation-delay:0ms]">
			<header className="flex max-w-3xl flex-col items-start gap-3">
				<div className="flex flex-col gap-2">
					<h1 className="font-heading text-2xl font-semibold tracking-tight">
						Vos données, vos choix.
					</h1>
					<p className="text-sm text-muted-foreground">
						Comprenez ce qui est utilisé, changez vos préférences et exercez vos
						droits depuis un seul endroit.
					</p>
				</div>
			</header>

			<PrivacyOverviewCard overview={overview} />

			<div className="flex flex-col gap-6">
				<Card>
					<CardHeader>
						<CardTitle>
							<h2>Cookies et diagnostics</h2>
						</CardTitle>
						<CardDescription>
							Choisissez précisément ce qui peut être utilisé au-delà du
							fonctionnement indispensable du service.
						</CardDescription>
						<CardAction>
							<Badge variant="secondary">
								{overview.optionalEnabledCount}/{overview.optionalTotal} actifs
							</Badge>
						</CardAction>
					</CardHeader>
					<CardContent>
						<CookieConsentSettings
							key={cookieConsentRevision}
							onConsentChange={handleCookieConsentChange}
						/>
					</CardContent>
					<CardFooter>
						<Button asChild variant="link">
							<Link to={"/legal/privacy-policy" as string}>
								Lire la politique de confidentialité
							</Link>
						</Button>
					</CardFooter>
				</Card>

				<GpcStatusAlert gpc={gpc} />
				<AccountPrivacyConsentList
					consents={consents}
					onRevoke={handleRevoke}
				/>

				<div className="flex flex-col gap-2">
					<h2 className="font-heading text-lg font-semibold">
						Exercer vos droits
					</h2>
					<p className="text-sm text-muted-foreground">
						Exportez vos informations ou lancez une suppression définitive.
					</p>
				</div>
				<PrivacyExportCard
					exporting={exporting}
					exportSuccess={exportSuccess}
					exportError={exportError}
					onExport={onExport}
				/>
				<PrivacyDeleteCard onOpenDelete={handleOpenDelete} />
			</div>

			<p className="text-xs text-muted-foreground">
				Vos choix peuvent être modifiés à tout moment. Leur retrait n’affecte
				pas la licéité des traitements déjà réalisés.
			</p>

			<DeleteAccountDialog
				open={showDeleteDialog}
				deleting={deleting}
				deleteConfirmText={deleteConfirmText}
				deleteError={deleteError}
				onOpenChange={setShowDeleteDialog}
				onDeleteConfirmTextChange={setDeleteConfirmText}
				onDelete={onDelete}
			/>
		</div>
	);
}
