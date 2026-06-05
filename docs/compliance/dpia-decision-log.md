# Journal de Decision AIPD

## Objet

Tracer la decision de realiser ou non une AIPD pour chaque traitement ou evolution significative.

## Decision initiale V1

Perimetre evalue:

- stockage et partage de fichiers pour petites equipes;
- logs de securite et audit;
- liens publics controles;
- analytics produit limites;
- billing et TVA;
- absence d'OCR, IA, scoring, surveillance a grande echelle ou categorie speciale intentionnelle en V1.

## Decision

Statut initial:

- `A COMPLETER`

Question a trancher:

- les traitements V1 sont-ils susceptibles d'engendrer un risque eleve pour les droits et libertes des personnes, notamment du fait de la nature potentiellement sensible des contenus heberges et de l'exposition via liens publics ?

## Critieres a examiner

- nature des contenus potentiellement stockes;
- volume et echelle;
- accessibilite via liens publics;
- monitoring securite;
- categories particulieres de donnees si clients en hebergent;
- capacite reelle d'acces interne aux contenus;
- mesures de chiffrement et cloisonnement;
- incidents raisonnablement previsibles.

## Format de decision

| Date | Périmètre | Décision | Justification | Owner | Revue suivante |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 2026-05-11 | nvbes Drive V1 | **OUI (AIPD Requise)** | Bien que nvbes ne traite pas de catégories spéciales par défaut, la nature de "Drive" pour entreprises sensibles implique le stockage de secrets d'affaires, données d'identité et potentiellement données de santé/justice par les clients. Risque élevé en cas de violation de confidentialité. | CTO / DPO | 2026-11-11 |
