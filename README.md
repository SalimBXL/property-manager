# property-manager

Petit outil en Rust pour gérer un portefeuille de biens immobiliers (parkings, appartements, etc.) : achats, dépenses, locations et suivi des loyers, avec stockage local via SQLite.

## Fonctionnalités

- **Gestion des biens** : enregistrement des achats (prix, date, adresse, notes)
- **Suivi des dépenses** : taxes, réparations, charges — ponctuelles ou récurrentes
- **Gestion locative** : locataires, baux (actifs ou terminés), historique des loyers encaissés
- **Rentabilité** : calcul automatique loyers encaissés − dépenses, par bien ou pour tout le portefeuille
- **Détection des loyers impayés** : identifie les mois sans paiement enregistré pour chaque bail actif
- **Intégrité des données** : suppression d'un bien bloquée tant que des baux ou dépenses y sont rattachés

## Stack technique

- [Rust](https://www.rust-lang.org/) (édition 2021)
- [`rusqlite`](https://crates.io/crates/rusqlite) (feature `bundled`) — accès SQLite sans dépendance système
- [`chrono`](https://crates.io/crates/chrono) — gestion des dates
- [`thiserror`](https://crates.io/crates/thiserror) — erreurs applicatives typées

## Structure du projet

```
property-manager/
├── Cargo.toml
├── src/
│   ├── lib.rs              # point d'entrée de la librairie
│   ├── main.rs             # binaire (point d'entrée exécutable)
│   ├── error.rs            # type d'erreur applicatif (AppError / AppResult)
│   ├── db/
│   │   ├── mod.rs          # connexion SQLite, PRAGMA, ouverture de la base
│   │   ├── schema.rs       # définition des tables (migrations simples)
│   │   ├── repository.rs   # CRUD par table (Property, Expense, Tenant, Lease, RentPayment)
│   │   └── reporting.rs    # requêtes composites : rentabilité, loyers en retard
│   └── models/
│       ├── property.rs
│       ├── expense.rs
│       ├── tenant.rs
│       ├── lease.rs
│       └── rent_payment.rs
└── tests/
    ├── property_lifecycle.rs   # scénario complet : achat → bail → loyers → dépenses
    └── reporting.rs            # scénario multi-biens : rentabilité et retards de paiement
```

## Modèle de données

| Table          | Description                                                        |
|----------------|----------------------------------------------------------------------|
| `property`     | Un bien immobilier : libellé, adresse, date et prix d'achat         |
| `expense`      | Une dépense liée à un bien (taxe, réparation…), ponctuelle ou récurrente |
| `tenant`       | Un locataire                                                        |
| `lease`        | Un bail : bien + locataire + loyer mensuel + dates de début/fin     |
| `rent_payment` | Un paiement de loyer, rattaché à un bail et à une période (`YYYY-MM`) |

Tous les montants sont stockés en **centimes** (`INTEGER`), jamais en `REAL`, pour éviter les erreurs d'arrondi sur des données financières. Les dates sont stockées en `TEXT` au format ISO 8601 (`YYYY-MM-DD`), ce qui permet un tri lexicographique correct directement en SQL.

## Installation

Prérequis : [Rust](https://www.rust-lang.org/tools/install) (édition 2021 ou plus récente).

```bash
git clone <url-du-repo>
cd property-manager
cargo build
```

Aucune installation de SQLite n'est nécessaire : la feature `bundled` de `rusqlite` compile SQLite directement avec le projet.

## Utilisation

```bash
cargo run
```

Au premier lancement, un fichier `property_manager.db` est créé dans le répertoire courant et le schéma est initialisé automatiquement.

> Le projet est pour l'instant une base de données et une couche métier testée ; une interface en ligne de commande pour ajouter/consulter des biens, baux et paiements est à venir.

## Tests

```bash
cargo test
```

La suite de tests couvre :
- les opérations CRUD de chaque modèle (`tests` inline dans `db/repository.rs`)
- les calculs de rentabilité et de loyers en retard (`tests` inline dans `db/reporting.rs`)
- des scénarios de bout en bout avec plusieurs biens dans des situations variées (`tests/property_lifecycle.rs`, `tests/reporting.rs`)

## Choix de conception

- **`Option<i64>` pour les identifiants** : un `Property` construit en mémoire (`Property::new`) n'a pas encore d'`id` (`None`) ; un `Property` lu depuis la base en a toujours un (`Some`). Le type rend cet état impossible à confondre.
- **Séparation `repository.rs` / `reporting.rs`** : le repository ne fait que du CRUD table par table ; `reporting.rs` regroupe les requêtes qui combinent plusieurs tables ou contiennent de la logique métier (ex. calcul des mois de loyer manquants).
- **Erreurs typées (`AppError`)** : les erreurs SQLite génériques sont converties en erreurs métier explicites (`PropertyNotFound`, `PropertyHasDependents`, etc.) au niveau du repository, plutôt que de laisser fuiter les détails d'implémentation SQLite dans le reste de l'application.
- **Suppression protégée** : SQLite (avec `PRAGMA foreign_keys = ON`) refuse par défaut de supprimer un bien encore référencé par un bail ou une dépense ; cette contrainte est traduite en erreur `AppError::PropertyHasDependents` plutôt que de laisser remonter une erreur SQLite brute.

## Feuille de route

- [ ] Interface en ligne de commande (`clap`)
- [ ] Export CSV des rapports de rentabilité
- [ ] Migrations versionnées (`rusqlite_migration`)

## Licence

À définir.
