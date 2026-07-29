export type ResetPasswordPageModel = {
  tokenFromLink: string;
  token: string;
  password: string;
  confirmPassword: string;
  error: string | null;
  success: boolean;
  isPending: boolean;
  navigateToLogin: () => void;
  setToken: (value: string) => void;
  setPassword: (value: string) => void;
  setConfirmPassword: (value: string) => void;
  handleSubmit: (event: React.SubmitEvent<HTMLFormElement>) => void;
};
