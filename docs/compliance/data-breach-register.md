# Registre des Violations de Donnees

## Statut

Template operationnel.

Chaque incident de securite impliquant potentiellement des donnees personnelles doit y etre consigne, y compris lorsqu'aucune notification externe n'est faite.

## Format

| Incident ID    | Date prise de connaissance | Systeme    | Categories de donnees | Volume estime | Risque              | Notification autorite    | Information personnes    | Statut          | Owner    |
| -------------- | -------------------------- | ---------- | --------------------- | ------------- | ------------------- | ------------------------ | ------------------------ | --------------- | -------- |
| `INC-YYYY-NNN` | `[DATE]`                   | `[SYSTEM]` | `[CATEGORIES]`        | `[ESTIMATE]`  | `[LOW_MEDIUM_HIGH]` | `[YES_NO_JUSTIFICATION]` | `[YES_NO_JUSTIFICATION]` | `[OPEN_CLOSED]` | `[NAME]` |

## Notes

- Conserver la justification lorsque la notification n'est pas faite.
- Lier chaque incident a son postmortem et aux actions correctives.
