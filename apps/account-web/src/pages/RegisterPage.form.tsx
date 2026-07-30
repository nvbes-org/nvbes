import { Link } from "@tanstack/react-router";
import { CheckIcon } from "lucide-react";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { AuthFooterLink } from "@/components/AuthFooterLink";
import { FeedbackAlert } from "@/components/FeedbackAlert";
import { PasswordStrengthMeter } from "@/components/PasswordStrengthMeter";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Field, FieldLabel } from "@/components/ui/field";
import { Spinner } from "@/components/ui/spinner";
import {
	MAX_PASSWORD_LENGTH,
	MIN_PASSWORD_LENGTH,
} from "../identity.password.policy";
import { MAX_USERNAME_LENGTH } from "../identity.username.policy";
import {
	registrationAvailabilityMessage,
	ValidatedInput,
} from "./RegisterPage.field";
import type { useRegisterPage } from "./useRegisterPage";

type RegisterFormProps = ReturnType<typeof useRegisterPage> & {
	loginTo?: string;
};

export function RegisterForm({
	canSubmit,
	email,
	emailAlreadyExists,
	emailAvailability,
	emailValid,
	error,
	handleEmailChange,
	handleSubmit,
	handleUsernameChange,
	legalDocumentsAccepted,
	loginTo = "/login",
	loading,
	marketingEmailsAccepted,
	password,
	passwordValid,
	setLegalDocumentsAccepted,
	setMarketingEmailsAccepted,
	setPassword,
	username,
	usernameTaken,
	usernameAvailability,
	usernameValid,
}: RegisterFormProps) {
	const emailInputRef = useRef<HTMLInputElement>(null);
	const usernameInputRef = useRef<HTMLInputElement>(null);
	const [emailTouched, setEmailTouched] = useState(false);
	const [passwordTouched, setPasswordTouched] = useState(false);
	const [usernameTouched, setUsernameTouched] = useState(false);

	const emailInvalid =
		emailAlreadyExists ||
		(emailTouched && (!emailValid || emailAvailability === "unavailable"));
	const emailCorrect =
		emailTouched &&
		emailValid &&
		!emailAlreadyExists &&
		emailAvailability === "available";
	const emailChecking =
		emailTouched && emailValid && emailAvailability === "checking";
	const usernameInvalid =
		usernameTaken ||
		(usernameTouched &&
			(!usernameValid || usernameAvailability === "unavailable"));
	const usernameCorrect =
		usernameTouched &&
		usernameValid &&
		!usernameTaken &&
		usernameAvailability === "available";
	const usernameChecking =
		usernameTouched && usernameValid && usernameAvailability === "checking";
	const passwordInvalid = passwordTouched && !passwordValid;
	const passwordCorrect = passwordTouched && passwordValid;

	useEffect(() => {
		if (emailAlreadyExists) {
			emailInputRef.current?.focus();
		} else if (usernameTaken) {
			usernameInputRef.current?.focus();
		}
	}, [emailAlreadyExists, usernameTaken]);

	return (
		<form
			onSubmit={handleSubmit}
			autoComplete="on"
			className="flex flex-col gap-4"
		>
			<Field>
				<FieldLabel htmlFor="register-email">
					Email <span className="text-destructive">*</span>
				</FieldLabel>
				<ValidatedInput
					id="register-email"
					name="email"
					inputRef={emailInputRef}
					type="email"
					placeholder="vous@exemple.fr"
					value={email}
					onChange={(event) => handleEmailChange(event.target.value)}
					onBlur={() => setEmailTouched(true)}
					required
					autoComplete="username"
					autoFocus
					className="h-11 rounded-xl px-3.5"
					correct={emailCorrect}
					invalid={emailInvalid}
					checking={emailChecking}
					message={
						emailAlreadyExists
							? "Cet email est déjà utilisé. Connectez-vous ou choisissez une autre adresse."
							: registrationAvailabilityMessage({
									availability: emailAvailability,
									availableMessage: "Adresse email valide.",
									unavailableMessage: "Cet email est déjà utilisé.",
									checkingMessage: "Vérification de l'email...",
									invalidMessage: "Saisissez une adresse email valide.",
									invalid: emailInvalid,
									touched: emailTouched,
								})
					}
				/>
			</Field>

			<Field>
				<FieldLabel htmlFor="register-username">
					Nom d&apos;utilisateur <span className="text-destructive">*</span>
				</FieldLabel>
				<ValidatedInput
					id="register-username"
					name="nickname"
					inputRef={usernameInputRef}
					type="text"
					placeholder="username"
					value={username}
					onChange={(event) => handleUsernameChange(event.target.value)}
					onBlur={() => setUsernameTouched(true)}
					required
					autoComplete="nickname"
					maxLength={MAX_USERNAME_LENGTH}
					spellCheck={false}
					className="h-11 rounded-xl px-3.5"
					correct={usernameCorrect}
					invalid={usernameInvalid}
					checking={usernameChecking}
					message={
						usernameTaken
							? "Ce nom d'utilisateur est déjà pris."
							: registrationAvailabilityMessage({
									availability: usernameAvailability,
									availableMessage: "Nom d'utilisateur valide.",
									unavailableMessage: "Ce nom d'utilisateur est déjà pris.",
									checkingMessage: "Vérification du nom d'utilisateur...",
									invalidMessage: "Saisissez un nom d'utilisateur.",
									invalid: usernameInvalid,
									touched: usernameTouched,
								})
					}
				/>
			</Field>

			<Field>
				<FieldLabel htmlFor="register-password">
					Mot de passe <span className="text-destructive">*</span>
				</FieldLabel>
				<ValidatedInput
					id="register-password"
					name="password"
					type="password"
					placeholder="••••••••"
					value={password}
					onChange={(event) => setPassword(event.target.value)}
					onBlur={() => setPasswordTouched(true)}
					required
					autoComplete="new-password"
					minLength={MIN_PASSWORD_LENGTH}
					maxLength={MAX_PASSWORD_LENGTH}
					spellCheck={false}
					className="h-11 rounded-xl px-3.5"
					correct={passwordCorrect}
					invalid={passwordInvalid}
					message={
						passwordCorrect
							? "Mot de passe valide."
							: passwordInvalid
								? password.length < MIN_PASSWORD_LENGTH
									? `Utilisez au moins ${MIN_PASSWORD_LENGTH} caractères.`
									: "Choisissez un mot de passe plus robuste."
								: `Utilisez au moins ${MIN_PASSWORD_LENGTH} caractères.`
					}
				/>
				<PasswordStrengthMeter password={password} />
			</Field>

			<div className="flex flex-col gap-3 border-t border-border pt-4">
				<ConsentCheckbox
					id="register-legal-documents"
					checked={legalDocumentsAccepted}
					onCheckedChange={setLegalDocumentsAccepted}
				>
					J&apos;accepte les{" "}
					<LegalLink to="/legal/terms-of-service">Conditions</LegalLink>, la{" "}
					<LegalLink to="/legal/privacy-policy">Confidentialité</LegalLink> et
					le <LegalLink to="/legal/data-processing-agreement">DPA</LegalLink>.{" "}
					<span className="text-destructive">*</span>
				</ConsentCheckbox>
				<ConsentCheckbox
					id="register-marketing-emails"
					checked={marketingEmailsAccepted}
					onCheckedChange={setMarketingEmailsAccepted}
					muted
				>
					Je souhaite recevoir les nouveautés et conseils nvbes par email.
				</ConsentCheckbox>
			</div>

			{error && !emailAlreadyExists && !usernameTaken && (
				<FeedbackAlert tone="error">{error}</FeedbackAlert>
			)}

			<div className="mt-1 flex flex-col-reverse gap-4 sm:flex-row sm:items-center sm:justify-between">
				<AuthFooterLink
					prompt="Déjà un compte ?"
					to={loginTo}
					label="Se connecter"
				/>
				<Button
					type="submit"
					className="h-11 rounded-full px-6"
					size="lg"
					disabled={loading || !canSubmit}
				>
					{loading ? (
						<>
							<Spinner data-icon="inline-start" />
							Inscription...
						</>
					) : (
						<>
							Créer mon compte
							<CheckIcon data-icon="inline-end" />
						</>
					)}
				</Button>
			</div>
		</form>
	);
}

function ConsentCheckbox({
	id,
	checked,
	onCheckedChange,
	muted = false,
	children,
}: {
	id: string;
	checked: boolean;
	onCheckedChange: (value: boolean) => void;
	muted?: boolean;
	children: ReactNode;
}) {
	return (
		<div
			className={`flex items-start gap-3 text-sm leading-5${muted ? " text-muted-foreground" : ""}`}
		>
			<Checkbox
				id={id}
				checked={checked}
				onCheckedChange={(value) => onCheckedChange(value === true)}
				className="mt-0.5"
			/>
			<label htmlFor={id}>{children}</label>
		</div>
	);
}

function LegalLink({ to, children }: { to: string; children: ReactNode }) {
	return (
		<Link to={to} className="font-medium text-primary hover:underline">
			{children}
		</Link>
	);
}
