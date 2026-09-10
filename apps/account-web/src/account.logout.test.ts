import { expect, it, vi } from 'vite-plus/test';
import { submitLogout } from '@nvbes/identity-sdk-web/oauth';

it('submits a POST form then removes the ID Token from the document', () => {
  const action = 'https://identity.example/oauth/end-session';
  const submit = vi
    .spyOn(HTMLFormElement.prototype, 'submit')
    .mockImplementation(function (this: HTMLFormElement) {
      expect(this.method).toBe('post');
      expect(this.action).toBe(action);
      expect(new FormData(this).get('id_token_hint')).toBe('private.id.token');
    });
  try {
    submitLogout({ action, fields: { id_token_hint: 'private.id.token' } });
    expect(submit).toHaveBeenCalledTimes(1);
    expect(document.querySelector('input[name="id_token_hint"]')).toBeNull();
  } finally {
    submit.mockRestore();
  }
});
