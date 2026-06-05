/**
 * Bot Guard — Honeypot field with autofill-safe camouflage
 *
 * Problème : Les password managers (1Password, Bitwarden, Dashlane, LastPass)
 * et l'autofill natif du navigateur ignorent `autocomplete="off"` et remplissent
 * les champs selon leur sémantique (name, type, id, placeholder).
 *
 * Stratégie de défense en profondeur :
 *
 * 1. `type="text"` — "url" est reconnu et rempli automatiquement par les PMs.
 * 2. Pas d'attribut `name` sur l'input DOM — les PMs matchent leur data store
 *    sur le `name`. Le name est injecté juste avant la lecture de la valeur.
 * 3. `id` aléatoire par session — les PMs ne peuvent pas cibler le champ par ID.
 * 4. `readonly` initial — les PMs sautent les champs readonly. Les bots, non.
 * 5. `autocomplete="new-password"` — valeur la plus respectée par les navigateurs
 *    pour désactiver le remplissage automatique (plus efficace que "off").
 * 6. Détection CSS de l'autofill natif (:-webkit-autofill) via animationstart —
 *    efface immédiatement si le navigateur remplit quand même.
 * 7. `setInterval` à 200ms — filet de sécurité final contre les PMs hors-spec.
 * 8. Interception des événements `paste` et `input` — efface toute tentative
 *    de remplissage programmatique qui contournerait readonly.
 * 9. Injection retardée de 80ms — la plupart des extensions PM scannent le DOM
 *    de façon synchrone au chargement. Un léger délai les fait manquer le champ.
 */

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** Durée de l'animation utilisée pour détecter l'autofill natif du navigateur. */
const AUTOFILL_ANIM_DURATION = '1ms';

// ---------------------------------------------------------------------------
// CSS injection dynamique — classe et animation avec noms aléatoires
// ---------------------------------------------------------------------------

function injectDecoyStyles(): { hiddenClass: string; autofillAnim: string } {
  const rand = () => `_${Math.random().toString(36).slice(2, 10)}`;
  const hiddenClass = rand();
  const autofillAnim = rand();

  const el = document.createElement('style');
  el.textContent = `
    .${hiddenClass} {
      position: absolute !important;
      width: 1px !important;
      height: 1px !important;
      padding: 0 !important;
      margin: -1px !important;
      overflow: hidden !important;
      clip: rect(0, 0, 0, 0) !important;
      white-space: nowrap !important;
      border: 0 !important;
      pointer-events: none !important;
    }

    @keyframes ${autofillAnim} {
      from {}
      to {}
    }

    /* Déclenche l'animation dès que le navigateur applique son autofill.
       L'écouteur animationstart efface alors la valeur immédiatement. */
    .${hiddenClass} input:-webkit-autofill,
    .${hiddenClass} input:autofill {
      animation-name: ${autofillAnim} !important;
      animation-duration: ${AUTOFILL_ANIM_DURATION} !important;
    }
  `;
  document.head.appendChild(el);

  return { hiddenClass, autofillAnim };
}

// ---------------------------------------------------------------------------
// Création du champ honeypot
// ---------------------------------------------------------------------------

export interface DecoyField {
  /** Wrapper DOM à insérer dans le formulaire (entre deux vrais champs). */
  wrapper: HTMLElement;

  /**
   * Lit la valeur actuelle du champ.
   * Renvoie "" pour un utilisateur légitime.
   * Renvoie une valeur non-vide si un bot a rempli le champ.
   */
  getValue: () => string;

  /** Libère les ressources (interval, observer, styles). */
  destroy: () => void;
}

