import { createRequestHeaders } from "@nvbes/http-client";
import { useLocation, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import type {
	VerifyEmailApiError,
	VerifyEmailResultState,
} from "./VerifyEmailResultPage.shared";

function csrfTokenFromCookie() {
	const match =
		typeof document !== "undefined"
			? document.cookie.match(/(?:^|;\s*)csrf_token=([^;]*)/)
			: null;
	return match?.[1];
}

export function useVerifyEmailResultPage() {
	const navigate = useNavigate();
	const location = useLocation();
	const searchParams = new URLSearchParams(location.searchStr);
	const token = searchParams.get("token");
	const [result, setResult] = useState<VerifyEmailResultState>({
		kind: "verifying",
	});

	useEffect(() => {
		if (!token) {
			setResult({ kind: "token_invalid" });
			return;
		}

		let cancelled = false;

		const verify = async () => {
			const headers: Record<string, string> = {
				"Content-Type": "application/json",
			};
			const csrfToken = csrfTokenFromCookie();
			if (csrfToken) {
				headers["X-CSRF-Token"] = csrfToken;
			}

			try {
				const response = await fetch("/auth/verify-email", {
					method: "POST",
					headers: createRequestHeaders("POST", headers),
					body: JSON.stringify({ token }),
					credentials: "include",
				});

				if (cancelled) {
					return;
				}

				if (response.ok) {
					const data = await response.json();
					setResult({ kind: "success", email: data.user?.email ?? "" });
					return;
				}

				const body = (await response
					.json()
					.catch(() => ({}))) as VerifyEmailApiError;
				const code = body.error?.code ?? "";
				const message = body.error?.message ?? "";

				if (
					code === "verification_token_expired" ||
					message.toLowerCase().includes("expired")
				) {
					setResult({ kind: "token_expired" });
				} else if (
					code === "verification_token_not_found" ||
					message.toLowerCase().includes("invalid")
				) {
					setResult({ kind: "token_invalid" });
				} else {
					setResult({
						kind: "error",
						message: message || "Échec de la vérification.",
					});
				}
			} catch (error) {
				console.error("Email verification failed:", error);
				if (!cancelled) {
					setResult({
						kind: "error",
						message: "Impossible de contacter le serveur.",
					});
				}
			}
		};

		void verify();

		return () => {
			cancelled = true;
		};
	}, [token]);

	return {
		navigate,
		result,
	};
}
