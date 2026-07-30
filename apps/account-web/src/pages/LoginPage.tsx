import { LoginPageContent } from "./LoginPage.content";
import { useLoginPage } from "./useLoginPage";
import { useLoginPageTransition } from "./useLoginPage.transition";

export default function LoginPage() {
	const page = useLoginPage();
	const transitionDirection = useLoginPageTransition(page.step);

	return (
		<LoginPageContent {...page} transitionDirection={transitionDirection} />
	);
}