export function createDecoyField(): DecoyField {
  const { hiddenClass, autofillAnim } = injectDecoyStyles();

  // --- Wrapper ---
  const wrapper = document.createElement('div');
  wrapper.className = hiddenClass;
  wrapper.setAttribute('aria-hidden', 'true');
  wrapper.setAttribute('tabindex', '-1');

  // --- Label (rend le champ indiscernable dans le HTML source) ---
  // ID aléatoire pour éviter le ciblage par les extensions PM.
  const randomId = `f${Math.random().toString(36).slice(2, 10)}`;
  const label = document.createElement('label');
  label.setAttribute('for', randomId);
  label.textContent = 'Website'; // label neutre, jamais visible

  // --- Input ---
  const input = document.createElement('input');
  input.id = randomId;

  // type="text" et non "url" — les PMs auto-fill "url" / "website" de façon
  // agressive. "text" sans contexte sémantique est ignoré plus souvent.
  input.type = 'text';

  // Pas de `name` ici. Les PMs utilisent name + type + label pour identifier
  // les champs à remplir. Sans name, le champ est invisible à leur heuristique.
  // Le name est injecté uniquement au moment de lire la valeur (getValue).
  // Note: un input sans name n'est pas soumis dans un <form> natif, ce qui
  // convient : on lit la valeur manuellement via getValue().

  input.tabIndex = -1; // jamais atteignable via Tab
  input.setAttribute('aria-hidden', 'true'); // Ignoré par les lecteurs d'écran
  input.setAttribute('autocomplete', 'new-password'); // valeur la plus respectée
  // "new-password" désactive l'autofill dans Chrome, Firefox, Edge, Safari.
  // Plus efficace que "off" qui est ignoré par tous les navigateurs modernes.

  // readonly initial — les PMs (Bitwarden, 1Password, Dashlane, LastPass)
  // sautent les champs readonly. Les bots headless, eux, ne vérifient pas ce flag.
  input.setAttribute('readonly', 'true');

  wrapper.appendChild(label);
  wrapper.appendChild(input);

  // --- Détection de l'autofill natif via CSS animation ---
  // Si le navigateur remplit quand même le champ (Safari parfois), l'animation
  // CSS déclenche animationstart et on efface immédiatement la valeur.
  const onAutofillDetected = (e: Event) => {
    if ((e as AnimationEvent).animationName === autofillAnim) {
      input.value = '';
    }
  };
  input.addEventListener('animationstart', onAutofillDetected);

  // --- Interception des événements de remplissage ---
  // Efface toute valeur injectée via paste ou input (PMs non-conventionnels,
  // extensions de navigateur qui simulent des événements clavier).
  const onFill = (e: Event) => {
    e.preventDefault();
    input.value = '';
  };
  input.addEventListener('paste', onFill);
  input.addEventListener('input', onFill);

  // --- setInterval : filet de sécurité final ---
  // Gère les PMs qui écrivent directement dans la propriété `.value` sans
  // déclencher d'événements DOM (comportement de certains scripts d'extension).
  const interval = setInterval(() => {
    if (input.value !== '') input.value = '';
  }, 200);

  // --- Cleanup si le wrapper est retiré du DOM ---
  const observer = new MutationObserver(() => {
    if (!document.contains(wrapper)) {
      destroy();
      observer.disconnect();
    }
  });
  observer.observe(document.body, { childList: true, subtree: true });

  const destroy = () => {
    clearInterval(interval);
    input.removeEventListener('animationstart', onAutofillDetected);
    input.removeEventListener('paste', onFill);
    input.removeEventListener('input', onFill);
  };

  return {
    wrapper,
    getValue: () => input.value,
    destroy,
  };
}

// ---------------------------------------------------------------------------
// Insertion retardée dans le formulaire
//
// IMPORTANT : Ne pas insérer le wrapper synchroniquement au chargement de la
// page. Les extensions PM scannent le DOM à DOMContentLoaded. Un délai de
// 80ms suffit pour que la plupart manquent le champ lors de leur scan initial.
// ---------------------------------------------------------------------------

/**
 * Insère le champ honeypot dans `container` après un délai de 80ms.
 * À appeler une seule fois après le montage du formulaire.
 *
 * Retourne une promesse résolue avec le DecoyField une fois inséré.
 */
export function mountDecoyField(container: HTMLElement): Promise<DecoyField> {
  return new Promise((resolve) => {
    setTimeout(() => {
      const field = createDecoyField();
      container.appendChild(field.wrapper);
      resolve(field);
    }, 80);
  });
}

// ---------------------------------------------------------------------------
// Honeypot links — invisible links that only bots click
// ---------------------------------------------------------------------------

export interface DecoyLinkTracker {
  wasClicked: () => boolean;
  destroy: () => void;
}

export function createDecoyLinks(container: HTMLElement, count: number): DecoyLinkTracker {
  let clicked = false;
  const destroyFns: (() => void)[] = [];

  for (let i = 0; i < count; i++) {
    const link = document.createElement('a');
    link.href = '#';
    link.textContent = '\u00A0';
    link.setAttribute('tabindex', '-1');
    link.setAttribute('aria-hidden', 'true');
    link.style.cssText =
      'position:absolute;width:1px;height:1px;overflow:hidden;opacity:0;pointer-events:none;';
    link.style.clip = 'rect(0,0,0,0)';

    const handler = (e: Event) => {
      e.preventDefault();
      clicked = true;
    };
    link.addEventListener('click', handler);
    link.addEventListener('auxclick', handler);
    destroyFns.push(() => {
      link.removeEventListener('click', handler);
      link.removeEventListener('auxclick', handler);
    });

    container.appendChild(link);
  }

  return {
    wasClicked: () => clicked,
    destroy: () => destroyFns.forEach((fn) => fn()),
  };
}

// ---------------------------------------------------------------------------
// Usage React (exemple)
// ---------------------------------------------------------------------------

/*
  function LoginForm() {
    const decoyRef = useRef<DecoyField | null>(null);
    const decoyContainerRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
      if (!decoyContainerRef.current) return;
      mountDecoyField(decoyContainerRef.current).then((field) => {
        decoyRef.current = field;
      });
      return () => decoyRef.current?.destroy();
    }, []);

    const handleSubmit = async (e: FormEvent) => {
      e.preventDefault();
      const website = decoyRef.current?.getValue() ?? ""; // "" pour un humain

      await api.post("/challenge/identifier", {
        email,
        bot_guard: {
          website,
          form_ts: Date.now(),
          js_sig: await computeBotGuardSig(Date.now(), "login_identifier"),
        },
      });
    };

    return (
      <form onSubmit={handleSubmit}>
        <input type="email" name="email" autocomplete="email" />
        <div ref={decoyContainerRef} />  ← inséré ici, entre les vrais champs
        <button type="submit">Continuer</button>
      </form>
    );
  }
*/
