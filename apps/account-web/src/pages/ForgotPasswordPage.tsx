import { useMutation } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { AuthFooterLink } from "@/components/AuthFooterLink";
import { FeedbackAlert } from "@/components/FeedbackAlert";
import { StandaloneAuthCard } from "@/components/StandaloneAuthCard";
import { Button } from "@/components/ui/button";
import { Field, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { forgotPassword } from "../identity.password.api";

export default function ForgotPasswordPage() {
	const navigate = useNavigate();
	const [email, setEmail] = useState("");
	const [error, setError] = useState<string | null>(null);
	const [success, setSuccess] = useState(false);

	const mutation = useMutation({
		mutationFn: (email: string) => forgotPassword(email),
		onSuccess: () => setSuccess(true),
		onError: () => setError("Une erreur est survenue. Veuillez reessayer."),
	});

	const handleSubmit = async (e: React.SubmitEvent<HTMLFormElement>) => {
		e.preventDefault();
		setError(null);
		mutation.mutate(email);
	};

	return (
		<StandaloneAuthCard
			title="Mot de passe oublie"
			description="Entrez votre adresse email pour recevoir un lien de reinitialisation."
			onBack={() => navigate({ to: "/login" })}
			footer={<AuthFooterLink to="/register" label="Creer un compte" />}
		>
			{success ? (
				<div className="flex flex-col gap-5">
					<FeedbackAlert>
						{`Si un compte existe avec l'adresse ${email}, vous recevrez un email avec les instructions pour reinitialiser votre mot de passe.`}
					</FeedbackAlert>
					<Button
						variant="outline"
						className="w-full"
						onClick={() => navigate({ to: "/login" })}
					>
						Retour a la connexion
					</Button>
				</div>
			) : (
				<form onSubmit={handleSubmit} className="flex flex-col gap-5">
					<Field>
						<FieldLabel htmlFor="forgot-email">Email</FieldLabel>
						<Input
							id="forgot-email"
							type="email"
							placeholder="vous@exemple.fr"
							value={email}
							onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
								setEmail(e.target.value)
							}
							required
							autoComplete="email"
							autoFocus
						/>
					</Field>
					{error ? <FeedbackAlert tone="error">{error}</FeedbackAlert> : null}
					<Button
						type="submit"
						disabled={mutation.isPending}
						className="w-full"
						size="lg"
					>
						{mutation.isPending ? "Envoi..." : "Envoyer le lien"}
					</Button>
				</form>
			)}
		</StandaloneAuthCard>
	);
}
