# Boundary Checks

Architecture checks that enforce the Big Bang restructure rules.

## Checks

- `check-product-boundaries.mjs`: product libraries under `libs/*/products`
  must not depend on apps, Cloud/Internal code or adapters.

These checks are intentionally narrow and executable. They do not claim that
legacy app-domain extraction is complete; they prevent new product libraries
from depending on the wrong layer while the rebuild progresses.
